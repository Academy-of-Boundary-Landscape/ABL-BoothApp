//! 赠送与报废登记。
//!
//! **记账口径上这两件事只是货从现场仓挪进一个虚拟仓**（`赠品` / `损耗`），
//! 钱那一侧默认一个字不动。所以它们不走订单、不走求解器、不走分摊——
//! 母 spec 4.6 写的「订单上一条 0 元行」被 ②-3 的 spec 偏离 1 推翻了，
//! 理由是做成订单行要让求解器和两个分摊函数全部容忍 0 元行，而收益是零：
//! 结算单上「赠送 4 / 报废 1」本来就是从 stock_movements 按位置汇总的。
//!
//! 唯一的例外是「摊主自掏」开关（spec 偏离 2）。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::guard::{check_read_permission, check_write_permission, require_event_open},
    domain::{
        ledger::{
            onsite_balance, post_journal, reverse_journal, Account, JournalKind, Location,
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
        .route("/events/:event_id/gifts", get(list_gifts).post(create_gift))
        .route(
            "/events/:event_id/scraps",
            get(list_scraps).post(create_scrap),
        )
        .route(
            "/events/:event_id/journals/:journal_id/reverse",
            post(reverse_entry),
        )
}

#[derive(Deserialize)]
pub struct LogRequest {
    event_product_id: i64,
    qty: i64,
    #[serde(default)]
    note: Option<String>,
    /// 只对赠送有意义。报废时忽略——报废的货没有「谁买单」这回事，
    /// 真要赔给货主是结算调整的事（协商结果，系统推不出来）。
    #[serde(default)]
    vendor_pays: bool,
}

#[derive(Serialize)]
pub struct LogResponse {
    journal_id: i64,
}

/// 一条登记记录。`vendor_paid` 由「这条 journal 有没有资金腿」推出来，
/// 不另存一列——存两处就会有一处先腐烂。
#[derive(Serialize, sqlx::FromRow)]
pub struct LogEntry {
    journal_id: i64,
    occurred_at: String,
    event_product_id: i64,
    product_code: String,
    name: String,
    owner_society_id: i64,
    owner_name: String,
    qty: i64,
    note: Option<String>,
    vendor_paid: bool,
}

async fn create_gift(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<LogRequest>,
) -> ApiResult<(StatusCode, Json<LogResponse>)> {
    // 展会守卫在 log_movement：两个入口共用同一个写事务，在那里查才在事务内
    log_movement(
        state,
        claims,
        event_id,
        payload,
        JournalKind::Gift,
        Location::Gift,
    )
    .await
}

async fn create_scrap(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    mut payload: Json<LogRequest>,
) -> ApiResult<(StatusCode, Json<LogResponse>)> {
    // 报废没有「谁买单」，静默忽略比报错友好
    payload.vendor_pays = false;
    // 展会守卫在 log_movement：两个入口共用同一个写事务，在那里查才在事务内
    log_movement(
        state,
        claims,
        event_id,
        payload.0,
        JournalKind::Scrap,
        Location::Loss,
    )
    .await
}

async fn log_movement(
    state: AppState,
    claims: Claims,
    event_id: i64,
    payload: LogRequest,
    kind: JournalKind,
    dest: Location,
) -> ApiResult<(StatusCode, Json<LogResponse>)> {
    check_write_permission(&claims, event_id)?;
    if payload.qty <= 0 {
        return Err(ApiError::BadRequest("数量必须为正".into()));
    }

    // BEGIN IMMEDIATE：余额检查和写入必须在同一个写事务里，否则两台设备
    // 同时登记会各自读到「还有 5 件」然后各扣 3 件，把余额扣成 −1。
    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    require_event_open(&mut tx, event_id).await?;

    let row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT owner_society_id, unit_price FROM event_products WHERE id = ? AND event_id = ?",
    )
    .bind(payload.event_product_id)
    .bind(event_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (owner_society_id, unit_price) =
        row.ok_or_else(|| ApiError::NotFound("商品不在这场展会里".into()))?;

    let available = onsite_balance(&mut *tx, payload.event_product_id).await?;
    if payload.qty > available {
        return Err(ApiError::Conflict(format!(
            "现场仓只剩 {available} 件，登记不了 {} 件",
            payload.qty
        )));
    }

    let stock = vec![StockLeg {
        event_product_id: payload.event_product_id,
        from: Location::OnSite,
        to: dest,
        qty: payload.qty,
    }];

    // 摊主自掏（spec 偏离 2）：方向和一次销售同形，只是把「实收-<渠道>」
    // 换成「摊主自有」——摊主把自己当顾客，用自己的钱买下这件货再送出去。
    //
    // ⚠️ 写成 `摊主自有 → 社团往来` 是**垫付**的方向，意思变成「货主欠我」，
    // 正好错一个符号，而且账面照样平、测不出来，只有对着结算单
    // 「我应转给」那个数才看得出。
    let money = if payload.vendor_pays {
        let amount = Money::from_cents(unit_price)
            .checked_mul_qty(payload.qty)
            .ok_or_else(|| ApiError::BadRequest("金额溢出".into()))?;
        vec![MoneyLeg {
            from: Account::SocietyDue(owner_society_id),
            to: Account::VendorOwn,
            amount,
        }]
    } else {
        Vec::new()
    };

    let journal_id = post_journal(
        &mut tx,
        event_id,
        kind,
        None,
        None,
        payload.note.as_deref(),
        &stock,
        &money,
    )
    .await?;

    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(LogResponse { journal_id })))
}

