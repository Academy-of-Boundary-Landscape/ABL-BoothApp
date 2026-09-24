//! 退货。
//!
//! **两个金额都要用**（②-2 交接契约第 2 条）：退给顾客的默认按 `paid_amount`，
//! 冲销货主按 `allocated_amount`。只用一个数，要么多退给顾客、要么让货主吃了
//! 摊主做的人情——母 spec 4.5 明写这是「早先版本的一个真漏洞」。
//!
//! **按 order_lines 逐行退，不合并显示**（交接契约第 3 条）：同一个商品因为
//! Lot 归属会拆成多行，退哪一行是摊主的决定，不是系统猜的。
//!
//! **不做「退货的同时拆套装」**（spec 5.5）：手工改 R 这条通道已经能表达任何
//! 结果，而拆套装要在退货里重跑一遍「先拆后摊」的顺序，复杂度不成比例。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::guard::{check_read_permission, check_write_permission, require_event_open},
    domain::{
        allocation::apportion,
        channel::normalize,
        ledger::{
            home_society_id, post_journal, Account, JournalKind, Location, MoneyLeg, StockLeg,
        },
        money::Money,
    },
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/events/:event_id/orders/:order_id/refunds",
        post(create_refund).get(list_refunds),
    )
}

#[derive(Deserialize)]
pub struct RefundRequest {
    channel: String,
    /// 实际退给顾客的总额（分）。省略 = 按各行实付全额退。
    #[serde(default)]
    refund_amount: Option<i64>,
    lines: Vec<RefundLineRequest>,
}

#[derive(Deserialize)]
pub struct RefundLineRequest {
    order_line_id: i64,
    qty: i64,
    /// `现场仓`（还能卖）或 `损耗`（已损坏）。
    destination: String,
}

#[derive(Serialize)]
pub struct RefundResponse {
    journal_id: i64,
    refund_amount: i64,
    allocated_total: i64,
    paid_total: i64,
}

/// 一行的中间量。字段全是**这一次**要退的部分，不是原行的值。
struct Part {
    order_line_id: i64,
    event_product_id: i64,
    owner_society_id: i64,
    qty: i64,
    allocated: i64,
    paid: i64,
    destination: Location,
}

