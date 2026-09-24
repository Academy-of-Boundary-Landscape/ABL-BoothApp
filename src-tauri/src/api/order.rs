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
    api::guard::{check_read_permission, check_write_permission},
    db::models::OrderRow,
    domain::{
        allocation::apply_manual_adjustment,
        ledger::{
            home_society_id, onsite_balance, post_journal, reverse_order_journals, Account,
            JournalKind, Location, MoneyLeg, StockLeg,
        },
        money::Money,
        pricing::{
            cart_lines, ensure_event_selling, load_lots, merge_items, price_cart, resolve_cart,
            CartItemRequest,
        },
    },
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

/// ③b 过渡：本模块的路由还没标 utoipa 注解，整体包进 OpenApiRouter——路由照常工作，
/// 只是文档里没有它的路径。阶段 2 迁移本模块时改为 `routes!(...)` 并删掉 `legacy_router`。
pub fn router() -> utoipa_axum::router::OpenApiRouter<AppState> {
    utoipa_axum::router::OpenApiRouter::from(legacy_router())
}

fn legacy_router() -> Router<AppState> {
    Router::new()
        // 公开：顾客下单，无需 token
        .route("/events/{event_id}/orders", post(create_order))
        // 管理员/摊主：查看订单列表
        .route("/events/{event_id}/orders", get(list_orders))
        // 管理员/摊主：更新订单状态（完成 / 取消）
        .route(
            "/events/{event_id}/orders/{order_id}/status",
            put(update_order_status),
        )
}

// ==========================================
// 请求 / 响应结构
// ==========================================

#[derive(Deserialize)]
struct CreateOrderRequest {
    items: Vec<CartItemRequest>,
}

#[derive(Deserialize)]
struct UpdateStatusRequest {
    status: String,
    channel: Option<String>,
    /// 摊主手工改的实收金额（分）。不给就等于 `solved_amount`。
    ///
    /// 落点是「确认收款」这一步（spec 4.3）：现场的手势本来就是
    /// 「报个数、收钱、点完成」，拆成两步只会多一次忘记。
    final_amount: Option<i64>,
    /// 要拆掉的套装实例（`order_lots.id`）。
    ///
    /// **拆 ≠ 改总价。** 改总价把差额记成手工折让、按 spec 4.4 整笔落本社团；
    /// 而「这个套装不该套用」是纠错，钱必须回到真正的货主头上。代卖货上
    /// 只改总价会让货主少拿钱、差额挂在本社团头上——金额总数对，归属错。
    unapply_lot_ids: Option<Vec<i64>>,
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
    lots: Vec<OrderLotResponse>,
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
    /// 这一行进了哪个套装。`None` = 散卖。
    ///
    /// 同一个商品可能在一张订单里出现两次（2 件进套装、1 件散着，spec 4.5），
    /// 摊主必须看得出哪一行是哪一种，否则配货时会以为系统重复计数了。
    lot_name: Option<String>,
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
    lot_name: Option<String>,
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
            lot_name: row.lot_name,
        }
    }
}

/// 订单上的一个套装**实例**。
///
/// `id` 是 `order_lots.id` 而不是 `lots.id`——同一个套装可以套用多次
/// （「任选3本100」买 6 本 = 两个实例），拆的时候必须能指到具体是哪一次。
#[derive(Serialize)]
struct OrderLotResponse {
    id: i64,
    /// 原始 Lot 的 id。套装被删掉后是 null（`ON DELETE SET NULL`），名字和价格仍在。
    lot_id: Option<i64>,
    name: String,
    /// 套装价（分），下单那一刻的快照。
    price: i64,
    /// 成分按原价的合计（分）。**摊主拆掉它时应收会回到这个数**，
    /// 收款弹窗靠它在本地把新的应收算出来，不必多一次往返。
    original_amount: i64,
}

#[derive(sqlx::FromRow)]
struct OrderLotRow {
    id: i64,
    order_id: i64,
    lot_id: Option<i64>,
    name: String,
    price: i64,
    original_amount: i64,
}

impl From<OrderLotRow> for OrderLotResponse {
    fn from(row: OrderLotRow) -> Self {
        OrderLotResponse {
            id: row.id,
            lot_id: row.lot_id,
            name: row.name,
            price: row.price,
            original_amount: row.original_amount,
        }
    }
}

const ITEMS_BY_ORDER: &str = "
SELECT ol.id, ol.order_id, ol.event_product_id, ol.qty, ol.unit_price,
       ol.allocated_amount, ol.paid_amount,
       ep.name AS product_name, mp.image_url AS product_image_url,
       olo.name AS lot_name
