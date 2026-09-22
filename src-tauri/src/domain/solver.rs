//! 折扣求解器（spec 4.2）。
//!
//! **纯函数**：不碰数据库、不碰网络、不碰 Tauri。spec 把它列为整个子项目里最值得
//! 写单测的一块，因为输入输出都是普通数据结构，而「替顾客挑哪几件进 Lot」正是最
//! 容易写错的地方——「任选 3 本 100」而车里有 30/35/40/45 四本时，放最贵的三本省 20，
//! 放最便宜的三本只省 5。
//!
//! 本质是加权集合覆盖。两条让它在摊位规模下够快的设计：
//!
//! 1. **状态规范化**：状态是「参与 Lot 的商品各剩几件」这个计数向量，每一步**只处理
//!    第一个还有剩余的商品**。不这么做的话「先卖 A 再卖 B」和「先卖 B 再卖 A」是两条
//!    不同的搜索路径，搜索树按排列数爆炸。
//! 2. **记忆化**：同一个剩余状态只算一次。
//!
//! **三条硬上限不是优化，是安全要求。** 入口是 `POST /events/:id/quote` 和下单，
//! 两个都是公开未鉴权端点；而 Tauri 进程和 UI 是同一个进程，CPU 打满就是界面卡死。

use crate::domain::money::Money;
use crate::error::{ApiError, ApiResult};
use std::collections::HashMap;

/// 参与求解的商品总件数上限。**它同时限住递归深度**——每一步至少消耗一件。
/// 没有这条，「一个商品买 20 万件」的状态空间恰好是 200001，卡在
/// `MAX_STATE_SPACE` 之内，然后把栈撑爆。
const MAX_UNITS: i64 = 300;

/// 状态空间上限：Π(qty_i + 1)，只数参与 Lot 的商品。
/// 15 件不同商品各 1 件是 32768，真实摊位到不了这个数量级。
const MAX_STATE_SPACE: i64 = 200_000;

/// 递归与枚举的总步数预算。状态空间上限挡不住「Lot 特别多、每个状态要枚举
/// 很多种凑法」这一维，所以还要一条总预算兜底。
const MAX_STEPS: u64 = 2_000_000;

fn too_large() -> ApiError {
    ApiError::BadRequest("购物车商品过多，无法自动计算优惠，请分单结算".into())
}

fn overflow() -> ApiError {
    ApiError::BadRequest("金额溢出".into())
}

/// 与 `api::lot::validate_payload` 的 `total_price` 上限共用同一个错误。
///
/// `best()` 里的加法本身用 `checked_add_money` 守住，但能走到溢出的唯一现实路径
/// 就是一个荒谬大的套装价（`lots.total_price` 只有 `CHECK (total_price >= 0)`）。
/// 报「套装价格超出合理范围」比「金额溢出」更贴近用户看到的东西。
fn price_out_of_range() -> ApiError {
    ApiError::BadRequest("套装价格超出合理范围".into())
}

/// 购物车的一行。同一商品在进来之前**必须已经合并**（`pricing::merge_items` 负责）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CartLine {
    pub event_product_id: i64,
    pub qty: i64,
    pub unit_price: Money,
}

/// 一个 Lot 的定义。`candidates` 是候选商品的 `event_product_id`。
///
/// 「固定成分」是「任选 N」的特例（N = 候选集大小），模型里只有一个概念（spec 4.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LotDef {
    pub id: i64,
    pub name: String,
    pub pick_count: i64,
    pub total_price: Money,
    pub candidates: Vec<i64>,
}

