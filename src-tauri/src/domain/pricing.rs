//! 把折扣求解器接到数据库。
//!
//! **下单（`api/order.rs`）和报价（`api/lot.rs` 的 `/quote`）共用这里。**
//! 这是「购物车显示的价」和「真正落账的价」不可能分叉的全部理由——它们跑的是
//! 同一份代码，而不是两份需要人去保持同步的实现。

use crate::domain::allocation::allocate_lot;
use crate::domain::money::Money;
use crate::domain::solver::{self, CartLine, LotDef, SolvedLot};
use crate::error::{ApiError, ApiResult};
use serde::Deserialize;
use sqlx::SqliteConnection;
use std::collections::HashMap;

/// 一次请求里最多多少个不同商品。`/quote` 是公开未鉴权端点，
/// 没有这条上限，一个带上万个 id 的请求光是逐个查库就够呛。真实购物车十几行。
const MAX_CART_ITEMS: usize = 200;

fn overflow() -> ApiError {
    ApiError::BadRequest("金额溢出".into())
}

/// 请求里的一项。下单与报价的请求体同形，所以只有这一个结构。
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct CartItemRequest {
    pub product_id: i64,
    pub quantity: i64,
}

/// 合并重复项。
///
/// 同一商品出现多次**必须先合并**：否则两行各自过库存检查、合起来超卖
/// （②-1 有一个专门的回归测试盯着这件事）。
pub fn merge_items(items: &[CartItemRequest]) -> ApiResult<Vec<(i64, i64)>> {
    if items.is_empty() {
        return Err(ApiError::BadRequest("订单至少需要一件商品".into()));
    }
    if items.len() > MAX_CART_ITEMS {
        return Err(ApiError::BadRequest("购物车商品过多，请分单结算".into()));
    }
    let mut merged: Vec<(i64, i64)> = Vec::new();
    for item in items {
        if item.quantity <= 0 {
            return Err(ApiError::BadRequest("数量必须为正".into()));
        }
        match merged.iter_mut().find(|(pid, _)| *pid == item.product_id) {
            // 公开未鉴权端点，累加必须 checked：release 回绕、debug 直接 panic。
            Some((_, qty)) => {
                *qty = qty
                    .checked_add(item.quantity)
                    .ok_or_else(|| ApiError::BadRequest("商品数量累加溢出".into()))?;
            }
            None => merged.push((item.product_id, item.quantity)),
        }
    }
    Ok(merged)
}

/// 下单与报价都要的商品快照。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CartProduct {
    pub id: i64,
    pub unit_price: i64,
    /// `/quote` 只读 id/单价，商品名由 Task 6 的下单响应消费
    /// （`api/order.rs::create_order` 的库存不足提示）。
    pub name: String,
    /// 同上，下单响应要带出商品图片（Task 6 的 `api/order.rs::create_order`）。
    pub image_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedLine {
    pub product: CartProduct,
    pub qty: i64,
}

/// 查 `event_products`，把 `(product_id, qty)` 变成带商品快照的行，顺序与 `merged` 一致。
///
/// 一条 `IN (...)` 查完再按 `merged` 重排：这是公开未鉴权端点，逐个 id 发查询意味着
/// 一个请求最多打 `MAX_CART_ITEMS` 次库。`IN` 不保证返回顺序，而
/// `PricedCart::lines` 的顺序是契约，所以重排不是可选项。
pub async fn resolve_cart(
    conn: &mut SqliteConnection,
    event_id: i64,
    merged: &[(i64, i64)],
) -> ApiResult<Vec<ResolvedLine>> {
    if merged.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = vec!["?"; merged.len()].join(",");
    let sql = format!(
        "SELECT ep.id, ep.unit_price, ep.name, mp.image_url
         FROM event_products ep
         JOIN master_products mp ON mp.id = ep.master_product_id
         WHERE ep.event_id = ? AND ep.id IN ({placeholders})"
    );
    let mut q = sqlx::query_as::<_, CartProduct>(sqlx::AssertSqlSafe(sql)).bind(event_id);
    for (product_id, _) in merged {
        q = q.bind(*product_id);
    }
    let products = q.fetch_all(&mut *conn).await?;

    let mut by_id: HashMap<i64, CartProduct> = products.into_iter().map(|p| (p.id, p)).collect();
    let mut out = Vec::with_capacity(merged.len());
    for (product_id, qty) in merged {
        let product = by_id
            .remove(product_id)
            .ok_or_else(|| ApiError::NotFound("商品不存在或不属于本场展会".into()))?;
        out.push(ResolvedLine { product, qty: *qty });
    }
    Ok(out)
}

