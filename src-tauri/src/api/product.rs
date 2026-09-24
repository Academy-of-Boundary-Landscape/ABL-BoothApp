//! 摊位商品：选品 + 进货。
//!
//! 把旧的 `products` 表 + `current_stock` 加减整个换成 `event_products` + 进货移动：
//! 库存余额是 `stock_movements` 的聚合（`domain::ledger`），不存在第二个可以漂移的数字。
//! 「多带了几本」是一次进货，「点了一下发现少了」是盘点（②-3），语义不同且都在账本里留痕。

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{query, query_as, query_scalar, FromRow, SqlitePool};

use crate::{
    api::guard::check_write_permission,
    domain::ledger::{
        onsite_balance, onsite_balances, post_journal, JournalKind, Location, StockLeg,
    },
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/events/{event_id}/products",
            get(list_event_products).post(add_product_to_event),
        )
        .route(
            "/events/{event_id}/products/{id}/restock",
            post(restock_product),
        )
        .route("/products/{id}", put(update_product).delete(delete_product))
}

// ==========================================
// 响应与行结构
// ==========================================

/// `event_products JOIN master_products JOIN societies`，按 id 查单个商品。
/// 列清单和 `EP_ROWS_BY_EVENT` 保持一致——`EventProductResponse` 的字段来自这两条 SQL。
const EP_ROW_BY_ID: &str = r#"
SELECT ep.id, ep.event_id, ep.master_product_id, ep.owner_society_id,
       s.name AS owner_society_name,
       ep.product_code, ep.name, ep.unit_price,
       mp.image_url, mp.category, mp.tags
FROM event_products ep
JOIN master_products mp ON mp.id = ep.master_product_id
JOIN societies s ON s.id = ep.owner_society_id
WHERE ep.id = ?
"#;

/// 同一 JOIN，按 event_id 查列表。
const EP_ROWS_BY_EVENT: &str = r#"
SELECT ep.id, ep.event_id, ep.master_product_id, ep.owner_society_id,
       s.name AS owner_society_name,
       ep.product_code, ep.name, ep.unit_price,
       mp.image_url, mp.category, mp.tags
FROM event_products ep
JOIN master_products mp ON mp.id = ep.master_product_id
JOIN societies s ON s.id = ep.owner_society_id
WHERE ep.event_id = ?
ORDER BY ep.product_code ASC
"#;

#[derive(Debug, FromRow)]
struct EventProductRow {
    id: i64,
    event_id: i64,
    master_product_id: i64,
    owner_society_id: i64,
    owner_society_name: String,
    product_code: String,
    name: String,
    unit_price: i64,
    image_url: Option<String>,
    category: Option<String>,
    tags: String,
}

/// 响应体。**没有 `current_stock` / `initial_stock`**：
/// `onsite_qty` 是聚合余额，`stocked_qty` 是累计进货（前端库存条的分母）。
#[derive(Debug, Serialize)]
struct EventProductResponse {
    id: i64,
    event_id: i64,
    master_product_id: i64,
    owner_society_id: i64,
    owner_society_name: String,
    product_code: String,
    name: String,
    unit_price: i64,
    stocked_qty: i64,
    onsite_qty: i64,
    image_url: Option<String>,
    category: Option<String>,
    tags: String,
}

impl EventProductResponse {
    fn from_row(row: EventProductRow, stocked_qty: i64, onsite_qty: i64) -> Self {
        EventProductResponse {
            id: row.id,
            event_id: row.event_id,
            master_product_id: row.master_product_id,
            owner_society_id: row.owner_society_id,
            owner_society_name: row.owner_society_name,
            product_code: row.product_code,
            name: row.name,
            unit_price: row.unit_price,
            stocked_qty,
            onsite_qty,
            image_url: row.image_url,
            category: row.category,
            tags: row.tags,
        }
    }
}

/// 单个商品的累计进货量（外部 → 现场仓 的总和）。
async fn stocked_qty(pool: &SqlitePool, event_product_id: i64) -> ApiResult<i64> {
    let qty: i64 = query_scalar(
        "SELECT COALESCE(SUM(qty), 0) FROM stock_movements
         WHERE event_product_id = ? AND from_location = '外部' AND to_location = '现场仓'",
    )
    .bind(event_product_id)
    .fetch_one(pool)
    .await?;
    Ok(qty)
}

