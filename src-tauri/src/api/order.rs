//! 订单三条路径：下单 / 收款 / 取消，全部落到移动账本上。
//!
//! 旧模型里 `products.current_stock` 是唯一真相，下单扣、取消退、改价硬调三个写入方
//! 各写各的，数字会漂。新模型下「库存」是 `stock_movements` 的聚合，下单/取消都变成
//! 「插一条移动」。防超卖因此从「一条原子 UPDATE」变成「查余额 → 插移动」两步，
//! 必须在同一个 `BEGIN IMMEDIATE` 事务里：默认的 deferred 事务在第一次写之前不持
//! 写锁，两台平板同时抢最后一本会双双通过检查。

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, query_scalar, SqlitePool};
use std::collections::HashMap;

use crate::{
    db::models::OrderRow,
    domain::{
        ledger::{
            onsite_balance, post_journal, reverse_order_journals, Account, JournalKind, Location,
            MoneyLeg, StockLeg,
        },
        money::Money,
    },
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> Router<AppState> {
    Router::new()
        // 公开：顾客下单，无需 token
        .route("/events/:event_id/orders", post(create_order))
        // 管理员/摊主：查看订单列表
        .route("/events/:event_id/orders", get(list_orders))
        // 管理员/摊主：更新订单状态（完成 / 取消）
        .route(
            "/events/:event_id/orders/:order_id/status",
            put(update_order_status),
        )
}

// ==========================================
// 请求 / 响应结构
// ==========================================

#[derive(Deserialize)]
struct CreateOrderItemRequest {
    product_id: i64,
    quantity: i64,
}

#[derive(Deserialize)]
struct CreateOrderRequest {
    items: Vec<CreateOrderItemRequest>,
}

#[derive(Deserialize)]
struct UpdateStatusRequest {
    status: String,
    channel: Option<String>,
}

#[derive(Deserialize)]
struct ListOrdersQuery {
    status: Option<String>,
}

/// 订单响应：`{...order, items: [...]}`。
/// `order` 是 `OrderRow` 摊平后的字段，`created_at` 经 serde rename 成 `timestamp`
/// （前端读的是 timestamp，见 db/models.rs 的注释，别动）。
#[derive(Serialize)]
struct OrderResponse {
    #[serde(flatten)]
    order: OrderRow,
    items: Vec<OrderItemResponse>,
}

#[derive(Serialize)]
struct OrderItemResponse {
    id: i64,
    product_id: i64,
    quantity: i64,
    product_name: String,
    /// 单位：分（展示端除以 100 是前端 Task 8 的事）。
    product_price: i64,
    product_image_url: Option<String>,
    allocated_amount: i64,
    paid_amount: i64,
}

/// 下单时查摊位商品用的行：id / 单价 / 名字 / 图。
#[derive(sqlx::FromRow)]
struct ProductRow {
    id: i64,
    unit_price: i64,
    name: String,
    image_url: Option<String>,
}

/// `order_lines JOIN event_products JOIN master_products` 的查询行。
#[derive(sqlx::FromRow)]
struct OrderItemRow {
    id: i64,
    order_id: i64,
    event_product_id: i64,
    qty: i64,
    unit_price: i64,
    allocated_amount: i64,
    paid_amount: i64,
    product_name: String,
    product_image_url: Option<String>,
}

impl From<OrderItemRow> for OrderItemResponse {
    fn from(row: OrderItemRow) -> Self {
        OrderItemResponse {
            id: row.id,
            product_id: row.event_product_id,
            quantity: row.qty,
            product_name: row.product_name,
            product_price: row.unit_price,
            product_image_url: row.product_image_url,
            allocated_amount: row.allocated_amount,
            paid_amount: row.paid_amount,
        }
    }
}

const ITEMS_BY_ORDER: &str = "
SELECT ol.id, ol.order_id, ol.event_product_id, ol.qty, ol.unit_price,
       ol.allocated_amount, ol.paid_amount,
       ep.name AS product_name, mp.image_url AS product_image_url
FROM order_lines ol
JOIN event_products ep ON ep.id = ol.event_product_id
JOIN master_products mp ON mp.id = ep.master_product_id
WHERE ol.order_id = ?
ORDER BY ol.id
";

const ITEMS_BY_EVENT: &str = "
SELECT ol.id, ol.order_id, ol.event_product_id, ol.qty, ol.unit_price,
       ol.allocated_amount, ol.paid_amount,
       ep.name AS product_name, mp.image_url AS product_image_url
FROM order_lines ol
JOIN event_products ep ON ep.id = ol.event_product_id
JOIN master_products mp ON mp.id = ep.master_product_id
JOIN orders o ON o.id = ol.order_id
WHERE o.event_id = ?
ORDER BY ol.order_id, ol.id
";

/// 读回单个订单 + 它的行，组装成响应。`update_order_status` 用。
async fn load_order_response(pool: &SqlitePool, order_id: i64) -> ApiResult<OrderResponse> {
    let order: OrderRow = query_as("SELECT * FROM orders WHERE id = ?")
        .bind(order_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound("订单不存在".into()))?;

    let items: Vec<OrderItemResponse> = query_as::<_, OrderItemRow>(ITEMS_BY_ORDER)
        .bind(order_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(OrderResponse { order, items })
}

// ==========================================
// 1. 创建订单（公开，BEGIN IMMEDIATE）
// ==========================================
async fn create_order(
    State(state): State<AppState>,
    Path(event_id): Path<i64>,
    Json(payload): Json<CreateOrderRequest>,
) -> ApiResult<impl IntoResponse> {
    if payload.items.is_empty() {
        return Err(ApiError::BadRequest("订单至少需要一件商品".into()));
    }

    // 同一商品在 items 里出现多次必须先合并：否则两行各自过库存检查、合起来超卖。
    let mut merged: Vec<(i64, i64)> = Vec::new();
    for item in &payload.items {
        if item.quantity <= 0 {
            return Err(ApiError::BadRequest("数量必须为正".into()));
        }
        match merged.iter_mut().find(|(pid, _)| *pid == item.product_id) {
            Some((_, qty)) => *qty += item.quantity,
            None => merged.push((item.product_id, item.quantity)),
        }
    }

    // 只有「进行中」的展会能下单：已结算的展会账本已冻结，往里写销售 journal
    // 会污染一本本该冻结的账（Task 6 审查者列的 Important 敞口）。放在事务之前，
    // 省得白拿写锁。
    let event_status: Option<String> = query_scalar("SELECT status FROM events WHERE id = ?")
        .bind(event_id)
        .fetch_optional(&state.db)
        .await?;
    match event_status.as_deref() {
        Some("进行中") => {}
        Some(other) => {
            return Err(ApiError::Conflict(format!(
                "展会当前状态为「{other}」，仅「进行中」的展会可以下单"
            )));
        }
        None => return Err(ApiError::NotFound("展会不存在".into())),
    }

    // 防超卖的检查方式跟着模型变了：旧代码靠一条原子 UPDATE；新模型是
    // 「查余额 → 插移动」两步。SQLite 单写者模型下这仍然安全，前提是两步在
    // 同一个 BEGIN IMMEDIATE 事务里——默认的 deferred 事务在第一次写之前不持写锁。
    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;

    // resolved: (商品行, 数量, 该行金额)。行金额 = unit_price × qty，
    // ②-1 里 allocated = paid = unit_price × qty。
    let mut resolved: Vec<(ProductRow, i64, i64)> = Vec::with_capacity(merged.len());
    let mut stock_legs: Vec<StockLeg> = Vec::with_capacity(merged.len());
    let mut gross: i64 = 0;

    for (product_id, qty) in &merged {
        let row: ProductRow = query_as(
            "SELECT ep.id, ep.unit_price, ep.name, mp.image_url
             FROM event_products ep
             JOIN master_products mp ON mp.id = ep.master_product_id
             WHERE ep.id = ? AND ep.event_id = ?",
        )
        .bind(*product_id)
        .bind(event_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::NotFound("商品不存在或不属于本场展会".into()))?;

        let left = onsite_balance(&mut *tx, row.id).await?;
        if left < *qty {
            return Err(ApiError::Conflict(format!("「{}」库存不足", row.name)));
        }

        let line_total = row
            .unit_price
            .checked_mul(*qty)
            .ok_or_else(|| ApiError::BadRequest("金额溢出".into()))?;
        gross = gross
            .checked_add(line_total)
            .ok_or_else(|| ApiError::BadRequest("金额溢出".into()))?;

        stock_legs.push(StockLeg {
            event_product_id: row.id,
            from: Location::OnSite,
            to: Location::Customer,
            qty: *qty,
        });
        resolved.push((row, *qty, line_total));
    }

    // ②-1 没有 Lot 也没有折让：gross = solved = final。
    let solved = gross;
    let final_amount = solved;

    let order: OrderRow = query_as(
        "INSERT INTO orders (event_id, status, gross_amount, solved_amount, final_amount)
         VALUES (?, 'pending', ?, ?, ?) RETURNING *",
    )
    .bind(event_id)
    .bind(gross)
    .bind(solved)
    .bind(final_amount)
    .fetch_one(&mut *tx)
    .await?;
    let order_id = order.id;

    let mut items = Vec::with_capacity(resolved.len());
    for (row, qty, line_total) in &resolved {
        let line_id: i64 = query_scalar(
            "INSERT INTO order_lines
               (order_id, event_product_id, qty, unit_price, allocated_amount, paid_amount)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(order_id)
        .bind(row.id)
        .bind(*qty)
        .bind(row.unit_price)
        .bind(*line_total)
        .bind(*line_total)
        .fetch_one(&mut *tx)
        .await?;

        items.push(OrderItemResponse {
            id: line_id,
            product_id: row.id,
            quantity: *qty,
            product_name: row.name.clone(),
            product_price: row.unit_price,
            product_image_url: row.image_url.clone(),
            allocated_amount: *line_total,
            paid_amount: *line_total,
        });
    }

    // 销售 journal：现场仓 → 顾客仓。每张订单必然带这条 stock 腿——
    // 这是 Task 5 删除守卫「只查 stock_movements 不查 order_lines」仍然正确的前提。
    post_journal(
        &mut tx,
        event_id,
        JournalKind::Sale,
        Some(order_id),
        None,
        Some("下单"),
        &stock_legs,
        &[],
    )
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(OrderResponse { order, items })))
}

// ==========================================
// 2. 查看订单列表（管理员/摊主）
// ==========================================
async fn list_orders(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Query(params): Query<ListOrdersQuery>,
) -> ApiResult<Json<Vec<OrderResponse>>> {
    check_read_permission(&claims, event_id)?;

    let status = params.status.as_deref();

    let orders: Vec<OrderRow> = match status {
        Some(s) => {
            query_as("SELECT * FROM orders WHERE event_id = ? AND status = ? ORDER BY id DESC")
                .bind(event_id)
                .bind(s)
                .fetch_all(&state.db)
                .await?
        }
        None => {
            query_as("SELECT * FROM orders WHERE event_id = ? ORDER BY id DESC")
                .bind(event_id)
                .fetch_all(&state.db)
                .await?
        }
    };

    if orders.is_empty() {
        return Ok(Json(Vec::new()));
    }

    // 一次查出本场全部订单的行再内存分组，避免 N+1。用 JOIN orders 过滤，
    // 而不是拼 IN 列表（IN 的元素个数无法参数化）。行里若带本场别的订单，
    // 只会留在 map 里不被消费，无妨。
    let rows: Vec<OrderItemRow> = query_as(ITEMS_BY_EVENT)
        .bind(event_id)
        .fetch_all(&state.db)
        .await?;

    let mut items_map: HashMap<i64, Vec<OrderItemResponse>> = HashMap::new();
    for row in rows {
        let oid = row.order_id;
        items_map.entry(oid).or_default().push(row.into());
    }

    let result = orders
        .into_iter()
        .map(|order| {
            let oid = order.id;
            OrderResponse {
                order,
                items: items_map.remove(&oid).unwrap_or_default(),
            }
        })
        .collect();

    Ok(Json(result))
}

// ==========================================
// 3. 更新订单状态（完成 / 取消）
// ==========================================
async fn update_order_status(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, order_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateStatusRequest>,
) -> ApiResult<Json<OrderResponse>> {
    check_write_permission(&claims, event_id)?;

    let target = payload.status.as_str();
    if !matches!(target, "pending" | "completed" | "cancelled") {
        return Err(ApiError::BadRequest(format!("未知的订单状态: {target}")));
    }
    // 任何状态都不能退回 pending。
    if target == "pending" {
        return Err(ApiError::BadRequest("不能退回待处理".into()));
    }

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;

    let current: Option<String> =
        query_scalar("SELECT status FROM orders WHERE id = ? AND event_id = ?")
            .bind(order_id)
            .bind(event_id)
            .fetch_optional(&mut *tx)
            .await?;
    let current = current.ok_or_else(|| ApiError::NotFound("订单不存在".into()))?;

    match (current.as_str(), target) {
        // 已取消的订单不能再改状态。
        ("cancelled", _) => {
            return Err(ApiError::Conflict("已取消的订单不能再改状态".into()));
        }
        ("completed", "completed") => {
            return Err(ApiError::Conflict("订单已完成，不能重复完成".into()));
        }
        ("pending", "completed") => {
            // completed 必须带 channel：钱那条腿 `社团往来 → 实收-<渠道>` 需要对手账户。
            let channel = payload
                .channel
                .as_deref()
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(str::to_string)
                .ok_or_else(|| ApiError::BadRequest("完成订单必须提供收款渠道".into()))?;

            // 按货主分组，Σ 该货主各行的 allocated_amount。
            let owner_totals: Vec<(i64, i64)> = query_as(
                "SELECT ep.owner_society_id, SUM(ol.allocated_amount)
                 FROM order_lines ol
                 JOIN event_products ep ON ep.id = ol.event_product_id
                 WHERE ol.order_id = ?
                 GROUP BY ep.owner_society_id",
            )
            .bind(order_id)
            .fetch_all(&mut *tx)
            .await?;

            // ②-2 会在这里追加一条手工折让的腿（spec 4.4）：
            //     实收-<渠道> → 社团往来:<本社团>   金额 = solved_amount − final_amount
            // 它整笔落在本社团头上，不按货主分摊——「我帮你卖货，我自己让的价不该由你买单」。
            // 金额为 0 时那条腿本就不该存在，post_journal 已经会过滤掉零金额的腿。
            // ②-1 里 solved == final，所以这里恒为 0，暂不生成。
            let money_legs: Vec<MoneyLeg> = owner_totals
                .into_iter()
                .map(|(owner_id, cents)| MoneyLeg {
                    from: Account::SocietyDue(owner_id),
                    to: Account::Received(channel.clone()),
                    amount: Money::from_cents(cents),
                })
                .filter(|leg| !leg.amount.is_zero())
                .collect();

            // 全 0 金额（全赠品订单）时没有钱可记，跳过收款 journal 而不是
            // 让 post_journal 拒绝「空的 journal」。
            if !money_legs.is_empty() {
                post_journal(
                    &mut tx,
                    event_id,
                    JournalKind::Receipt,
                    Some(order_id),
                    None,
                    Some("收款"),
                    &[],
                    &money_legs,
                )
                .await?;
            }

            query(
                "UPDATE orders SET status = 'completed', channel = ?, completed_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(&channel)
            .bind(order_id)
            .execute(&mut *tx)
            .await?;
        }
        ("pending", "cancelled") | ("completed", "cancelled") => {
            // 同一个调用把货和钱两个 journal 一起冲掉（货回到现场仓、钱按反方向回滚）。
            // 不要自己遍历 journal 反向插移动——reverse_order_journals 已经处理了
            // 「跳过已被冲正的」和「重复冲正被偏唯一索引拦住」。
            reverse_order_journals(&mut tx, order_id, Some("取消订单")).await?;

            query("UPDATE orders SET status = 'cancelled' WHERE id = ?")
                .bind(order_id)
                .execute(&mut *tx)
                .await?;
        }
        _ => {
            return Err(ApiError::BadRequest("非法的状态转换".into()));
        }
    }

    tx.commit().await?;

    let response = load_order_response(&state.db, order_id).await?;
    Ok(Json(response))
}

// ==========================================
// 权限检查辅助函数
// ==========================================
fn check_read_permission(claims: &Claims, event_id: i64) -> Result<(), ApiError> {
    if claims.role == "admin" {
        return Ok(());
    }
    if claims.role == "vendor" {
        if claims.access == "all" {
            return Ok(());
        }
        if let Some(eid) = claims.event_id {
            if eid == event_id {
                return Ok(());
            }
        }
    }
    Err(ApiError::Forbidden)
}

fn check_write_permission(claims: &Claims, event_id: i64) -> Result<(), ApiError> {
    // 读写权限目前一致
    check_read_permission(claims, event_id)
}

#[cfg(test)]
mod tests {
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn placing_an_order_moves_goods_out_of_the_onsite_warehouse_immediately() {
        // spec 6.1：下单即移动，天然防超卖——货在下单那一刻就离开现场仓了
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _ep_b) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None, // 下单是公开接口，顾客端没有 token
                json!({"items": [{"product_id": ep_a, "quantity": 2}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = read_json(res).await;
        assert_eq!(body["final_amount"], 6000, "2 × 3000 分");
        assert_eq!(body["status"], "pending");
        assert!(
            body["timestamp"].is_string(),
            "created_at 必须以 timestamp 的名字出现在响应里——前端两处依赖它，改名会静默白屏"
        );

        let left = crate::domain::ledger::onsite_balance(&pool, ep_a)
            .await
            .unwrap();
        assert_eq!(left, 8, "下单即扣，不等到收款");
    }

    #[tokio::test]
    async fn overselling_is_refused_and_leaves_no_trace() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 999}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);

        // 事务必须整体回滚：不能留下半张订单或一条孤儿 journal
        let orders: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders")
            .fetch_one(&pool)
            .await
            .unwrap();
        let journals: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(orders, 0);
        assert_eq!(journals, 1, "只该有 seed 时那一条进货 journal");
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_a)
                .await
                .unwrap(),
            10
        );
    }

    #[tokio::test]
    async fn completing_an_order_requires_a_channel_and_splits_money_by_owner() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        // ep_a 归本社团(1) 3000，ep_b 归代卖社团(2) 2000
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [
                    {"product_id": ep_a, "quantity": 2},
                    {"product_id": ep_b, "quantity": 1}
                ]}),
            ))
            .await
            .unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        // 不带渠道必须被拒——否则钱那条腿没有对手账户（spec 6.2）
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "微信"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        use crate::domain::ledger::{account_balance, Account};
        use crate::domain::money::Money;
        // 我欠本社团 6000、欠代卖社团 2000，手上多了 8000
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1))
                .await
                .unwrap(),
            Money::from_cents(-6000)
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(-2000)
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into()))
                .await
                .unwrap(),
            Money::from_cents(8000)
        );
    }

    #[tokio::test]
    async fn cancelling_a_completed_order_reverses_both_goods_and_money() {
        // 旧模型只退库存、不碰钱（那时也没有钱的账）。新模型下只回滚一半
        // 正是最该防住的事故。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 2}]}),
            ))
            .await
            .unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "现金"}),
            ))
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "cancelled"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        use crate::domain::ledger::{account_balance, onsite_balance, Account};
        use crate::domain::money::Money;
        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 10, "货回来了");
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1))
                .await
                .unwrap(),
            Money::ZERO,
            "钱也回去了"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into()))
                .await
                .unwrap(),
            Money::ZERO
        );
    }

    #[tokio::test]
    async fn cancelling_twice_does_not_refund_the_stock_twice() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 2}]}),
            ))
            .await
            .unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        let uri = format!("/api/events/{event_id}/orders/{order_id}/status");
        let r1 = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &uri,
                Some(&token),
                json!({"status":"cancelled"}),
            ))
            .await
            .unwrap();
        assert_eq!(r1.status(), StatusCode::OK);
        let r2 = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &uri,
                Some(&token),
                json!({"status":"cancelled"}),
            ))
            .await
            .unwrap();
        assert_eq!(r2.status(), StatusCode::CONFLICT, "已取消的订单不能再取消");

        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_a)
                .await
                .unwrap(),
            10,
            "库存只能退一次"
        );
    }

    #[tokio::test]
    async fn the_same_product_listed_twice_is_merged_before_the_stock_check() {
        // 同一个请求里同一商品出现多次，必须先合并再查库存。
        // 不合并的话两行各自通过检查（6 <= 10），合起来却卖出 12 件。
        // 这是同请求内唯一的防超卖机制，而它此前零测试覆盖。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [
                    {"product_id": ep_a, "quantity": 6},
                    {"product_id": ep_a, "quantity": 6}
                ]}),
            ))
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            StatusCode::CONFLICT,
            "6 + 6 = 12 超过库存 10，必须被拒"
        );
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_a)
                .await
                .unwrap(),
            10,
            "被拒的订单不能动库存"
        );

        // 合并后仍在库存内的情况要正常通过，且只扣一次合计量
        let res = router
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [
                    {"product_id": ep_a, "quantity": 3},
                    {"product_id": ep_a, "quantity": 4}
                ]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = read_json(res).await;
        assert_eq!(body["final_amount"], 21000, "7 件 × 3000 分");
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_a)
                .await
                .unwrap(),
            3,
            "10 − 7"
        );
    }

    #[tokio::test]
    async fn an_all_gift_order_completes_without_a_receipt_journal() {
        // spec 4.6：赠品是订单上的 0 元行。一张全是赠品的订单，各货主合计都是 0，
        // 所有资金腿都被滤掉 —— 此时必须**跳过整个收款 journal**，而不是让
        // post_journal 因为「空 journal」把「完成订单」整个打回 400。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        sqlx::query("UPDATE event_products SET unit_price = 0 WHERE id = ?")
            .bind(ep_b)
            .execute(&pool)
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_b, "quantity": 2}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "微信"}),
            ))
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            StatusCode::OK,
            "全赠品订单必须能完成，不能被空 journal 检查打回"
        );

        // 没有钱易手，就不该有任何资金移动——记 0 元收款才是造假
        let money_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM money_movements mm
             JOIN journals j ON j.id = mm.journal_id WHERE j.order_id = ?",
        )
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(money_rows, 0, "0 元订单不该产生任何资金移动");

        // 货照样要动
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_b)
                .await
                .unwrap(),
            3,
            "5 − 2"
        );

        // 取消这样的订单也必须干净：只有销售 journal 可冲，货要回来
        let res = router
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "cancelled"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_b)
                .await
                .unwrap(),
            5,
            "货必须回来"
        );
    }

    #[tokio::test]
    async fn creating_an_order_on_a_settled_event_is_refused() {
        // Task 7 转来的守卫：冻结（已结算）的账本不能再写销售 journal。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;

        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();

        let res = router
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);

        let body = read_json(res).await;
        let msg = body["error"].as_str().unwrap();
        assert!(msg.contains("已结算"), "错误消息必须说明当前状态: {msg}");

        // 被拒的订单不能留下任何订单或 journal
        let orders: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(orders, 0);
    }

    #[tokio::test]
    async fn creating_an_order_on_a_preparing_event_is_refused() {
        // 守卫的 `Some(other)` 分支不该只兜住「已结算」：展会还没开场（筹备）时
        // 同样不能下单——货还没录进现场仓，写销售 journal 一样是污染冻结前的账。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;

        sqlx::query("UPDATE events SET status = '筹备' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();

        let res = router
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);

        let body = read_json(res).await;
        let msg = body["error"].as_str().unwrap();
        assert!(msg.contains("筹备"), "错误消息必须说明当前状态: {msg}");

        let orders: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(orders, 0);
    }

    #[tokio::test]
    async fn begin_immediate_actually_takes_the_write_lock_up_front() {
        // brief 整篇在强调「查余额 → 插移动」两步必须在同一个 BEGIN IMMEDIATE 事务里：
        // 默认的 deferred 事务在第一次写之前不持写锁，两台平板会双双通过库存检查。
        // 但这条断言在 test_pool()（max_connections(1) 的内存库）里根本测不了。
        //
        // 这里用文件库 + 两条连接 + busy_timeout(0) 直接验行为本身：
        // IMMEDIATE 事务一开就持写锁 → 第二个 IMMEDIATE 立刻 SQLITE_BUSY；
        // 换成普通 begin() 则第二个会成功，这条测试就会红。
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use std::str::FromStr;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lock_probe.db");
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
            .unwrap()
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(0));
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(opts)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE probe (id INTEGER PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();

        let first = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
        let second = pool.begin_with("BEGIN IMMEDIATE").await;
        assert!(
            second.is_err(),
            "第一个 IMMEDIATE 事务必须立刻持有写锁，第二个应当 SQLITE_BUSY"
        );
        drop(first);
    }
}