/// `ResolvedLine` → 求解器的输入。
pub fn cart_lines(resolved: &[ResolvedLine]) -> Vec<CartLine> {
    resolved
        .iter()
        .map(|r| CartLine {
            event_product_id: r.product.id,
            qty: r.qty,
            unit_price: Money::from_cents(r.product.unit_price),
        })
        .collect()
}

/// 读一场展会的全部 Lot 定义。
///
/// 一条 JOIN 查完再在 Rust 里分组，靠 `ORDER BY l.id` 保证同一个 Lot 的候选是连续的。
/// 没有任何候选商品的 Lot 不会出现在结果里——它永远套用不上。
pub async fn load_lots(conn: &mut SqliteConnection, event_id: i64) -> ApiResult<Vec<LotDef>> {
    let rows: Vec<(i64, String, i64, i64, i64)> = sqlx::query_as(
        "SELECT l.id, l.name, l.pick_count, l.total_price, lc.event_product_id
         FROM lots l
         JOIN lot_candidates lc ON lc.lot_id = l.id
         WHERE l.event_id = ?
         ORDER BY l.id, lc.event_product_id",
    )
    .bind(event_id)
    .fetch_all(&mut *conn)
    .await?;

    let mut out: Vec<LotDef> = Vec::new();
    for (id, name, pick_count, total_price, candidate) in rows {
        match out.last_mut() {
            Some(last) if last.id == id => last.candidates.push(candidate),
            _ => out.push(LotDef {
                id,
                name,
                pick_count,
                total_price: Money::from_cents(total_price),
                candidates: vec![candidate],
                // Task 3 会从 `lots.allow_repeat` 读；这里先保住 Task 2 的编译。
                allow_repeat: false,
            }),
        }
    }
    Ok(out)
}

/// 求解结果落到「每一行」上。
#[derive(Debug, Clone)]
pub struct PricedLine {
    pub event_product_id: i64,
    pub qty: i64,
    pub unit_price: Money,
    /// 进了 `PricedCart::lots` 里的第几个 Lot 实例。None = 没进 Lot。
    pub lot_index: Option<usize>,
    pub allocated: Money,
}

#[derive(Debug, Clone)]
pub struct PricedCart {
    pub lots: Vec<SolvedLot>,
    /// Lot 内的行在前（按 Lot 实例顺序、实例内按商品 id 升序），散行在后（按商品 id 升序）。
    /// **Task 6 按这个顺序写 `order_lines`，所以它是契约不是巧合。**
    pub lines: Vec<PricedLine>,
    pub gross: Money,
    pub solved: Money,
}

