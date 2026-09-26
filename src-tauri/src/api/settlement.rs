//! 结算侧：渠道列表、垫付、结算调整、结算单、收摊清点、xlsx 导出。
//!
//! **本模块里有三类操作故意不调用 `require_event_open`**：垫付、结算调整、
//! 收摊清点。冻结之后它们仍然允许（spec 偏离 3）——「回家翻出一张打印费收据」
//! 和「回家发现少了一本书」是同一类事件。看见别处都守着就顺手补上去，
//! 会把有意的例外当成漏掉的守卫。改之前先读 spec 3.2。

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use rust_xlsxwriter::{Color, Format, Workbook};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::guard::{check_read_permission, check_write_permission},
    api::openapi::ApiErrorBody,
    domain::{
        channel::{normalize, PRESET_CHANNELS},
        ledger::{home_society_id, post_journal, reverse_journal, Account, JournalKind, MoneyLeg},
        money::Money,
        settlement::{
            build_report, ChannelCount, Entry, GoodsLine, SettlementInput, SettlementReport,
            SocietyInput,
        },
    },
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_channels))
        .routes(routes!(list_advances, create_advance))
        .routes(routes!(delete_advance))
        .routes(routes!(list_adjustments, create_adjustment))
        .routes(routes!(delete_adjustment))
        .routes(routes!(get_settlement))
        .routes(routes!(reconcile))
        .routes(routes!(download_settlement_xlsx))
}

/// 已用过的收款渠道，跨展会。
///
/// 挂在 `/api/channels` 而不是 `/api/events/:id/channels`：它按定义就是跨展会的。
/// 这是防「微信」和「微信支付」分裂成两个账户的那一条（②-1/②-2 交接段第 3 条）。
#[utoipa::path(
    get,
    path = "/channels",
    tag = "settlement",
    security(("bearer" = [])),
    responses(
        (status = 200, body = Vec<String>, description = "预置渠道在前，历史用过的在后，去重"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
    ),
)]
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

#[derive(Deserialize, ToSchema)]
pub struct AdvanceRequest {
    society_id: i64,
    label: String,
    /// 分，必须为正。方向是固定的（摊主掏钱给社团），不需要符号。
    #[schema(value_type = Money)]
    amount: i64,
}

#[derive(Serialize, ToSchema)]
pub struct CreatedEntry {
    id: i64,
    journal_id: i64,
}

/// 登记一笔垫付（摊主替货主掏的钱）。
#[utoipa::path(
    post,
    path = "/events/{event_id}/advances",
    tag = "settlement",
    description = "展会已结算后仍可调用（冻结例外：垫付、结算调整、收摊清点三类）。",
    params(("event_id" = i64, Path, description = "展会 id")),
    request_body = AdvanceRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, body = CreatedEntry),
        (status = 400, body = ApiErrorBody, description = "金额非正、说明为空或过长、社团不存在"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "展会不存在"),
    ),
)]
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

#[derive(Deserialize, ToSchema)]
pub struct AdjustmentRequest {
    society_id: i64,
    label: String,
    /// `to_them` = 我要多给他们；`to_me` = 他们要多给我。
    ///
    /// **界面不给摊主填正负号。** 母 spec 5.2 那个例子自己都要算一遍才对得上
    /// 方向，让人在收摊后的疲惫状态下判断「赔付该填正还是负」是设计失误。
    #[schema(value_type = crate::api::openapi::AdjustmentDirection)]
    direction: String,
    /// 分，必须为正。符号由 `direction` 决定。
    #[schema(value_type = Money)]
    amount: i64,
}

/// 登记一笔结算调整。方向由 `direction` 表达，金额恒为正。
#[utoipa::path(
    post,
    path = "/events/{event_id}/adjustments",
    tag = "settlement",
    description = "展会已结算后仍可调用（冻结例外：垫付、结算调整、收摊清点三类）。",
    params(("event_id" = i64, Path, description = "展会 id")),
    request_body = AdjustmentRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, body = CreatedEntry),
        (status = 400, body = ApiErrorBody, description = "金额非正、方向不认识、说明为空或过长、社团不存在"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "展会不存在"),
    ),
)]
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

#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct LedgerEntryRow {
    id: i64,
    owner_society_id: i64,
    society_name: String,
    label: String,
    /// 垫付恒为正；结算调整带符号（负 = 我要多给他们）。
    #[schema(value_type = Money)]
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

/// 这场展会的垫付列表。
#[utoipa::path(
    get,
    path = "/events/{event_id}/advances",
    tag = "settlement",
    params(("event_id" = i64, Path, description = "展会 id")),
    security(("bearer" = [])),
    responses((status = 200, body = Vec<LedgerEntryRow>), (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),),
)]
async fn list_advances(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<Vec<LedgerEntryRow>>> {
    // 不需要展会守卫：只读列表
    check_read_permission(&claims, event_id)?;
    Ok(Json(list_entries_of(&state, event_id, "advances").await?))
}

/// 删除一笔垫付——写一条冲正 journal，原记录保留。
#[utoipa::path(
    delete,
    path = "/events/{event_id}/advances/{id}",
    tag = "settlement",
    description = "展会已结算后仍可调用（冻结例外：垫付、结算调整、收摊清点三类）。",
    params(
        ("event_id" = i64, Path, description = "展会 id"),
        ("id" = i64, Path, description = "条目 id"),
    ),
    security(("bearer" = [])),
    responses(
        (status = 204, description = "已冲正"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "条目不存在或不属于这场展会"),
        (status = 409, body = ApiErrorBody, description = "已经删过了"),
    ),
)]
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

/// 这场展会的结算调整列表。金额带符号：负 = 我要多给他们。
#[utoipa::path(
    get,
    path = "/events/{event_id}/adjustments",
    tag = "settlement",
    params(("event_id" = i64, Path, description = "展会 id")),
    security(("bearer" = [])),
    responses((status = 200, body = Vec<LedgerEntryRow>), (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),),
)]
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

/// 删除一笔结算调整——写一条冲正 journal，原记录保留。
#[utoipa::path(
    delete,
    path = "/events/{event_id}/adjustments/{id}",
    tag = "settlement",
    description = "展会已结算后仍可调用（冻结例外：垫付、结算调整、收摊清点三类）。",
    params(
        ("event_id" = i64, Path, description = "展会 id"),
        ("id" = i64, Path, description = "条目 id"),
    ),
    security(("bearer" = [])),
    responses(
        (status = 204, description = "已冲正"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "条目不存在或不属于这场展会"),
        (status = 409, body = ApiErrorBody, description = "已经删过了"),
    ),
)]
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

// ==========================================
// 结算单与收摊清点（Task 8）
// ==========================================

#[derive(sqlx::FromRow)]
struct GoodsRow {
    owner_society_id: i64,
    event_product_id: i64,
    product_code: String,
    name: String,
    brought_in: i64,
    taken_back: i64,
    sold: i64,
    gifted: i64,
    scrapped: i64,
    variance: i64,
    on_site: i64,
}

/// 货的全部去向。每一项都从 `stock_movements` 按方向取，**没有任何缓存字段**。
///
/// 「带去」和「带回」是**方向和**而不是净额——结算单上这两个数是分开显示的。
/// 其余四项是**位置净额**，所以退货到损耗会自动让「卖出」减一、「报废」加一，
/// 不需要在这里为退货开分支。
///
/// ⚠️ `variance` **不要取反**：恒等式是
/// `带去 − 带回 = 卖出 + 赠送 + 报废 + 差异 + 现场仓`，盘亏时货走
/// `现场仓 → 差异`，差异位置净额为正、现场仓同时减少，两边同时变才平。
///
/// 这里**不再** `LEFT JOIN journals`。曾经挂着一个
/// `AND j.event_id = ep.event_id`，但 `j` 在 WHERE 里没有谓词，跨展会的
/// journal 行会以 NULL-join 存活下来，那个条件什么都没挡，只会让下一个读
/// 的人以为这里有展会守卫。按展会隔离完全靠下面的 `event_products`
/// （`ep.event_id = ?`），所以删掉它，顺便少一次 join。
const GOODS_SQL: &str = r#"
SELECT ep.owner_society_id, ep.id AS event_product_id, ep.product_code, ep.name,
  COALESCE(SUM(CASE WHEN sm.from_location='外部'   AND sm.to_location='现场仓' THEN sm.qty ELSE 0 END),0) AS brought_in,
  COALESCE(SUM(CASE WHEN sm.from_location='现场仓' AND sm.to_location='外部'   THEN sm.qty ELSE 0 END),0) AS taken_back,
  COALESCE(SUM(CASE WHEN sm.to_location='顾客仓' THEN sm.qty ELSE 0 END)
         - SUM(CASE WHEN sm.from_location='顾客仓' THEN sm.qty ELSE 0 END),0) AS sold,
  COALESCE(SUM(CASE WHEN sm.to_location='赠品'   THEN sm.qty ELSE 0 END)
         - SUM(CASE WHEN sm.from_location='赠品'   THEN sm.qty ELSE 0 END),0) AS gifted,
  COALESCE(SUM(CASE WHEN sm.to_location='损耗'   THEN sm.qty ELSE 0 END)
         - SUM(CASE WHEN sm.from_location='损耗'   THEN sm.qty ELSE 0 END),0) AS scrapped,
  COALESCE(SUM(CASE WHEN sm.to_location='差异'   THEN sm.qty ELSE 0 END)
         - SUM(CASE WHEN sm.from_location='差异'   THEN sm.qty ELSE 0 END),0) AS variance,
  COALESCE(SUM(CASE WHEN sm.to_location='现场仓' THEN sm.qty ELSE 0 END)
         - SUM(CASE WHEN sm.from_location='现场仓' THEN sm.qty ELSE 0 END),0) AS on_site
FROM event_products ep
LEFT JOIN stock_movements sm ON sm.event_product_id = ep.id
WHERE ep.event_id = ?
GROUP BY ep.id
ORDER BY ep.owner_society_id, ep.id
"#;

#[derive(sqlx::FromRow)]
struct MoneyRow {
    owner_society_id: i64,
    gross: i64,
    allocated_net: i64,
    paid_net: i64,
    refund_kept: i64,
}

