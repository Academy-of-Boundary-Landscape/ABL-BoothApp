//! 摊位商品：选品 + 进货。
//!
//! 把旧的 `products` 表 + `current_stock` 加减整个换成 `event_products` + 进货移动：
//! 库存余额是 `stock_movements` 的聚合（`domain::ledger`），不存在第二个可以漂移的数字。
//! 「多带了几本」是一次进货，「点了一下发现少了」是盘点（②-3），语义不同且都在账本里留痕。

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, query_scalar, FromRow, Sqlite, SqlitePool, Transaction};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{
        guard::check_write_permission,
        lot::{insert_lot, LotResponse},
        openapi::ApiErrorBody,
    },
    domain::{
        ledger::{onsite_balance, onsite_balances, post_journal, JournalKind, Location, StockLeg},
        money::Money,
    },
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_event_products, add_product_to_event))
        .routes(routes!(import_products))
        .routes(routes!(restock_product))
        .routes(routes!(update_product, delete_product))
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
#[derive(Debug, Serialize, ToSchema)]
#[schema(as = ProductEventProduct)]
struct EventProductResponse {
    id: i64,
    event_id: i64,
    master_product_id: i64,
    owner_society_id: i64,
    owner_society_name: String,
    product_code: String,
    name: String,
    /// 单位：分。
    #[schema(value_type = Money)]
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

/// 建一个场次商品 + `initial_stock > 0` 时记首批进货，**`add_product_to_event`
/// 与 `POST /products/import` 共用**——保证「建商品 + 首批进货」只有一份实现。
///
/// 调用方负责开事务、提交，以及 `require_event_open`。
///
/// ⚠️ 收的是 `&mut Transaction` 而不是 `&mut SqliteConnection`：里面要调
/// `post_journal`，而它只接受事务（`domain/ledger.rs`）。brief 的接口写的是
/// `SqliteConnection`，那样调不到 `post_journal`；见 task-1-report 的偏离记录。
/// 语义不变：两处调用点都在自己的 `BEGIN IMMEDIATE` 事务里。
///
/// 单位价格式：分。
pub(crate) async fn insert_event_product(
    tx: &mut Transaction<'_, Sqlite>,
    event_id: i64,
    master_product_id: i64,
    unit_price: i64,
    initial_stock: i64,
) -> ApiResult<i64> {
    // `owner_society_id` 从 master_products 抄一份**快照**。之后改全局商品库的归属
    // 不影响已有展会的账——否则展会结算完之后有人改了归属，冻结的账就跟着变（spec 3.1）。
    let master: Option<(String, String, i64)> =
        query_as("SELECT product_code, name, owner_society_id FROM master_products WHERE id = ?")
            .bind(master_product_id)
            .fetch_optional(&mut **tx)
            .await?;
    let (product_code, name, owner_society_id) =
        master.ok_or_else(|| ApiError::NotFound("Product not found in master catalog".into()))?;

    // 预先查重，否则撞 (event_id, master_product_id) UNIQUE 约束会走 ApiError::Db → 500。
    // 错误信息带上商品名：导入时摊主是照着名字勾的，只报一个 id 帮不上忙。
    let dup: Option<i64> =
        query_scalar("SELECT id FROM event_products WHERE event_id = ? AND master_product_id = ?")
            .bind(event_id)
            .bind(master_product_id)
            .fetch_optional(&mut **tx)
            .await?;
    if dup.is_some() {
        return Err(ApiError::Conflict(format!("商品「{name}」已经在本场")));
    }

    let new_id: i64 = query_scalar(
        "INSERT INTO event_products
           (event_id, master_product_id, owner_society_id, product_code, name, unit_price)
         VALUES (?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(event_id)
    .bind(master_product_id)
    .bind(owner_society_id)
    .bind(&product_code)
    .bind(&name)
    .bind(unit_price)
    .fetch_one(&mut **tx)
    .await?;

    // initial_stock 为 0 时不记 journal（post_journal 会拒绝空 journal）。
    if initial_stock > 0 {
        post_journal(
            tx,
            event_id,
            JournalKind::Restock,
            None,
            None,
            Some("开场进货"),
            &[StockLeg {
                event_product_id: new_id,
                from: Location::External,
                to: Location::OnSite,
                qty: initial_stock,
            }],
            &[],
        )
        .await?;
    }

    Ok(new_id)
}

// ==========================================
// 1. 获取场次商品列表 (Public)
// ==========================================
/// 列出某场展会全部商品，带现场库存与累计进货。公开接口。
#[utoipa::path(
    get,
    path = "/events/{event_id}/products",
    tag = "product",
    params(("event_id" = i64, Path, description = "展会 id")),
    responses(
        (status = 200, body = Vec<EventProductResponse>),
    ),
)]
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
#[derive(Deserialize, ToSchema)]
#[schema(as = ProductAddRequest)]
struct AddProductRequest {
    product_code: String,
    initial_stock: i64,
    /// 单位：分。缺省用 `master_products.default_price`。
    #[schema(value_type = Option<Money>)]
    unit_price: Option<i64>,
}

/// 把一个全局商品选进某场展会并记首批进货。需要管理员或本场摊主。
#[utoipa::path(
    post,
    path = "/events/{event_id}/products",
    tag = "product",
    params(("event_id" = i64, Path, description = "展会 id")),
    request_body = AddProductRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, body = EventProductResponse),
        (status = 400, body = ApiErrorBody, description = "进货数量或单价为负"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "展会不存在或商品编号不在全局商品库"),
        (status = 409, body = ApiErrorBody, description = "该商品已经在本场展会或展会已结算"),
    ),
)]
async fn add_product_to_event(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<AddProductRequest>,
) -> ApiResult<(StatusCode, Json<EventProductResponse>)> {
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

    let master: Option<(i64, f64)> =
        query_as("SELECT id, default_price FROM master_products WHERE product_code = ?")
            .bind(&payload.product_code)
            .fetch_optional(&state.db)
            .await?;
    let (master_id, default_price) = master
        .ok_or_else(|| ApiError::NotFound("Product code not found in master catalog".into()))?;

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

    // 建商品 + 首批进货必须在同一个事务里，走和 import 同一个 `insert_event_product`。
    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    crate::api::guard::require_event_open(&mut tx, event_id).await?;
    let new_id = insert_event_product(
        &mut tx,
        event_id,
        master_id,
        unit_price,
        payload.initial_stock,
    )
    .await?;
    tx.commit().await?;

    let product = load_one(&state.db, new_id).await?;
    Ok((StatusCode::CREATED, Json(product)))
}

