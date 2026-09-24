//! 结算单的全部算术。**纯函数：不 import sqlx，不 import axum，不碰时间。**
//!
//! `api/settlement.rs` 负责把各张表查出来喂进 `build_report`，页面 JSON 和
//! `settlement.xlsx` 渲染的是同一个 `SettlementReport`——这是「页面显示 1,170、
//! 导出写 1,150」在结构上不可能发生的唯一办法。
//!
//! 两条恒等式在这里断言，不平就写进 `warnings` 让页面显眼地标出来，
//! **不静默出一个错数**：
//!
//! ```text
//! 货： 带去 − 带回 = 卖出 + 赠送 + 报废 + 差异 + 现场仓余额
//! 钱： 净额 + 退货保留 + 自掏赠品 − 垫付 + 调整  ==  −(社团往来余额)
//! ```
//!
//! 第二条的左边从业务表加出来、右边从账本聚合出来，两条独立的路必须撞上。

// 消费方在 Task 8 的 api 层（结算单端点 + xlsx 导出）。在那之前本模块的类型只被
// 测试构造，`cargo clippy -- -D warnings` 下整块都是 dead_code。接上之后删掉这行。
#![allow(dead_code)]

use serde::Serialize;

use crate::domain::money::Money;

/// 一个商品在一场展会里的全部去向。**每一项都从 `stock_movements` 按方向取**，
/// 没有任何缓存字段——母 spec 的整个设计就是「不存在第二个可以漂移的数字」。
#[derive(Debug, Clone, Serialize)]
pub struct GoodsLine {
    pub event_product_id: i64,
    pub product_code: String,
    pub name: String,
    /// `外部 → 现场仓` 的合计。开场带货和中途补货是同一件事。
    pub brought_in: i64,
    /// `顾客仓` 的净余额。退货会自动减回去。
    pub sold: i64,
    pub gifted: i64,
    /// `损耗` 的净余额，含「退货到损耗」的那部分。
    pub scrapped: i64,
    /// `差异` 的净余额。盘亏为正、盘盈为负。
    pub variance: i64,
    /// `现场仓 → 外部` 的合计。
    pub taken_back: i64,
    /// 还在现场仓的。走完带回之后必须是 0。
    pub on_site: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub label: String,
    /// **按对「我应转给」的影响存**：垫付为正（算出来要减），
    /// 结算调整正数 = 我要多给他们（算出来要加）。
    /// 存进 DB 的符号是另一回事（那边存的是对往来余额的影响），换算在 api 层做。
    pub amount: Money,
}

#[derive(Debug, Clone)]
pub struct SocietyInput {
    pub society_id: i64,
    pub name: String,
    pub is_home: bool,
    pub goods: Vec<GoodsLine>,
    pub gross: Money,
    pub allocated_net: Money,
    /// 折让为正、加价为负。**只有本社团可能非零**（母 spec 4.4）。
    pub manual_discount_net: Money,
    pub refund_kept: Money,
    pub gift_self_paid: Money,
    pub advances: Vec<Entry>,
    pub adjustments: Vec<Entry>,
    /// 账本里 `社团往来:<id>` 的余额。**这是「我应转给」的权威来源**，
    /// 上面那些字段是用来核对它的。
    pub due_balance: Money,
}

#[derive(Debug, Clone)]
pub struct ChannelCount {
    pub channel: String,
    /// 账面应有 = `实收-<渠道>` 的余额。
    pub book: Money,
    /// 摊主录的实际到手数。`None` = 还没清点。
    pub actual: Option<Money>,
}

