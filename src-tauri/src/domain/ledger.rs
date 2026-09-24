//! 账本：journal + 两张移动表。
//!
//! **每条移动的 from 和 to 都非空。** 这是移动模型相对经典借贷分录的全部价值：
//! 「不平衡的分录」不是一个需要主动校验的不变量，而是一个写不出来的错误。
//!
//! 取代的旧形状：`products.current_stock` 是唯一真相，三个互不共享的写入方
//! （下单扣、取消退、管理员改初始库存，其中第三个还不在事务里）。所有
//! 「库存数字不对」的 bug 都是从那个形状长出来的。
//!
//! 这一层不认识 axum，也不认识 Tauri。

use crate::domain::money::Money;
use crate::error::{ApiError, ApiResult};
use sqlx::{Executor, Sqlite, SqlitePool, Transaction};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

// ==========================================
// 货的位置
// ==========================================

/// 货所在的位置。`External` 是**边界**：系统不追踪它的余额。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// 外部：社团库存 / 家里。边界账户。
    External,
    /// 现场仓：这次带到摊位、当前可售的货。
    OnSite,
    /// 顾客仓：已卖出的货。
    Customer,
    /// 损耗：报废、损坏。
    Loss,
    /// 赠品：送出去的货。
    Gift,
    /// 差异：盘点对不上的部分。
    Variance,
    /// 转换：破坏性组装/拆封的中转。
    Conversion,
}

impl Location {
    pub fn as_str(self) -> &'static str {
        match self {
            Location::External => "外部",
            Location::OnSite => "现场仓",
            Location::Customer => "顾客仓",
            Location::Loss => "损耗",
            Location::Gift => "赠品",
            Location::Variance => "差异",
            Location::Conversion => "转换",
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Location {
    type Err = ApiError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "外部" => Location::External,
            "现场仓" => Location::OnSite,
            "顾客仓" => Location::Customer,
            "损耗" => Location::Loss,
            "赠品" => Location::Gift,
            "差异" => Location::Variance,
            "转换" => Location::Conversion,
            other => return Err(ApiError::BadRequest(format!("未知的货物位置: {other}"))),
        })
    }
}

// ==========================================
// 钱的账户
// ==========================================

/// 资金账户。在数据库里存成字符串，因为 `实收-微信`、`摊主自有` 这些不挂任何社团，
/// 做不成一张统一的外键表。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Account {
    /// 摊主自己的钱。边界。
    VendorOwn,
    /// 摊主实际持有的某个渠道的款项。渠道名可自定义。
    Received(String),
    /// 与某个社团的往来。余额 = 他们欠我 − 我欠他们。**用 id 而不是名字，社团会改名。**
    SocietyDue(i64),
    /// 结算调整。边界。
    SettlementAdj,
    /// 对账差异。边界。默认由摊主个人吞（spec 第 5 节）。
    ReconDiff,
}

const RECEIVED_PREFIX: &str = "实收-";
const SOCIETY_PREFIX: &str = "社团往来:";

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Account::VendorOwn => f.write_str("摊主自有"),
            Account::Received(ch) => write!(f, "{RECEIVED_PREFIX}{ch}"),
            Account::SocietyDue(id) => write!(f, "{SOCIETY_PREFIX}{id}"),
            Account::SettlementAdj => f.write_str("结算调整"),
            Account::ReconDiff => f.write_str("对账差异"),
        }
    }
}

impl FromStr for Account {
    type Err = ApiError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 前缀匹配放在精确匹配之后，且用 strip_prefix 取**剩余全部**，
        // 这样渠道名里带横线（"自定义-渠道"）也能原样解回来。
        Ok(match s {
            "摊主自有" => Account::VendorOwn,
            "结算调整" => Account::SettlementAdj,
            "对账差异" => Account::ReconDiff,
            _ => {
                if let Some(ch) = s.strip_prefix(RECEIVED_PREFIX) {
                    Account::Received(ch.to_string())
                } else if let Some(id) = s.strip_prefix(SOCIETY_PREFIX) {
                    Account::SocietyDue(id.parse().map_err(|_| {
                        ApiError::BadRequest(format!("社团往来账户 id 不是数字: {s}"))
                    })?)
                } else {
                    return Err(ApiError::BadRequest(format!("未知的资金账户: {s}")));
                }
            }
        })
    }
}

