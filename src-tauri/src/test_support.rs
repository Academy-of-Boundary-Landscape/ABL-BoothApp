//! 测试夹具。整个模块只在 cfg(test) 下编译。
//!
//! 业务逻辑跑在内嵌 axum server 上而不是 tauri IPC，所以 handler 可以脱离
//! Tauri runtime 测试：内存 SQLite + tower::ServiceExt::oneshot 就够，不需要开窗口。

use crate::state::AppState;
use crate::vision::VisionRuntime;
use axum::Router;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use std::sync::Arc;
use tempfile::TempDir;

/// 内存库，跑完迁移并种好默认密码。
///
/// 可行的前提是项目用的是运行时 `sqlx::migrate!()` 而不是编译期 `query!` 宏，
/// 所以不需要 DATABASE_URL。
///
/// `max_connections(1)`：SQLite 的 `:memory:` 默认每条连接一个独立数据库，
/// 池子一旦开出第二条连接就会出现「迁移在连接 A 上建表、查询走到连接 B 发现
/// 表不存在」的诡异失败。测试场景下单连接足够，也让所有查询天然串行、结果
/// 可预期。
pub async fn test_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite");
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("run migrations on in-memory db");
    crate::db::seed_defaults(&pool)
        .await
        .expect("seed default passwords");
    pool
}

/// AppState + 它依赖的临时目录。
///
/// **TempDir 必须由调用方持有到测试结束**：它一旦 drop，目录就被删掉了。
pub async fn test_state() -> (AppState, TempDir) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let upload_dir = dir.path().join("uploads");
    std::fs::create_dir_all(&upload_dir).expect("create upload dir");

    let pool = test_pool().await;

    // VisionRuntime::new 不触碰 ONNX 运行时（ort 用 load-dynamic，只在真正建
    // session 时才 dlopen），所以这里不需要任何模型文件。
    let vision_runtime = Arc::new(VisionRuntime::new(
        dir.path().to_path_buf(),
        upload_dir.clone(),
        pool.clone(),
    ));

    let state = AppState {
        db: pool,
        upload_dir,
        jwt_secret: "test-secret".to_string(),
        vision_runtime,
    };

    (state, dir)
}

/// 挂在 /api 下的完整路由，已 with_state。
pub async fn test_router() -> (Router, TempDir) {
    let (state, dir) = test_state().await;
    let router = Router::new()
        .nest("/api", crate::api::router())
        .with_state(state);
    (router, dir)
}