/// 只算 `orders.status = 'completed'`。
///
/// **不要按「存在收款 journal」来判已收款订单**（②-1 交接段第 1 条）：
/// 全赠品单和被抹成 0 元的单都会让 journal 缺席或为空。按 orders.status 判。
///
/// 这个 `status = 'completed'` 还挡住了「已取消订单的退货被重复计入」：
/// 取消一张完成单时退货 journal 被冲正、`refunds` 行却还在，而 cancelled 的行
/// 整条不出现（连带它的 refunds 子查询），所以不会扣第二遍。
const MONEY_SQL: &str = r#"
SELECT ep.owner_society_id,
  COALESCE(SUM(ol.unit_price * (ol.qty - COALESCE(r.done_qty, 0))), 0)        AS gross,
  COALESCE(SUM(ol.allocated_amount - COALESCE(r.done_alloc, 0)), 0)           AS allocated_net,
  COALESCE(SUM(ol.paid_amount      - COALESCE(r.done_paid, 0)), 0)            AS paid_net,
  COALESCE(SUM(COALESCE(r.done_paid, 0) - COALESCE(r.done_refund, 0)), 0)     AS refund_kept
FROM order_lines ol
JOIN orders o          ON o.id = ol.order_id AND o.status = 'completed'
JOIN event_products ep ON ep.id = ol.event_product_id
LEFT JOIN (
  SELECT order_line_id,
         SUM(qty)               AS done_qty,
         SUM(allocated_amount)  AS done_alloc,
         SUM(paid_amount)       AS done_paid,
         SUM(refund_amount)     AS done_refund
  FROM refunds GROUP BY order_line_id
) r ON r.order_line_id = ol.id
WHERE o.event_id = ?
GROUP BY ep.owner_society_id
"#;

/// `MONEY_SQL` 的同形查询，但按 `event_product_id` 分组——「货主明细」sheet 要按商品
/// 给出原价 / 折让 / 净额。
///
/// `load_input` 里按社团分组的 `MONEY_SQL` **不能删**：`SocietyInput` 的
/// `gross` / `allocated_net` 仍从它来。两段查询各有各的分组，不要用一段替掉两段——
/// 按商品分组再在 Rust 里加总，会和按社团分组的取整结果出现分歧。
const MONEY_BY_PRODUCT_SQL: &str = r#"
SELECT ol.event_product_id,
  COALESCE(SUM(ol.unit_price * (ol.qty - COALESCE(r.done_qty, 0))), 0)        AS gross,
  COALESCE(SUM(ol.allocated_amount - COALESCE(r.done_alloc, 0)), 0)           AS allocated_net
FROM order_lines ol
JOIN orders o          ON o.id = ol.order_id AND o.status = 'completed'
JOIN event_products ep ON ep.id = ol.event_product_id
LEFT JOIN (
  SELECT order_line_id,
         SUM(qty)               AS done_qty,
         SUM(allocated_amount)  AS done_alloc,
         SUM(paid_amount)       AS done_paid,
         SUM(refund_amount)     AS done_refund
  FROM refunds GROUP BY order_line_id
) r ON r.order_line_id = ol.id
WHERE o.event_id = ?
GROUP BY ol.event_product_id
"#;

#[derive(sqlx::FromRow)]
struct MoneyByProductRow {
    event_product_id: i64,
    gross: i64,
    allocated_net: i64,
}

/// 摊主自掏赠品：kind = `赠送` 的 journal 里 `社团往来:X ↔ 摊主自有` 的净额。
/// 冲正条目的 kind 也是 `赠送`（`api/inventory.rs` 原样传回去），方向相反，
/// 所以净额天然把撤销掉的那些抵消了。
///
/// 注意两个 `?` 要**绑两次同一个账户名**——sqlx 的 SQLite 驱动按位置取参，
/// 没有重复编号参数。
const GIFT_SELF_PAID_SQL: &str = r#"
SELECT COALESCE(SUM(CASE WHEN mm.to_account   = '摊主自有' THEN mm.amount ELSE 0 END)
              - SUM(CASE WHEN mm.from_account = '摊主自有' THEN mm.amount ELSE 0 END), 0)
FROM money_movements mm
JOIN journals j ON j.id = mm.journal_id
WHERE j.event_id = ? AND j.kind = '赠送'
  AND (mm.from_account = ? OR mm.to_account = ?)
"#;

/// 对账差异对某个渠道的影响。清点写的是 `实收-<渠道> ↔ 对账差异`，
/// 所以「账面应有」= 当前余额 − 这个差额。
const RECON_DIFF_SQL: &str = r#"
SELECT COALESCE(SUM(CASE WHEN mm.to_account   = ? THEN mm.amount ELSE 0 END)
              - SUM(CASE WHEN mm.from_account = ? THEN mm.amount ELSE 0 END), 0)
FROM money_movements mm
JOIN journals j ON j.id = mm.journal_id
WHERE j.event_id = ?
  AND (mm.from_account = '对账差异' OR mm.to_account = '对账差异')
"#;

#[derive(sqlx::FromRow)]
struct AdvanceRow {
    owner_society_id: i64,
    label: String,
    amount: i64,
}

#[derive(sqlx::FromRow)]
struct AdjustmentRow {
    owner_society_id: i64,
    label: String,
    amount: i64,
    created_at: String,
}

/// 本场展会「用过」的全部渠道。**报表的 `channels[]` 和 `reconcile` 的校验
/// （白名单 + 收全量）用的是同一条查询**，由构造保证同步。
///
/// 以前 `reconcile` 只查 `orders ∪ refunds`，于是理论上存在一个渠道报表列得出来、
/// 清点却拒收：管理端会照着 `channels[]` 要摊主填这一格，提交时后端 400。
/// 两处各写一份清单迟早分歧，所以抽成这一条。
///
/// 还要并上本展会 `money_movements` 里出现过的所有 `实收-` 账户：报表的
/// actual_total / vendor_retained 只对这份清单求和，如果哪个别的路径往一个
/// 出人意料的实收账户写了腿而这里没列出，那笔钱就会在结算单上彻底隐形。
/// `substr(..., 4)` 从第 4 个字符起，正好剥掉 3 个字符的前缀 `实收-`。
const EVENT_CHANNELS_SQL: &str = r#"
SELECT channel FROM (
    SELECT channel FROM orders
     WHERE event_id = ? AND channel IS NOT NULL AND channel <> ''
    UNION
    SELECT r.channel AS channel FROM refunds r
      JOIN order_lines ol ON ol.id = r.order_line_id
      JOIN orders o ON o.id = ol.order_id
     WHERE o.event_id = ? AND r.channel <> ''
    UNION
    SELECT substr(mm.to_account, 4) AS channel
      FROM money_movements mm JOIN journals j ON j.id = mm.journal_id
     WHERE j.event_id = ? AND mm.to_account LIKE '实收-%'
    UNION
    SELECT substr(mm.from_account, 4) AS channel
      FROM money_movements mm JOIN journals j ON j.id = mm.journal_id
     WHERE j.event_id = ? AND mm.from_account LIKE '实收-%'
 ) WHERE channel <> '' ORDER BY 1
"#;

/// 跑 `EVENT_CHANNELS_SQL`。泛型到 executor，好让 `load_input`（池）和
/// `reconcile`（事务）都用同一条 SQL、同一段代码。
async fn event_channels<'e, E>(executor: E, event_id: i64) -> ApiResult<Vec<String>>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let names = sqlx::query_scalar(EVENT_CHANNELS_SQL)
        .bind(event_id)
        .bind(event_id)
        .bind(event_id)
        .bind(event_id)
        .fetch_all(executor)
        .await?;
    Ok(names)
}

/// 渠道名的**比对键**：优先用 `normalize`（折叠内部空白 + 限长）。历史库里可能
/// 存着超过 `MAX_CHANNEL_CHARS` 的名字，`normalize` 会因此报错——那种值退回原样
/// 参与比对：它已经是一个既存账户名了，再拿「最长 20 字」去卡它只会让那场展会
/// 永远清不了点。真正的重复/白名单判断仍在下游完成。
fn channel_key(raw: &str) -> String {
    normalize(raw).unwrap_or_else(|_| raw.to_string())
}

