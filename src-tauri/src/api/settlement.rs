//! 结算侧：渠道列表、垫付、结算调整、结算单、收摊清点、xlsx 导出。
//!
//! **本模块里有三类操作故意不调用 `require_event_open`**：垫付、结算调整、
//! 收摊清点（后者在 Task 8）。冻结之后它们仍然允许（spec 偏离 3）——「回家翻出一张打印费收据」
//! 和「回家发现少了一本书」是同一类事件。看见别处都守着就顺手补上去，
//! 会把有意的例外当成漏掉的守卫。改之前先读 spec 3.2。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::guard::{check_read_permission, check_write_permission},
    domain::{
        channel::PRESET_CHANNELS,
        ledger::{post_journal, reverse_journal, Account, JournalKind, MoneyLeg},
        money::Money,
    },
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/channels", get(list_channels))
        .route(
            "/events/:event_id/advances",
            get(list_advances).post(create_advance),
        )
        .route("/events/:event_id/advances/:id", delete(delete_advance))
        .route(
            "/events/:event_id/adjustments",
            get(list_adjustments).post(create_adjustment),
        )
        .route(
            "/events/:event_id/adjustments/:id",
            delete(delete_adjustment),
        )
}

/// 已用过的收款渠道，跨展会。
///
/// 挂在 `/api/channels` 而不是 `/api/events/:id/channels`：它按定义就是跨展会的。
/// 这是防「微信」和「微信支付」分裂成两个账户的那一条（②-1/②-2 交接段第 3 条）。
async fn list_channels(
    State(state): State<AppState>,
    _claims: Claims,
) -> ApiResult<Json<Vec<String>>> {
    // 不需要展会守卫：只读，且按定义跨展会
    let used: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT channel FROM orders  WHERE channel IS NOT NULL AND channel <> ''
         UNION
         SELECT DISTINCT channel FROM refunds WHERE channel <> ''
         ORDER BY 1",
    )
    .fetch_all(&state.db)
    .await?;

    let mut out: Vec<String> = PRESET_CHANNELS.iter().map(|s| s.to_string()).collect();
    for c in used {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    Ok(Json(out))
}

// ==========================================
// 垫付与结算调整
//
// 这四个写入口**故意不调用 `require_event_open`**（spec 偏离 3）：展会冻结
// 之后仍然允许补录。这是有意的例外，不是漏掉的守卫——看见别处都守着
// 就顺手补上，`advances_and_adjustments_still_work_after_the_event_is_settled`
// 会红。
// ==========================================

/// 名目的长度上限。和渠道名一样，长度不设限的自由文本迟早会有人贴一整段进来。
const MAX_LABEL_CHARS: usize = 50;

fn normalize_label(raw: &str) -> ApiResult<String> {
    let s = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.is_empty() {
        return Err(ApiError::BadRequest("名目不能为空".into()));
    }
    if s.chars().count() > MAX_LABEL_CHARS {
        return Err(ApiError::BadRequest(format!(
            "名目最长 {MAX_LABEL_CHARS} 个字"
        )));
    }
    Ok(s)
}

async fn ensure_society(conn: &mut sqlx::SqliteConnection, society_id: i64) -> ApiResult<()> {
    let ok: Option<i64> = sqlx::query_scalar("SELECT id FROM societies WHERE id = ?")
        .bind(society_id)
        .fetch_optional(&mut *conn)
        .await?;
    ok.map(|_| ())
        .ok_or_else(|| ApiError::BadRequest("社团不存在".into()))
}

/// 只查展会存不存在，**不查状态**。
///
/// 这四个写入口故意跳过 `require_event_open`（spec 偏离 3：冻结后仍然允许增删），
/// 但那也把「展会不存在」这道检查一起跳过了，于是不存在的 id 会一路走到
/// journals.event_id 的外键才炸成 500。这个函数补回 404，不碰冻结语义。
async fn ensure_event_exists(conn: &mut sqlx::SqliteConnection, event_id: i64) -> ApiResult<()> {
    let ok: Option<i64> = sqlx::query_scalar("SELECT id FROM events WHERE id = ?")
        .bind(event_id)
        .fetch_optional(&mut *conn)
        .await?;
    ok.map(|_| ())
        .ok_or_else(|| ApiError::NotFound("展会不存在".into()))
}

#[derive(Deserialize)]
pub struct AdvanceRequest {
    society_id: i64,
    label: String,
    /// 分，必须为正。方向是固定的（摊主掏钱给社团），不需要符号。
    amount: i64,
}