/// 求解 + 分摊：算出每一行进了哪个 Lot、以及这一行的货主应得多少。
///
/// **订单行按 Lot 归属拆分**（spec 4.5）：求解器可能把「某商品 3 件」里的 2 件放进套装、
/// 1 件留在外面，而 `order_lines.order_lot_id` 是每行一个，一行里表达不了两种归属。
pub fn price_cart(cart: &[CartLine], lots: &[LotDef]) -> ApiResult<PricedCart> {
    let mut gross = Money::ZERO;
    for l in cart {
        let row = l.unit_price.checked_mul_qty(l.qty).ok_or_else(overflow)?;
        gross = gross.checked_add_money(row).ok_or_else(overflow)?;
    }

    let price_of: HashMap<i64, Money> = cart
        .iter()
        .map(|l| (l.event_product_id, l.unit_price))
        .collect();
    let solution = solver::solve(cart, lots)?;

    let mut lines: Vec<PricedLine> = Vec::new();
    for (k, lot) in solution.lots.iter().enumerate() {
        let mut weights: Vec<i64> = Vec::with_capacity(lot.members.len());
        for (id, q) in &lot.members {
            weights.push(
                price_of[id]
                    .checked_mul_qty(*q)
                    .ok_or_else(overflow)?
                    .cents(),
            );
        }
        let fallback: Vec<i64> = lot.members.iter().map(|(_, q)| *q).collect();
        let allocated = allocate_lot(lot.price, &weights, &fallback)?;
        for ((id, q), a) in lot.members.iter().zip(allocated) {
            lines.push(PricedLine {
                event_product_id: *id,
                qty: *q,
                unit_price: price_of[id],
                lot_index: Some(k),
                allocated: a,
            });
        }
    }
    for (id, q) in &solution.leftovers {
        let unit_price = price_of[id];
        lines.push(PricedLine {
            event_product_id: *id,
            qty: *q,
            unit_price,
            lot_index: None,
            allocated: unit_price.checked_mul_qty(*q).ok_or_else(overflow)?,
        });
    }

    let solved: Money = lines.iter().map(|l| l.allocated).sum();
    debug_assert_eq!(
        solved, solution.solved_total,
        "Σ allocated 必须等于求解器算出的总价（spec 4.5 不变量）"
    );

    Ok(PricedCart {
        lots: solution.lots,
        lines,
        gross,
        solved,
    })
}