/// 把各张表查出来，组装成 `build_report` 要的输入。
///
/// 页面 JSON 和 `settlement.xlsx` 都走这里——「页面显示 1,170、导出写 1,150」
/// 在结构上不可能发生。
///
/// **整个函数包在一个读事务里**（`begin()`，deferred 即可）：下面有 ~2 + 2N 条
/// 查询，池上逐条跑的时候一次并发写会落在两条查询之间，读出一份内部不自洽的输入，
/// 于是 `build_report` 打出「结算对不上账本」的警告——而这一页的职责恰恰是可信。
/// 读事务让所有查询走同一条连接、看同一个快照，顺带把逐社团的 N+1 也收进同一视图。
pub(crate) async fn load_input(state: &AppState, event_id: i64) -> ApiResult<SettlementInput> {
    let mut tx = state.db.begin().await?;
    let event: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT name, event_date, stocktaken_at, reconciled_at FROM events WHERE id = ?",
    )
    .bind(event_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (event_name, event_date, stocktaken_at, reconciled_at) =
        event.ok_or_else(|| ApiError::NotFound("展会不存在".into()))?;

    // 本社团：手工折让整笔落在它头上，`is_home` 也靠它判定。
    let home_id = home_society_id(&mut tx).await?;

    let goods_rows: Vec<GoodsRow> = sqlx::query_as(GOODS_SQL)
        .bind(event_id)
        .fetch_all(&mut *tx)
        .await?;

    let money_rows: Vec<MoneyRow> = sqlx::query_as(MONEY_SQL)
        .bind(event_id)
        .fetch_all(&mut *tx)
        .await?;

    // 按商品分组的金额，只喂给「货主明细」sheet。一段查询一个分组，和上面
    // 按社团分组的 `money_rows` 并存，谁也不替谁。
    let product_money_rows: Vec<MoneyByProductRow> = sqlx::query_as(MONEY_BY_PRODUCT_SQL)
        .bind(event_id)
        .fetch_all(&mut *tx)
        .await?;

    // 手工折让 = solved − final，摊进各行之后就是 Σallocated − Σpaid。
    // 母 spec 4.4 定了它由本社团**全额**承担，所以整个数落在本社团头上，
    // 不按货主分。代卖货主身上出现非零手工折让，domain/settlement.rs 会报警。
    let manual_discount_net: i64 = money_rows
        .iter()
        .map(|m| m.allocated_net - m.paid_net)
        .sum();

    let advance_rows: Vec<AdvanceRow> = sqlx::query_as(
        "SELECT owner_society_id, label, amount FROM advances WHERE event_id = ? ORDER BY id",
    )
    .bind(event_id)
    .fetch_all(&mut *tx)
    .await?;

    let adjustment_rows: Vec<AdjustmentRow> = sqlx::query_as(
        "SELECT owner_society_id, label, amount, created_at
         FROM settlement_adjustments WHERE event_id = ? ORDER BY id",
    )
    .bind(event_id)
    .fetch_all(&mut *tx)
    .await?;

    // 没有任何交易的社团也要出现（有垫付但一件货没卖是合法的）。本社团也永远
    // 出现：手工折让和调整都可能落在它头上，哪怕它一件货都没带。
    let society_rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT DISTINCT s.id, s.name
         FROM societies s
         WHERE s.id = ?
            OR s.id IN (SELECT owner_society_id FROM event_products WHERE event_id = ?)
            OR s.id IN (SELECT owner_society_id FROM advances        WHERE event_id = ?)
            OR s.id IN (SELECT owner_society_id FROM settlement_adjustments WHERE event_id = ?)
         ORDER BY s.id",
    )
    .bind(home_id)
    .bind(event_id)
    .bind(event_id)
    .bind(event_id)
    .fetch_all(&mut *tx)
    .await?;

    let mut societies = Vec::with_capacity(society_rows.len());
    for (society_id, name) in society_rows {
        let is_home = society_id == home_id;

        let goods: Vec<GoodsLine> = goods_rows
            .iter()
            .filter(|g| g.owner_society_id == society_id)
            .map(|g| {
                // 没有任何销售的商品在这段查询里不出现，三个字段都是 0。
                let money = product_money_rows
                    .iter()
                    .find(|m| m.event_product_id == g.event_product_id);
                let gross = Money::from_cents(money.map(|m| m.gross).unwrap_or(0));
                let allocated = Money::from_cents(money.map(|m| m.allocated_net).unwrap_or(0));
                GoodsLine {
                    event_product_id: g.event_product_id,
                    product_code: g.product_code.clone(),
                    name: g.name.clone(),
                    brought_in: g.brought_in,
                    sold: g.sold,
                    gifted: g.gifted,
                    scrapped: g.scrapped,
                    variance: g.variance,
                    taken_back: g.taken_back,
                    on_site: g.on_site,
                    gross,
                    lot_discount: gross - allocated,
                    allocated,
                }
            })
            .collect();

        let money = money_rows.iter().find(|m| m.owner_society_id == society_id);
        let account = Account::SocietyDue(society_id);

        // 账户名一律用 Account 生成，不手工拼「社团往来:」——前缀常量在 ledger.rs。
        let account_name = account.to_string();
        let gift_self_paid: i64 = sqlx::query_scalar(GIFT_SELF_PAID_SQL)
            .bind(event_id)
            .bind(&account_name)
            .bind(&account_name)
            .fetch_one(&mut *tx)
            .await?;

        let advances = advance_rows
            .iter()
            .filter(|a| a.owner_society_id == society_id)
            .map(|a| Entry {
                label: a.label.clone(),
                // advances.amount 存正数（对往来余额 +amount）。对「我应转给」的
                // 影响是 −amount，而 SocietyBlock 里 advances_total 本来就是减项，
                // 所以 Entry 存正数即可。
                amount: Money::from_cents(a.amount),
                // advances 表没有 created_at，迁移已冻结。
                at: None,
            })
            .collect();

        let adjustments = adjustment_rows
            .iter()
            .filter(|a| a.owner_society_id == society_id)
            .map(|a| Entry {
                label: a.label.clone(),
                // DB 存的是对往来余额的影响（负 = 我要多给他们），对
                // 「我应转给」的影响正好相反，所以取负。
                amount: Money::from_cents(-a.amount),
                at: Some(a.created_at.clone()),
            })
            .collect();

        let due_balance = account_balance_tx(&mut tx, event_id, &account).await?;

        societies.push(SocietyInput {
            society_id,
            name,
            is_home,
            goods,
            gross: Money::from_cents(money.map(|m| m.gross).unwrap_or(0)),
            allocated_net: Money::from_cents(money.map(|m| m.allocated_net).unwrap_or(0)),
            manual_discount_net: Money::from_cents(if is_home { manual_discount_net } else { 0 }),
            refund_kept: Money::from_cents(money.map(|m| m.refund_kept).unwrap_or(0)),
            gift_self_paid: Money::from_cents(gift_self_paid),
            advances,
            adjustments,
            due_balance,
        });
    }

    // 渠道清单 = 本展会实际用过的（orders ∪ refunds ∪ 本场 `实收-` 账户），
    // 按展会过滤，不是跨展会的 `GET /channels`。与 `reconcile` 的校验共用
    // `event_channels`，两处由构造保证同步。
    let channel_names = event_channels(&mut *tx, event_id).await?;

    let mut channels = Vec::with_capacity(channel_names.len());
    for channel in channel_names {
        let account = Account::Received(channel.clone());
        let account_name = account.to_string();
        let current = account_balance_tx(&mut tx, event_id, &account).await?;
        let recon_diff: i64 = sqlx::query_scalar(RECON_DIFF_SQL)
            .bind(&account_name)
            .bind(&account_name)
            .bind(event_id)
            .fetch_one(&mut *tx)
            .await?;
        let book = current - Money::from_cents(recon_diff);
        // 清点只记「数过了」这一个全局事实（events.reconciled_at）；数是现场
        // 的，存在账本差额里，所以已清点渠道的 actual 就是当前余额。
        let actual = if reconciled_at.is_some() {
            Some(current)
        } else {
            None
        };
        channels.push(ChannelCount {
            channel,
            book,
            actual,
        });
    }

    let last_changed_at: Option<String> =
        sqlx::query_scalar("SELECT MAX(occurred_at) FROM journals WHERE event_id = ?")
            .bind(event_id)
            .fetch_one(&mut *tx)
            .await?;

    // 读事务到此结束。deferred 事务的 commit 只是释放快照/连接，不改任何数据。
    tx.commit().await?;

    Ok(SettlementInput {
        event_name,
        event_date,
        // 和 last_changed_at / 调整的 at 同一个钟：SQLite CURRENT_TIMESTAMP 是 UTC。
        // 接口里一律 UTC，由前端 formatTimestamp、xlsx 的 local_display 各自转本地显示。
        generated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        last_changed_at,
        stocktaken_at,
        societies,
        channels,
    })
}