/// 列表 SQL。`赠送` / `报废` 只有这一个字不同，所以共用常量，
/// kind 走 bind 而不是拼串——拼 SQL 是 sqlx 0.9 下还要包 AssertSqlSafe 的麻烦事。
///
/// 两层过滤缺一不可：
///   j.reverses_journal_id IS NULL   滤掉冲正条目本身
///   NOT EXISTS(...)                 滤掉已被冲正的原条目
const LOG_ROWS: &str = r#"
SELECT j.id                AS journal_id,
       j.occurred_at       AS occurred_at,
       sm.event_product_id AS event_product_id,
       ep.product_code     AS product_code,
       ep.name             AS name,
       ep.owner_society_id AS owner_society_id,
       s.name              AS owner_name,
       sm.qty              AS qty,
       j.note              AS note,
       EXISTS(SELECT 1 FROM money_movements mm WHERE mm.journal_id = j.id) AS vendor_paid
FROM journals j
JOIN stock_movements sm ON sm.journal_id = j.id
JOIN event_products ep  ON ep.id = sm.event_product_id
JOIN societies s        ON s.id = ep.owner_society_id
WHERE j.event_id = ?
  AND j.kind = ?
  AND j.reverses_journal_id IS NULL
  AND NOT EXISTS (SELECT 1 FROM journals r WHERE r.reverses_journal_id = j.id)
ORDER BY j.id DESC
"#;

async fn list_gifts(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<Vec<LogEntry>>> {
    list_entries(state, claims, event_id, JournalKind::Gift).await
}

async fn list_scraps(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<Vec<LogEntry>>> {
    list_entries(state, claims, event_id, JournalKind::Scrap).await
}

async fn list_entries(
    state: AppState,
    claims: Claims,
    event_id: i64,
    kind: JournalKind,
) -> ApiResult<Json<Vec<LogEntry>>> {
    check_read_permission(&claims, event_id)?;
    let rows: Vec<LogEntry> = sqlx::query_as(LOG_ROWS)
        .bind(event_id)
        .bind(kind.as_str())
        .fetch_all(&state.db)
        .await?;
    Ok(Json(rows))
}

/// 撤销一条赠送 / 报废登记。
///
/// **只接受这两种 kind。** 销售和收款有自己的冲正通道（取消订单，
/// `reverse_order_journals`），进货和盘点没有撤销语义（记错了就再记一条反向的）。
/// 不设这道闸，这个端点就成了一个能把任何 journal 冲掉的万能口子。
async fn reverse_entry(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, journal_id)): Path<(i64, i64)>,
) -> ApiResult<Json<LogResponse>> {
    check_write_permission(&claims, event_id)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    require_event_open(&mut tx, event_id).await?;

    let kind: Option<String> =
        sqlx::query_scalar("SELECT kind FROM journals WHERE id = ? AND event_id = ?")
            .bind(journal_id)
            .bind(event_id)
            .fetch_optional(&mut *tx)
            .await?;
    let kind = kind.ok_or_else(|| ApiError::NotFound("记录不存在".into()))?;
    let kind = match kind.as_str() {
        "赠送" => JournalKind::Gift,
        "报废" => JournalKind::Scrap,
        other => {
            return Err(ApiError::BadRequest(format!(
                "只有赠送和报废能在这里撤销，这是一条「{other}」"
            )))
        }
    };

    // DB 层的偏唯一索引 idx_journals_reverses 会拦住重复冲正，但直接撞上去是 500。
    let already: Option<i64> =
        sqlx::query_scalar("SELECT id FROM journals WHERE reverses_journal_id = ?")
            .bind(journal_id)
            .fetch_optional(&mut *tx)
            .await?;
    if already.is_some() {
        return Err(ApiError::Conflict("这条记录已经撤销过了".into()));
    }

    let new_id = reverse_journal(&mut tx, journal_id, kind, Some("撤销登记")).await?;
    tx.commit().await?;
    Ok(Json(LogResponse { journal_id: new_id }))
}

