//! 分摊与取整（spec 4.5）。纯函数。
//!
//! 两处用它：
//!
//! 1. **Lot 内部**——Lot 价在成分间按 `unit_price × qty` 分摊，得各行 `allocated_amount`
//!    （这一行的货主应得多少）。
//! 2. **手工折让/加价**——`solved − final` 在订单全部行间按 `allocated` 分摊，得各行
//!    `paid_amount`（顾客为这一行实付多少）。
//!
//! 这两个数在混货主订单里会分道扬镳：代卖货 20、摊主「算你 15」，货主该得 20、
//! 顾客只付了 15。只存一个数，退货要么多退给顾客要么少给货主（spec 4.5）。
//!
//! **取整规则是本模块唯一有技术含量的地方**，而且 spec 原文是错的——见
//! `apportion` 的文档注释。

use crate::domain::money::Money;
use crate::error::{ApiError, ApiResult};

/// 把 `total`（非负）按 `weights`（非负）比例分摊成同长的一组数，**和精确等于 `total`**。
///
/// `caps[i]` 是第 i 份的上限，`None` 表示不限。
///
/// # 取整（spec 4.5 第 3 条，2026-09-23 修正）
///
/// 先各自向下取整，余数按「权重降序、下标升序」**逐分派发**，跳过已经顶到 cap 的那一份，
/// 派完一圈再从头来。
///
/// spec 原文写的是「余数全部加到权重最大的那一行」，**那条规则会算出负数**：
/// 3 行各 1 元、摊主把 3 元抹成 0.01 元时，`D = 299`，各行 `floor(299×100/300) = 99`，
/// 余数 2 全给最大行 → 那行分到 101，而它只有 100 可让，`paid = −1`，
/// 撞 `CHECK (paid_amount >= 0)` 变成 500。根因是最大行的余量恒 ≥ 1（因为 `final ≥ 1`）
/// 但余数最大可达「行数 − 1」。逐分派发永远派得完：总余量 = `final + 余数` ≥ 余数。
///
/// 顺序固定 ⇒ 同输入必然同输出，这是 spec 4.5 明文要求的。
/// **退货也用它**（②-3）：把一行的剩余金额按件数切成「这次退的」和「还留着的」，
/// 以及把实退总额按各行实付摊回去。取整规则和 Lot 分摊是同一套，
/// 所以「退一半再退一半」和「一次退完」的总额必然一致。
pub fn apportion(total: i64, weights: &[i64], caps: Option<&[i64]>) -> ApiResult<Vec<i64>> {
    let n = weights.len();
    let mut out = vec![0i64; n];
    if total == 0 {
        return Ok(out);
    }
    if total < 0 {
        return Err(ApiError::BadRequest("待分摊金额不能为负".into()));
    }
    let sum_w: i128 = weights.iter().map(|w| *w as i128).sum();
    if sum_w <= 0 {
        return Err(ApiError::BadRequest("无法分摊：权重之和为零".into()));
    }

    // i128 中间量：total 和 weight 都可能接近 i64 上限，先乘后除会溢出。
    let mut assigned: i64 = 0;
    for (i, w) in weights.iter().enumerate() {
        let share = ((total as i128) * (*w as i128) / sum_w) as i64;
        out[i] = share;
        assigned += share;
    }

    // 派发顺序：权重降序、下标升序。
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|a, b| weights[*b].cmp(&weights[*a]).then(a.cmp(b)));

    let mut remainder = total - assigned;
    while remainder > 0 {
        let before = remainder;
        for &i in &order {
            if remainder == 0 {
                break;
            }
            if let Some(c) = caps {
                if out[i] >= c[i] {
                    continue;
                }
            }
            out[i] += 1;
            remainder -= 1;
        }
        if remainder == before {
            // 一整圈一分都派不出去：Σ caps < total，调用方给错了参数。
            return Err(ApiError::BadRequest(
                "无法分摊：各行上限之和小于待分摊总额".into(),
            ));
        }
    }
    Ok(out)
}