FROM order_lines ol
JOIN event_products ep ON ep.id = ol.event_product_id
JOIN master_products mp ON mp.id = ep.master_product_id
LEFT JOIN order_lots olo ON olo.id = ol.order_lot_id
WHERE ol.order_id = ?
ORDER BY ol.id
";

const ITEMS_BY_EVENT: &str = "
SELECT ol.id, ol.order_id, ol.event_product_id, ol.qty, ol.unit_price,
       ol.allocated_amount, ol.paid_amount,
       ep.name AS product_name, mp.image_url AS product_image_url,
       olo.name AS lot_name
FROM order_lines ol
JOIN event_products ep ON ep.id = ol.event_product_id
JOIN master_products mp ON mp.id = ep.master_product_id
LEFT JOIN order_lots olo ON olo.id = ol.order_lot_id
JOIN orders o ON o.id = ol.order_id
WHERE o.event_id = ?
ORDER BY ol.order_id, ol.id
";

const LOTS_BY_ORDER: &str = "
SELECT olo.id, olo.order_id, olo.lot_id, olo.name, olo.price,
       COALESCE(SUM(ol.unit_price * ol.qty), 0) AS original_amount
FROM order_lots olo
LEFT JOIN order_lines ol ON ol.order_lot_id = olo.id
WHERE olo.order_id = ?
GROUP BY olo.id, olo.order_id, olo.lot_id, olo.name, olo.price
ORDER BY olo.id
";

const LOTS_BY_EVENT: &str = "
SELECT olo.id, olo.order_id, olo.lot_id, olo.name, olo.price,
       COALESCE(SUM(ol.unit_price * ol.qty), 0) AS original_amount
FROM order_lots olo
JOIN orders o ON o.id = olo.order_id
LEFT JOIN order_lines ol ON ol.order_lot_id = olo.id
WHERE o.event_id = ?
GROUP BY olo.id, olo.order_id, olo.lot_id, olo.name, olo.price
ORDER BY olo.order_id, olo.id
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

    let lots: Vec<OrderLotResponse> = query_as::<_, OrderLotRow>(LOTS_BY_ORDER)
        .bind(order_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(OrderResponse { order, items, lots })
}

