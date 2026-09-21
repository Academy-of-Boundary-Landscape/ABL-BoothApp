pub mod models;
pub mod snapshot;

use crate::utils::security::hash_password;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Sqlite, SqlitePool,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub async fn init_db(app_data_dir: &PathBuf) -> Result<SqlitePool, sqlx::Error> {
    use sqlx::migrate::MigrateDatabase;

    // 1. 确保数据库文件路径存在
    if !app_data_dir.exists() {
        fs::create_dir_all(app_data_dir).expect("Failed to create app data dir");
    }

    // 2. 拼接数据库文件路径: /.../data/sale_system.db
    let db_path = app_data_dir.join("sale_system.db");
    let db_url = format!("sqlite://{}", db_path.to_string_lossy());

    // 3. 如果文件不存在，创建它
    let db_existed = Sqlite::database_exists(&db_url).await.unwrap_or(false);
    if !db_existed {
        Sqlite::create_database(&db_url).await?;
    }

    // 4. 连接（通过 ConnectOptions 让每条连接都自动设置 PRAGMA）
    let connect_options = SqliteConnectOptions::from_str(&db_url)?
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5))
        .pragma("foreign_keys", "ON");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    // 5+6. 迁移前快照（视情况）+ 运行迁移，失败就还原。
    let migrator = sqlx::migrate!();
    migrate_with_snapshot(&pool, &db_path, db_existed, &migrator).await?;

    // 7. 检查并初始化默认管理员/摊主密码
    seed_defaults(&pool).await?;

    Ok(pool)
}

/// 迁移前视情况落快照，然后跑迁移；迁移失败就从快照还原并把错误传回去。
///
/// 从 `init_db` 抽出来是为了能在测试里注入一个运行时构造的 `Migrator`（`sqlx::migrate!()`
/// 是编译期宏，塞不进故意写坏的迁移），行为与抽取前完全一致。
async fn migrate_with_snapshot(
    pool: &SqlitePool,
    db_path: &Path,
    db_existed: bool,
    migrator: &sqlx::migrate::Migrator,
) -> Result<(), sqlx::Error> {
    // 只在「库已存在 且 确有待跑迁移（或判不出）」时落，全新安装不落。
    let snap = if db_existed && should_snapshot_before_migrate(pool, migrator).await {
        match snapshot::take(pool, db_path, "premigrate").await {
            Ok(p) => {
                println!("[Booth Tool] pre-migration snapshot: {}", p.display());
                Some(p)
            }
            Err(e) => {
                // 快照失败不阻断启动，但要吼出来
                eprintln!("[Booth Tool] WARNING: pre-migration snapshot failed: {e}");
                None
            }
        }
    } else {
        None
    };

    // 运行迁移 (使用运行时方式避免编译时需要 DATABASE_URL)
    if let Err(e) = migrator.run(pool).await {
        if let Some(snap) = snap {
            eprintln!("[Booth Tool] migration failed ({e}); restoring snapshot");
            pool.close().await;
            if let Err(re) = snapshot::restore(db_path, &snap) {
                eprintln!("[Booth Tool] FATAL: restore also failed: {re}");
            }
        }
        return Err(e.into());
    }

    Ok(())
}

