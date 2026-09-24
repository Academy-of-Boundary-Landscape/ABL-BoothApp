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
    // spec 8：v1 → v2 是知情清零，老库必须永久留一份。
    // 和 ① 的 premigrate 快照不是一回事——那些按 KEEP=3 轮转会被后续迁移挤掉
    // （见路线图附录第 3 条），这一份不参与轮转、永不删除。
    if db_existed {
        backup_v1_once(&pool, &db_path).await;
    }
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

    // 已应用迁移的校验和若只是换行符不同，先修正过来（见 reconcile_line_ending_checksums）。
    // 放在快照之后：万一修正出了岔子，下面的失败路径照样会还原。
    let fixed = match reconcile_line_ending_checksums(pool, migrator).await {
        Ok(n) => n,
        Err(e) => {
            eprintln!("[Booth Tool] WARNING: checking migration checksums failed: {e}");
            0
        }
    };
    if fixed > 0 {
        println!("[Booth Tool] normalized line-ending checksums of {fixed} applied migration(s)");
    }

    // 运行迁移 (使用运行时方式避免编译时需要 DATABASE_URL)
    let run = migrator.run(pool).await;
    if let Err(e) = run {
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

/// 把「只差换行符」的已应用迁移校验和改成当前构建的，返回修正了几条。
///
/// sqlx 用迁移 SQL 文本的 SHA-384 做校验和，存进用户库的 `_sqlx_migrations`，启动时逐条比对，
/// 不一致就 `VersionMismatch` 拒绝启动。而文本取决于**构建机上 git 签出的换行符**：
/// v1.1.1 在 Windows 上构建，前 3 个迁移签出成了 CRLF、后 2 个是 LF（从线上 APK 里的
/// 校验和逐个核对过）；v1.2 在 Linux 上构建，全是 LF。不修正的话，所有老用户第一次打开
/// v1.2 都会闪退（2026-09-25 在真 Windows 上实测）。
///
/// 只放过换行符差异：库里存的校验和必须恰好等于「同一份 SQL 的 LF 版或 CRLF 版」的校验和。
/// 内容真改过的迁移仍然 `VersionMismatch`——那种情况必须暴露出来（见 docs/BUILD.md「发布前必查」）。
/// 仓库里的 `.gitattributes` 已禁止对迁移文件做换行转换，以后在哪个系统上构建都一样。
async fn reconcile_line_ending_checksums(
    pool: &SqlitePool,
    migrator: &sqlx::migrate::Migrator,
) -> Result<usize, sqlx::Error> {
    use sqlx::SqlSafeStr;

    let has_table: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_optional(pool)
    .await?;
    if has_table.is_none() {
        return Ok(0);
    }

    let applied: Vec<(i64, Vec<u8>)> =
        sqlx::query_as("SELECT version, checksum FROM _sqlx_migrations")
            .fetch_all(pool)
            .await?;

    let variant_checksum = |m: &sqlx::migrate::Migration, sql: String| {
        sqlx::migrate::Migration::new(
            m.version,
            m.description.clone(),
            m.migration_type,
            sqlx::AssertSqlSafe(sql).into_sql_str(),
            m.no_tx,
        )
        .checksum
        .into_owned()
    };

    let mut fixed = 0;
    for (version, stored) in applied {
        let Some(m) = migrator
            .iter()
            .find(|m| m.version == version && !m.migration_type.is_down_migration())
        else {
            continue;
        };
        if stored.as_slice() == m.checksum.as_ref() {
            continue;
        }
        let lf = m.sql.as_str().replace("\r\n", "\n");
        let crlf = lf.replace('\n', "\r\n");
        if stored == variant_checksum(m, lf) || stored == variant_checksum(m, crlf) {
            sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?")
                .bind(m.checksum.as_ref())
                .bind(version)
                .execute(pool)
                .await?;
            fixed += 1;
        }
    }
    Ok(fixed)
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

/// v1 库的一次性永久备份。只在「确实是 v1 库」且「备份还不存在」时落一份。
///
/// 判据是 `products` 表还在 —— 那是 v1 独有、v2 迁移会删掉的表。用它而不是版本号，
/// 是因为 `_sqlx_migrations` 在损坏库上读不出来，而这种库恰恰最需要备份。
/// 失败不阻断启动，但要吼出来。
async fn backup_v1_once(pool: &SqlitePool, db_path: &Path) {
    let dest = db_path.with_extension("db.v1-backup");

    // 只看 exists() 不够：见下面 v1_backup_is_readable 的说明。
    if dest.exists() {
        if v1_backup_is_readable(&dest).await {
            return;
        }
        eprintln!(
            "[Booth Tool] WARNING: existing v1 backup at {} is unreadable; taking a fresh one",
            dest.display()
        );
    }

    let is_v1: Result<Option<String>, sqlx::Error> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'products'",
    )
    .fetch_optional(pool)
    .await;

    match is_v1 {
        Ok(Some(_)) => {}   // 确认是 v1 库，继续备份
        Ok(None) => return, // 不是 v1 库（全新安装或已迁移过），正常跳过
        Err(e) => {
            // 连 sqlite_master 都读不出来，说明库可能已损坏——而这正是最需要备份的情况。
            // 但既然读不出来，VACUUM INTO 多半也会失败，所以只吼一声，不阻断启动。
            eprintln!("[Booth Tool] WARNING: cannot determine v1 schema ({e}); skipping v1 backup");
            return;
        }
    }

    // VACUUM INTO 而不是 fs::copy：库跑在 WAL 模式下，直接拷 .db 会漏掉
    // -wal 中尚未 checkpoint 的数据。理由同 db/snapshot.rs。
    //
    // 先写到临时名，成功后再 rename。同一文件系统上 rename 是原子的，所以 dest
    // 要么不存在、要么是一份完整备份，不会出现「半份」。
    // VACUUM INTO 中途失败会留下损坏的目标文件，而 SQLite 不会自己清理它。
    let tmp = db_path.with_extension("db.v1-backup.partial");
    let _ = std::fs::remove_file(&tmp); // 清掉上次失败留下的残骸

    let escaped = tmp.to_string_lossy().replace('\'', "''");
    match sqlx::query(sqlx::AssertSqlSafe(format!("VACUUM INTO '{escaped}'")))
        .execute(pool)
        .await
    {
        Ok(_) => match std::fs::rename(&tmp, &dest) {
            Ok(()) => println!("[Booth Tool] v1 database preserved at {}", dest.display()),
            Err(e) => {
                eprintln!("[Booth Tool] WARNING: v1 backup rename failed: {e}");
                let _ = std::fs::remove_file(&tmp);
            }
        },
        Err(e) => {
            eprintln!("[Booth Tool] WARNING: v1 backup failed: {e}");
            let _ = std::fs::remove_file(&tmp);
        }
    }
}

/// 已有的 v1 备份是否真的能打开、能查到 v1 的特征表。
///
/// 只判断文件存在是不够的：`VACUUM INTO` 中途失败（磁盘满、断电、进程被杀）会留下
/// 一个**存在但损坏**的文件，而备份是一次性动作——不校验的话，一次失败就会让
/// 「已备份」这个标记永久钉在一个打不开的空壳上。
///
/// 以只读方式打开（`mode=ro`），绝不碰用户这份数据。
async fn v1_backup_is_readable(path: &Path) -> bool {
    let url = format!("sqlite://{}?mode=ro", path.display());
    let Ok(pool) = SqlitePool::connect(&url).await else {
        return false;
    };
    let ok = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM products")
        .fetch_one(&pool)
        .await
        .is_ok();
    pool.close().await;
    ok
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

        // 第二批：在第一个基础上追加一条「先破坏数据、再失败」的迁移。
        //
        // 关键是 `-- no-transaction`（sqlx 从 sql 内容首行识别的指令，见
        // sqlx-core-0.9.0/src/migrate/source.rs 的 `sql.starts_with("-- no-transaction")`）：
        // 没有它，整条迁移脚本会被包在一个事务里跑，DELETE 会随着后面的报错一起回滚，
        // 探针数据照样活着——那样这条测试就算把 restore() 整个删掉也会通过，没有判别力。
        // 加了它之后 DELETE 单独提交（sqlite 默认逐语句自动提交），后一条语句再报错，
        // 「探针数据还在」就只能由 restore() 解释。
        std::fs::write(
            migrations_dir.join("2_boom.sql"),
            "-- no-transaction\nDELETE FROM probe;\nSELECT no_such_column_here FROM probe;",
        )
        .unwrap();
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

    /// F1 的核心行为：`_sqlx_migrations` 表读不到时必须 fail-safe（判不出就快照），
    /// 而不是像原来的 `has_pending` 那样 fail-open（判不出就当作不用快照）。
    /// 顺带用「已是最新」这一步守住反方向——不该快照的时候也别多落。
    #[tokio::test]
    async fn should_snapshot_before_migrate_is_fail_safe_when_tracking_table_missing() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        let pool = empty_wal_db(&db_path).await;

        // 用项目真实的迁移集把库跑到最新版本。
        let migrator = sqlx::migrate!();
        migrator.run(&pool).await.unwrap();

        // 已是最新：不该判定需要快照。
        assert!(
            !should_snapshot_before_migrate(&pool, &migrator).await,
            "库已是最新版本时不该判定需要快照"
        );

        // 迁移追踪表本身没了（库损坏 / 被手工改过 / 从旧备份还原回来）：判不出，必须 fail-safe。
        sqlx::query("DROP TABLE _sqlx_migrations")
            .execute(&pool)
            .await
            .unwrap();

        assert!(
            should_snapshot_before_migrate(&pool, &migrator).await,
            "迁移追踪表缺失时必须 fail-safe，判定需要快照"
        );
    }

    /// spec 8 的知情清零要求老库永久留存。这条守的是「只对 v1 库落、只落一次」。
    #[tokio::test]
    async fn backup_v1_once_preserves_old_db_exactly_once() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        let pool = empty_wal_db(&db_path).await;

        // 造一个 v1 特征：products 表存在
        sqlx::query("CREATE TABLE products (id INTEGER PRIMARY KEY, marker TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO products (marker) VALUES ('v1-data')")
            .execute(&pool)
            .await
            .unwrap();

        let dest = db_path.with_extension("db.v1-backup");
        backup_v1_once(&pool, &db_path).await;
        assert!(dest.exists(), "v1 库应被备份");

        // 备份内容必须读得到（VACUUM INTO 的事务一致性，fs::copy 在 WAL 下会漏）
        let snap = SqlitePool::connect(&format!("sqlite://{}", dest.display()))
            .await
            .unwrap();
        let marker: String = sqlx::query_scalar("SELECT marker FROM products")
            .fetch_one(&snap)
            .await
            .unwrap();
        assert_eq!(marker, "v1-data");
        snap.close().await;

        // 第二次调用必须是 no-op：不能把用户已经看过的备份覆盖掉
        let before = std::fs::metadata(&dest).unwrap().modified().unwrap();
        backup_v1_once(&pool, &db_path).await;
        let after = std::fs::metadata(&dest).unwrap().modified().unwrap();
        assert_eq!(before, after, "备份已存在时必须 no-op");
    }

    /// 全新安装（没有 products 表）不该产生 v1 备份文件。
    #[tokio::test]
    async fn backup_v1_once_skips_fresh_v2_database() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        let pool = empty_wal_db(&db_path).await;
        sqlx::migrate!().run(&pool).await.unwrap();

        backup_v1_once(&pool, &db_path).await;
        assert!(
            !db_path.with_extension("db.v1-backup").exists(),
            "v2 库不该产生 v1 备份"
        );
    }

    /// Critical 回归守卫：上一次 VACUUM INTO 中途失败留下的损坏文件，
    /// 不能让「已备份」这个标记永久钉死。
    #[tokio::test]
    async fn backup_v1_once_replaces_a_corrupt_previous_backup() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        let pool = empty_wal_db(&db_path).await;
        sqlx::query("CREATE TABLE products (id INTEGER PRIMARY KEY, marker TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO products (marker) VALUES ('v1-data')")
            .execute(&pool)
            .await
            .unwrap();

        // 造一个「存在但不是数据库」的残骸，模拟上次写到一半就挂了
        let dest = db_path.with_extension("db.v1-backup");
        std::fs::write(&dest, b"this is not a sqlite database").unwrap();

        backup_v1_once(&pool, &db_path).await;

        // 必须被重新备份成一份真能读的库
        let snap = SqlitePool::connect(&format!("sqlite://{}", dest.display()))
            .await
            .unwrap();
        let marker: String = sqlx::query_scalar("SELECT marker FROM products")
            .fetch_one(&snap)
            .await
            .unwrap();
        assert_eq!(marker, "v1-data", "损坏的旧备份必须被重新生成");
        snap.close().await;

        // 不能留下 .partial 残骸
        assert!(
            !db_path.with_extension("db.v1-backup.partial").exists(),
            "临时文件必须被清理"
        );
    }

    /// brief 的硬约束：备份必须在迁移**之前**跑。迁移会把 products 表删掉，
    /// 所以「备份里查得到 v1 数据」本身就是顺序正确的证据——有人日后把调用挪到
    /// 迁移之后，这条会立刻红。
    ///
    /// 注：products 必须带上 `event_id`，否则第一条老迁移里的
    /// `CREATE INDEX idx_products_event_id ON products(event_id)` 会因缺列而失败；
    /// 这里造的是「v1 库的业务表 + 数据」，老迁移的建表语句全部 `IF NOT EXISTS`，
    /// 会照常补齐其余表。
    #[tokio::test]
    async fn init_db_backs_up_v1_before_migrating_it_away() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");

        // 先造一个 v1 库：products 表存在并有数据（这正是 v1 判据）
        {
            let pool = empty_wal_db(&db_path).await;
            sqlx::query(
                "CREATE TABLE products (id INTEGER PRIMARY KEY, event_id INTEGER, marker TEXT)",
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query("INSERT INTO products (marker) VALUES ('legacy')")
                .execute(&pool)
                .await
                .unwrap();
            pool.close().await;
        }

        let pool = init_db(&dir.path().to_path_buf()).await.unwrap();
        pool.close().await;

        let dest = db_path.with_extension("db.v1-backup");
        assert!(dest.exists(), "init_db 必须落 v1 备份");

        let snap = SqlitePool::connect(&format!("sqlite://{}", dest.display()))
            .await
            .unwrap();
        let marker: String = sqlx::query_scalar("SELECT marker FROM products")
            .fetch_one(&snap)
            .await
            .unwrap();
        assert_eq!(
            marker, "legacy",
            "备份里必须有 v1 数据——查不到就说明备份跑在迁移之后了"
        );
        snap.close().await;
    }

    /// 用真实的 v1.1.1 结构（前 5 个迁移）和一批像用过的数据造一个老库，再走一遍真实启动的 init_db。
    ///
    /// 以前的升级测试只手工建了一张假的 products 表——「带数据的真实 v1.1.1 库 → 升级」从来没被测过，
    /// 而这正是每个老用户第一次打开 v1.2 时走的路。
    #[tokio::test]
    async fn init_db_upgrades_a_real_v1_1_1_database_with_data() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        {
            let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
                .unwrap()
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .pragma("foreign_keys", "ON");
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(opts)
                .await
                .unwrap();
            // v1.1.1 发布时的迁移集合：前 5 个
            let full = sqlx::migrate!();
            let mut v1 = sqlx::migrate::Migrator::DEFAULT;
            v1.migrations =
                std::borrow::Cow::Owned(full.migrations.iter().take(5).cloned().collect());
            v1.run(&pool).await.expect("v1.1.1 schema");
            for sql in [
                "INSERT INTO settings (key, value) VALUES ('admin_password', 'x'), ('vendor_password', 'y')",
                "INSERT INTO master_products (id, product_code, name, default_price, image_url, category, is_active, tags)
                 VALUES (1, 'A01', '本子A', 30.0, '/static/uploads/products/a.jpg', '本子', 1, '新刊,R18'),
                        (2, 'B02', '亚克力', 19.9, NULL, NULL, 1, ''),
                        (3, 'DISC', '优惠/抹零', -5.0, NULL, NULL, 1, ''),
                        (4, 'OLD', '停用商品', 0.0, NULL, NULL, 0, '')",
                "INSERT INTO events (id, name, event_date, location, status, vendor_password, payment_qr_code_path)
                 VALUES (1, 'CP30', '2026-03-01', '上海', '已结束', 'hash', '/static/uploads/qr/1.png'),
                        (2, 'COMICUP', '2026-05-01', NULL, '进行中', NULL, NULL),
                        (3, '未来展', '2026-12-01', NULL, '未进行', NULL, NULL)",
                "INSERT INTO products (id, event_id, master_product_id, product_code, name, price, initial_stock, current_stock)
                 VALUES (1, 1, 1, 'A01', '本子A', 30.0, 50, 12), (2, 1, 3, 'DISC', '优惠/抹零', -5.0, 999, 990),
                        (3, 2, 2, 'B02', '亚克力', 19.9, 20, 20)",
                "INSERT INTO orders (id, event_id, total_amount, status) VALUES (1, 1, 55.0, 'completed'), (2, 1, 30.0, 'cancelled'), (3, 2, 19.9, 'pending')",
                "INSERT INTO order_items (order_id, product_id, product_name, product_price, quantity)
                 VALUES (1, 1, '本子A', 30.0, 2), (1, 2, '优惠/抹零', -5.0, 1), (2, 1, '本子A', 30.0, 1), (3, 3, '亚克力', 19.9, 1)",
                "INSERT INTO master_product_images (id, master_product_id, image_url, kind) VALUES (1, 1, '/static/uploads/products/a.jpg', 'primary')",
            ] {
                sqlx::query(sql).execute(&pool).await.unwrap_or_else(|e| panic!("seed v1.1.1: {e}\n{sql}"));
            }
            pool.close().await;
        }

        let pool = init_db(&dir.path().to_path_buf())
            .await
            .expect("v1.1.1 带数据的老库必须能升级——失败就是老用户第一次打开 v1.2 时的闪退");

        // 保留的东西还在：密码、商品库（含停用和负价商品）、识图资产
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM master_products")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 4);
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM master_product_images")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 1);
        pool.close().await;
        assert!(db_path.with_extension("db.v1-backup").exists());
    }

    /// 用给定的迁移集合造一个老库（WAL + 外键，与生产一致），返回库所在目录。
    async fn old_db_with(migrations: Vec<sqlx::migrate::Migration>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
            .unwrap()
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .pragma("foreign_keys", "ON");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        let mut m = sqlx::migrate::Migrator::DEFAULT;
        m.migrations = std::borrow::Cow::Owned(migrations);
        m.run(&pool).await.expect("build old db");
        sqlx::query("INSERT INTO master_products (product_code, name, default_price, tags) VALUES ('A', '本子', 30.0, '')")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
        dir
    }

    /// 同一个迁移，换一份 SQL 文本（校验和随之重算）。
    fn with_sql(m: &sqlx::migrate::Migration, sql: String) -> sqlx::migrate::Migration {
        use sqlx::SqlSafeStr;
        sqlx::migrate::Migration::new(
            m.version,
            m.description.clone(),
            m.migration_type,
            sqlx::AssertSqlSafe(sql).into_sql_str(),
            m.no_tx,
        )
    }

    /// v1.1.1 是在 Windows 上构建的：前 3 个迁移文件签出成了 CRLF，后 2 个是 LF
    /// （从线上 v1.1.1 APK 里的 SHA-384 逐个核对过）。老用户库里记的就是这组混合校验和。
    /// 在 Linux 上构建的 v1.2 全是 LF——不处理的话，每个老用户第一次打开都会
    /// VersionMismatch(202601020001) 然后闪退（2026-09-25 真 Windows 上实测）。
    #[tokio::test]
    async fn init_db_accepts_old_databases_whose_migrations_differ_only_in_line_endings() {
        let full = sqlx::migrate!();
        let v111: Vec<_> = full
            .migrations
            .iter()
            .take(5)
            .enumerate()
            .map(|(i, m)| {
                if i < 3 {
                    with_sql(
                        m,
                        m.sql.as_str().replace("\r\n", "\n").replace('\n', "\r\n"),
                    )
                } else {
                    m.clone()
                }
            })
            .collect();
        assert_ne!(
            v111[0].checksum, full.migrations[0].checksum,
            "前提：CRLF 版本的校验和确实不同"
        );
        let dir = old_db_with(v111).await;

        let pool = init_db(&dir.path().to_path_buf())
            .await
            .expect("只差换行符的老库必须能升级");
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM master_products")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 1);
        pool.close().await;

        // 修正是持久的：第二次启动不需要再修
        let pool = init_db(&dir.path().to_path_buf())
            .await
            .expect("second start");
        pool.close().await;
    }

    /// 放宽只针对换行符：内容真的被改过的已发布迁移，照样拒绝启动（不能悄悄接受）。
    #[tokio::test]
    async fn init_db_still_rejects_a_migration_whose_content_really_changed() {
        let full = sqlx::migrate!();
        let tampered: Vec<_> = full
            .migrations
            .iter()
            .take(5)
            .enumerate()
            .map(|(i, m)| {
                if i == 0 {
                    with_sql(m, format!("{}\n-- 改了一行\n", m.sql.as_str()))
                } else {
                    m.clone()
                }
            })
            .collect();
        let dir = old_db_with(tampered).await;
        let err = init_db(&dir.path().to_path_buf()).await.unwrap_err();
        assert!(
            matches!(err, sqlx::Error::Migrate(ref e) if matches!(**e, sqlx::migrate::MigrateError::VersionMismatch(202601020001))),
            "{err:?}"
        );
    }
}