/// 结算单。页面与 xlsx 导出渲染的是同一个 `SettlementReport`。
#[utoipa::path(
    get,
    path = "/events/{event_id}/settlement",
    tag = "settlement",
    params(("event_id" = i64, Path, description = "展会 id")),
    security(("bearer" = [])),
    responses(
        (status = 200, body = SettlementReport),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "展会不存在"),
    ),
)]
async fn get_settlement(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<SettlementReport>> {
    // 不需要展会守卫：只读
    check_read_permission(&claims, event_id)?;
    let input = load_input(&state, event_id).await?;
    Ok(Json(build_report(&input)))
}

#[derive(Deserialize, ToSchema)]
pub struct ReconcileRequest {
    counts: Vec<ChannelActual>,
}

#[derive(Deserialize, ToSchema)]
pub struct ChannelActual {
    channel: String,
    /// 摊主数出来的实际到手（分）。
    #[schema(value_type = Money)]
    actual: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ReconcileResponse {
    /// 分文不差时为 `None`——没有腿可记，「数过了」靠 `events.reconciled_at`。
    journal_id: Option<i64>,
}

/// 事务内版的账户余额。签名和 `ledger::account_balance` 一致，只是换了 executor。
/// 清点必须「读到的余额就是待会儿要写差额的那个余额」。
///
/// （`ledger::account_balance` 那条 SQL 用的是 `?1`/`?2` 编号参数，照抄即可——
/// 它已经在生产里跑了两轮，不要顺手改写。）
async fn account_balance_tx(
    conn: &mut sqlx::SqliteConnection,
    event_id: i64,
    account: &Account,
) -> ApiResult<Money> {
    let name = account.to_string();
    let cents: i64 = sqlx::query_scalar(
        "SELECT COALESCE(
                  SUM(CASE WHEN mm.to_account   = ?1 THEN mm.amount ELSE 0 END)
                - SUM(CASE WHEN mm.from_account = ?1 THEN mm.amount ELSE 0 END), 0)
         FROM money_movements mm
         JOIN journals j ON j.id = mm.journal_id
         WHERE j.event_id = ?2",
    )
    .bind(&name)
    .bind(event_id)
    .fetch_one(&mut *conn)
    .await?;
    Ok(Money::from_cents(cents))
}

/// 收摊清点：按渠道填实际到手，差额记到摊主名下。
#[utoipa::path(
    post,
    path = "/events/{event_id}/settlement/reconcile",
    tag = "settlement",
    description = "展会已结算后仍可调用（冻结例外：垫付、结算调整、收摊清点三类）。必须覆盖这场展会用过的每个渠道，且每个渠道只出现一次。",
    params(("event_id" = i64, Path, description = "展会 id")),
    request_body = ReconcileRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, body = ReconcileResponse),
        (status = 400, body = ApiErrorBody, description = "渠道重复、漏报、或不是这场展会用过的渠道"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "展会不存在"),
    ),
)]
async fn reconcile(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<ReconcileRequest>,
) -> ApiResult<Json<ReconcileResponse>> {
    // 不需要展会守卫：spec 偏离 3——回家才有空数现金盒、对微信账单是常态。
    // 对账差异默认由摊主自吞，不进任何货主的结算，只影响「摊主留存」那一个数。
    check_write_permission(&claims, event_id)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    // 补齐「展会不存在」的 404：跳过 require_event_open 不等于允许对着空气记账。
    ensure_event_exists(&mut tx, event_id).await?;

    // 每条都在任何腿落库之前读余额，所以重复一次就把同一笔差额写两遍。
    // 必须在 normalize 之后判重——"现金" 和 " 现金 " 会折叠成同一个账户。
    // 同 api/closing.rs 的 stocktake。

    // 本场用过的渠道。清点一个本场没出现过的渠道，钱会写进账本却不出现在报表里。
    // **和报表的 `channels[]` 共用同一条 `EVENT_CHANNELS_SQL`**——两处各写一份
    // 迟早分歧，那时管理端会照着报表要摊主填一个清点拒收的渠道。
    //
    // 比对用**规范化后的 key**：历史库里可能有没经过 normalize 的值（本分支之前
    // 后端只 trim，内部多空格、超长都进得来），而报表把存储原样显示给摊主、摊主
    // 照抄提交，`normalize` 会把它折叠成另一个字符串。两边 key 不同的话，那个渠道
    // 既过不了白名单、也过不了收全量，整场展会永远清不了点。
    //
    // 但读余额 / 记腿必须落回**账本里真正存的那个原样名字**：渠道是 `实收-<名字>`
    // 这个字符串账户的一部分，规范化后的名字是另一个账户（多半余额为 0）。
    let used_raw = event_channels(&mut *tx, event_id).await?;
    let mut routes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for raw in &used_raw {
        routes
            .entry(channel_key(raw))
            .or_insert_with(|| raw.clone());
    }

    let mut seen = std::collections::HashSet::new();
    for c in &payload.counts {
        if !seen.insert(channel_key(&c.channel)) {
            return Err(ApiError::BadRequest(
                "同一个渠道在一次清点里只能报一次数".into(),
            ));
        }
    }

    let mut unknown: Vec<&String> = seen.iter().filter(|k| !routes.contains_key(*k)).collect();
    unknown.sort();
    if !unknown.is_empty() {
        let names: Vec<&str> = unknown.iter().map(|s| s.as_str()).collect();
        return Err(ApiError::BadRequest(format!(
            "「{}」这场展会没用过，无法清点。本场用过的渠道：{}",
            names.join("、"),
            used_raw.join("、")
        )));
    }

    // 收全量，和 `stocktake` 一字不差：「我数了，一致」和「我没数这个」是两件事。
    // 请求体不把它们混成同一个缺省，全局的 `reconciled_at` 才能代表「全都数过了」。
    // 沿用 `used_raw` 的顺序，报错里点名漏了哪几个渠道时是稳定的。
    let missing: Vec<&String> = used_raw
        .iter()
        .filter(|raw| !seen.contains(&channel_key(raw)))
        .collect();
    if !missing.is_empty() {
        let names: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
        return Err(ApiError::BadRequest(format!(
            "这些本场用过的渠道没报数：{}。数过一致的也要报，否则分不出「数了一致」和「没数」",
            names.join("、")
        )));
    }

    let mut money = Vec::new();
    for c in &payload.counts {
        let key = channel_key(&c.channel);
        // 白名单已经确认这个 key 在本场用过，取回账本里的原样名字。
        let raw = routes
            .get(&key)
            .expect("白名单已确认每个提交的渠道都在本场用过");
        let account = Account::Received(raw.clone());
        let current = account_balance_tx(&mut tx, event_id, &account).await?;
        // **允许 actual 为负**：母 spec §5.4 的「微信收、现金退」很常见，某渠道的
        // 退款超过收款时，摊主手上那个渠道实际是负持仓（他自掏了钱出去）。以前
        // 拒收负数，摊主只能填 0，那笔钱被算进对账差异、方向还抬高 actual_total，
        // 把「摊主留存」虚报。post_journal 不受影响：金额取绝对值、方向由 from/to 表达。
        let diff = Money::from_cents(c.actual) - current;
        if diff.is_zero() {
            continue; // 分文不差——没有腿可记，靠 reconciled_at 记「数过了」
        }
        // 账面多于实际 ⇒ 钱少了 ⇒ 实收流向对账差异；反之亦然。
        let (from, to) = if diff.is_negative() {
            (account.clone(), Account::ReconDiff)
        } else {
            (Account::ReconDiff, account.clone())
        };
        money.push(MoneyLeg {
            from,
            to,
            amount: diff.abs(),
        });
    }

    // 全都对得上就没有腿，post_journal 会拒绝空 journal，所以判空跳过。
    let journal_id = if money.is_empty() {
        None
    } else {
        Some(
            post_journal(
                &mut tx,
                event_id,
                JournalKind::Adjust,
                None,
                None,
                Some("收摊清点"),
                &[],
                &money,
            )
            .await?,
        )
    };

    sqlx::query("UPDATE events SET reconciled_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(event_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(ReconcileResponse { journal_id }))
}

// ==========================================
// 四 sheet 的 xlsx 导出（Task 9）
//
// 四个 sheet 全部从 Task 8 的 `load_input` + `build_report` 出发，**不另写一套算术**。
// 唯一的例外是「账本流水」「订单明细」两个纯流水 sheet，它们读的是业务表本身。
// ==========================================

#[derive(sqlx::FromRow)]
struct JournalRow {
    id: i64,
    occurred_at: String,
    kind: String,
    order_id: Option<i64>,
    note: Option<String>,
}

#[derive(sqlx::FromRow)]
struct StockLegRow {
    journal_id: i64,
    product_code: String,
    name: String,
    from_location: String,
    to_location: String,
    qty: i64,
}

#[derive(sqlx::FromRow)]
struct MoneyLegRow {
    journal_id: i64,
    from_account: String,
    to_account: String,
    amount: i64,
}

#[derive(sqlx::FromRow)]
struct OrderLineRow {
    order_id: i64,
    status: String,
    channel: Option<String>,
    line_id: i64,
    product_code: String,
    name: String,
    lot_name: Option<String>,
    qty: i64,
    unit_price: i64,
    allocated_amount: i64,
    paid_amount: i64,
    refunded_qty: i64,
}

/// 结算单导出为四 sheet 的 xlsx。
#[utoipa::path(
    get,
    path = "/events/{event_id}/settlement.xlsx",
    tag = "settlement",
    params(("event_id" = i64, Path, description = "展会 id")),
    security(("bearer" = [])),
    responses(
        (status = 200, content_type = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", body = Vec<u8>),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "展会不存在"),
    ),
)]
async fn download_settlement_xlsx(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Response> {
    // 不需要展会守卫：只读导出
    check_read_permission(&claims, event_id)?;

    let input = load_input(&state, event_id).await?;
    let report = build_report(&input);

    let journals: Vec<JournalRow> = sqlx::query_as(
        "SELECT id, occurred_at, kind, order_id, note
         FROM journals WHERE event_id = ? ORDER BY id",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    let stock_legs: Vec<StockLegRow> = sqlx::query_as(
        "SELECT sm.journal_id, ep.product_code, ep.name,
                sm.from_location, sm.to_location, sm.qty
         FROM stock_movements sm
         JOIN journals j ON j.id = sm.journal_id
         JOIN event_products ep ON ep.id = sm.event_product_id
         WHERE j.event_id = ?
         ORDER BY sm.journal_id, sm.id",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    let money_legs: Vec<MoneyLegRow> = sqlx::query_as(
        "SELECT mm.journal_id, mm.from_account, mm.to_account, mm.amount
         FROM money_movements mm
         JOIN journals j ON j.id = mm.journal_id
         WHERE j.event_id = ?
         ORDER BY mm.journal_id, mm.id",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    let order_lines: Vec<OrderLineRow> = sqlx::query_as(
        "SELECT o.id AS order_id, o.status, o.channel,
                ol.id AS line_id, ep.product_code, ep.name,
                olot.name AS lot_name, ol.qty, ol.unit_price,
                ol.allocated_amount, ol.paid_amount,
                COALESCE(r.done_qty, 0) AS refunded_qty
         FROM order_lines ol
         JOIN orders o ON o.id = ol.order_id
         JOIN event_products ep ON ep.id = ol.event_product_id
         LEFT JOIN order_lots olot ON olot.id = ol.order_lot_id
         LEFT JOIN (
             SELECT order_line_id, SUM(qty) AS done_qty
             FROM refunds GROUP BY order_line_id
         ) r ON r.order_line_id = ol.id
         WHERE o.event_id = ?
         ORDER BY o.id, ol.id",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    // rust_xlsxwriter 的错误不是 sqlx::Error，给 ApiError 加变体不在本轮范围。
    // 用 Conflict 语义不完美，但诚实：文案能到用户眼里，而不是被伪装成数据库错误。
    let buf = build_workbook(&report, &journals, &stock_legs, &money_legs, &order_lines)
        .map_err(|e| ApiError::Conflict(format!("生成表格失败：{e}")))?;

    // 文件名纯 ASCII。展会名可能是中文，不要放进 filename。
    Ok((
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"settlement-event-{event_id}.xlsx\""),
            ),
        ],
        buf,
    )
        .into_response())
}

fn build_workbook(
    report: &SettlementReport,
    journals: &[JournalRow],
    stock_legs: &[StockLegRow],
    money_legs: &[MoneyLegRow],
    order_lines: &[OrderLineRow],
) -> Result<Vec<u8>, rust_xlsxwriter::XlsxError> {
    let mut workbook = Workbook::new();
    write_summary_sheet(&mut workbook, report)?;
    write_goods_sheet(&mut workbook, report)?;
    write_journal_sheet(&mut workbook, journals, stock_legs, money_legs)?;
    write_order_sheet(&mut workbook, order_lines)?;
    workbook.save_to_buffer()
}