#[derive(Serialize)]
pub struct CreatedEntry {
    id: i64,
    journal_id: i64,
}

async fn create_advance(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<AdvanceRequest>,
) -> ApiResult<(StatusCode, Json<CreatedEntry>)> {
    // 不需要展会守卫：spec 偏离 3——冻结后仍然允许追加垫付。
    // 「回家翻出一张打印费收据」和「回家发现少了一本书」是同一类事件，
    // 母 spec 允许后者却禁止前者说不通。**不要顺手补上 require_event_open**，
    // settlement.rs 的模块注释和 tests 里那条测试都在守这件事。
    check_write_permission(&claims, event_id)?;
    let label = normalize_label(&payload.label)?;
    if payload.amount <= 0 {
        return Err(ApiError::BadRequest("垫付金额必须为正".into()));
    }

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    ensure_event_exists(&mut tx, event_id).await?;
    ensure_society(&mut tx, payload.society_id).await?;

    // 摊主掏钱、社团欠他：`摊主自有 → 社团往来:<社团>`。
    // 这和「摊主自掏赠品」（api/inventory.rs）恰好相反，别抄混。
    let journal_id = post_journal(
        &mut tx,
        event_id,
        JournalKind::Advance,
        None,
        None,
        Some(&label),
        &[],
        &[MoneyLeg {
            from: Account::VendorOwn,
            to: Account::SocietyDue(payload.society_id),
            amount: Money::from_cents(payload.amount),
        }],
    )
    .await?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO advances (event_id, owner_society_id, journal_id, label, amount)
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(event_id)
    .bind(payload.society_id)
    .bind(journal_id)
    .bind(&label)
    .bind(payload.amount)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(CreatedEntry { id, journal_id })))
}

#[derive(Deserialize)]
pub struct AdjustmentRequest {
    society_id: i64,
    label: String,
    /// `to_them` = 我要多给他们；`to_me` = 他们要多给我。
    ///
    /// **界面不给摊主填正负号。** 母 spec 5.2 那个例子自己都要算一遍才对得上
    /// 方向，让人在收摊后的疲惫状态下判断「赔付该填正还是负」是设计失误。
    direction: String,
    /// 分，必须为正。符号由 `direction` 决定。
    amount: i64,
}

async fn create_adjustment(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<AdjustmentRequest>,
) -> ApiResult<(StatusCode, Json<CreatedEntry>)> {
    // 不需要展会守卫：母 spec 6.4 明写这是冻结后唯一允许的操作
    //（spec 偏离 3 又加了垫付和收摊清点）。
    check_write_permission(&claims, event_id)?;
    let label = normalize_label(&payload.label)?;
    if payload.amount <= 0 {
        return Err(ApiError::BadRequest(
            "金额必须为正，方向用 direction 表达".into(),
        ));
    }
    let to_them = match payload.direction.as_str() {
        "to_them" => true,
        "to_me" => false,
        other => {
            return Err(ApiError::BadRequest(format!(
                "方向只能是 to_them 或 to_me，收到「{other}」"
            )))
        }
    };

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    ensure_event_exists(&mut tx, event_id).await?;
    ensure_society(&mut tx, payload.society_id).await?;

    // to_them：往来 −amount ⇒ 我应转给 +amount
    // to_me  ：往来 +amount ⇒ 我应转给 −amount
    let (from, to) = if to_them {
        (
            Account::SocietyDue(payload.society_id),
            Account::SettlementAdj,
        )
    } else {
        (
            Account::SettlementAdj,
            Account::SocietyDue(payload.society_id),
        )
    };
    let journal_id = post_journal(
        &mut tx,
        event_id,
        JournalKind::Adjust,
        None,
        None,
        Some(&label),
        &[],
        &[MoneyLeg {
            from,
            to,
            amount: Money::from_cents(payload.amount),
        }],
    )
    .await?;

    // 存进表里的是**对往来余额的影响**（带符号），和 journal 的方向一致。
    // DB 上有 CHECK (amount <> 0)。
    let signed = if to_them {
        -payload.amount
    } else {
        payload.amount
    };
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO settlement_adjustments (event_id, owner_society_id, journal_id, label, amount)
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(event_id)
    .bind(payload.society_id)
    .bind(journal_id)
    .bind(&label)
    .bind(signed)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(CreatedEntry { id, journal_id })))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct LedgerEntryRow {
    id: i64,
    owner_society_id: i64,
    society_name: String,
    label: String,
    /// 垫付恒为正；结算调整带符号（负 = 我要多给他们）。
    amount: i64,
}