/// 求解结果里的一个 Lot **实例**。同一个 Lot 可以出现多次
/// （「任选 3 本 100」买 6 本 = 两个实例）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolvedLot {
    pub lot_id: i64,
    pub name: String,
    pub price: Money,
    /// `(event_product_id, qty)`，按 id 升序。
    pub members: Vec<(i64, i64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Solution {
    pub lots: Vec<SolvedLot>,
    /// 没进任何 Lot 的件数，`(event_product_id, qty)`，按 id 升序。
    pub leftovers: Vec<(i64, i64)>,
    pub solved_total: Money,
}

/// 搜索时在某个状态下做的那一步选择。记下来是为了事后重建解，
/// 而不是在搜索过程中到处拷贝部分解。
#[derive(Debug, Clone)]
enum Choice {
    Done,
    /// 第 `pos` 个商品单卖一件。
    Single(usize),
    /// 套用第 `lot` 个 Lot，成分是 `picks`（`(pos, 件数)`，pos 升序）。
    Lot {
        lot: usize,
        picks: Vec<(usize, i64)>,
    },
}

/// 从 `positions`（状态向量的下标，升序）里按余量取 `need` 件，枚举所有取法。
///
/// 「按 cursor 单调推进」保证每个多重集**只被枚举一次**——不这么做的话
/// (甲1,乙2) 和 (乙2,甲1) 会各算一遍。
#[allow(clippy::too_many_arguments)]
fn enumerate_picks(
    positions: &[usize],
    state: &[i64],
    need: i64,
    cursor: usize,
    current: &mut Vec<(usize, i64)>,
    out: &mut Vec<Vec<(usize, i64)>>,
    steps: &mut u64,
) -> ApiResult<()> {
    *steps += 1;
    if *steps > MAX_STEPS {
        return Err(too_large());
    }
    if need == 0 {
        out.push(current.clone());
        return Ok(());
    }
    if cursor >= positions.len() {
        return Ok(());
    }
    let pos = positions[cursor];
    let max = state[pos].min(need);
    for take in 0..=max {
        if take > 0 {
            current.push((pos, take));
        }
        enumerate_picks(
            positions,
            state,
            need - take,
            cursor + 1,
            current,
            out,
            steps,
        )?;
        if take > 0 {
            current.pop();
        }
    }
    Ok(())
}

struct Search<'a> {
    prices: Vec<Money>,
    lots: Vec<&'a LotDef>,
    /// 每个 Lot 的候选商品在状态向量里的下标，升序去重。
    lot_positions: Vec<Vec<usize>>,
    memo: HashMap<Vec<i64>, (Money, Choice)>,
    steps: u64,
}

impl Search<'_> {
    fn best(&mut self, state: &[i64]) -> ApiResult<(Money, Choice)> {
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return Err(too_large());
        }
        if let Some(hit) = self.memo.get(state) {
            return Ok(hit.clone());
        }

        // 只处理「第一个还有剩余的商品」——排列重复就是在这里被砍掉的。
        let Some(first) = state.iter().position(|q| *q > 0) else {
            return Ok((Money::ZERO, Choice::Done));
        };

        // 选项 A：这一件单卖。
        let mut next = state.to_vec();
        next[first] -= 1;
        let (rest, _) = self.best(&next)?;
        let mut best_cost = self.prices[first]
            .checked_add_money(rest)
            .ok_or_else(price_out_of_range)?;
        let mut best_choice = Choice::Single(first);

        // 选项 B：把这一件放进某个含它的 Lot，再从候选里凑够 pick_count − 1 件。
        // 强制先拿一件 `first`，保证枚举出来的成分必然含它——否则「不含 first 的组合」
        // 会在别的分支里被重复枚举一遍。
        for li in 0..self.lots.len() {
            if !self.lot_positions[li].contains(&first) {
                continue;
            }
            let need = self.lots[li].pick_count - 1;
            let mut base = state.to_vec();
            base[first] -= 1;

            let mut combos: Vec<Vec<(usize, i64)>> = Vec::new();
            let mut current: Vec<(usize, i64)> = Vec::new();
            enumerate_picks(
                &self.lot_positions[li],
                &base,
                need,
                0,
                &mut current,
                &mut combos,
                &mut self.steps,
            )?;

            for combo in combos {
                let mut after = base.clone();
                for (pos, n) in &combo {
                    after[*pos] -= *n;
                }
                let (rest, _) = self.best(&after)?;
                let cost = self.lots[li]
                    .total_price
                    .checked_add_money(rest)
                    .ok_or_else(price_out_of_range)?;
                // 严格小于：并列时保留先找到的那个。遍历顺序固定 ⇒ 同输入同输出。
                if cost < best_cost {
                    let mut picks = combo.clone();
                    match picks.iter_mut().find(|(p, _)| *p == first) {
                        Some((_, n)) => *n += 1,
                        None => picks.push((first, 1)),
                    }
                    picks.sort_by_key(|(p, _)| *p);
                    best_cost = cost;
                    best_choice = Choice::Lot { lot: li, picks };
                }
            }
        }

        self.memo
            .insert(state.to_vec(), (best_cost, best_choice.clone()));
        Ok((best_cost, best_choice))
    }
}

