//! 金额类型。**单位是分，不是元。**
//!
//! 为什么必须换掉 `f64`：②-2 的分摊规则、②-3 的结算单，核心断言全是
//! 「各行金额之和必须精确等于总价」。浮点下这个断言写不出来——
//! `0.1 + 0.2 != 0.3`，而摊位场景里 19.9 这种价格遍地都是。
//!
//! 对外（JSON、数据库列）就是一个整数，`#[serde(transparent)]` 保证序列化后
//! 是裸数字而不是 `{"0": 1990}`。前端拿到分，显示时才除以 100
//! （`frontend/src/utils/money.js`）。

use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

/// 金额，单位：分。可以为负（账户余额、结算调整都可能是负的）。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Money(i64);

impl Money {
    pub const ZERO: Money = Money(0);

    /// 从分构造金额。
    pub const fn from_cents(cents: i64) -> Self {
        Money(cents)
    }

    pub const fn cents(self) -> i64 {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// 消费方在 ②-3（结算调整可负，显示时取绝对值），此前无人调用。
    #[allow(dead_code)]
    pub fn abs(self) -> Self {
        Money(self.0.abs())
    }

    /// 单价 × 数量。溢出返回 None——摊位场景溢不了，但把它变成一个
    /// 调用方必须处理的 Option，好过某天真的溢出时悄悄绕回负数。
    ///
    /// 消费方在 ②-2（折扣求解器算「单价 × 数量」），此前只在 test 里用。
    #[allow(dead_code)]
    pub fn checked_mul_qty(self, qty: i64) -> Option<Self> {
        self.0.checked_mul(qty).map(Money)
    }
}

impl Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Money {
        Money(self.0 + rhs.0)
    }
}

impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Money {
        Money(self.0 - rhs.0)
    }
}

impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money(-self.0)
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Money) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Money) {
        self.0 -= rhs.0;
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        iter.fold(Money::ZERO, |a, b| a + b)
    }
}

/// 只给日志和错误消息用，显示成「¥19.90」。响应体里永远是分。
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let a = self.0.abs();
        write!(f, "{sign}¥{}.{:02}", a / 100, a % 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_pads_cents_to_two_digits() {
        // 19.9 元这种价格在摊位上遍地都是，不补零会显示成「¥19.9」
        assert_eq!(Money::from_cents(1990).to_string(), "¥19.90");
        assert_eq!(Money::from_cents(5).to_string(), "¥0.05");
        assert_eq!(Money::from_cents(-1990).to_string(), "-¥19.90");
        assert_eq!(Money::ZERO.to_string(), "¥0.00");
    }

    #[test]
    fn sum_of_parts_is_exact() {
        // 这正是换掉 f64 的理由：0.1 + 0.2 == 0.3 在分上是恒真的
        let parts = [Money::from_cents(10), Money::from_cents(20)];
        assert_eq!(parts.into_iter().sum::<Money>(), Money::from_cents(30));
    }

    #[test]
    fn serializes_as_bare_integer() {
        // serde(transparent)：响应体里必须是 1990 而不是 {"0":1990} 或 "19.90"
        assert_eq!(
            serde_json::to_string(&Money::from_cents(1990)).unwrap(),
            "1990"
        );
        let back: Money = serde_json::from_str("1990").unwrap();
        assert_eq!(back, Money::from_cents(1990));
    }

    #[test]
    fn checked_mul_qty_reports_overflow_instead_of_wrapping() {
        assert_eq!(
            Money::from_cents(3000).checked_mul_qty(2),
            Some(Money::from_cents(6000))
        );
        assert_eq!(Money::from_cents(i64::MAX).checked_mul_qty(2), None);
    }
}