// ==========================================
// 2.5 批量导入商品与套装 (Admin/Vendor)
// ==========================================
//
// 前端「从商品库选」与「从上一场导入」共用这一个端点：前者 `lots` 为空。
// 整个请求在**一个 `BEGIN IMMEDIATE` 事务**里完成，任一商品/套装失败整批回滚——
// 摊主改完重试，不会留下半份名单。

#[derive(Deserialize, ToSchema)]
#[schema(as = ProductImportRequest)]
struct ImportRequest {
    /// 要上架的商品。可省略（只导入套装时）。
    #[serde(default)]
    products: Vec<ImportProduct>,
    /// 要从别的展会复制的套装。可省略（只上架商品时）。
    #[serde(default)]
    lots: Vec<ImportLot>,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = ProductImportItem)]
struct ImportProduct {
    master_product_id: i64,
    /// 单位：分。
    #[schema(value_type = Money)]
    unit_price: i64,
    initial_stock: i64,
}

#[derive(Deserialize, ToSchema)]
#[schema(as = LotImportItem)]
struct ImportLot {
    /// 源套装 id（**必须属于别的展会**）。
    source_lot_id: i64,
}

#[derive(Serialize, ToSchema)]
#[schema(as = ProductImportResponse)]
struct ImportResponse {
    products: Vec<EventProductResponse>,
    lots: Vec<LotResponse>,
}

/// 源套装的不可变快照。只读这四项，候选单独查。
#[derive(Debug, FromRow)]
struct SourceLotRow {
    event_id: i64,
    name: String,
    pick_count: i64,
    total_price: i64,
    allow_repeat: i64,
}