/// 求「哪几件进哪个 Lot」的最省方案。
///
/// 购物车里的每一件都必然有去向：要么在某个 Lot 实例里，要么在 `leftovers` 里。
pub fn solve(cart: &[CartLine], lots: &[LotDef]) -> ApiResult<Solution> {
    let in_cart: HashMap<i64, &CartLine> = cart.iter().map(|l| (l.event_product_id, l)).collect();

    // 1. 只保留「候选集里的件数够凑一次」的 Lot。凑不出一次的永远凑不出
    //    （件数只减不增），提前扔掉能让每个状态少枚举一轮。
    let usable: Vec<&LotDef> = lots
        .iter()
        .filter(|lot| {
            let avail: i64 = lot
                .candidates
                .iter()
                .filter_map(|c| in_cart.get(c).map(|l| l.qty))
                .sum();
            lot.pick_count > 0 && avail >= lot.pick_count
        })
        .collect();

    // 2. 只有出现在**可用** Lot 候选集里的商品才进搜索空间。
    let mut participating: Vec<i64> = Vec::new();
    for lot in &usable {
        for c in &lot.candidates {
            if in_cart.contains_key(c) && !participating.contains(c) {
                participating.push(*c);
            }
        }
    }
    participating.sort_unstable();

    // 固定部分：不参与任何 Lot 的商品，按原价。
    let mut fixed = Money::ZERO;
    let mut leftover_map: HashMap<i64, i64> = HashMap::new();
    for l in cart {
        if participating.binary_search(&l.event_product_id).is_err() {
            fixed = fixed
                .checked_add_money(l.unit_price.checked_mul_qty(l.qty).ok_or_else(overflow)?)
                .ok_or_else(overflow)?;
            leftover_map.insert(l.event_product_id, l.qty);
        }
    }

    if participating.is_empty() {
        let mut leftovers: Vec<(i64, i64)> = leftover_map.into_iter().collect();
        leftovers.sort_unstable();
        return Ok(Solution {
            lots: Vec::new(),
            leftovers,
            solved_total: fixed,
        });
    }

    let counts: Vec<i64> = participating.iter().map(|id| in_cart[id].qty).collect();
    let units: i64 = counts.iter().sum();
    if units > MAX_UNITS {
        return Err(too_large());
    }
    let mut space: i64 = 1;
    for c in &counts {
        space = space.saturating_mul(c + 1);
        if space > MAX_STATE_SPACE {
            return Err(too_large());
        }
    }

    let prices: Vec<Money> = participating
        .iter()
        .map(|id| in_cart[id].unit_price)
        .collect();
    let lot_positions: Vec<Vec<usize>> = usable
        .iter()
        .map(|lot| {
            let mut v: Vec<usize> = lot
                .candidates
                .iter()
                .filter_map(|c| participating.binary_search(c).ok())
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        })
        .collect();

    let mut search = Search {
        prices,
        lots: usable,
        lot_positions,
        memo: HashMap::new(),
        steps: 0,
    };
    search.best(&counts)?;

    // 重建解：从起始状态沿着记下来的 Choice 一路走到空。
    let mut state = counts;
    let mut solved_lots: Vec<SolvedLot> = Vec::new();
    // 只有全零状态查不到（best 对它提前返回，不写 memo），那时循环自然结束。
    while let Some((_, c)) = search.memo.get(&state) {
        let choice = c.clone();
        match choice {
            Choice::Done => break,
            Choice::Single(pos) => {
                *leftover_map.entry(participating[pos]).or_insert(0) += 1;
                state[pos] -= 1;
            }
            Choice::Lot { lot, picks } => {
                let def = search.lots[lot];
                solved_lots.push(SolvedLot {
                    lot_id: def.id,
                    name: def.name.clone(),
                    price: def.total_price,
                    members: picks.iter().map(|(p, n)| (participating[*p], *n)).collect(),
                });
                for (p, n) in &picks {
                    state[*p] -= *n;
                }
            }
        }
    }

    let mut leftovers: Vec<(i64, i64)> = leftover_map.into_iter().collect();
    leftovers.sort_unstable();

    let mut solved_total = fixed;
    for l in &solved_lots {
        solved_total = solved_total
            .checked_add_money(l.price)
            .ok_or_else(overflow)?;
    }
    for (id, qty) in &leftovers {
        if let Some(line) = in_cart.get(id) {
            // fixed 里已经算过的不再算第二遍
            if participating.binary_search(id).is_ok() {
                solved_total = solved_total
                    .checked_add_money(line.unit_price.checked_mul_qty(*qty).ok_or_else(overflow)?)
                    .ok_or_else(overflow)?;
            }
        }
    }

    Ok(Solution {
        lots: solved_lots,
        leftovers,
        solved_total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一条购物车行。单价单位是分。
    fn line(id: i64, qty: i64, cents: i64) -> CartLine {
        CartLine {
            event_product_id: id,
            qty,
            unit_price: Money::from_cents(cents),
        }
    }

    /// 造一个 Lot。`candidates` 是 event_product_id 列表。
    fn lot(id: i64, pick: i64, cents: i64, candidates: &[i64]) -> LotDef {
        LotDef {
            id,
            name: format!("Lot{id}"),
            pick_count: pick,
            total_price: Money::from_cents(cents),
            candidates: candidates.to_vec(),
        }
    }

    #[test]
    fn with_no_lots_everything_stays_at_its_own_price() {
        let cart = [line(1, 2, 3000), line(2, 1, 2000)];
        let sol = solve(&cart, &[]).unwrap();
        assert_eq!(sol.solved_total, Money::from_cents(8000));
        assert!(sol.lots.is_empty());
        assert_eq!(sol.leftovers, vec![(1, 2), (2, 1)]);
    }

    #[test]
    fn a_fixed_set_lot_replaces_the_sum_of_its_members() {
        // 「甲 + 乙 合购 40」：原价 50，套装价 40
        let cart = [line(1, 1, 3000), line(2, 1, 2000)];
        let lots = [lot(7, 2, 4000, &[1, 2])];
        let sol = solve(&cart, &lots).unwrap();
        assert_eq!(sol.solved_total, Money::from_cents(4000));
        assert_eq!(sol.lots.len(), 1);
        assert_eq!(sol.lots[0].lot_id, 7);
        assert_eq!(sol.lots[0].members, vec![(1, 1), (2, 1)]);
        assert!(sol.leftovers.is_empty());
    }

    #[test]
    fn pick_n_puts_the_most_expensive_units_into_the_lot() {
        // spec 4.2 点名的难点：30/35/40/45 四本，「任选 3 本 100」。
        // 放最贵的三本省 20，放最便宜的三本只省 5——挑错了「最优折扣」就名不副实。
        let cart = [
            line(1, 1, 3000),
            line(2, 1, 3500),
            line(3, 1, 4000),
            line(4, 1, 4500),
        ];
        let lots = [lot(7, 3, 10000, &[1, 2, 3, 4])];
        let sol = solve(&cart, &lots).unwrap();
        assert_eq!(
            sol.solved_total,
            Money::from_cents(13000),
            "100 + 剩下最便宜的 30"
        );
        assert_eq!(sol.lots.len(), 1);
        assert_eq!(sol.lots[0].members, vec![(2, 1), (3, 1), (4, 1)]);
        assert_eq!(sol.leftovers, vec![(1, 1)]);
    }

    #[test]
    fn the_same_lot_applies_twice_when_the_cart_allows() {
        // 「任选 3 本 100」买 6 本 = 200，不是 100
        let cart = [line(1, 6, 4000)];
        let lots = [lot(7, 3, 10000, &[1])];
        let sol = solve(&cart, &lots).unwrap();
        assert_eq!(sol.solved_total, Money::from_cents(20000));
        assert_eq!(sol.lots.len(), 2);
        assert!(sol.leftovers.is_empty());
    }

    #[test]
    fn a_lot_more_expensive_than_its_members_is_never_applied() {
        let cart = [line(1, 1, 3000), line(2, 1, 2000)];
        let lots = [lot(7, 2, 6000, &[1, 2])];
        let sol = solve(&cart, &lots).unwrap();
        assert_eq!(sol.solved_total, Money::from_cents(5000));
        assert!(sol.lots.is_empty());
    }

    #[test]
    fn two_lots_competing_for_the_same_item_pick_the_cheaper_split() {
        // 甲50 乙30 丙30。X{甲,乙}任选2=60，Y{乙,丙}任选2=35。
        // 选 X：60 + 丙30 = 90；选 Y：35 + 甲50 = 85。必须选 Y。
        let cart = [line(1, 1, 5000), line(2, 1, 3000), line(3, 1, 3000)];
        let lots = [lot(7, 2, 6000, &[1, 2]), lot(8, 2, 3500, &[2, 3])];
        let sol = solve(&cart, &lots).unwrap();
        assert_eq!(sol.solved_total, Money::from_cents(8500));
        assert_eq!(sol.lots.len(), 1);
        assert_eq!(sol.lots[0].lot_id, 8);
    }

    #[test]
    fn a_lot_that_cannot_be_filled_is_ignored() {
        // 「任选 3 本」但车里只有 2 本：凑不出一次，永远凑不出（件数只减不增）
        let cart = [line(1, 2, 4000)];
        let lots = [lot(7, 3, 10000, &[1])];
        let sol = solve(&cart, &lots).unwrap();
        assert_eq!(sol.solved_total, Money::from_cents(8000));
        assert!(sol.lots.is_empty());
    }

    #[test]
    fn solving_is_deterministic() {
        // spec 4.5：任何两次计算必须给出同一结果。并列解不能随 HashMap 的遍历顺序摇摆。
        let cart = [line(1, 2, 3000), line(2, 2, 3000), line(3, 1, 3000)];
        let lots = [lot(7, 2, 5000, &[1, 2]), lot(8, 2, 5000, &[2, 3])];
        let first = solve(&cart, &lots).unwrap();
        for _ in 0..20 {
            assert_eq!(solve(&cart, &lots).unwrap(), first);
        }
    }

    #[test]
    fn an_oversized_cart_is_refused_rather_than_hanging() {
        // /quote 是公开未鉴权端点，Tauri 进程和 UI 是同一个进程——CPU 打满就是界面卡死。
        let cart = [line(1, 301, 100)];
        let lots = [lot(7, 3, 200, &[1])];
        let err = solve(&cart, &lots).unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn items_outside_every_lot_do_not_enter_the_search_space() {
        // 不参与任何 Lot 的商品按原价固定，不该把状态空间撑爆
        let cart = [line(1, 2, 4000), line(99, 1000, 100)];
        let lots = [lot(7, 2, 6000, &[1])];
        let sol = solve(&cart, &lots).unwrap();
        assert_eq!(sol.solved_total, Money::from_cents(6000 + 100_000));
        assert_eq!(sol.lots.len(), 1);
        assert_eq!(sol.leftovers, vec![(99, 1000)]);
    }
}