// ==========================================
// journal 种类
// ==========================================

/// 除 `Restock`（Task 5 的 `api/product.rs` 构造）外，其余变体由后续 task 构造：
/// `Sale`/`Receipt`/`Refund`/`Cancel` → Task 6（订单），`Convert` → ②-2（Lot 拆封），
/// `Gift`/`Scrap`/`Stocktake`/`TakeBack`/`Advance`/`Adjust` → ②-3。
/// 在那之前它们是非 test 未构造变体。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalKind {
    Restock,
    Sale,
    Receipt,
    Refund,
    Cancel,
    Gift,
    Scrap,
    Stocktake,
    TakeBack,
    Convert,
    Advance,
    Adjust,
}

impl JournalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            JournalKind::Restock => "进货",
            JournalKind::Sale => "销售",
            JournalKind::Receipt => "收款",
            JournalKind::Refund => "退货",
            JournalKind::Cancel => "取消",
            JournalKind::Gift => "赠送",
            JournalKind::Scrap => "报废",
            JournalKind::Stocktake => "盘点",
            JournalKind::TakeBack => "带回",
            JournalKind::Convert => "拆封",
            JournalKind::Advance => "垫付",
            JournalKind::Adjust => "调整",
        }
    }
}

// ==========================================
// 移动的腿
// ==========================================