async fn list_entries_of(
    state: &AppState,
    event_id: i64,
    table: &'static str,
) -> ApiResult<Vec<LedgerEntryRow>> {
    let sql = format!(
        "SELECT t.id, t.owner_society_id, s.name AS society_name, t.label, t.amount
         FROM {table} t JOIN societies s ON s.id = t.owner_society_id
         WHERE t.event_id = ? ORDER BY t.id"
    );
    let rows = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(event_id)
        .fetch_all(&state.db)
        .await?;
    Ok(rows)
}

/// 删除 = 冲正 journal + 删业务表那一行。
///
/// 列表保持干净，账本留下「一条原始 + 一条冲正」的痕迹，而
/// 「结算单 = 往来账户余额」这条不变量自动成立——余额本来就是聚合出来的。
async fn delete_entry_of(
    state: &AppState,
    event_id: i64,
    id: i64,
    table: &'static str,
    kind: JournalKind,
) -> ApiResult<StatusCode> {
    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    ensure_event_exists(&mut tx, event_id).await?;

    let sql = format!("SELECT journal_id FROM {table} WHERE id = ? AND event_id = ?");
    let journal_id: Option<i64> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .bind(event_id)
        .fetch_optional(&mut *tx)
        .await?;
    let journal_id = journal_id.ok_or_else(|| ApiError::NotFound("记录不存在".into()))?;

    reverse_journal(&mut tx, journal_id, kind, Some("删除")).await?;

    let sql = format!("DELETE FROM {table} WHERE id = ?");
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_advances(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<Vec<LedgerEntryRow>>> {
    // 不需要展会守卫：只读列表
    check_read_permission(&claims, event_id)?;
    Ok(Json(list_entries_of(&state, event_id, "advances").await?))
}

async fn delete_advance(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, id)): Path<(i64, i64)>,
) -> ApiResult<StatusCode> {
    // 不需要展会守卫：与新增同理（spec 偏离 3）——冻结后仍然允许删除垫付。
    // 这是有意的例外，不要顺手补上 require_event_open。
    check_write_permission(&claims, event_id)?;
    delete_entry_of(&state, event_id, id, "advances", JournalKind::Advance).await
}

async fn list_adjustments(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<Vec<LedgerEntryRow>>> {
    // 不需要展会守卫：只读列表
    check_read_permission(&claims, event_id)?;
    Ok(Json(
        list_entries_of(&state, event_id, "settlement_adjustments").await?,
    ))
}

async fn delete_adjustment(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, id)): Path<(i64, i64)>,
) -> ApiResult<StatusCode> {
    // 不需要展会守卫：与新增同理（spec 偏离 3）——冻结后仍然允许删除结算调整。
    // 这是有意的母 spec 例外，不要顺手补上 require_event_open。
    check_write_permission(&claims, event_id)?;
    delete_entry_of(
        &state,
        event_id,
        id,
        "settlement_adjustments",
        JournalKind::Adjust,
    )
    .await
}

#[cfg(test)]
mod tests {
    use crate::domain::ledger::{account_balance, Account};
    use crate::domain::money::Money;
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn channels_list_starts_with_the_three_presets() {
        let (router, _dir, pool) = test_router_with().await;
        seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                "/api/channels",
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        let list: Vec<String> = serde_json::from_value(body).unwrap();
        assert_eq!(list, vec!["现金", "微信", "支付宝"]);
    }

    #[tokio::test]
    async fn channels_list_includes_history_across_events() {
        // 跨展会才有意义：上一场用过「银行转账」，这一场当然还想用。
        let (router, _dir, pool) = test_router_with().await;
        seed_event_and_product(&pool).await;
        sqlx::query(
            "INSERT INTO orders (event_id, status, channel, gross_amount, solved_amount, final_amount)
             VALUES (1, 'completed', '银行转账', 100, 100, 100)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                "/api/channels",
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        let list: Vec<String> = serde_json::from_value(read_json(res).await).unwrap();
        assert!(list.contains(&"银行转账".to_string()));
        assert_eq!(
            list.iter().filter(|c| *c == "现金").count(),
            1,
            "预置的不能重复出现"
        );
    }

    async fn post_advance(
        router: &axum::Router,
        event_id: i64,
        society_id: i64,
        label: &str,
        amount: i64,
    ) -> serde_json::Value {
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/advances"),
                Some(&admin_token()),
                json!({"society_id": society_id, "label": label, "amount": amount}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        read_json(res).await
    }

    #[tokio::test]
    async fn an_advance_is_a_journal_not_a_note() {
        // 「结算单 = 往来账户的余额」这条不变量成立的前提，就是垫付在账本里。
        // 早先的 schema 草图漏了这个外键，那样结算单就得从两处拼数字。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        let body = post_advance(&router, event_id, 2, "摊位费", 40000).await;
        assert!(body["journal_id"].as_i64().unwrap() > 0);

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(40000),
            "往来 +400 ⇒ 他们欠我 400"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::VendorOwn)
                .await
                .unwrap(),
            Money::from_cents(-40000),
            "摊主自己的口袋少了 400"
        );
    }