// ==========================================
// 1. 创建订单（公开，BEGIN IMMEDIATE）
// ==========================================
async fn create_order(
    State(state): State<AppState>,
    Path(event_id): Path<i64>,
    Json(payload): Json<CreateOrderRequest>,
) -> ApiResult<impl IntoResponse> {
    // 不需要展会守卫：下单要求更严的「进行中」，守卫在 ensure_event_selling，不能放宽成「不是已结算」
    let merged = merge_items(&payload.items)?;

    // 展会状态放在事务之前，省得白拿写锁。
    {
        let mut conn = state.db.acquire().await?;
        ensure_event_selling(&mut conn, event_id).await?;
    }

    // 防超卖是「查余额 → 插移动」两步，SQLite 单写者模型下仍然安全，前提是
    // 两步在同一个 BEGIN IMMEDIATE 事务里——默认的 deferred 事务在第一次写之前不持写锁。
    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    let resolved = resolve_cart(&mut tx, event_id, &merged).await?;

    let mut stock_legs: Vec<StockLeg> = Vec::with_capacity(resolved.len());
    for r in &resolved {
        let left = onsite_balance(&mut *tx, r.product.id).await?;
        if left < r.qty {
            return Err(ApiError::Conflict(format!(
                "「{}」库存不足",
                r.product.name
            )));
        }
        stock_legs.push(StockLeg {
            event_product_id: r.product.id,
            from: Location::OnSite,
            to: Location::Customer,
            qty: r.qty,
        });
    }

    // 求解 + 分摊。**下单和 /quote 走的是同一份 price_cart**，所以顾客在购物车里
    // 看到的价和这里算出来的价不可能因为实现漂移而分叉。
    let lots = load_lots(&mut tx, event_id).await?;
    let priced = price_cart(&cart_lines(&resolved), &lots)?;

    // 手工覆盖在「确认收款」那一步才发生（spec 4.3），所以下单时 final = solved。
    let order: OrderRow = query_as(
        "INSERT INTO orders (event_id, status, gross_amount, solved_amount, final_amount)
         VALUES (?, 'pending', ?, ?, ?) RETURNING *",
    )
    .bind(event_id)
    .bind(priced.gross.cents())
    .bind(priced.solved.cents())
    .bind(priced.solved.cents())
    .fetch_one(&mut *tx)
    .await?;
    let order_id = order.id;

    // Lot 实例：名字和价格在这里**快照**下来。之后摊主改 Lot 价、甚至删掉这个 Lot，
    // 都不影响已经下了的单（`order_lots.lot_id` 是 ON DELETE SET NULL）。
    let mut order_lot_ids: Vec<i64> = Vec::with_capacity(priced.lots.len());
    for lot in &priced.lots {
        let id: i64 = query_scalar(
            "INSERT INTO order_lots (order_id, lot_id, name, price) VALUES (?, ?, ?, ?) RETURNING id",
        )
        .bind(order_id)
        .bind(lot.lot_id)
        .bind(&lot.name)
        .bind(lot.price.cents())
        .fetch_one(&mut *tx)
        .await?;
        order_lot_ids.push(id);
    }

    // 商品快照按 event_product_id 索引：拆出来的两行共用同一份名字和图。
    let snapshot: HashMap<i64, &crate::domain::pricing::CartProduct> = resolved
        .iter()
        .map(|r| (r.product.id, &r.product))
        .collect();

    let mut items = Vec::with_capacity(priced.lines.len());
    for line in &priced.lines {
        let order_lot_id = line.lot_index.map(|k| order_lot_ids[k]);
        let line_id: i64 = query_scalar(
            "INSERT INTO order_lines
               (order_id, event_product_id, order_lot_id, qty, unit_price, allocated_amount, paid_amount)
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(order_id)
        .bind(line.event_product_id)
        .bind(order_lot_id)
        .bind(line.qty)
        .bind(line.unit_price.cents())
        .bind(line.allocated.cents())
        // 下单时 paid = allocated。手工折让要到「确认收款」那一步才摊进来（Task 7）。
        .bind(line.allocated.cents())
        .fetch_one(&mut *tx)
        .await?;

        let p = snapshot[&line.event_product_id];
        items.push(OrderItemResponse {
            id: line_id,
            product_id: line.event_product_id,
            quantity: line.qty,
            product_name: p.name.clone(),
            product_price: line.unit_price.cents(),
            product_image_url: p.image_url.clone(),
            allocated_amount: line.allocated.cents(),
            paid_amount: line.allocated.cents(),
            lot_name: line.lot_index.map(|k| priced.lots[k].name.clone()),
        });
    }

    // 销售 journal：现场仓 → 顾客仓，**每个商品一条腿，不跟着订单行拆**。
    // 这是 `api/product.rs` 的「有流水才禁止删除」守卫仍然完备的前提
    // （②-1 交接段第 2 条）。
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

    // 成分原价合计从 priced.lines 聚合，和分摊用的是同一组数。
    let lots_response: Vec<OrderLotResponse> = priced
        .lots
        .iter()
        .enumerate()
        .map(|(k, lot)| OrderLotResponse {
            id: order_lot_ids[k],
            lot_id: Some(lot.lot_id),
            name: lot.name.clone(),
            price: lot.price.cents(),
            original_amount: priced
                .lines
                .iter()
                .filter(|l| l.lot_index == Some(k))
                // 这里是未检查乘法，但安全：price_cart 对**同一组** (unit_price, qty)
                // 已经跑过 checked_mul_qty，溢出会在那一步提前返回 Err，能走到这里就已经证明乘不爆。
                // 不要改成 checked——那会多一条永远走不到的错误分支；将来挪动 price_cart 里的检查位置时，
                // 必须同步确认这层依赖仍然成立。
                .map(|l| l.unit_price.cents() * l.qty)
                .sum(),
        })
        .collect();

    Ok((
        StatusCode::CREATED,
        Json(OrderResponse {
            order,
            items,
            lots: lots_response,
        }),
    ))
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

    let lot_rows: Vec<OrderLotRow> = query_as(LOTS_BY_EVENT)
        .bind(event_id)
        .fetch_all(&state.db)
        .await?;
    let mut lots_map: HashMap<i64, Vec<OrderLotResponse>> = HashMap::new();
    for row in lot_rows {
        let oid = row.order_id;
        lots_map.entry(oid).or_default().push(row.into());
    }

    let result = orders
        .into_iter()
        .map(|order| {
            let oid = order.id;
            OrderResponse {
                order,
                items: items_map.remove(&oid).unwrap_or_default(),
                lots: lots_map.remove(&oid).unwrap_or_default(),
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

    // 静默忽略一个用户传了的字段是会咬人的。取消路径上拆套装没有意义。
    if payload.unapply_lot_ids.is_some() && target != "completed" {
        return Err(ApiError::BadRequest("只有在完成订单时才能拆套装".into()));
    }

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    // ②-1 交接段列的第 4 个敞口。放在事务内、读订单状态之前。
    crate::api::guard::require_event_open(&mut tx, event_id).await?;

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
            let raw = payload
                .channel
                .as_deref()
                .ok_or_else(|| ApiError::BadRequest("完成订单必须提供收款渠道".into()))?;
            // 渠道名会成为账户名的一部分，规范化必须在写库之前（domain/channel.rs 的模块注释）
            let channel = crate::domain::channel::normalize(raw)?;

            // 拆套装（spec 4.3 的辅助通道）。必须排在读 solved_amount 之前——
            // 拆完 solved 会变，而手工折让是相对**新的** solved 算的。
            if let Some(ids) = payload.unapply_lot_ids.as_deref() {
                let mut ids = ids.to_vec();
                ids.sort_unstable();
                ids.dedup(); // 传重了不该变成「第二次找不到」的 400
                for order_lot_id in &ids {
                    let belongs: Option<i64> =
                        query_scalar("SELECT id FROM order_lots WHERE id = ? AND order_id = ?")
                            .bind(order_lot_id)
                            .bind(order_id)
                            .fetch_optional(&mut *tx)
                            .await?;
                    belongs
                        .ok_or_else(|| ApiError::BadRequest("要拆的套装不属于这张订单".into()))?;

                    // 成分行金额恢复成「单价 × 件数」，并解除归属。
                    // **不重新求解**：纯拆解，结果唯一，也不依赖当前的 Lot 配置
                    // （摊主可能刚把那个 Lot 改了或删了）。
                    //
                    // 这里是未检查乘法，但不可达：`unit_price`/`qty` 是下单时写进
                    // `order_lines` 的快照，下单路径上 `price_cart` 已对同一组
                    // (unit_price, qty) 跑过 `checked_mul_qty`，件数又受 `MAX_UNITS = 300`
                    // 约束，能走到这里就已经证明乘不爆。不要改成 checked——那会多一条永远
                    // 走不到的错误分支。真溢出的话 SQLite 会把 INTEGER 提升成 REAL，往
                    // INTEGER 亲和性的列里写进一个浮点，后果比报错更隐蔽，所以这层依赖必须
                    // 随 `price_cart` 的检查位置一起复核。
                    query(
                        "UPDATE order_lines
                         SET allocated_amount = unit_price * qty, order_lot_id = NULL
                         WHERE order_lot_id = ?",
                    )
                    .bind(order_lot_id)
                    .execute(&mut *tx)
                    .await?;

                    query("DELETE FROM order_lots WHERE id = ?")
                        .bind(order_lot_id)
                        .execute(&mut *tx)
                        .await?;
                }

                // solved 重新从各行聚合。**gross 不动**——原价合计不因为拆而改变。
                // 货那一侧同样一个字不动（spec 4.3 第一条约束）。
                query(
                    "UPDATE orders SET solved_amount =
                       (SELECT COALESCE(SUM(allocated_amount), 0)
                        FROM order_lines WHERE order_id = ?)
                     WHERE id = ?",
                )
                .bind(order_id)
                .bind(order_id)
                .execute(&mut *tx)
                .await?;
            }

            let solved: i64 = query_scalar("SELECT solved_amount FROM orders WHERE id = ?")
                .bind(order_id)
                .fetch_one(&mut *tx)
                .await?;
            let final_amount = payload.final_amount.unwrap_or(solved);
            // 这里和 domain/allocation.rs 的 apply_manual_adjustment 里那道 final_amount < 0 检查重复，
            // 内层在这条路径上不可达。保留 handler 这一处，它更靠前、给出的错误更便宜；两处文案要一起改。
            // **不要删内层那道**——那是纯函数自己的前置条件，还有别的调用方会来。
            if final_amount < 0 {
                return Err(ApiError::BadRequest("实收金额不能为负".into()));
            }

            // 手工折让摊进各行 → paid_amount。**按 id 排序读行**：分摊的取整规则
            // 依赖稳定的顺序，同样的输入两次必须算出同一个结果（spec 4.5）。
            let lines: Vec<(i64, i64, i64)> = query_as(
                "SELECT id, allocated_amount, qty FROM order_lines WHERE order_id = ? ORDER BY id",
            )
            .bind(order_id)
            .fetch_all(&mut *tx)
            .await?;
            let allocated: Vec<Money> = lines
                .iter()
                .map(|(_, a, _)| Money::from_cents(*a))
                .collect();
            let qty: Vec<i64> = lines.iter().map(|(_, _, q)| *q).collect();
            let paid = apply_manual_adjustment(
                &allocated,
                &qty,
                Money::from_cents(solved),
                Money::from_cents(final_amount),
            )?;
            for ((line_id, _, _), p) in lines.iter().zip(&paid) {
                query("UPDATE order_lines SET paid_amount = ? WHERE id = ?")
                    .bind(p.cents())
                    .bind(line_id)
                    .execute(&mut *tx)
                    .await?;
            }

            // 按货主分组，Σ 该货主各行的 **allocated_amount**。
            // 用 allocated 而不是 paid 是 spec 4.4 的全部要点：代卖社团该得多少，
            // 不受摊主当天做了什么人情的影响。
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

            let mut money_legs: Vec<MoneyLeg> = owner_totals
                .into_iter()
                .map(|(owner_id, cents)| MoneyLeg {
                    from: Account::SocietyDue(owner_id),
                    to: Account::Received(channel.clone()),
                    amount: Money::from_cents(cents),
                })
                .filter(|leg| !leg.amount.is_zero())
                .collect();

            // 手工折让/加价：**整笔落在本社团头上，不按货主分摊**（spec 4.4）。
            // 这条规则不依赖订单里有没有本社团的行——整单都是代卖货、摊主说
            // 「算你 15」，那 5 块一样落到本社团头上。
            let diff = solved - final_amount;
            if diff != 0 {
                // 「本社团」有且只有一个，由 idx_societies_home 这个偏唯一索引保证，
                // 且 api/society.rs 不允许把它降级成「没有本社团」。
                // 没有时给一句能照着做的话，而不是 panic 成 500。
                let home = home_society_id(&mut tx).await?;
                // ⚠️ 金额恒取绝对值，方向由 from/to 表达（spec 4.4 末句）。
                // 照「金额 = solved − final」直接传，加价时它是负数，而 post_journal
                // 对非正金额直接返回「资金移动的金额必须为正」——每一张加价订单的
                // 「完成」都会 400。
                money_legs.push(if diff > 0 {
                    MoneyLeg {
                        from: Account::Received(channel.clone()),
                        to: Account::SocietyDue(home),
                        amount: Money::from_cents(diff),
                    }
                } else {
                    MoneyLeg {
                        from: Account::SocietyDue(home),
                        to: Account::Received(channel.clone()),
                        amount: Money::from_cents(-diff),
                    }
                });
            }

            // 全 0 金额（全赠品订单）时没有钱可记，跳过收款 journal 而不是
            // 让 post_journal 拒绝「空的 journal」。
            // ⚠️ 于是「订单是 completed」**不蕴含**「存在收款 journal」——
            // ②-3 做结算对账时要按 orders.status 判定，不能按 journal 存在性。
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
                "UPDATE orders SET status = 'completed', channel = ?, final_amount = ?,
                        completed_at = CURRENT_TIMESTAMP
                 WHERE id = ?",
            )
            .bind(&channel)
            .bind(final_amount)
            .bind(order_id)
            .execute(&mut *tx)
            .await?;
        }
        ("pending", "cancelled") | ("completed", "cancelled") => {
            // 同一个调用把货和钱两个 journal 一起冲掉（货回到现场仓、钱按反方向回滚）。
            // 不要自己遍历 journal 反向插移动——reverse_order_journals 已经处理了
            // 「跳过已被冲正的」和「重复冲正被偏唯一索引拦住」。
            reverse_order_journals(&mut tx, order_id, Some("取消订单")).await?;

            // 故意**不清** channel / completed_at：保留「这单曾用什么方式收款、
            // 何时完成」的审计痕迹。②-3 收摊对账会读 cancelled 单的 channel，
            // 别把这里当 bug 顺手清掉。
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

#[cfg(test)]
mod tests {
    use crate::test_support::{
        admin_token, json_request, place, read_json, seed_event_and_product, test_router_with,
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

    /// 给展会加一个只含 `ep` 的「任选 2 件 50」，返回 lot_id。
    ///
    /// 候选只有一种、任选 2 件 ⇒ 必须允许同款重复。
    async fn seed_pair_lot(pool: &sqlx::SqlitePool, event_id: i64, ep: i64) -> i64 {
        crate::test_support::seed_lot_repeat(pool, event_id, "任选2件50", 2, 5000, &[ep]).await
    }

    #[tokio::test]
    async fn an_order_records_which_units_went_into_a_lot() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let lot_id = seed_pair_lot(&pool, event_id, ep_a).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 3}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = read_json(res).await;
        let order_id = body["id"].as_i64().unwrap();

        assert_eq!(body["gross_amount"], 9000);
        assert_eq!(body["solved_amount"], 8000);
        assert_eq!(body["final_amount"], 8000, "下单时 final 还等于 solved");

        // 同一个商品拆成两行：2 件在套装里、1 件散着（spec 4.5）
        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["lot_name"], "任选2件50");
        assert_eq!(items[0]["quantity"], 2);
        assert_eq!(items[0]["allocated_amount"], 5000);
        assert!(items[1]["lot_name"].is_null());
        assert_eq!(items[1]["allocated_amount"], 3000);

        // order_lots 存的是快照
        let (snap_lot, snap_name, snap_price): (Option<i64>, String, i64) =
            sqlx::query_as("SELECT lot_id, name, price FROM order_lots WHERE order_id = ?")
                .bind(order_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(snap_lot, Some(lot_id));
        assert_eq!(snap_name, "任选2件50");
        assert_eq!(snap_price, 5000);
    }

    #[tokio::test]
    async fn the_goods_leg_is_unaffected_by_lots() {
        // spec 4.3：手工覆盖和 Lot 只动钱，货该怎么移动还怎么移动。
        // 拆行是 order_lines 的事，stock_movements 仍然是每个商品一条腿。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        seed_pair_lot(&pool, event_id, ep_a).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 3}]}),
            ))
            .await
            .unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        let legs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM stock_movements sm JOIN journals j ON j.id = sm.journal_id
             WHERE j.order_id = ?",
        )
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(legs, 1, "3 件同一个商品只有一条货的腿，不跟着订单行拆");
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_a)
                .await
                .unwrap(),
            7
        );
    }

    #[tokio::test]
    async fn changing_a_lot_after_the_order_does_not_change_the_order() {
        // 价格在下单那一刻快照进 order_lots / order_lines
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let lot_id = seed_pair_lot(&pool, event_id, ep_a).await;

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

        sqlx::query("UPDATE lots SET total_price = 100 WHERE id = ?")
            .bind(lot_id)
            .execute(&pool)
            .await
            .unwrap();

        let price: i64 = sqlx::query_scalar("SELECT price FROM order_lots WHERE order_id = ?")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let solved: i64 = sqlx::query_scalar("SELECT solved_amount FROM orders WHERE id = ?")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(price, 5000);
        assert_eq!(solved, 5000);
    }

    use crate::domain::ledger::{account_balance, Account};
    use crate::domain::money::Money;

    #[tokio::test]
    async fn a_manual_discount_falls_entirely_on_the_home_society() {
        // spec 4.4 里最要命的那个 case：**整单都是代卖货**，摊主还是让了价。
        // 规则不依赖订单里有没有本社团的行——那 5 块一样落到本社团头上。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 1}]),
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "现金", "final_amount": 1500}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // 代卖社团按**自己的定价**全额入账，一分不少
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(-2000),
            "「我帮你卖货，我自己让的价不该由你买单」"
        );
        // 本社团承担了这 5 块：债权减少 500（余额为正）
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1))
                .await
                .unwrap(),
            Money::from_cents(500)
        );
        // 实收净入 = 顾客实付
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into()))
                .await
                .unwrap(),
            Money::from_cents(1500)
        );
    }

    #[tokio::test]
    async fn marking_an_order_up_reverses_the_leg_instead_of_passing_a_negative_amount() {
        // ②-1 交接段点名的坑：照「金额 = solved − final」直接传，加价时它是负数，
        // post_journal 会用「资金移动的金额必须为正」把每一张加价订单的完成打成 400。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_a, "quantity": 1}]),
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "现金", "final_amount": 3500}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "加价不能 400");

        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into()))
                .await
                .unwrap(),
            Money::from_cents(3500)
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1))
                .await
                .unwrap(),
            Money::from_cents(-3500),
            "本社团那头也跟着多出 5 块"
        );
    }

    #[tokio::test]
    async fn marking_up_an_all_consignment_order_charges_the_home_society() {
        // ②-2 deferred #1。折让方向有测试，加价方向没有。
        // 加价时那条腿是反的（社团往来:本社团 → 实收），写反了照样平账，
        // 只有盯着本社团余额的符号才看得出。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 1}]),
        )
        .await;
        let token = admin_token();

        // 原价 2000，顾客给了 2500（凑整、打赏）
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "现金", "final_amount": 2500}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(-2000),
            "代卖社团只拿原价——多的 5 块不是他们的货挣的"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1))
                .await
                .unwrap(),
            Money::from_cents(-500),
            "多出来的 5 块归本社团。写反方向的话这里会是 +500"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into()))
                .await
                .unwrap(),
            Money::from_cents(2500)
        );
    }

    #[tokio::test]
    async fn paid_amounts_always_add_up_to_the_final_amount() {
        // spec 4.5 的第二条不变量。Task 9 的统计口径完全押在它上面。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_a, "quantity": 2}, {"product_id": ep_b, "quantity": 1}]),
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "微信", "final_amount": 7333}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let sum: i64 =
            sqlx::query_scalar("SELECT SUM(paid_amount) FROM order_lines WHERE order_id = ?")
                .bind(order_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(sum, 7333, "取整的余数不能凭空蒸发或多出来");

        // 货主那一侧完全不受影响（spec 4.3 第一条约束）
        let alloc: i64 =
            sqlx::query_scalar("SELECT SUM(allocated_amount) FROM order_lines WHERE order_id = ?")
                .bind(order_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(alloc, 8000);
    }

    #[tokio::test]
    async fn completing_without_a_final_amount_keeps_the_solved_price() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_a, "quantity": 1}]),
        )
        .await;

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
        assert_eq!(read_json(res).await["final_amount"], 3000);

        // 没有手工折让就不该有那条腿
        let legs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM money_movements mm JOIN journals j ON j.id = mm.journal_id
             WHERE j.order_id = ?",
        )
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(legs, 1, "只有按货主分组的那一条");
    }

    #[tokio::test]
    async fn a_negative_final_amount_is_refused() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_a, "quantity": 1}]),
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "微信", "final_amount": -1}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn cancelling_a_discounted_order_reverses_the_discount_leg_too() {
        // 冲正走的是 reverse_order_journals，它把 journal 的每条腿都反向——
        // 手工折让那条腿不需要任何特殊处理，但必须有测试钉住这一点。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_a, "quantity": 1}, {"product_id": ep_b, "quantity": 1}]),
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "现金", "final_amount": 4000}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

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

        for account in [
            Account::SocietyDue(1),
            Account::SocietyDue(2),
            Account::Received("现金".into()),
        ] {
            assert_eq!(
                account_balance(&pool, event_id, &account).await.unwrap(),
                Money::ZERO,
                "取消之后所有资金账户必须回到 0：{account}"
            );
        }
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_a)
                .await
                .unwrap(),
            10
        );
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_b)
                .await
                .unwrap(),
            5
        );
    }

    /// 给代卖社团（黄昏堂，id=2）的商品配一个「任选2件30」（原价 40），返回 lot_id。
    ///
    /// 刻意用**代卖**货：自家货上拆不拆都对得上，只有代卖货能暴露归属错位。
    async fn seed_consignment_lot(pool: &sqlx::SqlitePool, event_id: i64, ep_b: i64) -> i64 {
        crate::test_support::seed_lot_repeat(pool, event_id, "代卖任选2件30", 2, 3000, &[ep_b])
            .await
    }

    #[tokio::test]
    async fn the_order_response_carries_its_lot_instances() {
        // 摊主端的收款弹窗要按实例列勾选框，所以响应里必须有实例 id、名字、
        // 套装价和成分原价合计——最后一个是「拆掉它应收会回到多少」。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_consignment_lot(&pool, event_id, ep_b).await;
        let _order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 2}]),
        )
        .await;

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/orders?status=pending"),
                Some(&admin_token()),
                json!({}),
            ))
            .await
            .unwrap();
        let body = read_json(res).await;
        let lots = body[0]["lots"].as_array().unwrap();
        assert_eq!(lots.len(), 1);
        assert_eq!(lots[0]["name"], "代卖任选2件30");
        assert_eq!(lots[0]["price"], 3000);
        assert_eq!(lots[0]["original_amount"], 4000, "拆掉它应收回到 40");
        assert!(lots[0]["id"].as_i64().unwrap() > 0);
    }

    #[tokio::test]
    async fn unapplying_a_lot_puts_the_money_back_on_the_real_owner() {
        // 这是整条通道存在的理由。对照组写在断言里：只改实收不拆，
        // 代卖社团仍然只拿 30，多出的 10 会挂在本社团头上。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_consignment_lot(&pool, event_id, ep_b).await;
        let token = admin_token();
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 2}]),
        )
        .await;

        let lot_instance: i64 = sqlx::query_scalar("SELECT id FROM order_lots WHERE order_id = ?")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "现金", "unapply_lot_ids": [lot_instance]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        assert_eq!(body["solved_amount"], 4000, "应收回到原价");
        assert_eq!(body["final_amount"], 4000, "没另外改实收，就跟着新的应收走");
        assert_eq!(body["gross_amount"], 4000, "原价合计从来没变过");
        assert_eq!(body["lots"].as_array().unwrap().len(), 0, "实例被拆掉了");

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(-4000),
            "代卖社团拿全价——只改实收的话这里会是 -3000"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1))
                .await
                .unwrap(),
            Money::ZERO,
            "本社团完全不该被牵连——只改实收的话这里会是 -1000"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into()))
                .await
                .unwrap(),
            Money::from_cents(4000)
        );

        // 货那一侧一个字没动（spec 4.3 第一条约束）
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_b)
                .await
                .unwrap(),
            3
        );
    }

    #[tokio::test]
    async fn unapplying_one_instance_leaves_the_other_alone() {
        // 「任选2件30」买 4 件 = 两个实例。只拆一个。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_consignment_lot(&pool, event_id, ep_b).await;
        let token = admin_token();
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 4}]),
        )
        .await;

        let instances: Vec<i64> =
            sqlx::query_scalar("SELECT id FROM order_lots WHERE order_id = ? ORDER BY id")
                .bind(order_id)
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(instances.len(), 2, "8000 的货套两次 30，应收 6000");

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "现金", "unapply_lot_ids": [instances[0]]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        assert_eq!(
            body["solved_amount"], 7000,
            "一个实例回到 40，另一个还是 30"
        );
        assert_eq!(body["lots"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn unapplying_a_lot_that_belongs_to_another_order_is_refused() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_consignment_lot(&pool, event_id, ep_b).await;
        let token = admin_token();
        let first = place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 2}]),
        )
        .await;
        let second = place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 2}]),
        )
        .await;

        let other: i64 = sqlx::query_scalar("SELECT id FROM order_lots WHERE order_id = ?")
            .bind(first)
            .fetch_one(&pool)
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{second}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "现金", "unapply_lot_ids": [other]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        // 整体回滚：第一张单的实例还在，第二张单也没被完成
        let still: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM order_lots WHERE order_id = ?")
            .bind(first)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(still, 1);
        let status: String = sqlx::query_scalar("SELECT status FROM orders WHERE id = ?")
            .bind(second)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "pending");
    }

    #[tokio::test]
    async fn unapplying_and_then_discounting_compose() {
        // 拆完之后摊主还想让价：那一步才走 4.4 的「全额落本社团」。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_consignment_lot(&pool, event_id, ep_b).await;
        let token = admin_token();
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 2}]),
        )
        .await;

        let instance: i64 = sqlx::query_scalar("SELECT id FROM order_lots WHERE order_id = ?")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "现金",
                   "unapply_lot_ids": [instance], "final_amount": 3500}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2))
                .await
                .unwrap(),
            Money::from_cents(-4000),
            "货主仍然按自己的定价全额入账"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1))
                .await
                .unwrap(),
            Money::from_cents(500),
            "这 5 块是摊主自己让的，落本社团"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into()))
                .await
                .unwrap(),
            Money::from_cents(3500)
        );
    }

    #[tokio::test]
    async fn unapply_is_refused_on_a_cancellation() {
        // 静默忽略一个字段是会咬人的：取消路径上拆套装没有意义，明确拒绝。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_consignment_lot(&pool, event_id, ep_b).await;
        let token = admin_token();
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 2}]),
        )
        .await;
        let instance: i64 = sqlx::query_scalar("SELECT id FROM order_lots WHERE order_id = ?")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "cancelled", "unapply_lot_ids": [instance]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    /// ②-1 交接段列了 4 个「当前完全不查 events.status」的既有敞口，②-2 一个没补。
    /// 冻结语义靠守卫而不是「流程上到不了」，所以每个口子都要有自己的测试。
    #[tokio::test]
    async fn a_settled_event_refuses_order_status_change() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_a, "quantity": 1}]),
        )
        .await;
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
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "cancelled"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn completing_an_order_normalizes_the_channel() {
        // 账户名是 `实收-<渠道>`，"  微信  " 和 "微信" 必须是同一个账户。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_a, "quantity": 1}]),
        )
        .await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "  微信  "}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let stored: String = sqlx::query_scalar("SELECT channel FROM orders WHERE id = ?")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored, "微信");
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into()))
                .await
                .unwrap(),
            Money::from_cents(3000)
        );
    }

    #[tokio::test]
    async fn completing_an_order_rejects_an_overlong_channel() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let order_id = place(
            &router,
            event_id,
            json!([{"product_id": ep_a, "quantity": 1}]),
        )
        .await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed",
                       "channel": "一二三四五六七八九十一二三四五六七八九十超"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}