#[derive(Debug, Clone, Copy)]
pub struct StockLeg {
    pub event_product_id: i64,
    pub from: Location,
    pub to: Location,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct MoneyLeg {
    pub from: Account,
    pub to: Account,
    pub amount: Money,
}

// ==========================================
// 写入
// ==========================================

/// 记一次业务操作。**这是货和钱唯一的写入口。**
///
/// 所有腿和 journal 本身在同一个事务里落盘——调用方负责开事务和提交，
/// 因为一次业务操作常常还要连带写 `orders` / `advances` 这些业务表。
///
/// ⚠️ **`order_id` 和 `reverses` 相邻且都是 `Option<i64>`，编译器分不出传反。**
/// `order_id` 是「这条 journal 属于哪个订单」，`reverses` 是「这条 journal 冲正的是哪条 journal」。
/// 记普通业务操作时 `reverses` 永远是 `None`；只有 `reverse_journal` 才会填它。
/// 传反了 FK 未必拦得住（两个 id 恰好都有效时就穿过去了），而账会错得很隐蔽。
//
// 8 个参数是刻意的：tx / event / kind / order / reverses / note / stock / money
// 各自独立，没有哪几个天然属于同一个结构体。这个签名是后续 task 依赖的契约，
// 不能为了少一个参数就把 orders 的元数据硬塞进一个"options"壳里。
#[allow(clippy::too_many_arguments)]
pub async fn post_journal(
    tx: &mut Transaction<'_, Sqlite>,
    event_id: i64,
    kind: JournalKind,
    order_id: Option<i64>,
    reverses: Option<i64>,
    note: Option<&str>,
    stock: &[StockLeg],
    money: &[MoneyLeg],
) -> ApiResult<i64> {
    // 先把零金额的钱腿滤掉再判断是否为空。
    //
    // 为什么不能在函数开头只看入参 slice 是否为空：零金额的腿会被静默跳过
    // （②-2 的手工折让为 0 时那条腿本就不该存在），于是「money 非空但全是零」
    // 会插出一条两张腿表都空的 journal——而这样的 journal **无法被冲正**
    // （reverse_journal 读回 0 条腿，转调 post_journal(&[], &[]) 会被同一个检查拒掉），
    // 造得出来却造不掉。spec 4.6 的全赠品订单（0 元行）正好会走到这条路径。
    let money: Vec<&MoneyLeg> = money.iter().filter(|l| !l.amount.is_zero()).collect();

    if stock.is_empty() && money.is_empty() {
        return Err(ApiError::BadRequest(
            "空的 journal：既没有货也没有钱".into(),
        ));
    }

    let journal_id: i64 = sqlx::query_scalar(
        "INSERT INTO journals (event_id, kind, order_id, reverses_journal_id, note)
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(event_id)
    .bind(kind.as_str())
    .bind(order_id)
    .bind(reverses)
    .bind(note)
    .fetch_one(&mut **tx)
    .await?;

    for leg in stock {
        if leg.qty <= 0 {
            return Err(ApiError::BadRequest("移动数量必须为正".into()));
        }
        sqlx::query(
            "INSERT INTO stock_movements (journal_id, event_product_id, from_location, to_location, qty)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(journal_id)
        .bind(leg.event_product_id)
        .bind(leg.from.as_str())
        .bind(leg.to.as_str())
        .bind(leg.qty)
        .execute(&mut **tx)
        .await?;
    }

    for leg in money {
        // 金额为 0 的腿上面已经滤掉：②-2 的手工折让为 0 时这条腿本就不该存在，
        // 让调用方无脑传、这里过滤，比每个调用方各自判一遍可靠。
        if leg.amount.is_negative() {
            return Err(ApiError::BadRequest(
                "资金移动的金额必须为正——方向由 from/to 表达，不靠负数".into(),
            ));
        }
        sqlx::query(
            "INSERT INTO money_movements (journal_id, from_account, to_account, amount)
             VALUES (?, ?, ?, ?)",
        )
        .bind(journal_id)
        .bind(leg.from.to_string())
        .bind(leg.to.to_string())
        .bind(leg.amount.cents())
        .execute(&mut **tx)
        .await?;
    }

    Ok(journal_id)
}

/// 冲正一个 journal：把它的每条腿反向再记一遍，并用 `reverses_journal_id` 关联。
///
/// **冲正而不是删除**，是会计的标准做法，也留下审计痕迹（弃单率随时能查）。
/// 重复冲正由 `idx_journals_reverses` 这个偏唯一索引在 DB 层拦住——
/// 没有它，一次网络重试就能把库存退两遍。
pub async fn reverse_journal(
    tx: &mut Transaction<'_, Sqlite>,
    journal_id: i64,
    kind: JournalKind,
    note: Option<&str>,
) -> ApiResult<i64> {
    let (event_id, order_id): (i64, Option<i64>) =
        sqlx::query_as("SELECT event_id, order_id FROM journals WHERE id = ?")
            .bind(journal_id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("journal {journal_id} 不存在")))?;

    let stock_rows: Vec<(i64, String, String, i64)> = sqlx::query_as(
        "SELECT event_product_id, from_location, to_location, qty
         FROM stock_movements WHERE journal_id = ?",
    )
    .bind(journal_id)
    .fetch_all(&mut **tx)
    .await?;

    let money_rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT from_account, to_account, amount FROM money_movements WHERE journal_id = ?",
    )
    .bind(journal_id)
    .fetch_all(&mut **tx)
    .await?;

    // 注意 from/to 互换
    let stock: Vec<StockLeg> = stock_rows
        .into_iter()
        .map(|(ep, from, to, qty)| {
            Ok(StockLeg {
                event_product_id: ep,
                from: to.parse()?,
                to: from.parse()?,
                qty,
            })
        })
        .collect::<ApiResult<_>>()?;

    let money: Vec<MoneyLeg> = money_rows
        .into_iter()
        .map(|(from, to, amount)| {
            Ok(MoneyLeg {
                from: to.parse()?,
                to: from.parse()?,
                amount: Money::from_cents(amount),
            })
        })
        .collect::<ApiResult<_>>()?;

    post_journal(
        tx,
        event_id,
        kind,
        order_id,
        Some(journal_id),
        note,
        &stock,
        &money,
    )
    .await
}

/// 冲正某个订单名下**全部尚未被冲正**的 journal。
///
/// 取消一个已完成的订单要同时回滚货和钱两个 journal（spec 6.1）——
/// 只回滚一半正是旧模型反复出事的地方。返回冲正了几个。
pub async fn reverse_order_journals(
    tx: &mut Transaction<'_, Sqlite>,
    order_id: i64,
    note: Option<&str>,
) -> ApiResult<usize> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT j.id FROM journals j
         WHERE j.order_id = ?
           AND j.reverses_journal_id IS NULL
           AND NOT EXISTS (SELECT 1 FROM journals r WHERE r.reverses_journal_id = j.id)
         ORDER BY j.id",
    )
    .bind(order_id)
    .fetch_all(&mut **tx)
    .await?;

    for id in &ids {
        reverse_journal(tx, *id, JournalKind::Cancel, note).await?;
    }
    Ok(ids.len())
}