/// 一场展会里全部商品的累计进货量（外部 → 现场仓）。列表页用，避免 N+1。
async fn stocked_balances(pool: &SqlitePool, event_id: i64) -> ApiResult<HashMap<i64, i64>> {
    let rows: Vec<(i64, i64)> = query_as(
        "SELECT sm.event_product_id, COALESCE(SUM(sm.qty), 0)
         FROM stock_movements sm
         JOIN event_products ep ON ep.id = sm.event_product_id
         WHERE ep.event_id = ? AND sm.from_location = '外部' AND sm.to_location = '现场仓'
         GROUP BY sm.event_product_id",
    )
    .bind(event_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// 组装单个商品：JOIN 行 + 累计进货 + 聚合余额。
/// `add` / `update` / `restock` 四个 handler 都返回同一个 `EventProductResponse`，
/// 所以抽成这一个函数，别四处各拼一遍（字段一多必然漏掉某个）。
async fn load_one(pool: &SqlitePool, id: i64) -> ApiResult<EventProductResponse> {
    let row = query_as::<_, EventProductRow>(EP_ROW_BY_ID)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("商品不存在".into()))?;

    let onsite_qty = onsite_balance(pool, id).await?;
    let stocked_qty = stocked_qty(pool, id).await?;
    Ok(EventProductResponse::from_row(row, stocked_qty, onsite_qty))
}

// ==========================================
// 1. 获取场次商品列表 (Public)
// ==========================================
async fn list_event_products(
    State(state): State<AppState>,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<Vec<EventProductResponse>>> {
    // 一次拿全部余额 / 累计进货，再和 JOIN 结果拼起来；不在循环里逐个查（N+1）。
    let onsite = onsite_balances(&state.db, event_id).await?;
    let stocked = stocked_balances(&state.db, event_id).await?;

    let rows: Vec<EventProductRow> = query_as::<_, EventProductRow>(EP_ROWS_BY_EVENT)
        .bind(event_id)
        .fetch_all(&state.db)
        .await?;

    let products = rows
        .into_iter()
        .map(|row| {
            let onsite_qty = onsite.get(&row.id).copied().unwrap_or(0);
            let stocked_qty = stocked.get(&row.id).copied().unwrap_or(0);
            EventProductResponse::from_row(row, stocked_qty, onsite_qty)
        })
        .collect();

    Ok(Json(products))
}

// ==========================================
// 2. 添加商品到场次 + 首批进货 (Admin/Vendor)
// ==========================================
#[derive(Deserialize)]
struct AddProductRequest {
    product_code: String,
    initial_stock: i64,
    /// 单位：分。缺省用 `master_products.default_price`。
    unit_price: Option<i64>,
}

async fn add_product_to_event(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<AddProductRequest>,
) -> ApiResult<impl IntoResponse> {
    check_write_permission(&claims, event_id)?;

    if payload.initial_stock < 0 {
        return Err(ApiError::BadRequest("进货数量不能为负".into()));
    }

    let event_exists: Option<i64> = query_scalar("SELECT id FROM events WHERE id = ?")
        .bind(event_id)
        .fetch_optional(&state.db)
        .await?;
    if event_exists.is_none() {
        return Err(ApiError::NotFound("Event not found".into()));
    }

    let master: Option<(i64, String, String, f64, i64)> = query_as(
        "SELECT id, product_code, name, default_price, owner_society_id
         FROM master_products WHERE product_code = ?",
    )
    .bind(&payload.product_code)
    .fetch_optional(&state.db)
    .await?;
    let (master_id, product_code, name, default_price, owner_society_id) = master
        .ok_or_else(|| ApiError::NotFound("Product code not found in master catalog".into()))?;

    // 预先查重，否则撞 (event_id, master_product_id) UNIQUE 约束会走 ApiError::Db → 500。
    let dup: Option<i64> =
        query_scalar("SELECT id FROM event_products WHERE event_id = ? AND master_product_id = ?")
            .bind(event_id)
            .bind(master_id)
            .fetch_optional(&state.db)
            .await?;
    if dup.is_some() {
        return Err(ApiError::Conflict("Product already in this event".into()));
    }

    // unit_price 缺省用 default_price。default_price 列还是 REAL（元），要换算成分：
    // 商品库的列没跟着改成整数分，因为 `.boothpack` 的跨版本兼容要靠它。
    let unit_price = match payload.unit_price {
        Some(p) => {
            if p < 0 {
                return Err(ApiError::BadRequest("单价不能为负".into()));
            }
            p
        }
        None => (default_price * 100.0).round() as i64,
    };

    // 建商品 + 首批进货必须在同一个事务里。initial_stock 为 0 时不记 journal
    // （post_journal 会拒绝空 journal）。
    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    crate::api::guard::require_event_open(&mut tx, event_id).await?;

    // owner_society_id 从 master_products 抄一份**快照**。之后改全局商品库的归属
    // 不影响已有展会的账——否则展会结算完之后有人改了归属，冻结的账就跟着变
    // （spec 3.1）。
    let new_id: i64 = query_scalar(
        "INSERT INTO event_products
           (event_id, master_product_id, owner_society_id, product_code, name, unit_price)
         VALUES (?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(event_id)
    .bind(master_id)
    .bind(owner_society_id)
    .bind(&product_code)
    .bind(&name)
    .bind(unit_price)
    .fetch_one(&mut *tx)
    .await?;

    if payload.initial_stock > 0 {
        post_journal(
            &mut tx,
            event_id,
            JournalKind::Restock,
            None,
            None,
            Some("开场进货"),
            &[StockLeg {
                event_product_id: new_id,
                from: Location::External,
                to: Location::OnSite,
                qty: payload.initial_stock,
            }],
            &[],
        )
        .await?;
    }

    tx.commit().await?;

    let product = load_one(&state.db, new_id).await?;
    Ok((StatusCode::CREATED, Json(product)))
}

// ==========================================
// 3. 补货 (Admin/Vendor)
// ==========================================
#[derive(Deserialize)]
struct RestockRequest {
    qty: i64,
    note: Option<String>,
}

async fn restock_product(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, id)): Path<(i64, i64)>,
    Json(payload): Json<RestockRequest>,
) -> ApiResult<Json<EventProductResponse>> {
    check_write_permission(&claims, event_id)?;

    if payload.qty <= 0 {
        return Err(ApiError::BadRequest("进货数量必须为正".into()));
    }

    // 确认商品存在且属于该场次，避免把别的展会的商品 id 混进本场账本。
    let exists: Option<i64> =
        query_scalar("SELECT id FROM event_products WHERE id = ? AND event_id = ?")
            .bind(id)
            .bind(event_id)
            .fetch_optional(&state.db)
            .await?;
    if exists.is_none() {
        return Err(ApiError::NotFound("商品不存在".into()));
    }

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    crate::api::guard::require_event_open(&mut tx, event_id).await?;
    post_journal(
        &mut tx,
        event_id,
        JournalKind::Restock,
        None,
        None,
        payload.note.as_deref(),
        &[StockLeg {
            event_product_id: id,
            from: Location::External,
            to: Location::OnSite,
            qty: payload.qty,
        }],
        &[],
    )
    .await?;
    tx.commit().await?;

    let product = load_one(&state.db, id).await?;
    Ok(Json(product))
}

// ==========================================
// 4. 改价 (Admin/Vendor)
// ==========================================
#[derive(Deserialize)]
struct UpdateProductRequest {
    /// 单位：分。
    ///
    /// **`initial_stock` 不再接受**：旧接口用 `current_stock += (new - old)` 硬调数字，
    /// 正是 spec 点名的三个互不共享写入方之一。请求体里即使带了 `initial_stock`，
    /// 也会被 serde 默认忽略——改库存必须走进货/盘点。
    unit_price: Option<i64>,
}

async fn update_product(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateProductRequest>,
) -> ApiResult<Json<EventProductResponse>> {
    // 先查 event_id 以便校验权限。
    let event_id: Option<i64> = query_scalar("SELECT event_id FROM event_products WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let event_id = event_id.ok_or_else(|| ApiError::NotFound("Product not found".into()))?;

    check_write_permission(&claims, event_id)?;

    // 已结算的展会账已冻结：改价写 event_products，守卫必须和写入在同一个
    // BEGIN IMMEDIATE 事务里——事务外查一遍再写，中间隔着一个能被 settle 插进来的窗口。
    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    crate::api::guard::require_event_open(&mut tx, event_id).await?;

    if let Some(unit_price) = payload.unit_price {
        if unit_price < 0 {
            return Err(ApiError::BadRequest("单价不能为负".into()));
        }
        query("UPDATE event_products SET unit_price = ? WHERE id = ?")
            .bind(unit_price)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    let product = load_one(&state.db, id).await?;
    Ok(Json(product))
}

// ==========================================
// 5. 删除商品 (Admin/Vendor)
// ==========================================
async fn delete_product(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> ApiResult<impl IntoResponse> {
    let event_id: Option<i64> = query_scalar("SELECT event_id FROM event_products WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let event_id = event_id.ok_or_else(|| ApiError::NotFound("Product not found".into()))?;

    check_write_permission(&claims, event_id)?;

    // 已结算的展会账已冻结：删除写 event_products（商品名单也是冻结的一部分）。
    // 守卫、流水检查和 DELETE 必须在同一个 BEGIN IMMEDIATE 事务里，事务外查会有窗口。
    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    crate::api::guard::require_event_open(&mut tx, event_id).await?;

    // 有移动流水时拒绝删除：删掉等于账面凭空少一批货。
    let moves: i64 =
        query_scalar("SELECT COUNT(*) FROM stock_movements WHERE event_product_id = ?")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if moves > 0 {
        return Err(ApiError::Conflict(
            "这个商品已经有进出记录，不能删除".into(),
        ));
    }

    query("DELETE FROM event_products WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok((
        StatusCode::OK,
        Json(json!({"message": "Product removed from event"})),
    ))
}

#[cfg(test)]
mod tests {
    use crate::domain::ledger::onsite_balance;
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::ServiceExt; // for oneshot

    #[tokio::test]
    async fn adding_a_product_records_a_restock_movement_not_a_stock_column() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        // 再加第三个商品（夹具里那两个已经建好了）
        sqlx::query(
            "INSERT INTO master_products (id, product_code, name, default_price, owner_society_id)
             VALUES (3, 'C', '挂件C', 15.0, 1)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let res = router
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/products"),
                Some(&token),
                json!({"product_code": "C", "initial_stock": 8, "unit_price": 1500}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let body = read_json(res).await;
        assert_eq!(body["onsite_qty"], 8);
        assert_eq!(body["stocked_qty"], 8);
        assert_eq!(body["unit_price"], 1500);
        // 新模型里根本不该再有这两个字段
        assert!(body.get("current_stock").is_none());
        assert!(body.get("initial_stock").is_none());
    }

    #[tokio::test]
    async fn restocking_adds_to_the_onsite_balance() {
        // spec 6.4：补货在「进行中」随时可录，不是开场专属
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;

        let res = router
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/products/{ep_a}/restock"),
                Some(&admin_token()),
                json!({"qty": 5, "note": "朋友顺路带来"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(read_json(res).await["onsite_qty"], 15);
        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 15);
    }

    #[tokio::test]
    async fn updating_a_product_cannot_set_stock_directly() {
        // 旧接口用 current_stock += (new - old) 硬调数字（而且不在事务里），
        // 是 spec 第 1 节点名的「三个互不共享的写入方」之一。这个入口必须消失。
        let (router, _dir, pool) = test_router_with().await;
        let (_event_id, ep_a, _) = seed_event_and_product(&pool).await;

        let res = router
            .oneshot(json_request(
                "PUT",
                &format!("/api/products/{ep_a}"),
                Some(&admin_token()),
                json!({"unit_price": 2500, "initial_stock": 99}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body = read_json(res).await;
        assert_eq!(body["unit_price"], 2500, "价格应该改了");
        assert_eq!(body["onsite_qty"], 10, "库存不能被直接改");
        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 10);
    }

    #[tokio::test]
    async fn deleting_a_product_with_movements_is_refused() {
        let (router, _dir, pool) = test_router_with().await;
        let (_event_id, ep_a, _) = seed_event_and_product(&pool).await;

        let res = router
            .oneshot(json_request(
                "DELETE",
                &format!("/api/products/{ep_a}"),
                Some(&admin_token()),
                json!({}),
            ))
            .await
            .unwrap();
        // 夹具已经录了一笔进货，删掉等于账面凭空少一批货
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    /// 三个商品写入口在冻结后都要挡住。分开三条而不是循环，
    /// 是因为它们各自的 URL 形状和 body 不同，合起来写会把断言糊掉。
    ///
    /// ⚠️ 请求体字段按 `api/product.rs` 现有的 `Deserialize` 结构体写：
    /// 选品用的是 `product_code`（不是 `master_product_id`），字段对不上会得到
    /// 422 而不是 409，测试就变成假绿。
    #[tokio::test]
    async fn a_settled_event_refuses_adding_products() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        // 用一个还没进场的全局商品，否则先撞上 (event_id, master_product_id) 的
        // 重复检查——那也会返回 409，测试就测不到守卫了。
        sqlx::query(
            "INSERT INTO master_products (id, product_code, name, default_price, owner_society_id)
             VALUES (3, 'C', '挂件C', 15.0, 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/products"),
                Some(&token),
                json!({"product_code": "C", "unit_price": 1500, "initial_stock": 5}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn a_settled_event_refuses_restock() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/products/{ep_a}/restock"),
                Some(&token),
                json!({"qty": 3}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn a_settled_event_refuses_product_edit() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/products/{ep_a}"),
                Some(&token),
                json!({"unit_price": 9999}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn a_settled_event_refuses_product_delete() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        // 夹具给 A / B 都记了开场带货，那样删的是「有移动流水」那一条 409。
        // 另插一个没有任何流水的商品，测到的才一定是冻结守卫。
        // 夹具的 master_products 只到 2，所以先补一个全局商品（(event_id,
        // master_product_id) 有 UNIQUE 约束，复用 1 / 2 会直接撞上）。
        sqlx::query(
            "INSERT INTO master_products (id, product_code, name, default_price, owner_society_id)
             VALUES (3, 'C', '挂件C', 15.0, 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let clean_id: i64 = sqlx::query_scalar(
            "INSERT INTO event_products
               (event_id, master_product_id, owner_society_id, product_code, name, unit_price)
             VALUES (?, 3, 1, 'C', '挂件C', 1500)
             RETURNING id",
        )
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "DELETE",
                &format!("/api/products/{clean_id}"),
                Some(&token),
                json!({}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT, "冻结后不能删商品");
    }
}