/// 迁移前是否需要落快照。**fail-safe**：判断不出「是否有 pending 迁移」时，宁可多落
/// 一份快照（顶多多费点磁盘），也不能悄悄跳过——一个迁移追踪表读不到的库（损坏、被
/// 手工改过、从旧备份还原回来的）恰恰是最需要这张安全网的时候。
///
/// | 情况 | 结果 |
/// |---|---|
/// | `_sqlx_migrations` 表缺失 / 查询报错（如库损坏） | 快照（判不出，按危险处理） |
/// | 表存在但没有已应用记录（`MAX(version)` 为 NULL） | 快照（判不出） |
/// | 能读到已应用的最大版本，且本地已知最新版本 > 它 | 快照（确有 pending） |
/// | 能读到已应用的最大版本，且已经 >= 本地已知最新版本 | 不快照（已是最新） |
///
/// 调用方还会再 AND 上「库文件是否已存在」——全新安装那种情况不会走到这里。
async fn should_snapshot_before_migrate(
    pool: &SqlitePool,
    migrator: &sqlx::migrate::Migrator,
) -> bool {
    // fetch_optional 的外层 Option 表示「有没有这一行」，内层 Option 是列值是否为
    // NULL —— MAX(version) 在空表上仍然会返回恰好一行、值是 NULL。
    let query_result: Result<Option<Option<i64>>, sqlx::Error> =
        sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations")
            .fetch_optional(pool)
            .await;

    let applied: i64 = match query_result {
        // 唯一能确定「不用快照」的前提：查到了一个具体的已应用版本号。
        Ok(Some(Some(v))) => v,
        // 表存在但是空的（不该发生——初次迁移会先建表再插行，但防御性地按「判不出」处理）。
        Ok(Some(None)) => return true,
        // fetch_optional 返回了 None（理论上 MAX 聚合查询总会有一行；同样按「判不出」处理）。
        Ok(None) => return true,
        // 查询报错，典型原因是 `_sqlx_migrations` 表本身不存在（库损坏 / 被手工改过）。
        Err(_) => return true,
    };

    match migrator.iter().map(|m| m.version).max() {
        Some(latest) => latest > applied,
        // migrator 里一条迁移都没有，理论上不会发生；同样按「判不出」处理。
        None => true,
    }
}

/// 种入默认的管理员 / 摊主密码。已存在则跳过。
/// 从 init_db 拆出来，让测试夹具能用同一份逻辑建库，避免两处漂移。
pub async fn seed_defaults(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let admin_exists: (i64,) =
        sqlx::query_as("SELECT count(*) FROM settings WHERE key = 'admin_password'")
            .fetch_one(pool)
            .await?;
    if admin_exists.0 == 0 {
        sqlx::query("INSERT INTO settings (key, value) VALUES ('admin_password', ?)")
            .bind(hash_password("admin123"))
            .execute(pool)
            .await?;
    }

    let vendor_exists: (i64,) =
        sqlx::query_as("SELECT count(*) FROM settings WHERE key = 'vendor_password'")
            .fetch_one(pool)
            .await?;
    if vendor_exists.0 == 0 {
        sqlx::query("INSERT INTO settings (key, value) VALUES ('vendor_password', ?)")
            .bind(hash_password("vendor123"))
            .execute(pool)
            .await?;
    }

    Ok(())
}