/// 接口里的时间是 UTC 的 `YYYY-MM-DD HH:MM:SS`（SQLite `CURRENT_TIMESTAMP` 的格式）；
/// xlsx 在摊主这台机器上生成、给人看，转成本机时区。解析不了就原样写出。
fn local_display(utc: &str) -> String {
    chrono::NaiveDateTime::parse_from_str(utc, "%Y-%m-%d %H:%M:%S")
        .map(|t| {
            t.and_utc()
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| utc.to_string())
}

/// 金额一律写成**元的浮点**并套两位小数的 number format——xlsx 是给人看和给
/// 社团财务再加工的，写分会让每个数都要除 100。这是唯一允许把 Money 变成浮点的
/// 地方，因为它离开系统了。
fn money_format() -> Format {
    Format::new().set_num_format("0.00")
}

/// 写一个「分」金额到单元格（元、两位小数）。**只有这里允许 `as f64 / 100.0`。**
fn write_money(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    money: Money,
    format: &Format,
) -> Result<(), rust_xlsxwriter::XlsxError> {
    worksheet.write_number_with_format(row, col, money.cents() as f64 / 100.0, format)?;
    Ok(())
}

fn write_summary_sheet(
    workbook: &mut Workbook,
    report: &SettlementReport,
) -> Result<(), rust_xlsxwriter::XlsxError> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("结算汇总")?;

    let header = Format::new().set_bold();
    let money_fmt = money_format();
    let mut row: u32 = 0;

    // 导出的工作簿要认得出是哪场展会、手里这份是不是最新的。spec 8.1 的
    // `last_changed_at` 正是「冻结后仍可追加垫付/调整/清点」的补救措施——
    // 不写出来，那条补救就丢了。
    worksheet.write_string_with_format(
        row,
        0,
        format!("{} · {} · 结算单", report.event_name, report.event_date),
        &header,
    )?;
    row += 1;
    let mut meta = format!(
        "生成于 {}；最后更新于 {}",
        local_display(&report.generated_at),
        report
            .last_changed_at
            .as_deref()
            .map(local_display)
            .unwrap_or_else(|| "（无记录）".to_string())
    );
    if !report.stocktaken {
        meta.push_str("；未盘点，剩余数为账面推算");
    }
    worksheet.write_string_with_format(row, 0, meta, &header)?;
    row += 2; // 空一行再接下面

    // warnings 非空时，在第一行写一条醒目的红色提示。结算单自相矛盾却安安静静地
    // 导出去，比不导出更坏。
    if !report.warnings.is_empty() {
        let warn = Format::new().set_bold().set_font_color(Color::Red);
        worksheet.write_string_with_format(row, 0, "⚠ 这张表和账本对不上，见下方说明", &warn)?;
        row += 1;
        for w in &report.warnings {
            worksheet.write_string_with_format(row, 0, w, &warn)?;
            row += 1;
        }
        row += 1;
    }

    let headers = [
        "货主",
        "商品原价",
        "套装折让",
        "手工折让",
        "净额",
        "退货保留",
        "自掏赠品",
        "垫付",
        "调整",
        "我应转给",
        "备注",
    ];
    for (c, h) in headers.iter().enumerate() {
        worksheet.write_string_with_format(row, c as u16, *h, &header)?;
    }
    row += 1;
    for s in &report.societies {
        worksheet.write_string(row, 0, s.name.as_str())?;
        let values = [
            s.gross,
            s.lot_discount,
            s.manual_discount,
            s.net,
            s.refund_kept,
            s.gift_self_paid,
            s.advances_total,
            s.adjustments_total,
            s.transfer,
        ];
        for (i, m) in values.iter().enumerate() {
            write_money(worksheet, row, 1 + i as u16, *m, &money_fmt)?;
        }
        row += 1;

        // spec 8.2 要求「【我垫付】/【调整】逐条列名目与金额」。社团看到
        // 「垫付 400.00」却不知道是什么，正是母 spec 5.3 要避免的。
        // 金额写进对应的合计列，调整的日期放「备注」列。
        for a in &s.advances {
            worksheet.write_string(row, 0, format!("  垫付：{}", a.label))?;
            write_money(worksheet, row, 7, a.amount, &money_fmt)?;
            row += 1;
        }
        for a in &s.adjustments {
            worksheet.write_string(row, 0, format!("  调整：{}", a.label))?;
            write_money(worksheet, row, 8, a.amount, &money_fmt)?;
            if let Some(at) = &a.at {
                worksheet.write_string(row, 10, local_display(at))?;
            }
            row += 1;
        }
    }

    row += 1; // 空一行，下面是收摊清点栏

    let channel_headers = ["渠道", "账面应有", "实际到手", "差额", "是否清点"];
    for (c, h) in channel_headers.iter().enumerate() {
        worksheet.write_string_with_format(row, c as u16, *h, &header)?;
    }
    row += 1;
    for c in &report.channels {
        worksheet.write_string(row, 0, c.channel.as_str())?;
        write_money(worksheet, row, 1, c.book, &money_fmt)?;
        write_money(worksheet, row, 2, c.actual, &money_fmt)?;
        write_money(worksheet, row, 3, c.diff, &money_fmt)?;
        worksheet.write_string(row, 4, if c.counted { "是" } else { "否" })?;
        row += 1;
    }

    row += 1;
    for (label, value) in [
        ("实际到手合计", report.actual_total),
        ("Σ我应转给", report.transfer_total),
        ("摊主留存", report.vendor_retained),
    ] {
        worksheet.write_string(row, 0, label)?;
        write_money(worksheet, row, 1, value, &money_fmt)?;
        row += 1;
    }

    Ok(())
}

fn write_goods_sheet(
    workbook: &mut Workbook,
    report: &SettlementReport,
) -> Result<(), rust_xlsxwriter::XlsxError> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("货主明细")?;

    let header = Format::new().set_bold();
    let money_fmt = money_format();
    let headers = [
        "货主",
        "商品编号",
        "名称",
        "带去",
        "卖出",
        "赠送",
        "报废",
        "差异",
        "带回",
        "现场仓",
        "原价",
        "折让",
        "净额",
    ];
    for (c, h) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, c as u16, *h, &header)?;
    }

    let mut row: u32 = 1;
    for s in &report.societies {
        for g in &s.goods {
            worksheet.write_string(row, 0, s.name.as_str())?;
            worksheet.write_string(row, 1, g.product_code.as_str())?;
            worksheet.write_string(row, 2, g.name.as_str())?;
            let quantities = [
                g.brought_in,
                g.sold,
                g.gifted,
                g.scrapped,
                g.variance,
                g.taken_back,
                g.on_site,
            ];
            for (i, q) in quantities.iter().enumerate() {
                worksheet.write_number(row, 3 + i as u16, *q as f64)?;
            }
            // spec 8.2/8.4：按商品给出金额。只给数量等于让社团自己去乘单价，
            // 而 Lot 折让之后单价不等于成交价，乘出来是错的。
            write_money(worksheet, row, 10, g.gross, &money_fmt)?;
            write_money(worksheet, row, 11, g.lot_discount, &money_fmt)?;
            write_money(worksheet, row, 12, g.allocated, &money_fmt)?;
            row += 1;
        }
    }

    Ok(())
}

fn write_journal_sheet(
    workbook: &mut Workbook,
    journals: &[JournalRow],
    stock_legs: &[StockLegRow],
    money_legs: &[MoneyLegRow],
) -> Result<(), rust_xlsxwriter::XlsxError> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("账本流水")?;

    let header = Format::new().set_bold();
    let headers = [
        "journal id",
        "时间",
        "种类",
        "订单号",
        "摘要",
        "货腿（商品·从→到·数量）",
        "钱腿（从→到·金额）",
    ];
    for (c, h) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, c as u16, *h, &header)?;
    }

    // 腿按 journal 归组：一个 journal 的货腿/钱腿各拼成一个单元格。
    let mut stock_by_journal: std::collections::HashMap<i64, Vec<String>> =
        std::collections::HashMap::new();
    for leg in stock_legs {
        stock_by_journal
            .entry(leg.journal_id)
            .or_default()
            .push(format!(
                "{} {}·{}→{}·{}",
                leg.product_code, leg.name, leg.from_location, leg.to_location, leg.qty
            ));
    }
    let mut money_by_journal: std::collections::HashMap<i64, Vec<String>> =
        std::collections::HashMap::new();
    for leg in money_legs {
        money_by_journal
            .entry(leg.journal_id)
            .or_default()
            .push(format!(
                "{}→{}·{}",
                leg.from_account,
                leg.to_account,
                Money::from_cents(leg.amount)
            ));
    }

    for (row, j) in (1u32..).zip(journals) {
        worksheet.write_number(row, 0, j.id as f64)?;
        worksheet.write_string(row, 1, j.occurred_at.as_str())?;
        worksheet.write_string(row, 2, j.kind.as_str())?;
        if let Some(order_id) = j.order_id {
            worksheet.write_number(row, 3, order_id as f64)?;
        }
        worksheet.write_string(row, 4, j.note.as_deref().unwrap_or(""))?;
        worksheet.write_string(
            row,
            5,
            stock_by_journal
                .get(&j.id)
                .map(|legs| legs.join("；"))
                .unwrap_or_default(),
        )?;
        worksheet.write_string(
            row,
            6,
            money_by_journal
                .get(&j.id)
                .map(|legs| legs.join("；"))
                .unwrap_or_default(),
        )?;
    }

    Ok(())
}

