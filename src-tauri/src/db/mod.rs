pub mod models;

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
    if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
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

    // 5. 运行迁移 (使用运行时方式避免编译时需要 DATABASE_URL)
    sqlx::migrate!().run(&pool).await?;

    // ================== 新增逻辑 ==================
    // 6. 检查并初始化默认管理员密码
    // 假设默认密码是 "admin123" (生产环境应强制用户首次登录修改，这里为了演示简化)
    let admin_exists: (i64,) =
        sqlx::query_as("SELECT count(*) FROM settings WHERE key = 'admin_password'")
            .fetch_one(&pool)
            .await?;

    if admin_exists.0 == 0 {
        let password_hash = hash_password("admin123");
        sqlx::query("INSERT INTO settings (key, value) VALUES ('admin_password', ?)")
            .bind(password_hash)
            .execute(&pool)
            .await?;
        println!("Initialized default admin password.");
    }

    // 7. 检查并初始化默认摊主密码
    // 假设默认密码是 "vendor123"
    let vendor_exists: (i64,) =
        sqlx::query_as("SELECT count(*) FROM settings WHERE key = 'vendor_password'")
            .fetch_one(&pool)
            .await?;

    if vendor_exists.0 == 0 {
        let password_hash = hash_password("vendor123");
        sqlx::query("INSERT INTO settings (key, value) VALUES ('vendor_password', ?)")
            .bind(password_hash)
            .execute(&pool)
            .await?;
        println!("Initialized default vendor password.");
    }

    Ok(pool)
}

/// 完全重置数据库：删除所有数据并重新初始化
/// 警告：这是一个危险操作，会清空所有数据！
pub async fn reset_database(app_data_dir: &PathBuf) -> Result<SqlitePool, sqlx::Error> {
    use sqlx::migrate::MigrateDatabase;

    println!("[WARNING] Resetting database - all data will be lost!");

    // 1. 拼接数据库文件路径
    let db_path = app_data_dir.join("sale_system.db");
    let db_url = format!("sqlite://{}", db_path.to_string_lossy());

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