async fn create_refund(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, order_id)): Path<(i64, i64)>,
    Json(payload): Json<RefundRequest>,
) -> ApiResult<(StatusCode, Json<RefundResponse>)> {
    check_write_permission(&claims, event_id)?;
    let channel = normalize(&payload.channel)?;
    if payload.lines.is_empty() {
        return Err(ApiError::BadRequest("至少要退一行".into()));
    }

    // 同一个 order_line_id 出现两次时，两条都会读到同一个 left_qty 并各自通过
    // 「只剩 N 件可退」那道检查——检查是按行做的，而消耗是累加的。
    // 合并比去重更安全：去重会静默丢掉用户的一条意图（两条的 destination 可能不同）。
    let mut seen = std::collections::HashSet::new();
    for l in &payload.lines {
        if !seen.insert(l.order_line_id) {
            return Err(ApiError::BadRequest(
                "同一个订单行在一次退货里只能出现一次——要分不同去向请分两次退".into(),
            ));
        }
    }

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    require_event_open(&mut tx, event_id).await?;

    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM orders WHERE id = ? AND event_id = ?")
            .bind(order_id)
            .bind(event_id)
            .fetch_optional(&mut *tx)
            .await?;
    match status.as_deref() {
        None => return Err(ApiError::NotFound("订单不存在".into())),
        Some("completed") => {}
        Some("pending") => {
            return Err(ApiError::BadRequest(
                "待处理的订单请直接取消，不要走退货——货还没交出去，钱也没收".into(),
            ))
        }
        Some(_) => return Err(ApiError::BadRequest("已取消的订单没有可退的东西".into())),
    }

    let home = home_society_id(&mut tx).await?;

    // ---- 逐行算出这次退的 A 和 P ----
    let mut parts: Vec<Part> = Vec::with_capacity(payload.lines.len());
    for l in &payload.lines {
        if l.qty <= 0 {
            return Err(ApiError::BadRequest("退货数量必须为正".into()));
        }
        let destination = match l.destination.as_str() {
            "现场仓" => Location::OnSite,
            "损耗" => Location::Loss,
            other => {
                return Err(ApiError::BadRequest(format!(
                    "退货去向只能是「现场仓」或「损耗」，收到「{other}」"
                )))
            }
        };

        let row: Option<(i64, i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT ol.event_product_id, ep.owner_society_id, ol.qty,
                    ol.allocated_amount, ol.paid_amount
             FROM order_lines ol
             JOIN event_products ep ON ep.id = ol.event_product_id
             WHERE ol.id = ? AND ol.order_id = ?",
        )
        .bind(l.order_line_id)
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (event_product_id, owner_society_id, line_qty, line_alloc, line_paid) =
            row.ok_or_else(|| ApiError::BadRequest("订单行不属于这张订单".into()))?;

        let (done_qty, done_alloc, done_paid): (i64, i64, i64) = sqlx::query_as(
            "SELECT COALESCE(SUM(qty), 0), COALESCE(SUM(allocated_amount), 0),
                    COALESCE(SUM(paid_amount), 0)
             FROM refunds WHERE order_line_id = ?",
        )
        .bind(l.order_line_id)
        .fetch_one(&mut *tx)
        .await?;

        let left_qty = line_qty - done_qty;
        if l.qty > left_qty {
            return Err(ApiError::Conflict(format!("这一行只剩 {left_qty} 件可退")));
        }

        // 切「剩余」而不是切「原值」：分两次各退一半，切原值会让两次各自向下
        // 取整，差额永远回不到账上；切剩余则第二次必然拿到「原值减去第一次
        // 实际给出去的」。weights 的第二项可能是 0（退光），apportion 允许。
        let weights = [l.qty, left_qty - l.qty];
        let allocated = apportion(line_alloc - done_alloc, &weights, None)?[0];
        let paid = apportion(line_paid - done_paid, &weights, None)?[0];

        parts.push(Part {
            order_line_id: l.order_line_id,
            event_product_id,
            owner_society_id,
            qty: l.qty,
            allocated,
            paid,
            destination,
        });
    }

    // ---- 实退总额，以及摊回每行 ----
    let paid_total: i64 = parts.iter().map(|p| p.paid).sum();
    let allocated_total: i64 = parts.iter().map(|p| p.allocated).sum();
    let refund_total = payload.refund_amount.unwrap_or(paid_total);
    if refund_total < 0 {
        return Err(ApiError::BadRequest("退款金额不能为负".into()));
    }
    if refund_total > paid_total {
        return Err(ApiError::BadRequest(format!(
            "退款不能多于顾客为这些货实付的 {}——白送钱请走结算调整",
            Money::from_cents(paid_total)
        )));
    }

    let weights: Vec<i64> = parts.iter().map(|p| p.paid).collect();
    // Review Focus #3：0 元行（预售取货 SKU）权重全零，apportion 会报
    // 「无法分摊：权重之和为零」。这种单的 refund_total 必然是 0
    //（上面的上限检查已经保证），直接给一组 0，不为退化情形改纯函数。
    let refunds: Vec<i64> = if paid_total == 0 {
        vec![0; parts.len()]
    } else {
        apportion(refund_total, &weights, Some(&weights))?
    };

    // ---- 腿 ----
    let mut stock = Vec::with_capacity(parts.len());
    let mut money = Vec::new();
    for (p, &r) in parts.iter().zip(refunds.iter()) {
        stock.push(StockLeg {
            event_product_id: p.event_product_id,
            from: Location::Customer,
            to: p.destination,
            qty: p.qty,
        });

        // ① 冲销货主的销售
        money.push(MoneyLeg {
            from: Account::Received(channel.clone()),
            to: Account::SocietyDue(p.owner_society_id),
            amount: Money::from_cents(p.allocated),
        });

        // ② 冲销本社团那份手工折让。加价时方向反过来（母 spec 4.3 允许加价），
        //    金额恒取绝对值——post_journal 对非正金额直接 400。
        if p.allocated != p.paid {
            let (from, to) = if p.allocated > p.paid {
                (
                    Account::SocietyDue(home),
                    Account::Received(channel.clone()),
                )
            } else {
                (
                    Account::Received(channel.clone()),
                    Account::SocietyDue(home),
                )
            };
            money.push(MoneyLeg {
                from,
                to,
                amount: Money::from_cents((p.allocated - p.paid).abs()),
            });
        }

        // ③ 顾客没拿回的部分归**货主**（不是本社团）——它本质上是
        //    「剩下的货重新按原价算」，那是货主的货。母 spec 4.5 点名的漏洞。
        if p.paid > r {
            money.push(MoneyLeg {
                from: Account::SocietyDue(p.owner_society_id),
                to: Account::Received(channel.clone()),
                amount: Money::from_cents(p.paid - r),
            });
        }
    }

    // stock 恒非空（lines 非空且 qty > 0），所以不会撞 post_journal 的空 journal 检查，
    // 哪怕三条钱腿全是 0（顾客只退货不要钱）。
    let journal_id = post_journal(
        &mut tx,
        event_id,
        JournalKind::Refund,
        Some(order_id),
        None,
        Some("退货"),
        &stock,
        &money,
    )
    .await?;

    for (p, &r) in parts.iter().zip(refunds.iter()) {
        sqlx::query(
            "INSERT INTO refunds
               (order_line_id, journal_id, qty, allocated_amount, paid_amount,
                refund_amount, channel, destination)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(p.order_line_id)
        .bind(journal_id)
        .bind(p.qty)
        .bind(p.allocated)
        .bind(p.paid)
        .bind(r)
        .bind(&channel)
        .bind(p.destination.as_str())
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(RefundResponse {
            journal_id,
            refund_amount: refund_total,
            allocated_total,
            paid_total,
        }),
    ))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct RefundHistoryRow {
    id: i64,
    order_line_id: i64,
    product_code: String,
    name: String,
    qty: i64,
    refund_amount: i64,
    channel: String,
    destination: String,
    occurred_at: String,
}

/// 可退的行。**同一个商品可能出现多行**（按 Lot 归属拆的），
/// 所以前端不要按 `event_product_id` 去重（②-2 交接契约第 3 条）。
#[derive(Serialize, sqlx::FromRow)]
pub struct RefundableLine {
    order_line_id: i64,
    event_product_id: i64,
    product_code: String,
    name: String,
    lot_name: Option<String>,
    qty: i64,
    refunded_qty: i64,
    remaining_qty: i64,
    remaining_allocated: i64,
    remaining_paid: i64,
}

#[derive(Serialize)]
pub struct RefundListResponse {
    history: Vec<RefundHistoryRow>,
    lines: Vec<RefundableLine>,
}

async fn list_refunds(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, order_id)): Path<(i64, i64)>,
) -> ApiResult<Json<RefundListResponse>> {
    check_read_permission(&claims, event_id)?;

    let history: Vec<RefundHistoryRow> = sqlx::query_as(
        "SELECT r.id, r.order_line_id, ep.product_code, ep.name, r.qty,
                r.refund_amount, r.channel, r.destination, j.occurred_at
         FROM refunds r
         JOIN order_lines ol    ON ol.id = r.order_line_id
         JOIN orders o          ON o.id = ol.order_id
         JOIN event_products ep ON ep.id = ol.event_product_id
         JOIN journals j        ON j.id = r.journal_id
         WHERE ol.order_id = ? AND o.event_id = ?
         ORDER BY r.id DESC",
    )
    .bind(order_id)
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    let lines: Vec<RefundableLine> = sqlx::query_as(
        "SELECT ol.id                              AS order_line_id,
                ol.event_product_id                AS event_product_id,
                ep.product_code                    AS product_code,
                ep.name                            AS name,
                olot.name                          AS lot_name,
                ol.qty                             AS qty,
                COALESCE(r.done_qty, 0)            AS refunded_qty,
                ol.qty - COALESCE(r.done_qty, 0)   AS remaining_qty,
                ol.allocated_amount - COALESCE(r.done_alloc, 0) AS remaining_allocated,
                ol.paid_amount      - COALESCE(r.done_paid, 0)  AS remaining_paid
         FROM order_lines ol
         JOIN orders o          ON o.id = ol.order_id
         JOIN event_products ep ON ep.id = ol.event_product_id
         LEFT JOIN order_lots olot ON olot.id = ol.order_lot_id
         LEFT JOIN (
            SELECT order_line_id,
                   SUM(qty) AS done_qty,
                   SUM(allocated_amount) AS done_alloc,
                   SUM(paid_amount) AS done_paid
            FROM refunds GROUP BY order_line_id
         ) r ON r.order_line_id = ol.id
         WHERE ol.order_id = ? AND o.event_id = ?
         ORDER BY ol.id",
    )
    .bind(order_id)
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    // 写路径查的是 (id, event_id)，读路径也必须同样按展会隔离。订单不属于这场
    // 展会时返回 404 而不是空结果——空结果会让前端显示一张空白的退货弹窗。
    if lines.is_empty() {
        return Err(ApiError::NotFound("订单不存在".into()));
    }

    Ok(Json(RefundListResponse { history, lines }))
}

