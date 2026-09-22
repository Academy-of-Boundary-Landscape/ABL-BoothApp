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

/// `test_state()` 注入 AppState 的 JWT 密钥。造 token 的 helper 必须用同一个值，
/// 所以抽成常量而不是各处抄字面量。
pub const TEST_JWT_SECRET: &str = "test-secret";

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
        jwt_secret: TEST_JWT_SECRET.to_string(),
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

use crate::utils::security::create_jwt;
use axum::body::Body;
use axum::http::Request;

/// 管理员 token（`role = "admin"`，全局访问）。
pub fn admin_token() -> String {
    create_jwt("admin", "all", None, TEST_JWT_SECRET).unwrap_or_else(|_| panic!("sign admin jwt"))
}

/// 摊主 token，限定在某一场展会上。
///
/// 注意 `role`/`access` 的组合语义（见 `api/order.rs` 的 `check_read_permission`）：
/// `vendor` + `all` 是全局摊主，`vendor` + `event` 才受 `event_id` 限制。
pub fn vendor_token(event_id: i64) -> String {
    create_jwt("vendor", "event", Some(event_id), TEST_JWT_SECRET)
        .unwrap_or_else(|_| panic!("sign vendor jwt"))
}

/// 构造一个带 JSON body 的请求。`token` 为 None 时不加 Authorization 头。
///
/// 既有的两个测试是手抄 `Request::builder()` 的；新模型下每个 task 都要发好几个
/// 请求，抄六七遍 builder 只会让 diff 难读。
pub fn json_request(
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: serde_json::Value,
) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(t) = token {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    b.body(Body::from(body.to_string())).expect("build request")
}

/// 把响应 body 读成 JSON。axum 0.7 的 `to_bytes` 必须带 limit 参数。
pub async fn read_json(res: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("read body");
    if bytes.is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::from_slice(&bytes).expect("parse json body")
}

/// 同 `test_router()`，但把 pool 也还回来。
///
/// Task 5 / Task 6 的测试要直接查账本余额做断言——拿不到 pool 就只能靠 HTTP 反推，
/// 断言力弱很多（「响应里写着 8」和「账本聚合出来确实是 8」不是一回事）。
pub async fn test_router_with() -> (Router, TempDir, SqlitePool) {
    let (state, dir) = test_state().await;
    let pool = state.db.clone();
    let router = Router::new()
        .nest("/api", crate::api::router())
        .with_state(state);
    (router, dir, pool)
}
