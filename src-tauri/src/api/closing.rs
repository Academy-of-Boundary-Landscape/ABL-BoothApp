//! 收摊向导：清 pending → 盘点 → 带回 → 转已结算（母 spec 6.4）。
//!
//! **四步之间没有会话状态。** 每一步的成果都是已落库的 journal，向导只是按
//! `GET /closing` 的返回决定给你看哪一屏。退出、换设备、重进，都从当前真实
//! 状态继续。
//!
//! **能不能进下一步由后端说了算**（`blockers`），前端不自己判断。理由和 ②-2
//! 把试算放后端一样：判据写在两处就会有一处先腐烂。

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::guard::{check_read_permission, check_write_permission, require_event_open},
    domain::ledger::{post_journal, JournalKind, Location, StockLeg},
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events/:event_id/closing", get(get_closing))
        .route("/events/:event_id/closing/stocktake", post(stocktake))
        .route("/events/:event_id/closing/takeback", post(takeback))
        .route("/events/:event_id/closing/settle", post(settle))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct PendingOrderRow {
    id: i64,
    created_at: String,
    final_amount: i64,
    item_count: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct OnSiteRow {
    event_product_id: i64,
    product_code: String,
    name: String,
    owner_society_id: i64,
    owner_name: String,
    qty: i64,
}

#[derive(Serialize)]
pub struct ClosingState {
    status: String,
    pending_orders: Vec<PendingOrderRow>,
    onsite_remaining: Vec<OnSiteRow>,
    stocktaken_at: Option<String>,
    blockers: Vec<String>,
}

async fn get_closing(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<ClosingState>> {
    check_read_permission(&claims, event_id)?;

    let row: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT status, stocktaken_at FROM events WHERE id = ?")
            .bind(event_id)
            .fetch_optional(&state.db)
            .await?;
    let (status, stocktaken_at) = row.ok_or_else(|| ApiError::NotFound("展会不存在".into()))?;

    let pending_orders: Vec<PendingOrderRow> = sqlx::query_as(
        "SELECT o.id, o.created_at, o.final_amount,
                (SELECT COALESCE(SUM(qty), 0) FROM order_lines WHERE order_id = o.id) AS item_count
         FROM orders o
         WHERE o.event_id = ? AND o.status = 'pending'
         ORDER BY o.id",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    let onsite_remaining = onsite_rows(&state, event_id).await?;

    let mut blockers = Vec::new();
    if status == "已结算" {
        blockers.push("展会已结算，账本已冻结".into());
    }
    if !pending_orders.is_empty() {
        blockers.push(format!(
            "还有 {} 单待处理，逐单完成或取消之后才能盘点",
            pending_orders.len()
        ));
    }
    if !onsite_remaining.is_empty() {
        blockers.push(format!(
            "还有 {} 种商品在现场仓，带回之后才能结算",
            onsite_remaining.len()
        ));
    }

    Ok(Json(ClosingState {
        status,
        pending_orders,
        onsite_remaining,
        stocktaken_at,
        blockers,
    }))
}

/// 现场仓余额非 0 的商品。**从流水聚合，没有第二个可以漂移的数字。**
async fn onsite_rows(state: &AppState, event_id: i64) -> ApiResult<Vec<OnSiteRow>> {
    let rows: Vec<OnSiteRow> = sqlx::query_as(
        "SELECT ep.id AS event_product_id, ep.product_code, ep.name,
                ep.owner_society_id, s.name AS owner_name,
                COALESCE(SUM(CASE WHEN sm.to_location   = '现场仓' THEN sm.qty ELSE 0 END), 0)
              - COALESCE(SUM(CASE WHEN sm.from_location = '现场仓' THEN sm.qty ELSE 0 END), 0)
                AS qty
         FROM event_products ep
         JOIN societies s ON s.id = ep.owner_society_id
         LEFT JOIN stock_movements sm ON sm.event_product_id = ep.id
         WHERE ep.event_id = ?
         GROUP BY ep.id
         HAVING qty <> 0
         ORDER BY ep.id",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;
    Ok(rows)
}

/// 本展会每个商品的现场仓账面数。**走事务**——盘点和带回都要「读到的数就是
/// 待会儿要写的那个数」，事务外读会留下一个可以被下单插进来的窗口。
async fn book_balances(
    conn: &mut sqlx::SqliteConnection,
    event_id: i64,
) -> ApiResult<std::collections::HashMap<i64, i64>> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT ep.id,
                COALESCE(SUM(CASE WHEN sm.to_location   = '现场仓' THEN sm.qty ELSE 0 END), 0)
              - COALESCE(SUM(CASE WHEN sm.from_location = '现场仓' THEN sm.qty ELSE 0 END), 0)
         FROM event_products ep
         LEFT JOIN stock_movements sm ON sm.event_product_id = ep.id
         WHERE ep.event_id = ?
         GROUP BY ep.id",
    )
    .bind(event_id)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().collect())
}

#[derive(Deserialize)]
pub struct StocktakeRequest {
    counts: Vec<CountRow>,
}

#[derive(Deserialize)]
pub struct CountRow {
    event_product_id: i64,
    counted_qty: i64,
}

#[derive(Serialize)]
pub struct StocktakeResponse {
    /// 零差异时为 `None`——`post_journal` 拒绝空 journal，而
    /// 「盘了全对」这个事实靠 `events.stocktaken_at` 记，不靠 journal 存在性。
    journal_id: Option<i64>,
    diffs: Vec<DiffRow>,
}

#[derive(Serialize)]
pub struct DiffRow {
    event_product_id: i64,
    name: String,
    book_qty: i64,
    counted_qty: i64,
}

async fn stocktake(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<StocktakeRequest>,
) -> ApiResult<Json<StocktakeResponse>> {
    check_write_permission(&claims, event_id)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    require_event_open(&mut tx, event_id).await?;

    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE event_id = ? AND status = 'pending'")
            .bind(event_id)
            .fetch_one(&mut *tx)
            .await?;
    if pending > 0 {
        return Err(ApiError::Conflict(format!(
            "还有 {pending} 单待处理，清完才能盘点"
        )));
    }

    let book = book_balances(&mut tx, event_id).await?;

    // 收全量：现场仓余额非 0 的商品必须全部出现在请求里。
    // 「我数了，一致」和「我没数这个」是两件事。
    let mut missing: Vec<String> = Vec::new();
    for (ep_id, qty) in book.iter() {
        if *qty != 0 && !payload.counts.iter().any(|c| c.event_product_id == *ep_id) {
            let name: String = sqlx::query_scalar("SELECT name FROM event_products WHERE id = ?")
                .bind(ep_id)
                .fetch_one(&mut *tx)
                .await?;
            missing.push(name);
        }
    }
    if !missing.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "这些商品还在现场仓但没报数：{}。数过一致的也要报，否则分不出「数了一致」和「没数」",
            missing.join("、")
        )));
    }

    let mut stock = Vec::new();
    let mut diffs = Vec::new();
    for c in &payload.counts {
        if c.counted_qty < 0 {
            return Err(ApiError::BadRequest("实数不能为负".into()));
        }
        let name: Option<String> =
            sqlx::query_scalar("SELECT name FROM event_products WHERE id = ? AND event_id = ?")
                .bind(c.event_product_id)
                .bind(event_id)
                .fetch_optional(&mut *tx)
                .await?;
        let name = name.ok_or_else(|| ApiError::BadRequest("商品不在这场展会里".into()))?;

        let book_qty = *book.get(&c.event_product_id).unwrap_or(&0);
        if book_qty == c.counted_qty {
            continue;
        }
        // 盘亏：现场仓 → 差异；盘盈：差异 → 现场仓。只动货，不动钱。
        let (from, to, qty) = if c.counted_qty < book_qty {
            (
                Location::OnSite,
                Location::Variance,
                book_qty - c.counted_qty,
            )
        } else {
            (
                Location::Variance,
                Location::OnSite,
                c.counted_qty - book_qty,
            )
        };
        stock.push(StockLeg {
            event_product_id: c.event_product_id,
            from,
            to,
            qty,
        });
        diffs.push(DiffRow {
            event_product_id: c.event_product_id,
            name,
            book_qty,
            counted_qty: c.counted_qty,
        });
    }

    let journal_id = if stock.is_empty() {
        None
    } else {
        Some(
            post_journal(
                &mut tx,
                event_id,
                JournalKind::Stocktake,
                None,
                None,
                Some("收摊盘点"),
                &stock,
                &[],
            )
            .await?,
        )
    };

    // 有没有差异都要落时间戳——这正是那条迁移存在的理由。
    sqlx::query("UPDATE events SET stocktaken_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(event_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(StocktakeResponse { journal_id, diffs }))
}