#[cfg(test)]
mod tests {
    use crate::domain::ledger::{account_balance, onsite_balance, Account};
    use crate::domain::money::Money;
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn scrapping_moves_goods_and_touches_no_money() {
        // 报废在记账口径上就是货从现场仓挪进虚拟的损耗仓，钱那一侧一个字不动。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/scraps"),
                Some(&token),
                json!({"event_product_id": ep_a, "qty": 2, "note": "被雨淋了"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 8, "10 − 2");
        let money: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM money_movements mm JOIN journals j ON j.id = mm.journal_id
             WHERE j.event_id = ?",
        )
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(money, 0, "报废不该记任何资金腿");
    }

    #[tokio::test]
    async fn gifting_defaults_to_no_money_leg() {
        // 默认口径：货主自己承担。绝大多数赠品是送自家货，记一笔
        // 「摊主个人买下自家社团的货」只会让单人摊主看不懂结算单。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/gifts"),
                Some(&token),
                json!({"event_product_id": ep_b, "qty": 1}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 4);
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::ZERO,
            "默认不补偿货主"
        );
    }

    #[tokio::test]
    async fn gifting_with_vendor_pays_owes_the_owner() {
        // 方向和一次销售同形，只是把「实收-<渠道>」换成「摊主自有」：
        // 摊主把自己当顾客，用自己的钱买下这件货再送出去。
        // 反过来写就是垫付的方向，账面照样平，只有对着「我应转给」才看得出错。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/gifts"),
                Some(&token),
                json!({"event_product_id": ep_b, "qty": 2, "vendor_pays": true}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(-4000),
            "往来 −4000 ⇒ 我应转给黄昏堂 +4000。写成垫付方向的话这里会是 +4000"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::VendorOwn)
                .await
                .unwrap(),
            Money::from_cents(4000)
        );
    }

    #[tokio::test]
    async fn cannot_give_away_more_than_is_on_site() {
        // 现场仓余额是从流水聚合出来的，没有可以漂移的第二个数字；
        // 但没有这条检查，余额会被扣成负数，之后 settle 永远过不去。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/scraps"),
                Some(&token),
                json!({"event_product_id": ep_b, "qty": 6}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        assert_eq!(
            onsite_balance(&pool, ep_b).await.unwrap(),
            5,
            "一件都不该动"
        );
    }

    #[tokio::test]
    async fn successive_scraps_cannot_overdraw_either() {
        // Review Focus #1：两台手机同时报废同一件货。
        //
        // `test_pool()` 是 max_connections(1)，真并发在这个夹具里模拟不出来，
        // 所以这里钉的是「余额检查读的是事务内的最新值」——连着报两次，
        // 第二次必须看见第一次的结果。真正的并发安全靠 BEGIN IMMEDIATE，
        // 它在同一条代码路径上，这条测试一旦被改成事务外检查就会红。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let body = json!({"event_product_id": ep_b, "qty": 3});

        let first = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/scraps"),
                Some(&token),
                body.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::CREATED);

        let second = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/scraps"),
                Some(&token),
                body,
            ))
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::CONFLICT, "只剩 2 件了");
        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn reversing_a_gift_puts_everything_back() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/gifts"),
                Some(&token),
                json!({"event_product_id": ep_b, "qty": 2, "vendor_pays": true}),
            ))
            .await
            .unwrap();
        let journal_id = read_json(res).await["journal_id"].as_i64().unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/journals/{journal_id}/reverse"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 5);
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::ZERO
        );

        // 列表里不该再出现它（被冲正的和冲正条目都要滤掉）
        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/gifts"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        let list = read_json(res).await;
        assert_eq!(list.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn a_journal_cannot_be_reversed_twice() {
        // DB 层有偏唯一索引兜底，但直接撞上去会是 500。这里要的是可读的 409。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/scraps"),
                Some(&token),
                json!({"event_product_id": ep_a, "qty": 1}),
            ))
            .await
            .unwrap();
        let journal_id = read_json(res).await["journal_id"].as_i64().unwrap();

        let uri = format!("/api/events/{event_id}/journals/{journal_id}/reverse");
        let first = router
            .clone()
            .oneshot(json_request("POST", &uri, Some(&token), json!(null)))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        let second = router
            .clone()
            .oneshot(json_request("POST", &uri, Some(&token), json!(null)))
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn only_gift_and_scrap_journals_can_be_reversed_here() {
        // 销售和收款各有自己的冲正通道（取消订单），不能拿这个端点去冲它们。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        // 夹具的开场带货是一条「进货」journal
        let restock_id: i64 =
            sqlx::query_scalar("SELECT id FROM journals WHERE event_id = ? AND kind = '进货'")
                .bind(event_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/journals/{restock_id}/reverse"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn a_settled_event_refuses_gifts_and_scraps() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();
        let token = admin_token();

        for path in ["gifts", "scraps"] {
            let res = router
                .clone()
                .oneshot(json_request(
                    "POST",
                    &format!("/api/events/{event_id}/{path}"),
                    Some(&token),
                    json!({"event_product_id": ep_a, "qty": 1}),
                ))
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::CONFLICT, "{path} 应该被冻结挡住");
        }
    }

    #[tokio::test]
    async fn the_gift_list_decodes_every_column_and_excludes_scraps() {
        // 零行的列表什么都不解码。这条是唯一一条真的把三个 JOIN、
        // EXISTS(...) AS vendor_paid 的 bool 解码、DATETIME 列的 String 解码
        // 和 note 的 NULL 一起跑过的测试。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        // 带 note、带自掏的赠送
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/gifts"),
                Some(&token),
                json!({"event_product_id": ep_b, "qty": 2, "note": "送给隔壁摊", "vendor_pays": true}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        // 不带 note 的报废（note 为 NULL，且不该出现在 /gifts 里）
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/scraps"),
                Some(&token),
                json!({"event_product_id": ep_a, "qty": 1}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/gifts"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let list = read_json(res).await;
        let rows = list.as_array().unwrap();
        assert_eq!(rows.len(), 1, "报废不该出现在赠送列表里");
        let r = &rows[0];
        assert_eq!(r["event_product_id"], ep_b);
        assert_eq!(r["product_code"], "B");
        assert_eq!(r["name"], "本子B");
        assert_eq!(r["owner_society_id"], 2);
        assert_eq!(r["owner_name"], "黄昏堂");
        assert_eq!(r["qty"], 2);
        assert_eq!(r["note"], "送给隔壁摊");
        assert_eq!(r["vendor_paid"], true, "自掏的赠品有资金腿");
        assert!(
            r["occurred_at"].as_str().is_some(),
            "DATETIME 列要能解成字符串"
        );
        assert!(r["journal_id"].as_i64().unwrap() > 0);
    }

    #[tokio::test]
    async fn the_scrap_list_decodes_a_null_note_and_excludes_gifts() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/scraps"),
                Some(&token),
                json!({"event_product_id": ep_a, "qty": 1}),
            ))
            .await
            .unwrap();
        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/gifts"),
                Some(&token),
                json!({"event_product_id": ep_b, "qty": 1}),
            ))
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/scraps"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let list = read_json(res).await;
        let rows = list.as_array().unwrap();
        assert_eq!(rows.len(), 1, "赠送不该出现在报废列表里");
        assert_eq!(rows[0]["product_code"], "A");
        assert!(rows[0]["note"].is_null(), "没写备注时 note 是 null，不该炸");
        assert_eq!(rows[0]["vendor_paid"], false, "报废从来没有资金腿");
    }
}