/// 批量把商品上架到某场展会，并可选地从别的展会复制套装。需要管理员或本场摊主。
///
/// **单事务**：任一商品重复上架（409，写明商品名，**不跳过**）、任一候选商品映射不到
/// （400，写明套装名与商品名）、展会已结算（409），都会让整批回滚。套装映射按
/// `master_product_id`：本请求刚建的商品与目标展会原本就有的商品都算“在本场”。
#[utoipa::path(
    post,
    path = "/events/{event_id}/products/import",
    tag = "product",
    params(("event_id" = i64, Path, description = "展会 id")),
    request_body = ImportRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, body = ImportResponse, description = "整批成功；商品与新建套装的完整响应"),
        (status = 400, body = ApiErrorBody, description = "请求为空、单价/库存为负、源套装属于本场或候选商品映射不到"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "展会或源套装不存在"),
        (status = 409, body = ApiErrorBody, description = "商品已在本场（不跳过）或展会已结算"),
    ),
)]
async fn import_products(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<ImportRequest>,
) -> ApiResult<(StatusCode, Json<ImportResponse>)> {
    check_write_permission(&claims, event_id)?;

    if payload.products.is_empty() && payload.lots.is_empty() {
        return Err(ApiError::BadRequest(
            "导入请求至少要有一件商品或一个套装".into(),
        ));
    }
    // 数字校验放在事务外：这些都是请求自身的形状错误，不必白拿写锁。
    for p in &payload.products {
        if p.unit_price < 0 {
            return Err(ApiError::BadRequest("单价不能为负".into()));
        }
        if p.initial_stock < 0 {
            return Err(ApiError::BadRequest("进货数量不能为负".into()));
        }
    }

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    crate::api::guard::require_event_open(&mut tx, event_id).await?;

    // 先商品，再套装：套装的候选可以映射到本请求刚建出来的商品。
    let mut new_product_ids: Vec<i64> = Vec::with_capacity(payload.products.len());
    for p in &payload.products {
        let id = insert_event_product(
            &mut tx,
            event_id,
            p.master_product_id,
            p.unit_price,
            p.initial_stock,
        )
        .await?;
        new_product_ids.push(id);
    }

    let mut lots: Vec<LotResponse> = Vec::with_capacity(payload.lots.len());
    for l in &payload.lots {
        let source: Option<SourceLotRow> = query_as(
            "SELECT event_id, name, pick_count, total_price, allow_repeat
             FROM lots WHERE id = ?",
        )
        .bind(l.source_lot_id)
        .fetch_optional(&mut *tx)
        .await?;
        let source = source.ok_or_else(|| ApiError::NotFound("套装不存在".into()))?;
        if source.event_id == event_id {
            return Err(ApiError::BadRequest("不能从本场导入".into()));
        }

        // 源候选 → master_product_id → 目标展会同 master_product_id 的 event_product。
        // 刚建的商品已经在同一个事务里可见，所以这里不必额外维护映射表。
        let candidates: Vec<(i64, String)> = query_as(
            "SELECT ep.master_product_id, ep.name
             FROM lot_candidates lc
             JOIN event_products ep ON ep.id = lc.event_product_id
             WHERE lc.lot_id = ?
             ORDER BY lc.event_product_id",
        )
        .bind(l.source_lot_id)
        .fetch_all(&mut *tx)
        .await?;

        let mut target_ids: Vec<i64> = Vec::with_capacity(candidates.len());
        for (master_product_id, product_name) in &candidates {
            let target: Option<i64> = query_scalar(
                "SELECT id FROM event_products WHERE event_id = ? AND master_product_id = ?",
            )
            .bind(event_id)
            .bind(master_product_id)
            .fetch_optional(&mut *tx)
            .await?;
            let target = target.ok_or_else(|| {
                ApiError::BadRequest(format!(
                    "套装「{}」的候选商品「{}」不在本场",
                    source.name, product_name
                ))
            })?;
            target_ids.push(target);
        }

        // 沿用源套装的 name / pick_count / total_price / allow_repeat，
        // 校验走 create_lot 的同一套函数（同一货主等）。
        let lot = insert_lot(
            &mut tx,
            event_id,
            &source.name,
            source.pick_count,
            source.total_price,
            source.allow_repeat != 0,
            &target_ids,
        )
        .await?;
        lots.push(lot);
    }

    tx.commit().await?;

    // 提交后再取商品响应：`load_one` 需要聚合余额与累计进货，是新事务的读。
    let mut products: Vec<EventProductResponse> = Vec::with_capacity(new_product_ids.len());
    for id in new_product_ids {
        products.push(load_one(&state.db, id).await?);
    }

    Ok((StatusCode::OK, Json(ImportResponse { products, lots })))
}

// ==========================================
// 3. 补货 (Admin/Vendor)
// ==========================================
#[derive(Deserialize, ToSchema)]
#[schema(as = ProductRestockRequest)]
struct RestockRequest {
    qty: i64,
    note: Option<String>,
}