    #[tokio::test]
    async fn deleting_an_advance_reverses_it_and_keeps_the_trail() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let id = post_advance(&router, event_id, 2, "打印费", 8000).await["id"]
            .as_i64()
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "DELETE",
                &format!("/api/events/{event_id}/advances/{id}"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::ZERO
        );
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM advances")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 0, "列表保持干净");
        let journals: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journals WHERE kind = '垫付'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(journals, 2, "账本留下「一条垫付 + 一条冲正」");
    }

    #[tokio::test]
    async fn adjustment_direction_decides_the_sign_so_the_user_never_has_to() {
        // 界面上不给摊主填正负号。让人在收摊后的疲惫状态下判断
        // 「赔付该填正还是负」是设计失误。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/adjustments"),
                Some(&admin_token()),
                json!({"society_id": 2, "label": "清点少一本按成本赔", "direction": "to_them",
                       "amount": 2000}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(-2000),
            "「我要多给他们 20」⇒ 往来 −20 ⇒ 我应转给 +20"
        );

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/adjustments"),
                Some(&admin_token()),
                json!({"society_id": 2, "label": "上次多结的尾数", "direction": "to_me",
                       "amount": 500}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(-1500)
        );

        // 账本余额对不代表存进去的符号对——Task 8 读的是这一列。
        // 把 signed 的两个分支对调，上面的余额断言仍然全绿，只有这里会红。
        let amounts: Vec<i64> =
            sqlx::query_scalar("SELECT amount FROM settlement_adjustments ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(
            amounts,
            vec![-2000, 500],
            "存的是对往来余额的影响：to_them 为负、to_me 为正"
        );
    }

    #[tokio::test]
    async fn advances_and_adjustments_still_work_after_the_event_is_settled() {
        // spec 偏离 3：「回家翻出一张打印费收据」和「回家发现少了一本书」
        // 是同一类事件，允许后者却禁止前者说不通。
        //
        // 这条测试同时是给未来的人看的：看见别处都守着 require_event_open
        // 就顺手补上去的话，它会红。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/advances"),
                Some(&admin_token()),
                json!({"society_id": 2, "label": "回家翻出的打印费", "amount": 8000}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED, "垫付在冻结后仍然能补");

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/adjustments"),
                Some(&admin_token()),
                json!({"society_id": 2, "label": "协商赔付", "direction": "to_them", "amount": 2000}),
            ))
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            StatusCode::CREATED,
            "结算调整是母 spec 原有的例外"
        );
    }

    #[tokio::test]
    async fn rejects_nonpositive_amounts_and_blank_labels() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        let cases = [
            (
                "advances",
                json!({"society_id": 2, "label": "x", "amount": 0}),
            ),
            (
                "advances",
                json!({"society_id": 2, "label": "x", "amount": -100}),
            ),
            (
                "advances",
                json!({"society_id": 2, "label": "   ", "amount": 100}),
            ),
            (
                "adjustments",
                json!({"society_id": 2, "label": "x", "direction": "to_them", "amount": 0}),
            ),
            (
                "adjustments",
                json!({"society_id": 2, "label": "x", "direction": "to_me", "amount": -100}),
            ),
            (
                "adjustments",
                json!({"society_id": 2, "label": "   ", "direction": "to_them", "amount": 100}),
            ),
        ];
        for (endpoint, body) in cases {
            let res = router
                .clone()
                .oneshot(json_request(
                    "POST",
                    &format!("/api/events/{event_id}/{endpoint}"),
                    Some(&admin_token()),
                    body,
                ))
                .await
                .unwrap();
            assert_eq!(
                res.status(),
                StatusCode::BAD_REQUEST,
                "{endpoint} 应拒绝非法金额 / 空名目"
            );
        }
    }

    #[tokio::test]
    async fn rejects_an_unknown_society() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        for endpoint in ["advances", "adjustments"] {
            let body = if endpoint == "advances" {
                json!({"society_id": 999, "label": "摊位费", "amount": 100})
            } else {
                json!({"society_id": 999, "label": "赔付", "direction": "to_them", "amount": 100})
            };
            let res = router
                .clone()
                .oneshot(json_request(
                    "POST",
                    &format!("/api/events/{event_id}/{endpoint}"),
                    Some(&admin_token()),
                    body,
                ))
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::BAD_REQUEST, "{endpoint}");
        }
    }

    #[tokio::test]
    async fn rejects_a_label_longer_than_fifty_chars() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        let long = "名".repeat(super::MAX_LABEL_CHARS + 1);
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/advances"),
                Some(&admin_token()),
                json!({"society_id": 2, "label": long, "amount": 100}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rejects_an_unknown_direction() {
        // 界面不给填正负号，direction 就成了一段客户端自己拼的字符串——
        // 拼错不是异常，是最可能发生的事情。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/adjustments"),
                Some(&admin_token()),
                json!({"society_id": 2, "label": "赔付", "direction": "sideways", "amount": 100}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn a_write_to_a_missing_event_is_a_404_not_a_foreign_key_500() {
        // 四个写入口故意跳过 require_event_open，连带把「展会存不存在」也跳过了，
        // 于是不存在的 id 会一路走到 journals.event_id 的外键才炸成 500。
        let (router, _dir, pool) = test_router_with().await;
        seed_event_and_product(&pool).await;
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/events/99999/advances",
                Some(&admin_token()),
                json!({"society_id": 2, "label": "摊位费", "amount": 100}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn the_two_lists_do_not_bleed_into_each_other_or_across_events() {
        // 四个薄 handler 只靠一个手打的表名字面量区分。表名写反、漏掉 event_id 条件、
        // 列别名对不上——三种错测试都看不见，除非真的两张表两场展会都摆上。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        sqlx::query(
            "INSERT INTO events (id, name, event_date, status)
                     VALUES (2, '另一场', '2026-11-01', '进行中')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let token = admin_token();

        post_advance(&router, event_id, 2, "本场摊位费", 40000).await;
        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/adjustments"),
                Some(&token),
                json!({"society_id": 2, "label": "本场赔付", "direction": "to_them", "amount": 2000}),
            ))
            .await
            .unwrap();
        post_advance(&router, 2, 2, "别场摊位费", 11111).await;
        router
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/events/2/adjustments",
                Some(&token),
                json!({"society_id": 2, "label": "别场赔付", "direction": "to_me", "amount": 3333}),
            ))
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/advances"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        let rows = read_json(res).await;
        let rows = rows.as_array().unwrap();
        assert_eq!(rows.len(), 1, "只该看见本场的垫付");
        assert_eq!(rows[0]["label"], "本场摊位费", "串到调整表或别场了");
        assert_eq!(rows[0]["amount"], 40000);
        assert_eq!(rows[0]["owner_society_id"], 2);
        assert_eq!(rows[0]["society_name"], "黄昏堂", "JOIN societies 的列别名");

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/adjustments"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        let rows = read_json(res).await;
        let rows = rows.as_array().unwrap();
        assert_eq!(rows.len(), 1, "只该看见本场的调整");
        assert_eq!(rows[0]["label"], "本场赔付");
        assert_eq!(
            rows[0]["amount"], -2000,
            "to_them 存的是对往来余额的影响，是负数"
        );
    }

    #[tokio::test]
    async fn deleting_an_adjustment_touches_the_adjustment_not_the_advance() {
        // 两张表的 id 都从 1 开始。delete_adjustment 若传错表名，
        // 删调整 #1 会去冲正垫付 #1——两者在同一场展会里同时存在时才暴露得出来。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let adv_id = post_advance(&router, event_id, 2, "摊位费", 40000).await["id"]
            .as_i64()
            .unwrap();
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/adjustments"),
                Some(&token),
                json!({"society_id": 2, "label": "赔付", "direction": "to_them", "amount": 2000}),
            ))
            .await
            .unwrap();
        let adj_id = read_json(res).await["id"].as_i64().unwrap();

        // 往来 = +40000（垫付）− 2000（调整）
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(38000)
        );

        let res = router
            .clone()
            .oneshot(json_request(
                "DELETE",
                &format!("/api/events/{event_id}/adjustments/{adj_id}"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(40000),
            "只该冲掉调整那 2000，垫付原封不动"
        );
        let advances_left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM advances")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(advances_left, 1, "垫付那行不该被删");
        let adjustments_left: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM settlement_adjustments")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(adjustments_left, 0);
        let adjust_journals: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM journals WHERE kind = '调整'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(adjust_journals, 2, "账本留下「一条调整 + 一条冲正」");
        let _ = adv_id;
    }
}