/// 权重全零时退回 `fallback`（spec 4.5 的退化情形）。
fn weights_or<'a>(weights: &'a [i64], fallback: &'a [i64]) -> &'a [i64] {
    if weights.iter().all(|w| *w == 0) {
        fallback
    } else {
        weights
    }
}

/// Lot 价在成分间分摊 → 各成分行的 `allocated_amount`。
///
/// `weights` 是各成分的 `unit_price × qty`；整组都是 0 元时退回按 `fallback`（件数）分。
pub fn allocate_lot(lot_price: Money, weights: &[i64], fallback: &[i64]) -> ApiResult<Vec<Money>> {
    let w = weights_or(weights, fallback);
    Ok(apportion(lot_price.cents(), w, None)?
        .into_iter()
        .map(Money::from_cents)
        .collect())
}

/// 手工折让/加价摊进各行 → 各行 `paid_amount`。
///
/// - `solved > final`（折让）：各行 `paid = allocated − 分到的折让`，上限是自己的 `allocated`，
///   所以永远不会变成负数。
/// - `solved < final`（加价，spec 4.3 明说允许）：各行 `paid = allocated + 分到的加价`。
///   全赠品单（`Σ allocated = 0`）此时按 `qty` 分。
pub fn apply_manual_adjustment(
    allocated: &[Money],
    qty: &[i64],
    solved: Money,
    final_amount: Money,
) -> ApiResult<Vec<Money>> {
    if final_amount.is_negative() {
        return Err(ApiError::BadRequest("实收金额不能为负".into()));
    }
    let diff = solved.cents() - final_amount.cents();
    if diff == 0 {
        return Ok(allocated.to_vec());
    }

    let weights: Vec<i64> = allocated.iter().map(|m| m.cents()).collect();
    if diff > 0 {
        // 折让：上限就是各行自己的 allocated（让到 0 为止）。
        // Σ weights = solved ≥ diff，所以一定派得完。
        let shares = apportion(diff, &weights, Some(&weights))?;
        Ok(allocated
            .iter()
            .zip(shares)
            .map(|(a, s)| Money::from_cents(a.cents() - s))
            .collect())
    } else {
        let w = weights_or(&weights, qty);
        let shares = apportion(-diff, w, None)?;
        Ok(allocated
            .iter()
            .zip(shares)
            .map(|(a, s)| Money::from_cents(a.cents() + s))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cents(v: &[i64]) -> Vec<Money> {
        v.iter().copied().map(Money::from_cents).collect()
    }

    #[test]
    fn a_lot_price_is_split_in_proportion_and_sums_exactly() {
        // Lot 价 100，成分原价 30/35/40（合计 105）
        let out = allocate_lot(Money::from_cents(10000), &[3000, 3500, 4000], &[1, 1, 1]).unwrap();
        assert_eq!(out.iter().copied().sum::<Money>(), Money::from_cents(10000));
        // 余数派给权重最大的那一份
        assert_eq!(out, cents(&[2857, 3333, 3810]));
    }

    #[test]
    fn a_zero_priced_lot_allocates_zero_to_every_member() {
        let out = allocate_lot(Money::ZERO, &[3000, 2000], &[1, 1]).unwrap();
        assert_eq!(out, cents(&[0, 0]));
    }

    #[test]
    fn a_lot_made_of_free_items_falls_back_to_quantity() {
        // 权重全零（整组都是 0 元赠品）却有价：按件数分，而不是报错
        let out = allocate_lot(Money::from_cents(300), &[0, 0], &[1, 2]).unwrap();
        assert_eq!(out, cents(&[100, 200]));
    }

    #[test]
    fn no_adjustment_leaves_paid_equal_to_allocated() {
        let allocated = cents(&[6000, 2000]);
        let out = apply_manual_adjustment(
            &allocated,
            &[2, 1],
            Money::from_cents(8000),
            Money::from_cents(8000),
        )
        .unwrap();
        assert_eq!(out, allocated);
    }

    #[test]
    fn a_discount_is_spread_by_allocated_amount() {
        // 原价 80，摊主算 70：10 元折让按 6:2 摊成 7.5 / 2.5
        let allocated = cents(&[6000, 2000]);
        let out = apply_manual_adjustment(
            &allocated,
            &[2, 1],
            Money::from_cents(8000),
            Money::from_cents(7000),
        )
        .unwrap();
        assert_eq!(out.iter().copied().sum::<Money>(), Money::from_cents(7000));
        assert_eq!(out, cents(&[5250, 1750]));
    }

    #[test]
    fn a_markup_is_spread_the_same_way() {
        // spec 4.3 明说允许改高
        let allocated = cents(&[6000, 2000]);
        let out = apply_manual_adjustment(
            &allocated,
            &[2, 1],
            Money::from_cents(8000),
            Money::from_cents(9000),
        )
        .unwrap();
        assert_eq!(out.iter().copied().sum::<Money>(), Money::from_cents(9000));
        assert_eq!(out, cents(&[6750, 2250]));
    }

    #[test]
    fn the_remainder_never_pushes_a_line_below_zero() {
        // spec 4.5 第 3 条 2026-09-23 修正的那个反例：3 行各 1 元，抹成 0.01 元。
        // 原规则「余数全给 allocated 最大的行」会算出 paid = −1，撞 CHECK 变 500。
        let allocated = cents(&[100, 100, 100]);
        let out = apply_manual_adjustment(
            &allocated,
            &[1, 1, 1],
            Money::from_cents(300),
            Money::from_cents(1),
        )
        .unwrap();
        assert_eq!(out.iter().copied().sum::<Money>(), Money::from_cents(1));
        assert!(
            out.iter().all(|m| !m.is_negative()),
            "任何一行都不能被摊成负数"
        );
        assert_eq!(out, cents(&[0, 0, 1]));
    }

    #[test]
    fn giving_the_whole_order_away_zeroes_every_line() {
        let allocated = cents(&[6000, 2000]);
        let out =
            apply_manual_adjustment(&allocated, &[2, 1], Money::from_cents(8000), Money::ZERO)
                .unwrap();
        assert_eq!(out, cents(&[0, 0]));
    }

    #[test]
    fn an_all_gift_order_can_still_be_marked_up() {
        // spec 4.5 的退化情形：Σ allocated = 0 时按 allocated 的权重全是 0，退回按 qty 分
        let allocated = cents(&[0, 0]);
        let out = apply_manual_adjustment(&allocated, &[1, 2], Money::ZERO, Money::from_cents(300))
            .unwrap();
        assert_eq!(out, cents(&[100, 200]));
    }

    #[test]
    fn a_negative_final_amount_is_refused() {
        let allocated = cents(&[6000]);
        let err = apply_manual_adjustment(
            &allocated,
            &[1],
            Money::from_cents(6000),
            Money::from_cents(-1),
        )
        .unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn the_parts_always_add_up_to_the_whole() {
        // spec 第 9 节要求的 property test。不引入 proptest——一个 LCG 就够，
        // 而且种子固定，失败时可以原样复现。
        let mut seed: u64 = 0x5eed_1234_abcd_ef01;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as i64
        };
        for _ in 0..2000 {
            let n = (next() % 8 + 1) as usize;
            let allocated: Vec<Money> = (0..n)
                .map(|_| Money::from_cents(next() % 100_000))
                .collect();
            let qty: Vec<i64> = (0..n).map(|_| next() % 5 + 1).collect();
            let solved: Money = allocated.iter().copied().sum();
            // final 在 [0, solved * 2] 里取，同时覆盖折让与加价
            let span = solved.cents().saturating_mul(2) + 1;
            let final_amount = Money::from_cents(next() % span);

            let paid = apply_manual_adjustment(&allocated, &qty, solved, final_amount).unwrap();
            assert_eq!(paid.len(), n);
            assert_eq!(
                paid.iter().copied().sum::<Money>(),
                final_amount,
                "Σ paid 必须精确等于 final（allocated={allocated:?} final={final_amount:?}）"
            );
            assert!(
                paid.iter().all(|m| !m.is_negative()),
                "任何一行都不能是负数（allocated={allocated:?} final={final_amount:?}）"
            );
        }
    }
}