/// 给某场展会里的商品补货（外部 → 现场仓）。需要管理员或本场摊主。
#[utoipa::path(
    post,
    path = "/events/{event_id}/products/{id}/restock",
    tag = "product",
    params(
        ("event_id" = i64, Path, description = "展会 id"),
        ("id" = i64, Path, description = "场次商品 id"),
    ),
    request_body = RestockRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, body = EventProductResponse),
        (status = 400, body = ApiErrorBody, description = "补货数量必须为正"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "商品不存在或不属于这场展会"),
        (status = 409, body = ApiErrorBody, description = "展会已结算"),
    ),
)]
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
#[derive(Deserialize, ToSchema)]
#[schema(as = ProductUpdateRequest)]
struct UpdateProductRequest {
    /// 单位：分。
    ///
    /// **`initial_stock` 不再接受**：旧接口用 `current_stock += (new - old)` 硬调数字，
    /// 正是 spec 点名的三个互不共享写入方之一。请求体里即使带了 `initial_stock`，
    /// 也会被 serde 默认忽略——改库存必须走进货/盘点。
    #[schema(value_type = Option<Money>)]
    unit_price: Option<i64>,
}

/// 改某场展会里商品的单价。需要管理员或本场摊主。
#[utoipa::path(
    put,
    path = "/products/{id}",
    tag = "product",
    params(("id" = i64, Path, description = "场次商品 id")),
    request_body = UpdateProductRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, body = EventProductResponse),
        (status = 400, body = ApiErrorBody, description = "单价不能为负"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "商品不存在"),
        (status = 409, body = ApiErrorBody, description = "展会已结算"),
    ),
)]
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
/// 删除成功后的固定消息体。字段名 `message` 是既有响应形状的一部分。
#[derive(Serialize, ToSchema)]
#[schema(as = ProductDeleteMessage)]
struct DeleteProductResponse {
    message: String,
}