#[derive(Debug, Clone)]
pub struct SettlementInput {
    pub event_name: String,
    pub event_date: String,
    pub generated_at: String,
    /// 本展会最新一条 journal 的时间。冻结之后垫付、调整、清点仍然会动账
    /// （spec 偏离 3），摊主得能看出手里那张导出的表是不是最新的。
    pub last_changed_at: Option<String>,
    pub stocktaken_at: Option<String>,
    pub societies: Vec<SocietyInput>,
    pub channels: Vec<ChannelCount>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct GoodsTotals {
    pub brought_in: i64,
    pub sold: i64,
    pub gifted: i64,
    pub scrapped: i64,
    pub variance: i64,
    pub taken_back: i64,
    pub on_site: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SocietyBlock {
    pub society_id: i64,
    pub name: String,
    pub is_home: bool,
    pub goods: Vec<GoodsLine>,
    pub totals: GoodsTotals,
    pub gross: Money,
    pub lot_discount: Money,
    pub manual_discount: Money,
    pub net: Money,
    pub refund_kept: Money,
    pub gift_self_paid: Money,
    pub advances: Vec<Entry>,
    pub advances_total: Money,
    pub adjustments: Vec<Entry>,
    pub adjustments_total: Money,
    /// 我应转给他们 = −（往来余额）。
    pub transfer: Money,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelLine {
    pub channel: String,
    pub book: Money,
    pub actual: Money,
    pub diff: Money,
    pub counted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettlementReport {
    pub event_name: String,
    pub event_date: String,
    pub generated_at: String,
    pub last_changed_at: Option<String>,
    pub stocktaken: bool,
    pub stocktaken_at: Option<String>,
    pub societies: Vec<SocietyBlock>,
    pub channels: Vec<ChannelLine>,
    pub actual_total: Money,
    pub transfer_total: Money,
    /// 摊主留存 = 实际到手合计 − Σ 我应转给各货主。
    pub vendor_retained: Money,
    /// 恒等式没撞上的地方。**页面和导出都要显眼地显示它**。
    pub warnings: Vec<String>,
}

pub fn build_report(input: &SettlementInput) -> SettlementReport {
    let mut warnings = Vec::new();
    let mut societies = Vec::with_capacity(input.societies.len());
    let mut transfer_total = Money::ZERO;

    for s in &input.societies {
        let mut totals = GoodsTotals::default();
        for g in &s.goods {
            totals.brought_in += g.brought_in;
            totals.sold += g.sold;
            totals.gifted += g.gifted;
            totals.scrapped += g.scrapped;
            totals.variance += g.variance;
            totals.taken_back += g.taken_back;
            totals.on_site += g.on_site;

            // 货那一侧的借贷必平。带回之前 on_site 非零也照样成立——
            // 恒等式里有它，所以这条检查在向导走完之前就能用。
            let left = g.brought_in - g.taken_back;
            let right = g.sold + g.gifted + g.scrapped + g.variance + g.on_site;
            if left != right {
                warnings.push(format!(
                    "{}（{}）的货对不上：带去 {} − 带回 {} = {}，但卖出 {} + 赠送 {} + 报废 {} + 差异 {} + 现场仓 {} = {}",
                    s.name, g.name, g.brought_in, g.taken_back, left,
                    g.sold, g.gifted, g.scrapped, g.variance, g.on_site, right
                ));
            }
        }

        // 手工折让只可能落在本社团（母 spec 4.4）。落到别人头上说明
        // api 层的归集查询写错了——那会让代卖货主平白少拿钱。
        if !s.is_home && !s.manual_discount_net.is_zero() {
            warnings.push(format!(
                "{} 是代卖货主，身上不该有手工折让（{}）——母 spec 4.4 定了它由本社团全额承担",
                s.name, s.manual_discount_net
            ));
        }

        let lot_discount = s.gross - s.allocated_net;
        let net = s.allocated_net - s.manual_discount_net;
        let advances_total: Money = s.advances.iter().map(|e| e.amount).sum();
        let adjustments_total: Money = s.adjustments.iter().map(|e| e.amount).sum();

        // 权威值：账本。上面那些是用来核对它的。
        let transfer = -s.due_balance;
        let expected = net + s.refund_kept + s.gift_self_paid - advances_total + adjustments_total;
        if expected != transfer {
            warnings.push(format!(
                "{} 的结算对不上账本：明细加出来是 {}，账本余额算出来是 {}，差 {}",
                s.name,
                expected,
                transfer,
                expected - transfer
            ));
        }

        transfer_total += transfer;
        societies.push(SocietyBlock {
            society_id: s.society_id,
            name: s.name.clone(),
            is_home: s.is_home,
            goods: s.goods.clone(),
            totals,
            gross: s.gross,
            lot_discount,
            manual_discount: s.manual_discount_net,
            net,
            refund_kept: s.refund_kept,
            gift_self_paid: s.gift_self_paid,
            advances: s.advances.clone(),
            advances_total,
            adjustments: s.adjustments.clone(),
            adjustments_total,
            transfer,
        });
    }

    let mut channels = Vec::with_capacity(input.channels.len());
    let mut actual_total = Money::ZERO;
    for c in &input.channels {
        // 没清点的渠道回落到账面值。算成 0 会让「摊主留存」凭空少一大截，
        // 而那个数是摊主唯一会盯着看的。
        let counted = c.actual.is_some();
        let actual = c.actual.unwrap_or(c.book);
        actual_total += actual;
        channels.push(ChannelLine {
            channel: c.channel.clone(),
            book: c.book,
            actual,
            diff: actual - c.book,
            counted,
        });
    }

    SettlementReport {
        event_name: input.event_name.clone(),
        event_date: input.event_date.clone(),
        generated_at: input.generated_at.clone(),
        last_changed_at: input.last_changed_at.clone(),
        stocktaken: input.stocktaken_at.is_some(),
        stocktaken_at: input.stocktaken_at.clone(),
        societies,
        channels,
        actual_total,
        transfer_total,
        vendor_retained: actual_total - transfer_total,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::too_many_arguments)]
    fn goods(
        code: &str,
        brought: i64,
        sold: i64,
        gift: i64,
        scrap: i64,
        var: i64,
        back: i64,
        on: i64,
    ) -> GoodsLine {
        GoodsLine {
            event_product_id: 1,
            product_code: code.into(),
            name: code.into(),
            brought_in: brought,
            sold,
            gifted: gift,
            scrapped: scrap,
            variance: var,
            taken_back: back,
            on_site: on,
        }
    }

    fn society(id: i64, is_home: bool) -> SocietyInput {
        SocietyInput {
            society_id: id,
            name: format!("社团{id}"),
            is_home,
            goods: vec![],
            gross: Money::ZERO,
            allocated_net: Money::ZERO,
            manual_discount_net: Money::ZERO,
            refund_kept: Money::ZERO,
            gift_self_paid: Money::ZERO,
            advances: vec![],
            adjustments: vec![],
            due_balance: Money::ZERO,
        }
    }

    fn input(societies: Vec<SocietyInput>, channels: Vec<ChannelCount>) -> SettlementInput {
        SettlementInput {
            event_name: "ABC漫展".into(),
            event_date: "2026-10-01".into(),
            generated_at: "2026-10-01 20:00:00".into(),
            last_changed_at: Some("2026-10-01 19:40:00".into()),
            stocktaken_at: Some("2026-10-01 18:23:00".into()),
            societies,
            channels,
        }
    }

    #[test]
    fn the_spec_sample_adds_up() {
        // 母 spec 第 7 节的样例，原样搬过来。
        // 星见社（本社团）：原价 1830、Lot 折让 120、手工折让 60 ⇒ 净额 1650；
        //                  垫付 480 ⇒ 我应转给 1170
        // 黄昏堂（代卖）  ：原价 360 ⇒ 净额 360；调整「赔 20」⇒ 我应转给 380
        // 清点：微信 1540 ✓、支付宝 320 ✓、现金盒 145（应 150，短 5）
        // 摊主留存 = 2005 − 1170 − 380 = 455
        let mut home = society(1, true);
        home.name = "星见社".into();
        home.goods = vec![goods("A", 90, 61, 4, 1, 0, 24, 0)];
        home.gross = Money::from_cents(183000);
        home.allocated_net = Money::from_cents(171000);
        home.manual_discount_net = Money::from_cents(6000);
        home.advances = vec![
            Entry {
                label: "摊位费".into(),
                amount: Money::from_cents(40000),
            },
            Entry {
                label: "打印费".into(),
                amount: Money::from_cents(8000),
            },
        ];
        home.due_balance = Money::from_cents(-117000);

        let mut other = society(2, false);
        other.name = "黄昏堂".into();
        other.goods = vec![goods("B", 30, 12, 0, 0, 0, 18, 0)];
        other.gross = Money::from_cents(36000);
        other.allocated_net = Money::from_cents(36000);
        other.adjustments = vec![Entry {
            label: "带回后清点少 1 本，按成本赔".into(),
            amount: Money::from_cents(2000),
        }];
        other.due_balance = Money::from_cents(-38000);

        let report = build_report(&input(
            vec![home, other],
            vec![
                ChannelCount {
                    channel: "微信".into(),
                    book: Money::from_cents(154000),
                    actual: Some(Money::from_cents(154000)),
                },
                ChannelCount {
                    channel: "支付宝".into(),
                    book: Money::from_cents(32000),
                    actual: Some(Money::from_cents(32000)),
                },
                ChannelCount {
                    channel: "现金".into(),
                    book: Money::from_cents(15000),
                    actual: Some(Money::from_cents(14500)),
                },
            ],
        ));

        assert!(
            report.warnings.is_empty(),
            "样例应当自洽：{:?}",
            report.warnings
        );
        assert_eq!(report.societies[0].lot_discount, Money::from_cents(12000));
        assert_eq!(report.societies[0].net, Money::from_cents(165000));
        assert_eq!(report.societies[0].transfer, Money::from_cents(117000));
        assert_eq!(report.societies[1].net, Money::from_cents(36000));
        assert_eq!(report.societies[1].transfer, Money::from_cents(38000));
        assert_eq!(report.actual_total, Money::from_cents(200500));
        assert_eq!(report.transfer_total, Money::from_cents(155000));
        assert_eq!(report.vendor_retained, Money::from_cents(45500));
        assert_eq!(report.channels[2].diff, Money::from_cents(-500), "现金短 5");
    }

    #[test]
    fn a_broken_goods_identity_becomes_a_warning_not_a_wrong_number() {
        // 带去 90，卖 61 + 赠 4 + 废 1 + 带回 24 = 90，但差异写成 3 ⇒ 对不上。
        let mut s = society(1, true);
        s.goods = vec![goods("A", 90, 61, 4, 1, 3, 24, 0)];
        let report = build_report(&input(vec![s], vec![]));
        assert_eq!(report.warnings.len(), 1);
        assert!(
            report.warnings[0].contains("A"),
            "要说清是哪个商品：{:?}",
            report.warnings
        );
    }

    #[test]
    fn a_transfer_that_disagrees_with_the_ledger_becomes_a_warning() {
        // 这条是整套设计里最强的一个断言：业务表加出来的数和账本聚合出来的数
        // 必须撞上。撞不上说明某条腿方向写错了或者漏记了。
        let mut s = society(2, false);
        s.allocated_net = Money::from_cents(36000);
        s.due_balance = Money::from_cents(-30000); // 账本说只欠 300
        let report = build_report(&input(vec![s], vec![]));
        assert_eq!(report.warnings.len(), 1);
        assert!(
            report.warnings[0].contains("60.00"),
            "要把差额写出来才有得查：{:?}",
            report.warnings
        );
    }

    #[test]
    fn refund_coverage_and_self_paid_gifts_land_in_the_transfer() {
        // 退货第 ③ 条腿（顾客没拿回的部分归货主）和摊主自掏赠品，
        // 两者都不经过「净额」那一栏，但都影响「我应转给」。
        let mut s = society(2, false);
        s.allocated_net = Money::from_cents(10000);
        s.refund_kept = Money::from_cents(1500);
        s.gift_self_paid = Money::from_cents(2000);
        s.due_balance = Money::from_cents(-13500);
        let report = build_report(&input(vec![s], vec![]));
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert_eq!(report.societies[0].transfer, Money::from_cents(13500));
    }

    #[test]
    fn manual_discount_only_ever_lands_on_the_home_society() {
        // 母 spec 4.4。代卖货主的净额恒等于 allocated_net。
        let mut other = society(2, false);
        other.allocated_net = Money::from_cents(36000);
        other.manual_discount_net = Money::from_cents(5000); // 不该发生
        other.due_balance = Money::from_cents(-36000);
        let report = build_report(&input(vec![other], vec![]));
        assert!(
            report.warnings.iter().any(|w| w.contains("手工折让")),
            "代卖货主身上出现手工折让必须报出来：{:?}",
            report.warnings
        );
    }

    #[test]
    fn an_unstocktaken_event_says_so() {
        let mut i = input(vec![society(1, true)], vec![]);
        i.stocktaken_at = None;
        let report = build_report(&i);
        assert!(!report.stocktaken);
    }

    #[test]
    fn an_uncounted_channel_falls_back_to_the_book_value() {
        // 没清点的渠道不该把「实际到手」算成 0，那会让摊主留存凭空少一大截。
        let report = build_report(&input(
            vec![],
            vec![ChannelCount {
                channel: "微信".into(),
                book: Money::from_cents(154000),
                actual: None,
            }],
        ));
        assert_eq!(report.actual_total, Money::from_cents(154000));
        assert_eq!(report.channels[0].diff, Money::ZERO);
        assert!(!report.channels[0].counted);
    }
}