/// 完全重置数据库：删除所有数据并重新初始化
/// 警告：这是一个危险操作，会清空所有数据！
///
/// 目前没有调用方——api/admin.rs 的 `/reset-database` 路由走的是另一条「原地 DELETE +
/// 重建默认数据」的路径，没有用这个「删库文件重新 migrate」的版本。留着是因为两种重置
/// 语义不完全等价（这个版本连 schema 迁移都会重跑），后续如果要做「出厂重置」之类更彻底
/// 的功能可能用得上。
#[allow(dead_code)]
pub async fn reset_database(app_data_dir: &PathBuf) -> Result<SqlitePool, sqlx::Error> {
    use sqlx::migrate::MigrateDatabase;

    println!("[WARNING] Resetting database - all data will be lost!");

    // 1. 拼接数据库文件路径
    let db_path = app_data_dir.join("sale_system.db");
    let db_url = format!("sqlite://{}", db_path.to_string_lossy());

    // 删库之前先落一份快照。这是全应用最危险的操作，没有第二次机会。
    if Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        match SqlitePool::connect(&db_url).await {
            Ok(p) => {
                match snapshot::take(&p, &db_path, "prereset").await {
                    Ok(s) => println!("[Booth Tool] pre-reset snapshot: {}", s.display()),
                    Err(e) => eprintln!("[Booth Tool] WARNING: pre-reset snapshot failed: {e}"),
                }
                p.close().await;
            }
            Err(e) => eprintln!("[Booth Tool] WARNING: cannot open db for pre-reset snapshot: {e}"),
        }
    }

    // 2. 删除现有数据库文件
    if Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        Sqlite::drop_database(&db_url).await?;
        println!("[INFO] Existing database dropped.");
    }

    // 3. 删除物理文件（以防万一）
    if db_path.exists() {
        fs::remove_file(&db_path).ok();
    }

    // 4. 重新初始化数据库
    println!("[INFO] Reinitializing database...");
    init_db(app_data_dir).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::migrate::Migrator;

    /// 建一个 WAL 模式的空文件库，不跑任何迁移——迁移由测试里单独构造的 `Migrator` 控制。
    async fn empty_wal_db(db_path: &Path) -> SqlitePool {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
            .unwrap()
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        SqlitePool::connect_with(opts).await.unwrap()
    }

    /// F2：端到端验证「快照落下 → 迁移炸了 → pool 关闭 → 从快照还原 → 返回错误」这整条链路，
    /// 而不只是 snapshot::restore() 自身的单元测试。
    ///
    /// `sqlx::migrate!()` 是编译期宏，没法往里塞一条故意写坏的迁移；`Migrator::new(路径)`
    /// 是运行时从目录读的（本机 sqlx 0.9 签名：
    /// `pub async fn new<'s, S: MigrationSource<'s>>(source: S) -> Result<Self, MigrateError>`，
    /// `&Path` 实现了 `MigrationSource`），所以测试自己现造一个只有两个文件的迁移目录。
    #[tokio::test]
    async fn migrate_with_snapshot_restores_on_failed_migration() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        let pool = empty_wal_db(&db_path).await;

        let migrations_dir = dir.path().join("migrations");
        std::fs::create_dir_all(&migrations_dir).unwrap();

        // 第一批：只有一个能成功跑的迁移，建一张探针表。
        std::fs::write(
            migrations_dir.join("1_create_probe.sql"),
            "CREATE TABLE probe (id INTEGER PRIMARY KEY, note TEXT NOT NULL);",
        )
        .unwrap();

        let migrator_v1 = Migrator::new(migrations_dir.as_path()).await.unwrap();
        migrate_with_snapshot(&pool, &db_path, false, &migrator_v1)
            .await
            .unwrap();

        sqlx::query("INSERT INTO probe (note) VALUES ('still here')")
            .execute(&pool)
            .await
            .unwrap();

        // 第二批：在第一个基础上追加一条必炸的迁移（语法错误，任何 sqlite 版本都会报错）。
        std::fs::write(migrations_dir.join("2_boom.sql"), "THIS IS NOT VALID SQL;").unwrap();
        let migrator_v2 = Migrator::new(migrations_dir.as_path()).await.unwrap();

        let result = migrate_with_snapshot(&pool, &db_path, true, &migrator_v2).await;
        assert!(
            result.is_err(),
            "坏迁移必须让 migrate_with_snapshot 返回 Err"
        );

        // migrate_with_snapshot 在回滚路径里关闭了 pool，重新连接来验证落盘的数据。
        let reopened = SqlitePool::connect(&format!("sqlite://{}", db_path.display()))
            .await
            .unwrap();
        let note: String = sqlx::query_scalar("SELECT note FROM probe WHERE id = 1")
            .fetch_one(&reopened)
            .await
            .unwrap();
        assert_eq!(note, "still here", "还原后探针数据应该还在");

        // 迁移前落的那份快照文件应该还留在目录里。
        let has_snapshot = std::fs::read_dir(dir.path()).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".bak-premigrate-")
        });
        assert!(has_snapshot, "应该留下迁移前快照文件");
    }
}