#[derive(Serialize)]
pub struct TakebackResponse {
    journal_id: Option<i64>,
    moved: i64,
}

async fn takeback(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<TakebackResponse>> {
    check_write_permission(&claims, event_id)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    require_event_open(&mut tx, event_id).await?;

    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE event_id = ? AND status = 'pending'")
            .bind(event_id)
            .fetch_one(&mut *tx)
            .await?;
    if pending > 0 {
        return Err(ApiError::Conflict(format!(
            "还有 {pending} 单待处理，清完才能带回"
        )));
    }

    let book = book_balances(&mut tx, event_id).await?;
    let mut stock = Vec::new();
    let mut moved = 0i64;
    for (ep_id, qty) in book {
        if qty > 0 {
            stock.push(StockLeg {
                event_product_id: ep_id,
                from: Location::OnSite,
                to: Location::External,
                qty,
            });
            moved += qty;
        } else if qty < 0 {
            // 结构上不该出现（下单是 CAS，报废/退货都在事务内复核余额）。
            // 出现了就是有路径绕过了检查，硬报出来比默默带回一个负数好。
            return Err(ApiError::Conflict(format!(
                "商品 {ep_id} 的现场仓余额是 {qty}，账本不自洽，不能带回"
            )));
        }
    }

    // 全卖光了也要能收摊——post_journal 拒绝空 journal，所以判空跳过。
    let journal_id = if stock.is_empty() {
        None
    } else {
        Some(
            post_journal(
                &mut tx,
                event_id,
                JournalKind::TakeBack,
                None,
                None,
                Some("收摊带回"),
                &stock,
                &[],
            )
            .await?,
        )
    };

    tx.commit().await?;
    Ok(Json(TakebackResponse { journal_id, moved }))
}

#[derive(Serialize)]
pub struct SettleResponse {
    status: String,
}

async fn settle(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<SettleResponse>> {
    check_write_permission(&claims, event_id)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    require_event_open(&mut tx, event_id).await?;

    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE event_id = ? AND status = 'pending'")
            .bind(event_id)
            .fetch_one(&mut *tx)
            .await?;
    if pending > 0 {
        return Err(ApiError::Conflict(format!("还有 {pending} 单待处理")));
    }

    let leftovers: Vec<(String, i64)> = book_balances(&mut tx, event_id)
        .await?
        .into_iter()
        .filter(|(_, q)| *q != 0)
        .map(|(id, q)| (id.to_string(), q))
        .collect();
    if !leftovers.is_empty() {
        return Err(ApiError::Conflict(format!(
            "还有 {} 种商品在现场仓，带回之后才能结算",
            leftovers.len()
        )));
    }

    // 盘点没做也放行——总有意外（展会清场赶人）。结算单上会写
    //「未盘点，剩余数为账面推算」，靠 events.stocktaken_at 是不是 null。
    sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
        .bind(event_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(SettleResponse {
        status: "已结算".into(),
    }))
}

#[cfg(test)]
mod tests {
    use crate::domain::ledger::onsite_balance;
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::ServiceExt;

    async fn closing(router: &axum::Router, event_id: i64) -> serde_json::Value {
        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/closing"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        read_json(res).await
    }

    async fn place_pending(router: &axum::Router, event_id: i64, ep: i64) -> i64 {
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep, "quantity": 1}]}),
            ))
            .await
            .unwrap();
        read_json(res).await["id"].as_i64().unwrap()
    }

    async fn settle(router: &axum::Router, event_id: i64) -> axum::http::StatusCode {
        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/settle"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap()
            .status()
    }

    async fn takeback(router: &axum::Router, event_id: i64) -> axum::http::StatusCode {
        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/takeback"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap()
            .status()
    }

    #[tokio::test]
    async fn pending_orders_block_everything_downstream() {
        // 硬性阻断（母 spec 6.4）：pending 订单的货已经在顾客仓、钱一分没收。
        // 留着它们，「顾客仓余额 = 卖出」这条不变量就不成立。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        place_pending(&router, event_id, ep_a).await;

        let body = closing(&router, event_id).await;
        assert_eq!(body["pending_orders"].as_array().unwrap().len(), 1);
        assert!(!body["blockers"].as_array().unwrap().is_empty());

        assert_eq!(takeback(&router, event_id).await, StatusCode::CONFLICT);
        assert_eq!(settle(&router, event_id).await, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn a_stocktake_with_no_difference_writes_no_journal_but_still_counts() {
        // 这条是那条新迁移存在的全部理由。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/stocktake"),
                Some(&admin_token()),
                json!({"counts": [{"event_product_id": ep_a, "counted_qty": 10},
                              {"event_product_id": ep_b, "counted_qty": 5}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(
            read_json(res).await["journal_id"].is_null(),
            "没差异就没有 journal"
        );

        let journals: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM journals WHERE event_id = ? AND kind = '盘点'",
        )
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(journals, 0);

        let body = closing(&router, event_id).await;
        assert!(
            !body["stocktaken_at"].is_null(),
            "「盘了，全对」必须留得下痕迹"
        );
    }

    #[tokio::test]
    async fn a_stocktake_writes_both_directions() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/stocktake"),
                Some(&admin_token()),
                json!({"counts": [{"event_product_id": ep_a, "counted_qty": 8},   // 盘亏 2
                              {"event_product_id": ep_b, "counted_qty": 7}]}), // 盘盈 2
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 8);
        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 7);

        let (to_var, from_var): (i64, i64) = sqlx::query_as(
            "SELECT COALESCE(SUM(CASE WHEN to_location = '差异' THEN qty ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN from_location = '差异' THEN qty ELSE 0 END), 0)
             FROM stock_movements sm JOIN journals j ON j.id = sm.journal_id
             WHERE j.kind = '盘点'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((to_var, from_var), (2, 2), "盘亏进差异、盘盈出差异");

        // 钱一分没动——赔付是协商结果，走结算调整，系统推不出来
        let money: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM money_movements mm JOIN journals j ON j.id = mm.journal_id
             WHERE j.kind = '盘点'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(money, 0);
    }

    #[tokio::test]
    async fn a_stocktake_must_cover_every_product_still_on_site() {
        // 「我数了，一致」和「我没数这个」是两件事，请求体不该把它们混成同一个缺省。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/stocktake"),
                Some(&admin_token()),
                json!({"counts": [{"event_product_id": ep_a, "counted_qty": 10}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = read_json(res).await;
        assert!(
            body["error"].as_str().unwrap().contains("本子B"),
            "要说清楚漏了哪个：{body}"
        );
    }

    #[tokio::test]
    async fn takeback_empties_the_on_site_location() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;

        assert_eq!(takeback(&router, event_id).await, StatusCode::OK);
        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 0);
        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 0);
        assert_eq!(settle(&router, event_id).await, StatusCode::OK);

        let status: String = sqlx::query_scalar("SELECT status FROM events WHERE id = ?")
            .bind(event_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "已结算");
    }

    #[tokio::test]
    async fn takeback_on_an_empty_booth_is_a_no_op_not_an_error() {
        // 全卖光了也要能收摊。post_journal 拒绝空 journal，所以这里必须
        // 判空跳过，不能硬调。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        assert_eq!(takeback(&router, event_id).await, StatusCode::OK);

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/takeback"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        assert_eq!(body["moved"], 0);
        assert!(body["journal_id"].is_null());
    }

    #[tokio::test]
    async fn settling_is_refused_while_goods_are_still_on_site() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        assert_eq!(settle(&router, event_id).await, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn settling_is_allowed_without_a_stocktake() {
        // 可跳过，不硬性阻断——总有意外（展会清场赶人）。
        // 但结算单要能看出来没盘过，所以 stocktaken_at 保持 null。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        assert_eq!(takeback(&router, event_id).await, StatusCode::OK);
        assert_eq!(settle(&router, event_id).await, StatusCode::OK);

        let at: Option<String> =
            sqlx::query_scalar("SELECT stocktaken_at FROM events WHERE id = ?")
                .bind(event_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(at.is_none());
    }

    #[tokio::test]
    async fn a_settled_event_refuses_the_whole_wizard() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(takeback(&router, event_id).await, StatusCode::CONFLICT);
        assert_eq!(settle(&router, event_id).await, StatusCode::CONFLICT);
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/stocktake"),
                Some(&admin_token()),
                json!({"counts": [{"event_product_id": ep_a, "counted_qty": 0}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn the_wizard_survives_being_interrupted() {
        // 四步之间没有会话状态，每一步的成果都是已落库的 journal。
        // 盘完退出、重新进来，看到的是真实状态而不是从头开始。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;

        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/stocktake"),
                Some(&admin_token()),
                json!({"counts": [{"event_product_id": ep_a, "counted_qty": 9},
                              {"event_product_id": ep_b, "counted_qty": 5}]}),
            ))
            .await
            .unwrap();

        let body = closing(&router, event_id).await;
        assert!(!body["stocktaken_at"].is_null());
        let remaining: i64 = body["onsite_remaining"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["qty"].as_i64().unwrap())
            .sum();
        assert_eq!(remaining, 14, "9 + 5，盘点的结果留下来了");
        let _ = pool;
    }
}