/// 从某场展会下架商品。需要管理员或本场摊主。
#[utoipa::path(
    delete,
    path = "/products/{id}",
    tag = "product",
    params(("id" = i64, Path, description = "场次商品 id")),
    security(("bearer" = [])),
    responses(
        (status = 200, body = DeleteProductResponse),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "商品不存在"),
        (status = 409, body = ApiErrorBody, description = "商品已有进出记录或展会已结算"),
    ),
)]
async fn delete_product(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> ApiResult<(StatusCode, Json<DeleteProductResponse>)> {
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
        Json(DeleteProductResponse {
            message: "Product removed from event".to_string(),
        }),
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

    // ==========================================
    // import：批量上架 + 从上一场复制套装
    // ==========================================

    /// 源展会 2：master A(1)/C(3) 各上架一次（event_product 10/11，**同属本社团**），
    /// 外加一个候选为二者的套装。返回 `(source_lot_id, source_ep_a, source_ep_c)`。
    ///
    /// 两个候选必须同一货主：`insert_lot` 会走 `validate_candidates`，
    /// 跨货主的套装源展会自己就建不出来（`create_lot` 就拒了）。
    async fn seed_source_event(pool: &sqlx::SqlitePool) -> (i64, i64, i64) {
        sqlx::query(
            "INSERT INTO events (id, name, event_date, status)
             VALUES (2, '上一场', '2026-09-01', '进行中')",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO master_products (id, product_code, name, default_price, owner_society_id)
             VALUES (3, 'C', '挂件C', 18.0, 1)",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO event_products
               (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
             VALUES (10, 2, 1, 1, 'A', '本子A', 2800),
                    (11, 2, 3, 1, 'C', '挂件C', 1800)",
        )
        .execute(pool)
        .await
        .unwrap();
        let lot_id =
            crate::test_support::seed_lot_repeat(pool, 2, "上一场任选2件40", 2, 4000, &[10, 11])
                .await;
        (lot_id, 10, 11)
    }

    /// 空的目标展会 3，用来验证「从别场导入」。
    async fn seed_empty_target(pool: &sqlx::SqlitePool) {
        sqlx::query(
            "INSERT INTO events (id, name, event_date, status)
             VALUES (3, '本场', '2026-10-01', '进行中')",
        )
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn import_products_and_lot_from_previous_event() {
        let (router, _dir, pool) = test_router_with().await;
        seed_event_and_product(&pool).await;
        let (lot_id, _, _) = seed_source_event(&pool).await;
        seed_empty_target(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/events/3/products/import",
                Some(&token),
                json!({
                    "products": [
                        {"master_product_id": 1, "unit_price": 2800, "initial_stock": 4},
                        {"master_product_id": 3, "unit_price": 1800, "initial_stock": 0}
                    ],
                    "lots": [{"source_lot_id": lot_id}]
                }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;

        let products = body["products"].as_array().unwrap();
        assert_eq!(products.len(), 2);
        let a = products
            .iter()
            .find(|p| p["master_product_id"] == 1)
            .unwrap();
        let c = products
            .iter()
            .find(|p| p["master_product_id"] == 3)
            .unwrap();
        assert_eq!(a["unit_price"], 2800, "沿用请求里的本次售价");
        assert_eq!(a["onsite_qty"], 4);
        assert_eq!(a["stocked_qty"], 4);
        assert_eq!(c["unit_price"], 1800);
        assert_eq!(c["onsite_qty"], 0, "库存 0 不预填");
        assert_eq!(c["stocked_qty"], 0);

        let target_a = a["id"].as_i64().unwrap();
        let target_c = c["id"].as_i64().unwrap();
        let lots = body["lots"].as_array().unwrap();
        assert_eq!(lots.len(), 1);
        assert_eq!(lots[0]["name"], "上一场任选2件40");
        assert_eq!(lots[0]["pick_count"], 2);
        assert_eq!(lots[0]["total_price"], 4000);
        assert_eq!(lots[0]["allow_repeat"], true, "allow_repeat 必须保留");
        let mut cands: Vec<i64> = lots[0]["candidate_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap())
            .collect();
        cands.sort_unstable();
        let mut expected = vec![target_a, target_c];
        expected.sort_unstable();
        assert_eq!(cands, expected, "候选必须映射成目标展会的 event_product id");
    }

    #[tokio::test]
    async fn import_maps_lot_to_existing_target_product() {
        // Review Focus 1：目标展会原本就有 A，请求只带 C + 套装 {A, C}。
        // A 必须映射到目标已有的 event_product，套装照常建出。
        let (router, _dir, pool) = test_router_with().await;
        seed_event_and_product(&pool).await;
        let (lot_id, _, _) = seed_source_event(&pool).await;
        seed_empty_target(&pool).await;
        sqlx::query(
            "INSERT INTO event_products
               (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
             VALUES (20, 3, 1, 1, 'A', '本子A', 3000)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/events/3/products/import",
                Some(&token),
                json!({"products": [{"master_product_id": 3, "unit_price": 1800, "initial_stock": 0}],
                       "lots": [{"source_lot_id": lot_id}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        let c_id = body["products"][0]["id"].as_i64().unwrap();
        let mut cands: Vec<i64> = body["lots"][0]["candidate_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap())
            .collect();
        cands.sort_unstable();
        let mut expected = vec![20, c_id];
        expected.sort_unstable();
        assert_eq!(cands, expected, "已在本场的 A 用 20，新导入的 C 用新 id");
    }

    #[tokio::test]
    async fn import_rolls_back_all_on_lot_failure() {
        // Review Focus 5：套装有一件候选既不在本场也不在请求里 → 400，
        // 已经建好的商品与首批进货 journal 都必须跟着回滚。
        let (router, _dir, pool) = test_router_with().await;
        seed_event_and_product(&pool).await;
        let (lot_id, _, _) = seed_source_event(&pool).await;
        seed_empty_target(&pool).await;
        let token = admin_token();
        let journals_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool)
            .await
            .unwrap();

        // 只导入 A；套装还需要 C（源 11 / master 3，本场没有、请求也没带）。
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/events/3/products/import",
                Some(&token),
                json!({"products": [{"master_product_id": 1, "unit_price": 2800, "initial_stock": 3}],
                       "lots": [{"source_lot_id": lot_id}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = read_json(res).await;
        let error = body["error"].as_str().unwrap();
        assert!(
            error.contains("上一场任选2件40"),
            "要说清是哪个套装：{error}"
        );
        assert!(error.contains("挂件C"), "要说清是哪个候选：{error}");

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM event_products WHERE event_id = 3")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 0, "整批回滚：半路建出来的商品也不能留下");
        let journals_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(journals_after, journals_before, "首批进货 journal 也要回滚");
    }

    #[tokio::test]
    async fn import_duplicate_product_conflicts() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, _ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let before: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM event_products WHERE event_id = ?")
                .bind(event_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/products/import"),
                Some(&token),
                json!({"products": [{"master_product_id": 1, "unit_price": 1000, "initial_stock": 5}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        let body = read_json(res).await;
        assert!(
            body["error"].as_str().unwrap().contains("本子A"),
            "重复上架的错误必须含商品名：{body}"
        );

        let after: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM event_products WHERE event_id = ?")
                .bind(event_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(after, before, "409 不能有任何写入");
    }

    #[tokio::test]
    async fn import_rejected_when_event_settled() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
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
                &format!("/api/events/{event_id}/products/import"),
                Some(&token),
                json!({"products": [{"master_product_id": 1, "unit_price": 1000, "initial_stock": 1}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn import_rejects_lot_from_same_event() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let lot_id =
            crate::test_support::seed_lot_repeat(&pool, event_id, "本场套装", 1, 1000, &[ep_a])
                .await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/products/import"),
                Some(&token),
                json!({"lots": [{"source_lot_id": lot_id}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = read_json(res).await;
        assert!(
            body["error"].as_str().unwrap().contains("不能从本场导入"),
            "{body}"
        );
    }

    #[tokio::test]
    async fn import_zero_stock_writes_no_journal() {
        let (router, _dir, pool) = test_router_with().await;
        seed_event_and_product(&pool).await;
        seed_empty_target(&pool).await;
        let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool)
            .await
            .unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/events/3/products/import",
                Some(&token),
                json!({"products": [{"master_product_id": 1, "unit_price": 2800, "initial_stock": 0}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        assert_eq!(body["products"][0]["onsite_qty"], 0);
        assert_eq!(body["products"][0]["stocked_qty"], 0);
        let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(after, before, "initial_stock = 0 不写 journal");
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

    /// 一场进行中的展会。A/B 是夹具原有的两个商品；A 补上图片/分类/标签，让列表的
    /// 可选字段至少出现一次非空。C 直接挂进场次且没有任何流水（删得掉），D 留给选品
    /// 测试添加（所以不能预先在场次里）。
    async fn seeded() -> (Router, tempfile::TempDir, i64, i64, i64) {
        let (router, dir, pool) = test_router_with().await;
        let (event_id, ep_a, _ep_b) = seed_event_and_product(&pool).await;

        sqlx::query(
            "UPDATE master_products
                SET image_url = '/uploads/products/a.png', category = '本子', tags = '原创,文具'
              WHERE id = 1",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO master_products
               (id, product_code, name, default_price, owner_society_id, image_url, category, tags)
             VALUES (3, 'C', '挂件C', 15.0, 1, '/uploads/products/c.png', '挂件', '原创'),
                    (4, 'D', '挂件D', 18.0, 2, '/uploads/products/d.png', '挂件', '原创')",
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

        (router, dir, event_id, ep_a, clean_id)
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

    /// 列表：A 有图片/分类，B 没有 → 两个可选字段合并成 "null|string"。
    fn product_row_list() -> Value {
        json!({
            "id": "int",
            "event_id": "int",
            "master_product_id": "int",
            "owner_society_id": "int",
            "owner_society_name": "string",
            "product_code": "string",
            "name": "string",
            "unit_price": "int",
            "stocked_qty": "int",
            "onsite_qty": "int",
            "image_url": "null|string",
            "category": "null|string",
            "tags": "string",
        })
    }

    /// 单个：A 或新选的 D 都有图片/分类。
    fn product_row_single() -> Value {
        json!({
            "id": "int",
            "event_id": "int",
            "master_product_id": "int",
            "owner_society_id": "int",
            "owner_society_name": "string",
            "product_code": "string",
            "name": "string",
            "unit_price": "int",
            "stocked_qty": "int",
            "onsite_qty": "int",
            "image_url": "string",
            "category": "string",
            "tags": "string",
        })
    }

    #[tokio::test]
    async fn shape_list_event_products() {
        let (router, _dir, event_id, _, _) = seeded().await;
        let (s, body) = call(
            &router,
            "GET",
            &format!("/api/events/{event_id}/products"),
            Some(&admin_token()),
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(shape_of(&body), json!([product_row_list()]));
    }

    #[tokio::test]
    async fn shape_add_product_to_event() {
        let (router, _dir, event_id, _, _) = seeded().await;
        let t = admin_token();
        let (s, body) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/products"),
            Some(&t),
            json!({"product_code": "D", "initial_stock": 8, "unit_price": 1500}),
        )
        .await;
        assert_eq!(s, StatusCode::CREATED);
        assert_eq!(shape_of(&body), product_row_single());

        // 错误分支：已经在场次里的商品 → 409
        let (s, body) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/products"),
            Some(&t),
            json!({"product_code": "A", "initial_stock": 1}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::CONFLICT, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_restock_product() {
        let (router, _dir, event_id, ep_a, _) = seeded().await;
        let t = admin_token();
        let (s, body) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/products/{ep_a}/restock"),
            Some(&t),
            json!({"qty": 5, "note": "朋友顺路带来"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(shape_of(&body), product_row_single());

        // 错误分支：非正数量 → 400
        let (s, body) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/products/{ep_a}/restock"),
            Some(&t),
            json!({"qty": 0}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::BAD_REQUEST, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_update_product() {
        let (router, _dir, _event_id, ep_a, _) = seeded().await;
        let t = admin_token();
        let (s, body) = call(
            &router,
            "PUT",
            &format!("/api/products/{ep_a}"),
            Some(&t),
            json!({"unit_price": 2500}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(shape_of(&body), product_row_single());

        // 错误分支：负单价 → 400
        let (s, body) = call(
            &router,
            "PUT",
            &format!("/api/products/{ep_a}"),
            Some(&t),
            json!({"unit_price": -1}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::BAD_REQUEST, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_delete_product() {
        let (router, _dir, _event_id, ep_a, clean_id) = seeded().await;
        let t = admin_token();
        let (s, body) = call(
            &router,
            "DELETE",
            &format!("/api/products/{clean_id}"),
            Some(&t),
            json!({}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(shape_of(&body), json!({"message": "string"}));

        // 错误分支：有进货流水的商品删不掉 → 409
        let (s, body) = call(
            &router,
            "DELETE",
            &format!("/api/products/{ep_a}"),
            Some(&t),
            json!({}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::CONFLICT, json!({"error": "string"}))
        );
    }

    /// 目标展会 1（有 A/B/C）+ 源展会 2（上架了 C、配了一个只含 C 的套装，
    /// 供 import 复制）。C 带图片/分类/标签，单商品的形状快照才钉得住 string。
    async fn seeded_import() -> (Router, tempfile::TempDir, i64, i64) {
        let (router, dir, pool) = test_router_with().await;
        let (event_id, _ep_a, _ep_b) = seed_event_and_product(&pool).await;
        sqlx::query(
            "INSERT INTO master_products
               (id, product_code, name, default_price, owner_society_id, image_url, category, tags)
             VALUES (3, 'C', '挂件C', 15.0, 1, '/uploads/products/c.png', '挂件', '原创')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO events (id, name, event_date, status)
             VALUES (2, '上一场', '2026-09-01', '进行中')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO event_products
               (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
             VALUES (10, 2, 3, 1, 'C', '挂件C', 1500)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let lot_id =
            crate::test_support::seed_lot_repeat(&pool, 2, "上一场套装", 1, 1000, &[10]).await;
        (router, dir, event_id, lot_id)
    }

    fn import_lot_shape() -> Value {
        json!({
            "id": "int", "event_id": "int", "name": "string", "pick_count": "int",
            "total_price": "int", "allow_repeat": "bool",
            "candidate_ids": ["int"], "owner_society_id": "int", "owner_society_name": "string",
        })
    }

    #[tokio::test]
    async fn shape_import_products_and_lots() {
        let (router, _dir, event_id, lot_id) = seeded_import().await;
        let t = admin_token();
        let (s, body) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/products/import"),
            Some(&t),
            json!({"products": [{"master_product_id": 3, "unit_price": 1500, "initial_stock": 0}],
                   "lots": [{"source_lot_id": lot_id}]}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({"products": [product_row_single()], "lots": [import_lot_shape()]})
        );

        // 错误分支：两样都空 → 400
        let (s, body) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/products/import"),
            Some(&t),
            json!({}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::BAD_REQUEST, json!({"error": "string"}))
        );

        // 错误分支：重复上架 → 409（C 已经在上面那次导入进了本场）
        let (s, body) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/products/import"),
            Some(&t),
            json!({"products": [{"master_product_id": 3, "unit_price": 1500, "initial_stock": 0}]}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::CONFLICT, json!({"error": "string"}))
        );
    }
}
