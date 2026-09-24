//! 结算侧：渠道列表、垫付、结算调整、结算单、收摊清点、xlsx 导出。
//!
//! **本模块里有三类操作故意不调用 `require_event_open`**：垫付、结算调整、
//! 收摊清点（后者在 Task 8）。冻结之后它们仍然允许（spec 偏离 3）——「回家翻出一张打印费收据」
//! 和「回家发现少了一本书」是同一类事件。看见别处都守着就顺手补上去，
//! 会把有意的例外当成漏掉的守卫。改之前先读 spec 3.2。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::guard::{check_read_permission, check_write_permission},
    domain::{
        channel::{normalize, PRESET_CHANNELS},
        ledger::{
            account_balance, home_society_id, post_journal, reverse_journal, Account, JournalKind,
            MoneyLeg,
        },
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
        .route("/events/:event_id/settlement", get(get_settlement))
        .route("/events/:event_id/settlement/reconcile", post(reconcile))
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
LEFT JOIN journals j ON j.id = sm.journal_id AND j.event_id = ep.event_id
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

/// 把各张表查出来，组装成 `build_report` 要的输入。
///
/// 页面 JSON 和 `settlement.xlsx` 都走这里——「页面显示 1,170、导出写 1,150」
/// 在结构上不可能发生。
async fn load_input(state: &AppState, event_id: i64) -> ApiResult<SettlementInput> {
    let event: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT name, event_date, stocktaken_at, reconciled_at FROM events WHERE id = ?",
    )
    .bind(event_id)
    .fetch_optional(&state.db)
    .await?;
    let (event_name, event_date, stocktaken_at, reconciled_at) =
        event.ok_or_else(|| ApiError::NotFound("展会不存在".into()))?;

    // 本社团：手工折让整笔落在它头上，`is_home` 也靠它判定。
    let home_id = {
        let mut conn = state.db.acquire().await?;
        home_society_id(&mut conn).await?
    };

    let goods_rows: Vec<GoodsRow> = sqlx::query_as(GOODS_SQL)
        .bind(event_id)
        .fetch_all(&state.db)
        .await?;

    let money_rows: Vec<MoneyRow> = sqlx::query_as(MONEY_SQL)
        .bind(event_id)
        .fetch_all(&state.db)
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
    .fetch_all(&state.db)
    .await?;

    let adjustment_rows: Vec<AdjustmentRow> = sqlx::query_as(
        "SELECT owner_society_id, label, amount, created_at
         FROM settlement_adjustments WHERE event_id = ? ORDER BY id",
    )
    .bind(event_id)
    .fetch_all(&state.db)
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
    .fetch_all(&state.db)
    .await?;

    let mut societies = Vec::with_capacity(society_rows.len());
    for (society_id, name) in society_rows {
        let is_home = society_id == home_id;

        let goods: Vec<GoodsLine> = goods_rows
            .iter()
            .filter(|g| g.owner_society_id == society_id)
            .map(|g| GoodsLine {
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
            .fetch_one(&state.db)
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

        let due_balance = account_balance(&state.db, event_id, &account).await?;

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

    // 渠道清单 = 本展会实际用过的（orders ∪ refunds），按展会过滤，
    // 不是跨展会的 `GET /channels`。
    let channel_names: Vec<String> = sqlx::query_scalar(
        "SELECT channel FROM (
            SELECT channel FROM orders
             WHERE event_id = ? AND channel IS NOT NULL AND channel <> ''
            UNION
            SELECT r.channel AS channel FROM refunds r
              JOIN order_lines ol ON ol.id = r.order_line_id
              JOIN orders o ON o.id = ol.order_id
             WHERE o.event_id = ? AND r.channel <> ''
         ) ORDER BY 1",
    )
    .bind(event_id)
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    let mut channels = Vec::with_capacity(channel_names.len());
    for channel in channel_names {
        let account = Account::Received(channel.clone());
        let account_name = account.to_string();
        let current = account_balance(&state.db, event_id, &account).await?;
        let recon_diff: i64 = sqlx::query_scalar(RECON_DIFF_SQL)
            .bind(&account_name)
            .bind(&account_name)
            .bind(event_id)
            .fetch_one(&state.db)
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
            .fetch_one(&state.db)
            .await?;

    Ok(SettlementInput {
        event_name,
        event_date,
        generated_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        last_changed_at,
        stocktaken_at,
        societies,
        channels,
    })
}

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

#[derive(Deserialize)]
pub struct ReconcileRequest {
    counts: Vec<ChannelActual>,
}

#[derive(Deserialize)]
pub struct ChannelActual {
    channel: String,
    /// 摊主数出来的实际到手（分）。
    actual: i64,
}

#[derive(Serialize)]
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

    let mut money = Vec::new();
    for c in &payload.counts {
        let channel = normalize(&c.channel)?;
        if c.actual < 0 {
            return Err(ApiError::BadRequest("实际到手不能为负".into()));
        }
        let account = Account::Received(channel);
        let current = account_balance_tx(&mut tx, event_id, &account).await?;
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
}