// ==========================================
// 余额：对流水实时聚合，不做缓存
// ==========================================

/// 某个摊位商品当前在现场仓的数量。
///
/// 代价是每次读库存都要聚合。摊位规模下不是问题：一场展会几十个商品、
/// 几百到几千条移动，`idx_stock_movements_product` 下是微秒级。
/// 若实测慢了再加物化缓存——那时它是纯优化，有流水做权威源可随时重算校验。
pub async fn onsite_balance<'e, E>(executor: E, event_product_id: i64) -> ApiResult<i64>
where
    E: Executor<'e, Database = Sqlite>,
{
    let qty: i64 = sqlx::query_scalar(
        "SELECT COALESCE(
                  SUM(CASE WHEN to_location   = '现场仓' THEN qty ELSE 0 END)
                - SUM(CASE WHEN from_location = '现场仓' THEN qty ELSE 0 END), 0)
         FROM stock_movements WHERE event_product_id = ?",
    )
    .bind(event_product_id)
    .fetch_one(executor)
    .await?;
    Ok(qty)
}

/// 一场展会里全部商品的现场仓余额。列表页用，避免 N+1。
pub async fn onsite_balances(pool: &SqlitePool, event_id: i64) -> ApiResult<HashMap<i64, i64>> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT ep.id,
                COALESCE(
                    SUM(CASE WHEN sm.to_location   = '现场仓' THEN sm.qty ELSE 0 END)
                  - SUM(CASE WHEN sm.from_location = '现场仓' THEN sm.qty ELSE 0 END), 0)
         FROM event_products ep
         LEFT JOIN stock_movements sm ON sm.event_product_id = ep.id
         WHERE ep.event_id = ?
         GROUP BY ep.id",
    )
    .bind(event_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// 某个资金账户在某场展会里的余额。
///
/// **永远按展会过滤**：跨展会没有连续账（设备不同步，做不对）。
///
/// 消费方在 ②-3（结算对账），此前只在 test 里用。
#[allow(dead_code)]
pub async fn account_balance(
    pool: &SqlitePool,
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
    .fetch_one(pool)
    .await?;
    Ok(Money::from_cents(cents))
}

/// 本社团的 id。手工折让和退货的第二条腿都要拿它当对手账户。
///
/// **Review Focus #5**：`societies` 表理论上保证有且只有一个 `is_home = 1`
/// （迁移里建了偏索引），但用户删得掉社团，手工改过的库也进得来。
/// 没有就 panic 会把整个 handler 打成 500，摊主看到「服务器错误」无从下手；
/// 这里给一句能照着做的话。
pub async fn home_society_id(conn: &mut sqlx::SqliteConnection) -> ApiResult<i64> {
    sqlx::query_scalar("SELECT id FROM societies WHERE is_home = 1")
        .fetch_optional(&mut *conn)
        .await?
        .ok_or_else(|| {
            ApiError::Conflict(
                "还没有设置本社团，无法记账——请先到「社团管理」把自己的社团标成本社团".into(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_pool;
    use sqlx::SqlitePool;

    /// 造一场展会 + 两个社团 + 两个商品，返回 (event_id, ep_a, ep_b)。
    /// ep_a 归本社团(1)，ep_b 归代卖社团(2)。
    async fn fixture(pool: &SqlitePool) -> (i64, i64, i64) {
        sqlx::query("INSERT INTO societies (id, name, is_home) VALUES (2, '黄昏堂', 0)")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO master_products (id, product_code, name, default_price, owner_society_id)
             VALUES (1, 'A', '本子A', 0, 1), (2, 'B', '本子B', 0, 2)",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO events (id, name, event_date, status) VALUES (1, 'ABC漫展', '2026-10-01', '进行中')",
        ).execute(pool).await.unwrap();
        sqlx::query(
            "INSERT INTO event_products
               (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
             VALUES (1, 1, 1, 1, 'A', '本子A', 3000), (2, 1, 2, 2, 'B', '本子B', 2000)",
        )
        .execute(pool)
        .await
        .unwrap();
        (1, 1, 2)
    }

    #[tokio::test]
    async fn stock_balance_is_the_aggregate_of_movements() {
        let pool = test_pool().await;
        let (event_id, ep_a, ep_b) = fixture(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        post_journal(
            &mut tx,
            event_id,
            JournalKind::Restock,
            None,
            None,
            Some("开场带货"),
            &[
                StockLeg {
                    event_product_id: ep_a,
                    from: Location::External,
                    to: Location::OnSite,
                    qty: 10,
                },
                StockLeg {
                    event_product_id: ep_b,
                    from: Location::External,
                    to: Location::OnSite,
                    qty: 5,
                },
            ],
            &[],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let balances = onsite_balances(&pool, event_id).await.unwrap();
        assert_eq!(balances.get(&ep_a), Some(&10));
        assert_eq!(balances.get(&ep_b), Some(&5));
    }

    #[tokio::test]
    async fn reversing_a_journal_restores_every_balance_it_touched() {
        let pool = test_pool().await;
        let (event_id, ep_a, _) = fixture(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        post_journal(
            &mut tx,
            event_id,
            JournalKind::Restock,
            None,
            None,
            None,
            &[StockLeg {
                event_product_id: ep_a,
                from: Location::External,
                to: Location::OnSite,
                qty: 10,
            }],
            &[],
        )
        .await
        .unwrap();
        let sale = post_journal(
            &mut tx,
            event_id,
            JournalKind::Sale,
            None,
            None,
            None,
            &[StockLeg {
                event_product_id: ep_a,
                from: Location::OnSite,
                to: Location::Customer,
                qty: 3,
            }],
            &[MoneyLeg {
                from: Account::SocietyDue(1),
                to: Account::Received("微信".into()),
                amount: Money::from_cents(9000),
            }],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 7);
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into()))
                .await
                .unwrap(),
            Money::from_cents(9000)
        );

        let mut tx = pool.begin().await.unwrap();
        reverse_journal(&mut tx, sale, JournalKind::Cancel, Some("取消订单"))
            .await
            .unwrap();
        tx.commit().await.unwrap();

        // 货和钱必须同时归位——只回滚一半是这个模型最该防住的事故
        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 10);
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into()))
                .await
                .unwrap(),
            Money::ZERO
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1))
                .await
                .unwrap(),
            Money::ZERO
        );
    }

    #[tokio::test]
    async fn a_journal_cannot_be_reversed_twice() {
        let pool = test_pool().await;
        let (event_id, ep_a, _) = fixture(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        let j = post_journal(
            &mut tx,
            event_id,
            JournalKind::Restock,
            None,
            None,
            None,
            &[StockLeg {
                event_product_id: ep_a,
                from: Location::External,
                to: Location::OnSite,
                qty: 10,
            }],
            &[],
        )
        .await
        .unwrap();
        reverse_journal(&mut tx, j, JournalKind::Cancel, None)
            .await
            .unwrap();
        // 第二次必须被 idx_journals_reverses 这个偏唯一索引拦住。
        // 没有这条约束，一次网络重试就能把库存退两遍。
        let second = reverse_journal(&mut tx, j, JournalKind::Cancel, None).await;
        assert!(second.is_err(), "同一个 journal 不该能冲正两次");
    }

    #[tokio::test]
    async fn account_round_trip_through_string_form() {
        // 账户在 DB 里是字符串，解析不回来就等于账丢了
        for acc in [
            Account::VendorOwn,
            Account::Received("微信".into()),
            Account::Received("自定义-渠道".into()), // 渠道名里带横线也必须能解回来
            Account::SocietyDue(42),
            Account::SettlementAdj,
            Account::ReconDiff,
        ] {
            let s = acc.to_string();
            assert_eq!(s.parse::<Account>().unwrap(), acc, "往返失败: {s}");
        }
    }

    #[tokio::test]
    async fn posting_a_movement_from_and_to_the_same_place_is_rejected() {
        let pool = test_pool().await;
        let (event_id, ep_a, _) = fixture(&pool).await;
        let mut tx = pool.begin().await.unwrap();
        let r = post_journal(
            &mut tx,
            event_id,
            JournalKind::Restock,
            None,
            None,
            None,
            &[StockLeg {
                event_product_id: ep_a,
                from: Location::OnSite,
                to: Location::OnSite,
                qty: 1,
            }],
            &[],
        )
        .await;
        assert!(r.is_err(), "from == to 必须被 CHECK 约束拦住");
    }

    #[tokio::test]
    async fn a_journal_whose_legs_are_all_zero_is_rejected_not_created() {
        // 能造出来却造不掉的「幽灵 journal」：money 非空但全是零金额时，
        // 零值过滤会把腿全跳过，留下一条两张腿表都空的 journal，
        // 而它之后无法被冲正（reverse 读回 0 条腿 → 转调 post_journal(&[],&[]) → 被拒）。
        // spec 4.6 的全赠品订单（0 元行）正好会走到这条路径。
        let pool = test_pool().await;
        let (event_id, _ep_a, _) = fixture(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        let r = post_journal(
            &mut tx,
            event_id,
            JournalKind::Receipt,
            None,
            None,
            None,
            &[],
            &[MoneyLeg {
                from: Account::SocietyDue(1),
                to: Account::Received("微信".into()),
                amount: Money::ZERO,
            }],
        )
        .await;
        assert!(
            r.is_err(),
            "全零金额的 journal 必须被拒绝，不能插出无腿的 journal"
        );
        drop(tx);

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "被拒绝的 journal 不能在表里留下痕迹");
    }

    #[tokio::test]
    async fn reversing_an_order_undoes_both_its_goods_and_its_money_journals() {
        // Task 6 取消订单完全依赖这个函数。它必须一次把该订单名下**全部**
        // 未冲正的 journal 都冲掉——只冲一半（比如只退货不退钱）正是旧模型反复出事的地方。
        let pool = test_pool().await;
        let (event_id, ep_a, _) = fixture(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        post_journal(
            &mut tx,
            event_id,
            JournalKind::Restock,
            None,
            None,
            None,
            &[StockLeg {
                event_product_id: ep_a,
                from: Location::External,
                to: Location::OnSite,
                qty: 10,
            }],
            &[],
        )
        .await
        .unwrap();

        // 一张订单产生两条 journal：先记货，后记钱
        sqlx::query("INSERT INTO orders (id, event_id, status, channel, gross_amount, solved_amount, final_amount)
                     VALUES (7, ?, 'completed', '微信', 9000, 9000, 9000)")
            .bind(event_id)
            .execute(&mut *tx)
            .await
            .unwrap();
        post_journal(
            &mut tx,
            event_id,
            JournalKind::Sale,
            Some(7),
            None,
            None,
            &[StockLeg {
                event_product_id: ep_a,
                from: Location::OnSite,
                to: Location::Customer,
                qty: 3,
            }],
            &[],
        )
        .await
        .unwrap();
        post_journal(
            &mut tx,
            event_id,
            JournalKind::Receipt,
            Some(7),
            None,
            None,
            &[],
            &[MoneyLeg {
                from: Account::SocietyDue(1),
                to: Account::Received("微信".into()),
                amount: Money::from_cents(9000),
            }],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 7);

        let mut tx = pool.begin().await.unwrap();
        let n = reverse_order_journals(&mut tx, 7, Some("取消订单"))
            .await
            .unwrap();
        tx.commit().await.unwrap();

        assert_eq!(n, 2, "该订单名下两条 journal 都要被冲正");
        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 10, "货必须回来");
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into()))
                .await
                .unwrap(),
            Money::ZERO,
            "钱也必须回去——只回滚一半是这个模型最该防住的事故"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1))
                .await
                .unwrap(),
            Money::ZERO
        );
    }

    #[tokio::test]
    async fn reversing_an_order_twice_is_a_no_op_not_a_double_refund() {
        // 幂等性：第二次调用必须一条都不冲（候选集已空），而不是把库存再退一遍。
        // 没有这条，一次网络重试就能让账面凭空多出一批货。
        let pool = test_pool().await;
        let (event_id, ep_a, _) = fixture(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        post_journal(
            &mut tx,
            event_id,
            JournalKind::Restock,
            None,
            None,
            None,
            &[StockLeg {
                event_product_id: ep_a,
                from: Location::External,
                to: Location::OnSite,
                qty: 10,
            }],
            &[],
        )
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO orders (id, event_id, status, gross_amount, solved_amount, final_amount)
                     VALUES (8, ?, 'pending', 6000, 6000, 6000)",
        )
        .bind(event_id)
        .execute(&mut *tx)
        .await
        .unwrap();
        post_journal(
            &mut tx,
            event_id,
            JournalKind::Sale,
            Some(8),
            None,
            None,
            &[StockLeg {
                event_product_id: ep_a,
                from: Location::OnSite,
                to: Location::Customer,
                qty: 2,
            }],
            &[],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        assert_eq!(reverse_order_journals(&mut tx, 8, None).await.unwrap(), 1);
        tx.commit().await.unwrap();
        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 10);

        let mut tx = pool.begin().await.unwrap();
        let second = reverse_order_journals(&mut tx, 8, None).await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(second, 0, "第二次必须一条都不冲");
        assert_eq!(
            onsite_balance(&pool, ep_a).await.unwrap(),
            10,
            "库存只能退一次"
        );
    }

    #[tokio::test]
    async fn the_test_database_enforces_foreign_keys() {
        // post_journal 的 order_id / reverses 相邻且同为 Option<i64>，传反时的第一道
        // （也是唯一一道自动）防线就是外键。sqlx 默认开 foreign_keys=ON，但这是
        // 依赖库的默认值——哪天它变了、或 test_pool 换了构造方式，这条会立刻红，
        // 而不是让一堆「传错 id」的 bug 悄悄溜过测试。
        let pool = test_pool().await;
        let on: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(on, 1, "测试库必须开启外键约束");

        let violated = sqlx::query(
            "INSERT INTO journals (event_id, kind, reverses_journal_id) VALUES (1, '取消', 999999)",
        )
        .execute(&pool)
        .await;
        assert!(
            violated.is_err(),
            "指向不存在 journal 的 reverses_journal_id 必须被外键拒绝"
        );
    }
}