fn write_order_sheet(
    workbook: &mut Workbook,
    order_lines: &[OrderLineRow],
) -> Result<(), rust_xlsxwriter::XlsxError> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("订单明细")?;

    let header = Format::new().set_bold();
    let money_fmt = money_format();
    let headers = [
        "订单号",
        "状态",
        "渠道",
        "行号",
        "商品",
        "套装",
        "件数",
        "单价",
        "货主应得",
        "顾客实付",
        "已退件数",
    ];
    for (c, h) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, c as u16, *h, &header)?;
    }

    for (row, l) in (1u32..).zip(order_lines) {
        worksheet.write_number(row, 0, l.order_id as f64)?;
        worksheet.write_string(row, 1, l.status.as_str())?;
        worksheet.write_string(row, 2, l.channel.as_deref().unwrap_or(""))?;
        worksheet.write_number(row, 3, l.line_id as f64)?;
        worksheet.write_string(row, 4, format!("{} {}", l.product_code, l.name))?;
        worksheet.write_string(row, 5, l.lot_name.as_deref().unwrap_or(""))?;
        worksheet.write_number(row, 6, l.qty as f64)?;
        write_money(
            worksheet,
            row,
            7,
            Money::from_cents(l.unit_price),
            &money_fmt,
        )?;
        write_money(
            worksheet,
            row,
            8,
            Money::from_cents(l.allocated_amount),
            &money_fmt,
        )?;
        write_money(
            worksheet,
            row,
            9,
            Money::from_cents(l.paid_amount),
            &money_fmt,
        )?;
        worksheet.write_number(row, 10, l.refunded_qty as f64)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::domain::ledger::{account_balance, Account};
    use crate::domain::money::Money;
    use crate::test_support::{
        admin_token, json_request, place, read_json, seed_event_and_product, test_router_with,
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

    /// 端到端：下单 → 完成（带手工折让）→ 退一部分 → 垫付 → 调整 → 带回 → 结算单。
    /// 断言只有一条：**warnings 为空**。
    ///
    /// 这比逐个字段对数字有力得多——warnings 里那两条恒等式分别从业务表和账本
    /// 两条独立的路算出来，只要有一条腿方向写错或漏记，它们就撞不上。
    #[tokio::test]
    async fn a_full_event_produces_a_self_consistent_settlement() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        // 卖一单：A×1（本社团）+ B×2（黄昏堂），原价 7000，实收改成 6500（手工折让 500）
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 1},
                                 {"product_id": ep_b, "quantity": 2}]}),
            ))
            .await
            .unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "微信", "final_amount": 6500}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // 退 B 的一件，只退 800（顾客没拿回的部分归黄昏堂）
        let line_b: i64 = sqlx::query_scalar(
            "SELECT id FROM order_lines WHERE order_id = ? AND event_product_id = ? LIMIT 1",
        )
        .bind(order_id)
        .bind(ep_b)
        .fetch_one(&pool)
        .await
        .unwrap();
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&token),
                json!({"channel": "现金", "refund_amount": 800,
                       "lines": [{"order_line_id": line_b, "qty": 1, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        // 送一件 A（默认口径）、摊主自掏送一件 B、报废一件 A
        for body in [
            json!({"event_product_id": ep_a, "qty": 1}),
            json!({"event_product_id": ep_b, "qty": 1, "vendor_pays": true}),
        ] {
            let res = router
                .clone()
                .oneshot(json_request(
                    "POST",
                    &format!("/api/events/{event_id}/gifts"),
                    Some(&token),
                    body,
                ))
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::CREATED);
        }
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/scraps"),
                Some(&token),
                json!({"event_product_id": ep_a, "qty": 1, "note": "压坏了"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        // 垫付 + 调整
        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/advances"),
                Some(&token),
                json!({"society_id": 2, "label": "摊位费", "amount": 40000}),
            ))
            .await
            .unwrap();
        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/adjustments"),
                Some(&token),
                json!({"society_id": 2, "label": "赔一本", "direction": "to_them", "amount": 2000}),
            ))
            .await
            .unwrap();

        // 盘点 + 带回 + 结算
        let remaining = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/closing"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        let counts: Vec<serde_json::Value> = read_json(remaining).await["onsite_remaining"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| json!({"event_product_id": r["event_product_id"], "counted_qty": r["qty"]}))
            .collect();
        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/stocktake"),
                Some(&token),
                json!({"counts": counts}),
            ))
            .await
            .unwrap();
        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/takeback"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/settle"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // ---- 结算单 ----
        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/settlement"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let report = read_json(res).await;
        assert_eq!(
            report["warnings"].as_array().unwrap().len(),
            0,
            "结算单自相矛盾：{}",
            report["warnings"]
        );
        assert!(report["stocktaken"].as_bool().unwrap());

        // 摊主留存 + Σ我应转给 == 实际到手
        let retained = report["vendor_retained"].as_i64().unwrap();
        let transfer_total = report["transfer_total"].as_i64().unwrap();
        let actual_total = report["actual_total"].as_i64().unwrap();
        assert_eq!(retained + transfer_total, actual_total);
    }

    #[tokio::test]
    async fn reconciling_records_the_shortfall_against_the_vendor_not_the_owners() {
        // 母 spec 第 5 节：对账差异默认由摊主自吞，不进任何货主的结算。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
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

        let before = read_json(
            router
                .clone()
                .oneshot(json_request(
                    "GET",
                    &format!("/api/events/{event_id}/settlement"),
                    Some(&token),
                    json!(null),
                ))
                .await
                .unwrap(),
        )
        .await;
        let transfer_before = before["societies"][0]["transfer"].as_i64().unwrap();

        // 现金盒里少了 5 块
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/settlement/reconcile"),
                Some(&token),
                json!({"counts": [{"channel": "现金", "actual": 2500}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let after = read_json(
            router
                .clone()
                .oneshot(json_request(
                    "GET",
                    &format!("/api/events/{event_id}/settlement"),
                    Some(&token),
                    json!(null),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert!(
            after["warnings"].as_array().unwrap().is_empty(),
            "{}",
            after["warnings"]
        );
        assert_eq!(
            after["societies"][0]["transfer"].as_i64().unwrap(),
            transfer_before,
            "货主该拿多少和摊主数出来多少钱无关"
        );
        assert_eq!(after["channels"][0]["book"], 3000);
        assert_eq!(after["channels"][0]["actual"], 2500);
        assert_eq!(after["channels"][0]["diff"], -500);
        assert_eq!(after["channels"][0]["counted"], true);
        assert_eq!(
            after["vendor_retained"].as_i64().unwrap(),
            2500 - transfer_before,
            "短的 5 块由摊主自吞"
        );
    }

    #[tokio::test]
    async fn reconciling_accepts_a_negative_actual_when_cash_is_under_water() {
        // 母 spec §5.4：「微信收、现金退」很常见。某渠道的退款超过收款时，
        // book(现金) 就是负的——摊主手上那个渠道实际是负持仓（他自掏了钱出去）。
        // 以前 `reconcile` 拒收负数，摊主只能填 0，那笔钱被算成对账差异、方向还
        // 抬高 actual_total，把「摊主留存」虚报。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        // 现金收 20：B 1 件。
        let order_a = place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 1}]),
        )
        .await;
        // 微信收 60：B 3 件。跨渠道退款（微信收、现金退）是允许的。
        let order_b = place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 3}]),
        )
        .await;
        for (order_id, channel) in [(order_a, "现金"), (order_b, "微信")] {
            let res = router
                .clone()
                .oneshot(json_request(
                    "PUT",
                    &format!("/api/events/{event_id}/orders/{order_id}/status"),
                    Some(&token),
                    json!({"status": "completed", "channel": channel}),
                ))
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
        }
        let line_b: i64 =
            sqlx::query_scalar("SELECT id FROM order_lines WHERE order_id = ? ORDER BY id LIMIT 1")
                .bind(order_b)
                .fetch_one(&pool)
                .await
                .unwrap();
        // 现金退 50。
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_b}/refunds"),
                Some(&token),
                json!({"channel": "现金", "refund_amount": 5000,
                   "lines": [{"order_line_id": line_b, "qty": 3, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        // 现金 20 − 50 = −30；微信 +60。
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into()))
                .await
                .unwrap(),
            Money::from_cents(-3000)
        );

        let before = read_json(
            router
                .clone()
                .oneshot(json_request(
                    "GET",
                    &format!("/api/events/{event_id}/settlement"),
                    Some(&token),
                    json!(null),
                ))
                .await
                .unwrap(),
        )
        .await;
        let transfer_total = before["transfer_total"].as_i64().unwrap();
        assert_eq!(before["actual_total"], 3000, "−30 + 60");

        // 清点填 −30。以前这里会 400（「实际到手不能为负」）。
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/settlement/reconcile"),
                Some(&token),
                json!({"counts": [{"channel": "现金", "actual": -3000},
                                  {"channel": "微信", "actual": 6000}]}),
            ))
            .await
            .unwrap();
        let status = res.status();
        let body = read_json(res).await;
        assert_eq!(status, StatusCode::OK, "负持仓也要能清点：{body}");
        assert!(body["journal_id"].is_null(), "两边都分文不差，不该写腿");

        let after = read_json(
            router
                .clone()
                .oneshot(json_request(
                    "GET",
                    &format!("/api/events/{event_id}/settlement"),
                    Some(&token),
                    json!(null),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert!(
            after["warnings"].as_array().unwrap().is_empty(),
            "{}",
            after["warnings"]
        );
        let cash = after["channels"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["channel"] == "现金")
            .expect("现金渠道必须在报表里");
        assert_eq!(cash["book"], -3000);
        assert_eq!(cash["actual"], -3000);
        assert_eq!(cash["diff"], 0);
        assert_eq!(cash["counted"], true);
        assert_eq!(
            after["vendor_retained"].as_i64().unwrap(),
            3000 - transfer_total,
            "负持仓不能被当成 0 去虚报摊主留存"
        );
    }

    #[tokio::test]
    async fn counting_an_exact_match_still_marks_it_counted() {
        // 分文不差时写不出 journal（零金额腿被 post_journal 滤掉，
        // 只剩空 journal 会被拒），所以「数过了」靠 events.reconciled_at 记。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
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
                "POST",
                &format!("/api/events/{event_id}/settlement/reconcile"),
                Some(&token),
                json!({"counts": [{"channel": "现金", "actual": 3000}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(read_json(res).await["journal_id"].is_null());

        let report = read_json(
            router
                .clone()
                .oneshot(json_request(
                    "GET",
                    &format!("/api/events/{event_id}/settlement"),
                    Some(&token),
                    json!(null),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(report["channels"][0]["counted"], true);
        assert_eq!(report["channels"][0]["diff"], 0);
    }

    #[tokio::test]
    async fn reconciling_twice_writes_only_the_new_delta() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
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

        for actual in [2500, 2800] {
            let res = router
                .clone()
                .oneshot(json_request(
                    "POST",
                    &format!("/api/events/{event_id}/settlement/reconcile"),
                    Some(&token),
                    json!({"counts": [{"channel": "现金", "actual": actual}]}),
                ))
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
        }

        let report = read_json(
            router
                .clone()
                .oneshot(json_request(
                    "GET",
                    &format!("/api/events/{event_id}/settlement"),
                    Some(&token),
                    json!(null),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(report["channels"][0]["actual"], 2800, "第二次数出来是 28");
        assert_eq!(report["channels"][0]["book"], 3000, "账面应有始终是 30");
    }

    #[tokio::test]
    async fn reconciling_the_same_channel_twice_in_one_request_is_refused() {
        // 两条都对着「写入前余额」算完整差额 ⇒ 同一笔差额被写两遍。
        // " 现金 " 和 "现金" 会被 normalize 折叠成同一个账户，所以要在规范化之后判重。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        // 先卖一单制造出「现金」这个渠道
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
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
                "POST",
                &format!("/api/events/{event_id}/settlement/reconcile"),
                Some(&token),
                json!({"counts": [{"channel": "现金", "actual": 2500},
                                  {"channel": " 现金 ", "actual": 2500}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into()))
                .await
                .unwrap(),
            Money::from_cents(3000),
            "一分都不该动"
        );
    }

    #[tokio::test]
    async fn reconciling_a_channel_the_event_never_used_is_refused() {
        // 否则钱写进账本却不在报表的渠道清单里 ⇒ actual_total / vendor_retained 都看不见它。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
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
                "POST",
                &format!("/api/events/{event_id}/settlement/reconcile"),
                Some(&token),
                json!({"counts": [{"channel": "微信支付", "actual": 10000}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = read_json(res).await;
        assert!(
            body["error"].as_str().unwrap().contains("现金"),
            "要告诉摊主本场用过哪些渠道：{body}"
        );
    }

    #[tokio::test]
    async fn reconciling_must_cover_every_channel_the_event_used() {
        // 全局的 events.reconciled_at 只能代表「全都数过了」，前提是请求收全量。
        // 和盘点一字不差：「我数了，一致」和「我没数这个」是两件事。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        for channel in ["现金", "微信"] {
            let res = router
                .clone()
                .oneshot(json_request(
                    "POST",
                    &format!("/api/events/{event_id}/orders"),
                    None,
                    json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
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
                    json!({"status": "completed", "channel": channel}),
                ))
                .await
                .unwrap();
        }

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/settlement/reconcile"),
                Some(&token),
                json!({"counts": [{"channel": "现金", "actual": 3000}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = read_json(res).await;
        assert!(
            body["error"].as_str().unwrap().contains("微信"),
            "要点名漏了哪个渠道：{body}"
        );
    }

    /// 绕过 API 直接写一条本展会没用过的 `实收-<渠道>` 腿，模拟「别的路径」。
    async fn post_stray_received(
        pool: &sqlx::SqlitePool,
        event_id: i64,
        channel: &str,
        cents: i64,
    ) {
        let mut tx = pool.begin().await.unwrap();
        crate::domain::ledger::post_journal(
            &mut tx,
            event_id,
            crate::domain::ledger::JournalKind::Adjust,
            None,
            None,
            Some("测试：意外的实收账户"),
            &[],
            &[crate::domain::ledger::MoneyLeg {
                from: Account::VendorOwn,
                to: Account::Received(channel.into()),
                amount: Money::from_cents(cents),
            }],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn a_stray_received_account_the_event_never_used_still_shows_in_the_report() {
        // 报表的渠道清单若只来自 orders ∪ refunds，一条别的路径写进本场没用过的
        // 实收账户的腿就会既不在清单里、也不被 actual_total / vendor_retained 计入，
        // 钱静默消失。并上 money_movements 里的 `实收-` 账户后它必须成为一行。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        // 绕过 API 直接写一条腿，模拟「别的路径」。
        post_stray_received(&pool, event_id, "微信支付", 10000).await;

        let report = read_json(
            router
                .clone()
                .oneshot(json_request(
                    "GET",
                    &format!("/api/events/{event_id}/settlement"),
                    Some(&token),
                    json!(null),
                ))
                .await
                .unwrap(),
        )
        .await;

        let channels = report["channels"].as_array().unwrap();
        let stray = channels
            .iter()
            .find(|c| c["channel"] == "微信支付")
            .expect("意外的实收账户必须在渠道清单里，钱才不会隐形");
        assert_eq!(stray["book"], 10000);
        assert_eq!(
            report["actual_total"], 10000,
            "actual_total 要把它算进去，否则摊主留存少一截"
        );
    }

    #[tokio::test]
    async fn reconciling_accepts_a_channel_the_report_lists_from_a_stray_movement() {
        // `reconcile` 的白名单必须和报表的 `channels[]` 完全一致。一条本展会的
        // `实收-微信支付` 腿会让报表列出这个渠道；以前 `reconcile` 只认
        // orders ∪ refunds，于是管理端照着报表要摊主填这一格、提交却 400。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        post_stray_received(&pool, event_id, "微信支付", 10000).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/settlement/reconcile"),
                Some(&token),
                json!({"counts": [{"channel": "微信支付", "actual": 10000}]}),
            ))
            .await
            .unwrap();
        let status = res.status();
        let body = read_json(res).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "报表列得出来的渠道，清点就必须收：{body}"
        );
    }

    #[tokio::test]
    async fn reconciling_accepts_legacy_channel_names_that_normalize_differently() {
        // 本分支之前后端只在写单时 trim，所以库里可能存着内部多空格、甚至超过
        // `MAX_CHANNEL_CHARS` 的渠道名。报表把它们原样列出来，摊主照着填，而
        // `reconcile` 用 normalize 折叠提交值——不把 `used` 也规范化，两边 key
        // 对不上，那个渠道既过不了白名单也过不了收全量，整场展会永远清不了点。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let padded = "银行  转账"; // 内部双空格
        let long: String = "超".repeat(25); // 超过 20 字，normalize 会报错
        post_stray_received(&pool, event_id, padded, 10000).await;
        post_stray_received(&pool, event_id, &long, 20000).await;

        let report = read_json(
            router
                .clone()
                .oneshot(json_request(
                    "GET",
                    &format!("/api/events/{event_id}/settlement"),
                    Some(&token),
                    json!(null),
                ))
                .await
                .unwrap(),
        )
        .await;
        let names: Vec<&str> = report["channels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["channel"].as_str().unwrap())
            .collect();
        assert!(names.contains(&padded), "报表要列原样存储值：{names:?}");
        assert!(
            names.contains(&long.as_str()),
            "报表要列原样存储值：{names:?}"
        );

        // 摊主照着报表原文提交。内部多空格的那个靠 normalize 后的 key 对上；
        // 超长的 normalize 会报错，退回原样 key 仍能对上，且读的是原样账户。
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/settlement/reconcile"),
                Some(&token),
                json!({"counts": [
                    {"channel": padded, "actual": 10000},
                    {"channel": long.as_str(), "actual": 20000}
                ]}),
            ))
            .await
            .unwrap();
        let status = res.status();
        let body = read_json(res).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "历史渠道名不能把整场清点挡死：{body}"
        );
        assert!(body["journal_id"].is_null(), "两边都分文不差，不该写腿");

        let after = read_json(
            router
                .clone()
                .oneshot(json_request(
                    "GET",
                    &format!("/api/events/{event_id}/settlement"),
                    Some(&token),
                    json!(null),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert!(
            after["warnings"].as_array().unwrap().is_empty(),
            "{}",
            after["warnings"]
        );
        for c in after["channels"].as_array().unwrap() {
            assert_eq!(c["diff"], 0, "账面和实际都对上：{c}");
            assert_eq!(c["counted"], true);
        }
    }

    #[tokio::test]
    async fn reconciling_works_after_the_event_is_settled() {
        // spec 偏离 3：回家才有空数现金盒、对微信账单是常态。
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
                &format!("/api/events/{event_id}/settlement/reconcile"),
                Some(&admin_token()),
                json!({"counts": []}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    /// 已取消订单的退货不能被重复计入（batch-4 dispatch 第 1 条）。
    ///
    /// 取消一张已完成订单时，`reverse_order_journals` 会把它名下**包括退货**
    /// 在内的所有未冲正 journal 都冲掉，但 `refunds` 表的行还在。`MONEY_SQL`
    /// 用 `JOIN orders ... AND o.status = 'completed'` 过滤，整张取消单（连同
    /// 它的 refunds 子查询）都不出现，所以不会把退货金额算第二遍。
    ///
    /// 把那个 status 条件去掉，refunds 行就会继续扣减 allocated/paid，而账本
    /// 早已被取消冲回 0——货主净额变成负数、warnings 里出现「结算对不上账本」。
    #[tokio::test]
    async fn a_cancelled_order_with_a_partial_refund_is_not_counted_twice() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        // 卖两件 B（黄昏堂），现金收 4000
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
        let order_id = read_json(res).await["id"].as_i64().unwrap();
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "现金"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // 退一件，只退 800
        let line_b: i64 = sqlx::query_scalar(
            "SELECT id FROM order_lines WHERE order_id = ? AND event_product_id = ? LIMIT 1",
        )
        .bind(order_id)
        .bind(ep_b)
        .fetch_one(&pool)
        .await
        .unwrap();
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&token),
                json!({"channel": "现金", "refund_amount": 800,
                       "lines": [{"order_line_id": line_b, "qty": 1, "destination": "现场仓"}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        // 整单取消：退货 journal 被冲正了，但 refunds 行还在
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

        let refunds_left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM refunds")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(refunds_left, 1, "refunds 行必须还在——这正是需要防的陷阱");

        let report = read_json(
            router
                .clone()
                .oneshot(json_request(
                    "GET",
                    &format!("/api/events/{event_id}/settlement"),
                    Some(&token),
                    json!(null),
                ))
                .await
                .unwrap(),
        )
        .await;

        assert!(
            report["warnings"].as_array().unwrap().is_empty(),
            "取消单的退货被重复计入了：{}",
            report["warnings"]
        );

        let other = report["societies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"] == "黄昏堂")
            .expect("黄昏堂应在结算单里");
        assert_eq!(other["gross"], 0, "整单取消，原价不该出现");
        assert_eq!(other["net"], 0, "净额必须回到 0");
        assert_eq!(other["refund_kept"], 0);
        assert_eq!(other["transfer"], 0, "账本也已冲平");
    }

    #[tokio::test]
    async fn the_xlsx_is_a_real_workbook_with_four_sheets() {
        // 不解析 xlsx 内容（那要引一个读库）；但列出 zip 条目不需要解析器。
        // 断言状态码、Content-Type、body 是个 zip，以及四个 worksheet 条目都在——
        // 删掉任何一个 `write_*_sheet` 这条测试都会红。
        // 数字对不对由 domain/settlement.rs 的纯函数测试和端到端测试保证——
        // 导出和页面渲染的是同一个 SettlementReport，这正是那样设计的理由。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/settlement.xlsx"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let ct = res
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(ct.contains("spreadsheetml"), "content-type 是 {ct}");

        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(bytes.len() > 1000, "空文件");
        assert_eq!(&bytes[..4], b"PK\x03\x04", "xlsx 应当是个 zip");

        let archive =
            zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).expect("xlsx 应能当 zip 读");
        let names: Vec<&str> = archive.file_names().collect();
        for sheet in [
            "xl/worksheets/sheet1.xml",
            "xl/worksheets/sheet2.xml",
            "xl/worksheets/sheet3.xml",
            "xl/worksheets/sheet4.xml",
        ] {
            assert!(names.contains(&sheet), "缺 {sheet}，实际条目：{names:?}");
        }
    }

    #[tokio::test]
    async fn the_xlsx_filename_is_ascii_safe() {
        // 中文文件名在部分浏览器/CDN 上会变成 ???，下载直接坏掉——
        // 安装包那边已经踩过一次（见项目 CLAUDE.md 的发版流程）。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/settlement.xlsx"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        let cd = res
            .headers()
            .get("content-disposition")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cd.is_ascii(), "Content-Disposition 必须是纯 ASCII：{cd}");
        assert!(cd.contains("settlement"));
    }
}

/// 2026-09-26 收摊走查发现：结算单上两个时间戳不在同一个时区。
#[cfg(test)]
mod timestamp_tests {
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::ServiceExt;

    /// 曾经 `generated_at` 用 `chrono::Local::now()`（本地时间），而 `last_changed_at`
    /// 取 `journals.occurred_at`（SQLite `CURRENT_TIMESTAMP`，UTC），页面
    /// （`SettlementReportView.vue` 的「生成于 … · 账本最后变动于 …」）和 xlsx
    /// 都原样拼出来。东八区真机上看到的是
    /// 「生成于 2026-09-26 00:35 · 账本最后变动于 2026-09-25 16:34」——
    /// 刚改过的账看起来像 8 小时前的，而这个时间戳存在的全部理由就是让摊主判断
    /// 手里的表是不是最新的（spec 3.2）。结算调整那一行的 `at` 同样是 UTC 原样显示。
    ///
    /// **只在本机时区不是 UTC 时复现**（CI 若跑在 UTC 上会假绿）。
    #[tokio::test]
    async fn generated_at_and_last_changed_at_share_a_clock() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        // 刚写一条 journal，「最后变动」就应该是此刻。
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

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/settlement"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let report = read_json(res).await;

        let parse =
            |s: &str| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap();
        let generated = parse(report["generated_at"].as_str().unwrap());
        let changed = parse(report["last_changed_at"].as_str().unwrap());
        let gap = (generated - changed).num_seconds().abs();
        assert!(
            gap < 120,
            "刚改完账就生成，两个时间应该几乎相同，实际差 {gap} 秒：generated_at={generated} last_changed_at={changed}"
        );
    }
}

/// ③b 形状快照：钉住每个路由的 JSON 形状（键 + 类型），类型化前后必须一行不改照样绿。
#[cfg(test)]
mod shape_tests {
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, shape_of, test_router_with,
    };
    use axum::http::StatusCode;
    use axum::Router;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    /// 一场有成交、有垫付、有调整的展会，让每个可选字段和数组都至少出现一次非空。
    async fn seeded() -> (Router, tempfile::TempDir, i64, i64, i64) {
        let (router, dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let (_, body) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/orders"),
            None,
            json!({"items": [{"product_id": ep_a, "quantity": 1}, {"product_id": ep_b, "quantity": 2}]}),
        )
        .await;
        let order_id = body["id"].as_i64().unwrap();
        let (s, _) = call(
            &router,
            "PUT",
            &format!("/api/events/{event_id}/orders/{order_id}/status"),
            Some(&token),
            json!({"status": "completed", "channel": "微信", "final_amount": 6500}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        let (_, adv) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/advances"),
            Some(&token),
            json!({"society_id": 2, "label": "打印费", "amount": 1200}),
        )
        .await;
        let (_, adj) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/adjustments"),
            Some(&token),
            json!({"society_id": 2, "label": "赔付", "direction": "to_them", "amount": 300}),
        )
        .await;
        (
            router,
            dir,
            event_id,
            adv["id"].as_i64().unwrap(),
            adj["id"].as_i64().unwrap(),
        )
    }

    async fn call(
        router: &Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Value,
    ) -> (StatusCode, Value) {
        let res = router
            .clone()
            .oneshot(json_request(method, uri, token, body))
            .await
            .unwrap();
        let status = res.status();
        (status, read_json(res).await)
    }

    fn entry_row() -> Value {
        json!([{"amount": "int", "id": "int", "label": "string", "owner_society_id": "int", "society_name": "string"}])
    }

    #[tokio::test]
    async fn shape_list_channels() {
        let (router, _dir, _, _, _) = seeded().await;
        let (s, body) = call(
            &router,
            "GET",
            "/api/channels",
            Some(&admin_token()),
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(shape_of(&body), json!(["string"]));
    }

    #[tokio::test]
    async fn shape_list_advances_and_adjustments() {
        let (router, _dir, event_id, _, _) = seeded().await;
        for kind in ["advances", "adjustments"] {
            let (s, body) = call(
                &router,
                "GET",
                &format!("/api/events/{event_id}/{kind}"),
                Some(&admin_token()),
                json!(null),
            )
            .await;
            assert_eq!(s, StatusCode::OK, "{kind}");
            assert_eq!(shape_of(&body), entry_row(), "{kind}");
        }
    }

    #[tokio::test]
    async fn shape_create_advance_and_adjustment() {
        let (router, _dir, event_id, _, _) = seeded().await;
        let t = admin_token();
        let created = json!({"id": "int", "journal_id": "int"});
        let (s, body) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/advances"),
            Some(&t),
            json!({"society_id": 1, "label": "场地费", "amount": 500}),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::CREATED, created.clone()));
        let (s, body) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/adjustments"),
            Some(&t),
            json!({"society_id": 1, "label": "x", "direction": "to_me", "amount": 100}),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::CREATED, created));
        let (s, body) = call(
            &router,
            "POST",
            "/api/events/999/advances",
            Some(&t),
            json!({"society_id": 1, "label": "x", "amount": 1}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::NOT_FOUND, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_delete_advance_and_adjustment() {
        let (router, _dir, event_id, adv_id, adj_id) = seeded().await;
        let t = admin_token();
        let (s, body) = call(
            &router,
            "DELETE",
            &format!("/api/events/{event_id}/advances/{adv_id}"),
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!((s, body), (StatusCode::NO_CONTENT, Value::Null));
        let (s, body) = call(
            &router,
            "DELETE",
            &format!("/api/events/{event_id}/adjustments/{adj_id}"),
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!((s, body), (StatusCode::NO_CONTENT, Value::Null));
    }

    #[tokio::test]
    async fn shape_get_settlement() {
        let (router, _dir, event_id, _, _) = seeded().await;
        let t = admin_token();
        let (s, body) = call(
            &router,
            "GET",
            &format!("/api/events/{event_id}/settlement"),
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        let counts = json!({"brought_in": "int", "gifted": "int", "on_site": "int", "scrapped": "int",
                            "sold": "int", "taken_back": "int", "variance": "int"});
        let mut goods = counts.clone();
        for (k, v) in [
            ("allocated", "int"),
            ("event_product_id", "int"),
            ("gross", "int"),
            ("lot_discount", "int"),
            ("name", "string"),
            ("product_code", "string"),
        ] {
            goods[k] = json!(v);
        }
        assert_eq!(
            shape_of(&body),
            json!({
                "event_name": "string", "event_date": "string", "generated_at": "string",
                "last_changed_at": "string", "stocktaken": "bool", "stocktaken_at": "null",
                "actual_total": "int", "transfer_total": "int", "vendor_retained": "int",
                "warnings": ["empty"],
                "channels": [{"actual": "int", "book": "int", "channel": "string", "counted": "bool", "diff": "int"}],
                "societies": [{
                    "society_id": "int", "name": "string", "is_home": "bool",
                    "goods": [goods], "totals": counts,
                    "gross": "int", "lot_discount": "int", "manual_discount": "int", "refund_kept": "int",
                    "gift_self_paid": "int", "net": "int", "transfer": "int",
                    "advances": [{"amount": "int", "at": "null", "label": "string"}], "advances_total": "int",
                    "adjustments": [{"amount": "int", "at": "string", "label": "string"}], "adjustments_total": "int",
                }],
            })
        );
        let (s, body) = call(
            &router,
            "GET",
            "/api/events/999/settlement",
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::NOT_FOUND, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_reconcile() {
        let (router, _dir, event_id, _, _) = seeded().await;
        let (s, body) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/settlement/reconcile"),
            Some(&admin_token()),
            json!({"counts": [{"channel": "微信", "actual": 6000}]}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::OK, json!({"journal_id": "int"}))
        );
    }

    #[tokio::test]
    async fn shape_download_settlement_xlsx() {
        let (router, _dir, event_id, _, _) = seeded().await;
        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/settlement.xlsx"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
    }
}
