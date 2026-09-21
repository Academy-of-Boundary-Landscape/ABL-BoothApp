pub mod models;
pub mod snapshot;

use crate::utils::security::hash_password;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Sqlite, SqlitePool,
};
use std::fs;
use std::path::PathBuf;
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

    // 5. 迁移前快照 —— 只在「库已存在 且 确有待跑迁移」时落，全新安装不落。
    let migrator = sqlx::migrate!();
    let snap = if db_existed && has_pending(&pool, &migrator).await {
        match snapshot::take(&pool, &db_path, "premigrate").await {
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

    // 6. 运行迁移 (使用运行时方式避免编译时需要 DATABASE_URL)
    if let Err(e) = migrator.run(&pool).await {
        if let Some(snap) = snap {
            eprintln!("[Booth Tool] migration failed ({e}); restoring snapshot");
            pool.close().await;
            if let Err(re) = snapshot::restore(&db_path, &snap) {
                eprintln!("[Booth Tool] FATAL: restore also failed: {re}");
            }
        }
        return Err(e.into());
    }

    // 7. 检查并初始化默认管理员/摊主密码
    seed_defaults(&pool).await?;

    Ok(pool)
}

/// 是否有尚未应用的迁移。表不存在（全新库）时返回 false——那种情况不需要快照。
async fn has_pending(pool: &SqlitePool, migrator: &sqlx::migrate::Migrator) -> bool {
    let applied: Option<i64> = sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .flatten();
    match (applied, migrator.iter().map(|m| m.version).max()) {
        (Some(a), Some(latest)) => latest > a,
        _ => false,
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
