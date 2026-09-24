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
        app_data_dir: dir.path().to_path_buf(),
        jwt_secret: TEST_JWT_SECRET.to_string(),
        vision_runtime,
    };

    (state, dir)
}

/// 挂在 /api 下的完整路由，已 with_state。
pub async fn test_router() -> (Router, TempDir) {
    let (state, dir) = test_state().await;
    let router = Router::new()
        .nest("/api", crate::api::router().split_for_parts().0)
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

/// 下一张单并返回 `order_id`。
///
/// `api/order.rs` 和 `api/stats.rs` 各抄过一份逐字相同的副本，放这里共用。
pub async fn place(router: &Router, event_id: i64, items: serde_json::Value) -> i64 {
    use tower::ServiceExt;
    let res = router
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/api/events/{event_id}/orders"),
            None,
            serde_json::json!({ "items": items }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::CREATED);
    read_json(res).await["id"].as_i64().unwrap()
}

/// 同 `test_router()`，但把 pool 也还回来。
///
/// Task 5 / Task 6 的测试要直接查账本余额做断言——拿不到 pool 就只能靠 HTTP 反推，
/// 断言力弱很多（「响应里写着 8」和「账本聚合出来确实是 8」不是一回事）。
pub async fn test_router_with() -> (Router, TempDir, SqlitePool) {
    let (state, dir) = test_state().await;
    let pool = state.db.clone();
    let router = Router::new()
        .nest("/api", crate::api::router().split_for_parts().0)
        .with_state(state);
    (router, dir, pool)
}

/// 种一场「进行中」的展会 + 两个商品，各带 10 / 5 件进货，返回
/// `(event_id, event_product_a, event_product_b)`。
///
/// - A 归**本社团**（id 1），单价 3000 分
/// - B 归**代卖社团「黄昏堂」**（id 2），单价 2000 分
///
/// 混货主是刻意的：按货主拆分收款是新模型的核心行为之一，单货主的夹具测不出来。
/// 直接写 SQL 而不是打 API，是因为建全局商品走的是 multipart 接口，
/// 构造成本高且不是这些测试的被测对象。
pub async fn seed_event_and_product(pool: &SqlitePool) -> (i64, i64, i64) {
    sqlx::query("INSERT INTO societies (id, name, is_home) VALUES (2, '黄昏堂', 0)")
        .execute(pool)
        .await
        .expect("seed society");
    sqlx::query(
        "INSERT INTO master_products (id, product_code, name, default_price, owner_society_id)
         VALUES (1, 'A', '本子A', 30.0, 1), (2, 'B', '本子B', 20.0, 2)",
    )
    .execute(pool)
    .await
    .expect("seed master products");
    sqlx::query(
        "INSERT INTO events (id, name, event_date, status)
         VALUES (1, 'ABC漫展', '2026-10-01', '进行中')",
    )
    .execute(pool)
    .await
    .expect("seed event");
    sqlx::query(
        "INSERT INTO event_products
           (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
         VALUES (1, 1, 1, 1, 'A', '本子A', 3000),
                (2, 1, 2, 2, 'B', '本子B', 2000)",
    )
    .execute(pool)
    .await
    .expect("seed event products");

    // 开场带货：A 10 件、B 5 件。走账本而不是直接塞 stock_movements，
    // 这样夹具本身也在守 post_journal 的行为。
    let mut tx = pool.begin().await.expect("begin");
    crate::domain::ledger::post_journal(
        &mut tx,
        1,
        crate::domain::ledger::JournalKind::Restock,
        None,
        None,
        Some("夹具：开场带货"),
        &[
            crate::domain::ledger::StockLeg {
                event_product_id: 1,
                from: crate::domain::ledger::Location::External,
                to: crate::domain::ledger::Location::OnSite,
                qty: 10,
            },
            crate::domain::ledger::StockLeg {
                event_product_id: 2,
                from: crate::domain::ledger::Location::External,
                to: crate::domain::ledger::Location::OnSite,
                qty: 5,
            },
        ],
        &[],
    )
    .await
    .expect("seed restock");
    tx.commit().await.expect("commit");

    (1, 1, 2)
}

/// 给展会加一个 Lot，返回 lot_id。`candidates` 是 `event_product_id` 列表。
///
/// 默认 `allow_repeat = false`（每种最多 1 件）——这是模型默认语义。
/// 需要「同款能拿多件」的测试用 `seed_lot_repeat`。
///
/// 直接写 SQL 而不是打 API：Lot 的 CRUD 是 Task 4 才有的东西，而 Task 3 的
/// 测试现在就要用它。
pub async fn seed_lot(
    pool: &SqlitePool,
    event_id: i64,
    name: &str,
    pick_count: i64,
    total_price: i64,
    candidates: &[i64],
) -> i64 {
    seed_lot_with_repeat(
        pool,
        event_id,
        name,
        pick_count,
        total_price,
        candidates,
        false,
    )
    .await
}

/// 同 `seed_lot`，但允许同一个候选在一个套装实例里算多件。
pub async fn seed_lot_repeat(
    pool: &SqlitePool,
    event_id: i64,
    name: &str,
    pick_count: i64,
    total_price: i64,
    candidates: &[i64],
) -> i64 {
    seed_lot_with_repeat(
        pool,
        event_id,
        name,
        pick_count,
        total_price,
        candidates,
        true,
    )
    .await
}

async fn seed_lot_with_repeat(
    pool: &SqlitePool,
    event_id: i64,
    name: &str,
    pick_count: i64,
    total_price: i64,
    candidates: &[i64],
    allow_repeat: bool,
) -> i64 {
    let lot_id: i64 = sqlx::query_scalar(
        "INSERT INTO lots (event_id, name, pick_count, total_price, allow_repeat)
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(event_id)
    .bind(name)
    .bind(pick_count)
    .bind(total_price)
    .bind(allow_repeat as i64)
    .fetch_one(pool)
    .await
    .expect("seed lot");
    for c in candidates {
        sqlx::query("INSERT INTO lot_candidates (lot_id, event_product_id) VALUES (?, ?)")
            .bind(lot_id)
            .bind(c)
            .execute(pool)
            .await
            .expect("seed lot candidate");
    }
    lot_id
}