/// 只有「进行中」的展会能下单——也只有它能报价。
///
/// 报价端也挡一道，是为了不让顾客拿到一个**下不了的报价**：筹备中或已结算的展会
/// 报价能出、下单会 409，那顾客点「去结算」时才发现白填了。
pub async fn ensure_event_selling(conn: &mut SqliteConnection, event_id: i64) -> ApiResult<()> {
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM events WHERE id = ?")
        .bind(event_id)
        .fetch_optional(&mut *conn)
        .await?;
    match status.as_deref() {
        Some("进行中") => Ok(()),
        Some(other) => Err(ApiError::Conflict(format!(
            "展会当前状态为「{other}」，仅「进行中」的展会可以下单"
        ))),
        None => Err(ApiError::NotFound("展会不存在".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{seed_event_and_product, seed_lot, test_pool};

    fn req(product_id: i64, quantity: i64) -> CartItemRequest {
        CartItemRequest {
            product_id,
            quantity,
        }
    }

    fn line(id: i64, qty: i64, cents: i64) -> CartLine {
        CartLine {
            event_product_id: id,
            qty,
            unit_price: Money::from_cents(cents),
        }
    }

    #[test]
    fn duplicate_items_are_merged_before_anything_else_sees_them() {
        // 不合并的话两行各自过库存检查、合起来超卖
        let merged = merge_items(&[req(1, 2), req(2, 1), req(1, 3)]).unwrap();
        assert_eq!(merged, vec![(1, 5), (2, 1)]);
    }

    #[test]
    fn a_non_positive_quantity_is_refused() {
        assert!(merge_items(&[req(1, 0)]).is_err());
        assert!(merge_items(&[req(1, -1)]).is_err());
        assert!(merge_items(&[]).is_err());
    }

    #[test]
    fn a_cart_with_too_many_distinct_items_is_refused() {
        // /quote 是公开未鉴权端点，拼 200 个以上的占位符本身就是负担
        let items: Vec<CartItemRequest> = (1..=201).map(|i| req(i, 1)).collect();
        assert!(merge_items(&items).is_err());
    }

    #[tokio::test]
    async fn resolving_refuses_a_product_from_another_event() {
        let pool = test_pool().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let mut conn = pool.acquire().await.unwrap();

        assert!(resolve_cart(&mut conn, event_id, &[(ep_a, 1)])
            .await
            .is_ok());
        let err = resolve_cart(&mut conn, 999, &[(ep_a, 1)])
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn lots_come_back_with_their_candidates_grouped() {
        let pool = test_pool().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_lot(&pool, event_id, "两本合购", 2, 4000, &[ep_a, ep_b]).await;
        seed_lot(&pool, event_id, "只含A", 1, 2500, &[ep_a]).await;
        let mut conn = pool.acquire().await.unwrap();

        let lots = load_lots(&mut conn, event_id).await.unwrap();
        assert_eq!(lots.len(), 2);
        assert_eq!(lots[0].candidates, vec![ep_a, ep_b]);
        assert_eq!(lots[0].total_price, Money::from_cents(4000));
        assert_eq!(lots[1].candidates, vec![ep_a]);
    }

    #[test]
    fn a_lot_price_is_allocated_across_its_members() {
        // Lot 价 40，成分 30 + 20（原价 50）
        let cart = [line(1, 1, 3000), line(2, 1, 2000)];
        let lots = [LotDef {
            id: 7,
            name: "两本合购".into(),
            pick_count: 2,
            total_price: Money::from_cents(4000),
            candidates: vec![1, 2],
            allow_repeat: false,
        }];
        let priced = price_cart(&cart, &lots).unwrap();
        assert_eq!(priced.gross, Money::from_cents(5000));
        assert_eq!(priced.solved, Money::from_cents(4000));
        assert_eq!(priced.lines.len(), 2);
        assert_eq!(priced.lines[0].allocated, Money::from_cents(2400));
        assert_eq!(priced.lines[1].allocated, Money::from_cents(1600));
        assert!(priced.lines.iter().all(|l| l.lot_index == Some(0)));
    }

    #[test]
    fn a_product_is_split_into_two_lines_when_only_some_units_go_into_a_lot() {
        // spec 4.5：order_lines.order_lot_id 是每行一个，「3 件里 2 件进套装」
        // 在一行里表达不了，必须拆成两行——同一个商品在一张订单里出现两次。
        let cart = [line(1, 3, 4000)];
        let lots = [LotDef {
            id: 7,
            name: "任选2件60".into(),
            pick_count: 2,
            total_price: Money::from_cents(6000),
            candidates: vec![1],
            allow_repeat: true,
        }];
        let priced = price_cart(&cart, &lots).unwrap();
        assert_eq!(priced.gross, Money::from_cents(12000));
        assert_eq!(priced.solved, Money::from_cents(10000));
        assert_eq!(priced.lines.len(), 2, "同一个商品拆成了两行");

        let in_lot: Vec<&PricedLine> = priced
            .lines
            .iter()
            .filter(|l| l.lot_index.is_some())
            .collect();
        let loose: Vec<&PricedLine> = priced
            .lines
            .iter()
            .filter(|l| l.lot_index.is_none())
            .collect();
        assert_eq!(in_lot.len(), 1);
        assert_eq!(in_lot[0].qty, 2);
        assert_eq!(in_lot[0].allocated, Money::from_cents(6000));
        assert_eq!(loose.len(), 1);
        assert_eq!(loose[0].qty, 1);
        assert_eq!(loose[0].allocated, Money::from_cents(4000));
    }

    #[test]
    fn the_sum_of_allocated_is_the_solved_total() {
        // spec 4.5 的不变量之一。Task 7 的收款分腿全靠它。
        let cart = [
            line(1, 1, 3000),
            line(2, 1, 3500),
            line(3, 1, 4000),
            line(4, 2, 1700),
        ];
        let lots = [LotDef {
            id: 7,
            name: "任选3件100".into(),
            pick_count: 3,
            total_price: Money::from_cents(10000),
            candidates: vec![1, 2, 3, 4],
            allow_repeat: false,
        }];
        let priced = price_cart(&cart, &lots).unwrap();
        let sum: Money = priced.lines.iter().map(|l| l.allocated).sum();
        assert_eq!(sum, priced.solved);
        // 每一件货都有去向：各行 qty 之和 == 购物车件数之和
        let qty: i64 = priced.lines.iter().map(|l| l.qty).sum();
        assert_eq!(qty, 5);
    }
}