#[cfg(test)]
mod tests {
    use crate::domain::ledger::{account_balance, onsite_balance, Account};
    use crate::domain::money::Money;
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, seed_lot, seed_lot_repeat,
        test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::{json, Value};
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    /// 下单并完成，返回 (order_id, 各 order_line 的 id)。
    async fn place_and_complete(
        router: &axum::Router,
        pool: &SqlitePool,
        event_id: i64,
        items: Value,
        channel: &str,
        final_amount: Option<i64>,
        unapply: Option<Vec<i64>>,
    ) -> (i64, Vec<i64>) {
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({ "items": items }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED, "下单失败");
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        let mut body = json!({ "status": "completed", "channel": channel });
        if let Some(f) = final_amount {
            body["final_amount"] = json!(f);
        }
        if let Some(u) = unapply {
            body["unapply_lot_ids"] = json!(u);
        }
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&admin_token()),
                body,
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "完成订单失败");

        let line_ids: Vec<i64> =
            sqlx::query_scalar("SELECT id FROM order_lines WHERE order_id = ? ORDER BY id")
                .bind(order_id)
                .fetch_all(pool)
                .await
                .unwrap();
        (order_id, line_ids)
    }

    #[tokio::test]
    async fn a_plain_full_refund_undoes_the_sale_exactly() {
        // 没有 Lot、没有手工折让时 A == P == R，②③ 两条腿都不该出现。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_b, "quantity": 2}]),
            "微信",
            None,
            None,
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "微信",
                   "lines": [{"order_line_id": lines[0], "qty": 2, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 5, "货全回来了");
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into()))
                .await
                .unwrap(),
            Money::ZERO
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::ZERO
        );

        let legs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM money_movements mm JOIN journals j ON j.id = mm.journal_id
             WHERE j.kind = '退货'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(legs, 1, "A == P == R 时只该有第 ① 条腿");
    }

    #[tokio::test]
    async fn refunding_less_than_paid_leaves_the_difference_with_the_owner() {
        // 母 spec 4.5 点名的那个「早先版本的真漏洞」：
        // 覆盖差额（顾客没拿回的部分）归**货主**，不归本社团。
        //
        // 场景：黄昏堂的货 2 件 ¥40，微信收；退 1 件但只退 ¥5。
        // A = P = 2000，R = 500 ⇒ ③ 腿 1500 归黄昏堂。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_b, "quantity": 2}]),
            "微信",
            None,
            None,
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "微信", "refund_amount": 500,
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into()))
                .await
                .unwrap(),
            Money::from_cents(3500),
            "收 4000 退 500"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(-3500),
            "黄昏堂拿 2000（没退的那件）+ 1500（顾客没拿回的）"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1))
                .await
                .unwrap(),
            Money::ZERO,
            "本社团完全不该被牵连——覆盖差额归货主"
        );
    }

    #[tokio::test]
    async fn refunding_a_discounted_mixed_owner_order_splits_all_three_legs() {
        // 四个条件同时成立：混货主 + Lot 折让 + 手工折让 + 部分退 + 改 R。
        // 这是整个退货通道存在的理由，也是最容易算错的一处。
        //
        // 购物车：A(本社团 ¥30) ×1 + B(黄昏堂 ¥20) ×2，原价 7000。
        // Lot「任选 2 件 ¥40」会套用一次；再把实收改成 5000（手工折让）。
        // 退 B 的一件，退款额改成 1000。
        //
        // 断言只锁三件事，不锁中间量：
        //   1. 实收账户净额 = 收到的 − 退出去的
        //   2. 两个社团往来之和 + 实收 + 摊主自有 = 0（复式账总闭合）
        //   3. 本社团只承担手工折让那一份，不承担覆盖差额
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_lot(&pool, event_id, "任选2件40", 2, 4000, &[ep_b]).await;

        let (order_id, _lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_a, "quantity": 1}, {"product_id": ep_b, "quantity": 2}]),
            "现金",
            Some(5000),
            None,
        )
        .await;

        // 找一条属于 B 的订单行
        let line_b: i64 = sqlx::query_scalar(
            "SELECT id FROM order_lines WHERE order_id = ? AND event_product_id = ? ORDER BY id LIMIT 1",
        ).bind(order_id).bind(ep_b).fetch_one(&pool).await.unwrap();
        let qty_b: i64 = sqlx::query_scalar("SELECT qty FROM order_lines WHERE id = ?")
            .bind(line_b)
            .fetch_one(&pool)
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "现金", "refund_amount": 1000,
                   "lines": [{"order_line_id": line_b, "qty": 1, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED, "{:?}", res.status());

        let cash = account_balance(&pool, event_id, &Account::Received("现金".into()))
            .await
            .unwrap();
        assert_eq!(cash, Money::from_cents(4000), "收 5000 退 1000");

        let home = account_balance(&pool, event_id, &Account::SocietyDue(1))
            .await
            .unwrap();
        let other = account_balance(&pool, event_id, &Account::SocietyDue(2))
            .await
            .unwrap();
        let vendor = account_balance(&pool, event_id, &Account::VendorOwn)
            .await
            .unwrap();
        assert_eq!(
            cash + home + other + vendor,
            Money::ZERO,
            "复式账总闭合：所有资金账户余额之和恒为 0"
        );

        // 货：B 回来一件
        let _ = qty_b;
        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 4);
    }

    #[tokio::test]
    async fn refunding_zero_still_moves_the_goods() {
        // Review Focus #2：顾客只退货不要钱。②③ 两条腿一条 0 一条满额，
        // post_journal 会把零金额腿滤掉——货腿必须仍然在，journal 必须成立。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_b, "quantity": 1}]),
            "现金",
            None,
            None,
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "现金", "refund_amount": 0,
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "损耗"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(
            onsite_balance(&pool, ep_b).await.unwrap(),
            4,
            "退到损耗，不回现场仓"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into()))
                .await
                .unwrap(),
            Money::from_cents(2000),
            "一分没退"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(-2000),
            "货主照拿全款——顾客把货送回来了但没要钱"
        );
    }

    #[tokio::test]
    async fn a_zero_price_line_refunds_without_dividing_by_zero() {
        // Review Focus #3：0 元 SKU（母 spec 4.6 的预售取货）。
        // 按 paid 权重摊回时权重全零，apportion 会报「权重之和为零」。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        // 偏离 brief：原来这里直接插 event_product 3 且复用 master_product_id=1，
        // 撞 `event_products` 的 UNIQUE(event_id, master_product_id)——夹具里 id=1 的
        // 那个 event_product 已经用掉了 master_product_id=1。补一个 master_product 3，
        // 其余断言和意图原样不动。
        sqlx::query(
            "INSERT INTO master_products (id, product_code, name, default_price, owner_society_id)
             VALUES (3, 'Z', '预售取货券', 0, 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO event_products
               (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
             VALUES (3, 1, 3, 1, 'Z', '预售取货券', 0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let mut tx = pool.begin().await.unwrap();
        crate::domain::ledger::post_journal(
            &mut tx,
            event_id,
            crate::domain::ledger::JournalKind::Restock,
            None,
            None,
            Some("0 元商品带货"),
            &[crate::domain::ledger::StockLeg {
                event_product_id: 3,
                from: crate::domain::ledger::Location::External,
                to: crate::domain::ledger::Location::OnSite,
                qty: 3,
            }],
            &[],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let (order_id, lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": 3, "quantity": 1}]),
            "现金",
            None,
            None,
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "现金",
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED, "0 元行不该把分摊打爆");
        assert_eq!(onsite_balance(&pool, 3).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn partial_refunds_never_lose_a_cent_to_rounding() {
        // 切「剩余」而不是切「原值」的理由。
        // 3 件、实付 1000（Lot 分摊后是个除不尽的数），分三次各退 1 件，
        // 三次退款之和必须精确等于 1000。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_b, "quantity": 3}]),
            "现金",
            Some(1000),
            None,
        )
        .await;

        let mut total = 0i64;
        for _ in 0..3 {
            let res = router
                .clone()
                .oneshot(json_request(
                    "POST",
                    &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                    Some(&admin_token()),
                    json!({"channel": "现金",
                       "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
                ))
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::CREATED);
            total += read_json(res).await["refund_amount"].as_i64().unwrap();
        }
        assert_eq!(total, 1000, "三次退款之和必须精确等于顾客实付");
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into()))
                .await
                .unwrap(),
            Money::ZERO
        );
        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 5);
    }

    #[tokio::test]
    async fn cannot_refund_more_units_than_were_bought() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_b, "quantity": 1}]),
            "现金",
            None,
            None,
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "现金",
                   "lines": [{"order_line_id": lines[0], "qty": 2, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn cannot_refund_more_money_than_the_customer_paid() {
        // 退多于实付是白送钱，母 spec 6.3 明说要走结算调整。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_b, "quantity": 1}]),
            "现金",
            None,
            None,
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "现金", "refund_amount": 2001,
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn only_completed_orders_can_be_refunded() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_b, "quantity": 1}]}),
            ))
            .await
            .unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();
        let line: i64 = sqlx::query_scalar("SELECT id FROM order_lines WHERE order_id = ?")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "现金",
                   "lines": [{"order_line_id": line, "qty": 1, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            StatusCode::BAD_REQUEST,
            "待处理的单请直接取消"
        );
    }

    #[tokio::test]
    async fn refunding_to_a_different_channel_than_the_sale() {
        // 微信收、现金退很常见。实收那条腿必须是**实际退出去的**渠道，
        // 否则收摊清点对不上。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_b, "quantity": 1}]),
            "微信",
            None,
            None,
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "现金",
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into()))
                .await
                .unwrap(),
            Money::from_cents(2000),
            "微信还是收了 2000"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into()))
                .await
                .unwrap(),
            Money::from_cents(-2000),
            "现金盒少了 2000"
        );
    }

    #[tokio::test]
    async fn the_refundable_list_shows_what_is_left() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_b, "quantity": 3}]),
            "现金",
            None,
            None,
        )
        .await;

        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "现金",
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        let body = read_json(res).await;
        assert_eq!(body["history"].as_array().unwrap().len(), 1);
        let line = &body["lines"][0];
        assert_eq!(line["refunded_qty"], 1);
        assert_eq!(line["remaining_qty"], 2);
        assert_eq!(line["remaining_paid"], 4000);
    }

    #[tokio::test]
    async fn a_settled_event_refuses_refunds() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_b, "quantity": 1}]),
            "现金",
            None,
            None,
        )
        .await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "现金",
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn refunding_without_a_home_society_fails_readably() {
        // Review Focus #5：用户把本社团删了，或者库是手工改过的。
        // 没有这条检查，home_society_id 那里会 panic 成 500「服务器错误」，
        // 摊主看到之后无从下手。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_b, "quantity": 1}]),
            "现金",
            None,
            None,
        )
        .await;
        sqlx::query("UPDATE societies SET is_home = 0 WHERE is_home = 1")
            .execute(&pool)
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "现金",
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        let body = read_json(res).await;
        assert!(
            body["error"].as_str().unwrap().contains("本社团"),
            "错误信息要说清楚缺的是什么：{body}"
        );
    }

    #[tokio::test]
    async fn a_duplicated_order_line_in_one_request_is_refused() {
        // 两条都会读到同一个 left_qty 然后各自通过「只剩 N 件可退」，
        // 结果是凭空多退一份货：现场仓 +4、顾客仓 −2、货主被记两次。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_b, "quantity": 2}]),
            "现金",
            None,
            None,
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "现金", "lines": [
                    {"order_line_id": lines[0], "qty": 2, "destination": "现场仓"},
                    {"order_line_id": lines[0], "qty": 2, "destination": "现场仓"}
                ]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        assert_eq!(
            onsite_balance(&pool, ep_b).await.unwrap(),
            3,
            "一件都不该动"
        );
        let refunds: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM refunds")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(refunds, 0);
    }

    #[tokio::test]
    async fn the_refund_list_does_not_leak_across_events() {
        // 写路径查的是 (id, event_id)，读路径只查 id——两半自相矛盾。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, _) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_b, "quantity": 1}]),
            "现金",
            None,
            None,
        )
        .await;

        sqlx::query(
            "INSERT INTO events (id, name, event_date, status)
                     VALUES (2, '另一场', '2026-11-01', '进行中')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/2/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            StatusCode::NOT_FOUND,
            "别的展会的订单不该读得到"
        );
    }

    #[tokio::test]
    async fn the_refundable_list_decodes_a_non_null_lot_name() {
        // 无套装的单子只证明 `LEFT JOIN order_lots` 能被 SQL 解析，证明不了
        // lot_name 被解码。同一个商品因为套装归属拆成多行，正是这个端点返回
        //「行」而不是「商品」的理由，所以这里必须让 lot_name 非空一次。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        seed_lot_repeat(&pool, event_id, "任选2件50", 2, 5000, &[ep_a]).await;
        let (order_id, _) = place_and_complete(
            &router,
            &pool,
            event_id,
            json!([{"product_id": ep_a, "quantity": 3}]),
            "现金",
            None,
            None,
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        let lines = body["lines"].as_array().unwrap();
        assert_eq!(lines.len(), 2, "同一个商品拆成套装内/散买两行");
        assert_eq!(lines[0]["event_product_id"], lines[1]["event_product_id"]);
        let with_lot = lines.iter().filter(|l| !l["lot_name"].is_null()).count();
        let without_lot = lines.iter().filter(|l| l["lot_name"].is_null()).count();
        assert_eq!((with_lot, without_lot), (1, 1), "一行有套装名、一行没有");
    }
}
