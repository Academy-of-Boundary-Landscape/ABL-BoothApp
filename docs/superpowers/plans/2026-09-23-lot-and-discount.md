# ②-2 Lot 与最优折扣求解器 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把「套装（Lot）」和「最优折扣」做成一级实体——摊主按展会配 Lot，顾客购物车实时看到折后价与优惠明细，摊主收款时可以一口价改总价，三个价（原价 / 求解器价 / 实收）与两个行金额（货主应得 / 顾客实付）在账本上全程闭合。

**Architecture:** 求解器是纯函数（`domain/solver.rs`），本质是加权集合覆盖，靠「只处理第一个还有剩余的商品」做状态规范化、靠记忆化避免重复子问题、靠三条硬上限防住公开未鉴权端点上的指数爆炸。分摊（`domain/allocation.rs`）也是纯函数，取整余数按固定顺序逐分派发，保证 `Σ 各行 = 总价` 且任何一行都不会被摊成负数。两者由 `domain/pricing.rs` 组合起来，**下单与报价共用同一份实现**，所以购物车看到的价和实际算出的价不可能分叉。手工覆盖并进「确认收款」这一步，落账时在按货主分组的资金腿之外追加一条手工折让腿，整笔落在本社团头上。

**Tech Stack:** Rust 1.98.1 / axum 0.7 / sqlx 0.9（sqlite，运行时 `migrate!()`，无编译期宏）/ tower 0.4 / Vue 3 + Pinia + Vite / vitest / naive-ui

**Spec:** `docs/superpowers/specs/2026-09-22-double-entry-domain-design.md`（2026-09-23 修订版，4.1–4.5）
**Umbrella:** `docs/superpowers/specs/2026-09-22-v1.2-roadmap.md`（含 ① 执行后的附录）
**前一份 plan:** `docs/superpowers/plans/2026-09-22-double-entry-ledger-core.md`（②-1，含「交给 ②-2 / ②-3 的接口契约与已知不变量」一节，**动手前必读**）

---

## 这份 plan 在 ② 里的位置

| plan | 内容 | 状态 |
|---|---|---|
| ②-1 | 全量新 schema、账本内核、社团与归属、进货、下单·收款·取消、前端接线 | 已完成（`1.2-dev`） |
| **②-2（本文）** | Lot、最优折扣求解器、分摊（`allocated`/`paid`）、手工覆盖、混货主规则 + 配置与购物车 UI | 本次执行 |
| ②-3 | 补货之外的展会闭环（盘点/带回/冻结/清 pending）、赠品损耗、退货 UI、垫付调整、结算单与导出 | 之后 |

**不需要新迁移。** `lots` / `lot_candidates` / `order_lots` 三张表和 `order_lines` 的 `order_lot_id` / `allocated_amount` / `paid_amount`、`orders` 的 `gross_amount` / `solved_amount` / `final_amount` 在 ②-1 的 Task 2 就全部建好了。本 plan 只往里写数据，**一行 DDL 都不加**。

---

## Global Constraints

- **分支**：全部工作在 `1.2-dev` 上。不要合并到 `main`。
- **本机构建必须走 `tauri-env`**（见 `CLAUDE.md`）：`tauri-env linux cargo <...>`。裸跑 `cargo` 会因为缺 webkit2gtk 失败。
- **`cargo` 命令一律在 `src-tauri/` 下跑**。仓库根没有 `Cargo.toml`。
- **rustc 钉死 1.98.1**（`rust-toolchain.toml`）。
- **CI 的三条 Rust 门禁必须全绿**（`.github/workflows/ci.yml` 的 rust job，`working-directory: src-tauri`）：
  ```
  cargo fmt --all --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --all-features
  ```
  注意 clippy 带 `--all-targets`，**测试代码也吃 `-D warnings`**。
- **前端门禁**：`npm run lint --prefix frontend`（eslint `--max-warnings 0`）、`npm run format:check --prefix frontend`、`npm run test:unit --prefix frontend`、`npm run build --prefix frontend`。
- **金额一律整数分**（`Money(i64)`）。数据库列是 `INTEGER`，API 传的 JSON 是整数分，前端显示时才除以 100。**任何地方不得出现 `f64` 表示金额**（`stats.rs` 里 CSV/xlsx 的显示层换算是既有例外，别动）。
- **错误响应体必须保持 `{"error": "..."}` 形状**。`frontend/src/services/api.js` 全仓在读 `err.response?.data?.error`。
- **全仓没有编译期 sqlx 宏**，所以 **SQL 写错不会引发编译错误，只会在运行时炸**。每一条新 SQL 都必须有测试覆盖。
- **不引入新的依赖**，Rust 和 npm 两侧都是。求解器和分摊都是手写纯函数，property test 用一个 3 行的 LCG 生成输入，不上 `proptest`。
- **注释写中文，解释 why 而不是 what**，匹配仓库既有风格（`//!` 模块级说明设计取舍、`///` 说明为什么必须这么做、行内注释记录踩过的坑和「改这里要同步改哪里」）。
- **不碰这两个文件**：`my-release-key.jks`、`src-tauri/updater-key.key`。
- **不得让复式记账制造出「钱已到账」的错觉**（spec 第 11 节第 2 条）。本 plan 把「确认收款」弹窗从「只选渠道」扩成「还能改金额」，它**看起来**更像在处理真钱——`ChannelPicker.vue` 现有的那句「这只是记账。请先确认手机上真的收到了到账提示，再点确认。」必须原样保留到新弹窗里，不是可选的润色。
- **不做**（留给 ②-3）：退货、盘点、带回、冻结、清 pending 阻断、赠品与损耗的写入路径、拆封/转换、垫付、结算调整、结算单、导出重做、自定义渠道。
- **明确不做**（spec 第 10 节）：**手工指定哪几件进哪个 Lot**、前端本地求解、预算内尽力搜的求解器。
  注意「**拆掉整个 Lot 实例**」是**要做**的（Task 8）——spec 4.3 于 2026-09-23 推翻了先前「辅助通道不做」的判定。
- 提交信息用仓库现有的 gitmoji 风格，结尾带
  `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>`。

### 三条本 plan 期间知情接受的状态

1. **Task 6 之前，Lot 配好了也不生效。** Task 4/5 让 Lot 能配、能报价，但下单仍然按原价走（②-1 的 `create_order` 原样）。这是刻意的切法：求解器先独立可测，接线才动交易路径。
   同理 **Task 8 之前没有「拆套装」这条出口**，摊主只能靠改实收——而那在代卖货上会把钱记错货主（Task 8 开头有完整说明）。两者之间不要发版。
2. **`cargo test --no-default-features` 本来就是坏的**（`test_support.rs` 无条件 `use crate::vision::VisionRuntime`）。这是 ① 留下的腐烂，**不在本 plan 范围内**，不要顺手修。CI 跑的是 `--all-features`。
3. **`events.status` 的写入守卫仍然不全。** ②-1 交接段列了 4 个「当前不查 `events.status`」的敞口留给 ②-3。本 plan 给**新增**的 Lot 写入路径补上守卫（Task 4），但不去补那 4 个既有的——那是 ②-3 的冻结语义的一部分，单独改一半反而会让 ②-3 误以为已经做过了。

---

## File Structure

**新建**

| 文件 | 职责 |
|---|---|
| `src-tauri/src/domain/solver.rs` | 折扣求解器。纯函数，不碰 DB/网络/Tauri |
| `src-tauri/src/domain/allocation.rs` | 分摊与取整。纯函数 |
| `src-tauri/src/domain/pricing.rs` | 把求解器接到 DB：合并购物车、查商品、读 Lot、组合出每行的归属与金额 |
| `src-tauri/src/api/lot.rs` | Lot CRUD + `POST /events/:id/quote` |
| `frontend/src/stores/lotStore.js` | Lot 列表与增删改 |
| `frontend/src/views/AdminEventLots.vue` | 套装与优惠配置页（最小实现） |
| `frontend/src/components/vendor/ReceiptModal.vue` | 收款确认弹窗（取代 `ChannelPicker.vue`） |

**修改**

| 文件 | 改什么 |
|---|---|
| `src-tauri/src/domain/mod.rs` | 挂三个新模块 |
| `src-tauri/src/api/mod.rs` | 挂 `lot::router()` |
| `src-tauri/src/api/guard.rs` | 收口 `check_read_permission` / `check_write_permission`（现在 `order.rs` 和 `product.rs` 各有一份一模一样的） |
| `src-tauri/src/api/order.rs` | 删本地权限函数改用 guard；`create_order` 接求解器；`update_order_status` 接 `final_amount` 与 `unapply_lot_ids`；响应带上套装实例 |
| `src-tauri/src/api/product.rs` | 删本地权限函数改用 guard |
| `src-tauri/src/api/stats.rs` | 三处 `SUM(ol.unit_price * ol.qty)` → `SUM(ol.paid_amount)` |
| `src-tauri/src/test_support.rs` | 加 `seed_lot()` 夹具 |
| `frontend/src/stores/customerStore.js` | 接 `/quote`，debounce + 请求序号 |
| `frontend/src/stores/orderStore.js` | `markOrderAsCompleted` 带 `final_amount` |
| `frontend/src/components/customer/ShoppingCart.vue` | 三行结算区：原价 / 优惠明细 / 应付 |
| `frontend/src/views/CustomerView.vue` | 传新 props；付款金额改用下单响应的 `final_amount` |
| `frontend/src/views/VendorView.vue` | 换 `ReceiptModal`，传三个价 |
| `frontend/src/components/order/OrderCard.vue` | 行上显示所属套装；页脚显示原价 → 实收 |
| `frontend/src/router/index.js` | 加 `events/:id/lots` 路由 |
| `frontend/src/views/AdminLayout.vue` | 侧栏展会组加「套装与优惠」 |
| `frontend/src/views/Help.vue` | 第 374 行的「负价格假商品」改写 |
| `docs/guide/workflow.md` | 第 44 行的「负价格套装折扣」改写 |

**删除**

| 文件 | 为什么 |
|---|---|
| `frontend/src/components/vendor/ChannelPicker.vue` | 被 `ReceiptModal.vue` 取代（Task 12） |

---

## Task 1: 折扣求解器（纯函数）

**Files:**
- Create: `src-tauri/src/domain/solver.rs`
- Modify: `src-tauri/src/domain/mod.rs`
- Test: 同文件的 `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `crate::domain::money::Money`、`crate::error::{ApiError, ApiResult}`（已存在）
- Produces:
  ```rust
  pub struct CartLine { pub event_product_id: i64, pub qty: i64, pub unit_price: Money }
  pub struct LotDef { pub id: i64, pub name: String, pub pick_count: i64, pub total_price: Money, pub candidates: Vec<i64> }
  pub struct SolvedLot { pub lot_id: i64, pub name: String, pub price: Money, pub members: Vec<(i64, i64)> }
  pub struct Solution { pub lots: Vec<SolvedLot>, pub leftovers: Vec<(i64, i64)>, pub solved_total: Money }
  pub fn solve(cart: &[CartLine], lots: &[LotDef]) -> ApiResult<Solution>
  ```
  `SolvedLot::members` 与 `Solution::leftovers` 都是 `(event_product_id, qty)`，**按 `event_product_id` 升序**。`Solution::lots` 的顺序是求解器拆解的顺序，**同输入必然同顺序**。

- [ ] **Step 1: 挂模块**

`src-tauri/src/domain/mod.rs` 里 `pub mod ledger;` 之前加一行（按字母序）：

```rust
pub mod allocation;
pub mod ledger;
pub mod money;
pub mod pricing;
pub mod solver;
```

**注意**：`allocation` / `pricing` 要到 Task 2 / Task 3 才建。本 step 只加 `pub mod solver;` 这一行，另外两行等对应 task 再加，否则编译不过。

- [ ] **Step 2: 写失败的测试**

新建 `src-tauri/src/domain/solver.rs`，只放测试（实现留到 Step 4）：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// 造一条购物车行。单价单位是分。
    fn line(id: i64, qty: i64, cents: i64) -> CartLine {
        CartLine { event_product_id: id, qty, unit_price: Money::from_cents(cents) }
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
        let cart = [line(1, 1, 3000), line(2, 1, 3500), line(3, 1, 4000), line(4, 1, 4500)];
        let lots = [lot(7, 3, 10000, &[1, 2, 3, 4])];
        let sol = solve(&cart, &lots).unwrap();
        assert_eq!(sol.solved_total, Money::from_cents(13000), "100 + 剩下最便宜的 30");
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
```

- [ ] **Step 3: 跑测试确认失败**

Run: `cd src-tauri && tauri-env linux cargo test --all-features domain::solver`
Expected: 编译失败，`cannot find type CartLine` / `cannot find function solve`。

- [ ] **Step 4: 写实现**

在 `src-tauri/src/domain/solver.rs` 的测试模块**之前**插入：

```rust
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
    Lot { lot: usize, picks: Vec<(usize, i64)> },
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
        enumerate_picks(positions, state, need - take, cursor + 1, current, out, steps)?;
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
        let mut best_cost = self.prices[first] + rest;
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
                let cost = self.lots[li].total_price + rest;
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
    let in_cart: HashMap<i64, &CartLine> =
        cart.iter().map(|l| (l.event_product_id, l)).collect();

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
        return Ok(Solution { lots: Vec::new(), leftovers, solved_total: fixed });
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

    let prices: Vec<Money> = participating.iter().map(|id| in_cart[id].unit_price).collect();
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
    loop {
        let choice = match search.memo.get(&state) {
            Some((_, c)) => c.clone(),
            None => break, // 只有全零状态查不到（best 对它提前返回，不写 memo）
        };
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
        solved_total = solved_total.checked_add_money(l.price).ok_or_else(overflow)?;
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

    Ok(Solution { lots: solved_lots, leftovers, solved_total })
}
```

**`Money` 要补一个方法**（`src-tauri/src/domain/money.rs`，紧挨 `checked_mul_qty`）：

```rust
    /// 加法的溢出安全版本。求解器的输入来自公开端点，`Add` 的实现会在
    /// debug 下 panic、release 下回绕，两个都不能接受。
    pub fn checked_add_money(self, rhs: Money) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Money)
    }
```

同时把 `checked_mul_qty` 上的 `#[allow(dead_code)]` 删掉——本 task 之后它有真实调用方了。

- [ ] **Step 5: 跑测试确认通过**

Run: `cd src-tauri && tauri-env linux cargo test --all-features domain::solver`
Expected: 10 个测试全绿。

- [ ] **Step 6: 过门禁**

```bash
cd src-tauri
tauri-env linux cargo fmt --all
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
```
Expected: clippy 零警告；全量测试绿。

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/domain/solver.rs src-tauri/src/domain/mod.rs src-tauri/src/domain/money.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 折扣求解器——纯函数、状态规范化、三条硬上限

spec 4.2。本质是加权集合覆盖，难点不在求最小值而在「替顾客挑哪几件
进 Lot」：任选 3 本 100 而车里有 30/35/40/45 时，放最贵的三本省 20，
放最便宜的三本只省 5。

两条让它在摊位规模下够快：每一步只处理「第一个还有剩余的商品」，
把「先卖 A 再卖 B」和「先卖 B 再卖 A」这类排列重复砍掉；剩下的靠记忆化。

三条上限不是优化是安全要求——入口是公开未鉴权端点，而 Tauri 进程
和 UI 是同一个进程。MAX_UNITS 还兼着限递归深度：少了它，
「一个商品买 20 万件」的状态空间恰好卡在上限之内，然后把栈撑爆。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: 分摊与取整（纯函数）

**Files:**
- Create: `src-tauri/src/domain/allocation.rs`
- Modify: `src-tauri/src/domain/mod.rs`（加 `pub mod allocation;`）
- Test: 同文件的 `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `crate::domain::money::Money`、`crate::error::{ApiError, ApiResult}`
- Produces:
  ```rust
  pub fn allocate_lot(lot_price: Money, weights: &[i64], fallback: &[i64]) -> ApiResult<Vec<Money>>
  pub fn apply_manual_adjustment(allocated: &[Money], qty: &[i64], solved: Money, final_amount: Money) -> ApiResult<Vec<Money>>
  ```
  两者返回的 `Vec` 与入参同长同序。`allocate_lot` 的 `weights` 是各成分的 `unit_price × qty`，`fallback` 是各成分的 `qty`（权重全零时用，比如整组都是 0 元赠品）。

- [ ] **Step 1: 写失败的测试**

新建 `src-tauri/src/domain/allocation.rs`，只放测试：

```rust
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
        let out = apply_manual_adjustment(&allocated, &[2, 1], Money::from_cents(8000), Money::from_cents(8000)).unwrap();
        assert_eq!(out, allocated);
    }

    #[test]
    fn a_discount_is_spread_by_allocated_amount() {
        // 原价 80，摊主算 70：10 元折让按 6:2 摊成 7.5 / 2.5
        let allocated = cents(&[6000, 2000]);
        let out = apply_manual_adjustment(&allocated, &[2, 1], Money::from_cents(8000), Money::from_cents(7000)).unwrap();
        assert_eq!(out.iter().copied().sum::<Money>(), Money::from_cents(7000));
        assert_eq!(out, cents(&[5250, 1750]));
    }

    #[test]
    fn a_markup_is_spread_the_same_way() {
        // spec 4.3 明说允许改高
        let allocated = cents(&[6000, 2000]);
        let out = apply_manual_adjustment(&allocated, &[2, 1], Money::from_cents(8000), Money::from_cents(9000)).unwrap();
        assert_eq!(out.iter().copied().sum::<Money>(), Money::from_cents(9000));
        assert_eq!(out, cents(&[6750, 2250]));
    }

    #[test]
    fn the_remainder_never_pushes_a_line_below_zero() {
        // spec 4.5 第 3 条 2026-09-23 修正的那个反例：3 行各 1 元，抹成 0.01 元。
        // 原规则「余数全给 allocated 最大的行」会算出 paid = −1，撞 CHECK 变 500。
        let allocated = cents(&[100, 100, 100]);
        let out = apply_manual_adjustment(&allocated, &[1, 1, 1], Money::from_cents(300), Money::from_cents(1)).unwrap();
        assert_eq!(out.iter().copied().sum::<Money>(), Money::from_cents(1));
        assert!(out.iter().all(|m| !m.is_negative()), "任何一行都不能被摊成负数");
        assert_eq!(out, cents(&[0, 0, 1]));
    }

    #[test]
    fn giving_the_whole_order_away_zeroes_every_line() {
        let allocated = cents(&[6000, 2000]);
        let out = apply_manual_adjustment(&allocated, &[2, 1], Money::from_cents(8000), Money::ZERO).unwrap();
        assert_eq!(out, cents(&[0, 0]));
    }

    #[test]
    fn an_all_gift_order_can_still_be_marked_up() {
        // spec 4.5 的退化情形：Σ allocated = 0 时按 allocated 的权重全是 0，退回按 qty 分
        let allocated = cents(&[0, 0]);
        let out = apply_manual_adjustment(&allocated, &[1, 2], Money::ZERO, Money::from_cents(300)).unwrap();
        assert_eq!(out, cents(&[100, 200]));
    }

    #[test]
    fn a_negative_final_amount_is_refused() {
        let allocated = cents(&[6000]);
        let err = apply_manual_adjustment(&allocated, &[1], Money::from_cents(6000), Money::from_cents(-1)).unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn the_parts_always_add_up_to_the_whole() {
        // spec 第 9 节要求的 property test。不引入 proptest——一个 LCG 就够，
        // 而且种子固定，失败时可以原样复现。
        let mut seed: u64 = 0x5eed_1234_abcd_ef01;
        let mut next = move || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as i64
        };
        for _ in 0..2000 {
            let n = (next() % 8 + 1) as usize;
            let allocated: Vec<Money> = (0..n).map(|_| Money::from_cents(next() % 100_000)).collect();
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
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd src-tauri && tauri-env linux cargo test --all-features domain::allocation`
Expected: 编译失败，`cannot find function allocate_lot`。

- [ ] **Step 3: 写实现**

在测试模块之前插入：

```rust
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
fn apportion(total: i64, weights: &[i64], caps: Option<&[i64]>) -> ApiResult<Vec<i64>> {
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
```

`src-tauri/src/domain/mod.rs` 加一行 `pub mod allocation;`。

- [ ] **Step 4: 跑测试确认通过**

Run: `cd src-tauri && tauri-env linux cargo test --all-features domain::allocation`
Expected: 11 个测试全绿，包含 2000 轮的 property test。

- [ ] **Step 5: 过门禁**

```bash
cd src-tauri
tauri-env linux cargo fmt --all
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
```

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/domain/allocation.rs src-tauri/src/domain/mod.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 分摊与取整——并修掉 spec 4.5 会算出负数的取整规则

spec 4.5 原文是「余数全部加到 allocated 最大的那一行」。反例：3 行各 1 元，
摊主把 3 元抹成 0.01 元，各行 floor(299×100/300)=99、余数 2 全给最大行 →
那行分到 101 而它只有 100 可让，paid = −1，撞 CHECK (paid_amount >= 0) 变 500。

改成按「权重降序、下标升序」逐分派发、跳过顶到上限的那一份。总余量恒 ≥ 余数，
所以永远派得完；顺序固定所以同输入同输出（spec 4.5 明文要求）。
spec 已于本轮同步修订。

property test 用一个固定种子的 LCG 跑 2000 组，断言 Σ paid == final
且没有一行是负数——不引入 proptest 依赖。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: `pricing` —— 把求解器接到数据库

**Files:**
- Create: `src-tauri/src/domain/pricing.rs`
- Modify: `src-tauri/src/domain/mod.rs`（加 `pub mod pricing;`）、`src-tauri/src/test_support.rs`（加 `seed_lot`）
- Test: 同文件的 `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: Task 1 的 `solver::{solve, CartLine, LotDef, SolvedLot}`、Task 2 的 `allocation::allocate_lot`
- Produces:
  ```rust
  pub struct CartItemRequest { pub product_id: i64, pub quantity: i64 }     // Deserialize
  pub struct CartProduct { pub id: i64, pub unit_price: i64, pub name: String, pub image_url: Option<String> }
  pub struct ResolvedLine { pub product: CartProduct, pub qty: i64 }
  pub struct PricedLine { pub event_product_id: i64, pub qty: i64, pub unit_price: Money, pub lot_index: Option<usize>, pub allocated: Money }
  pub struct PricedCart { pub lots: Vec<SolvedLot>, pub lines: Vec<PricedLine>, pub gross: Money, pub solved: Money }

  pub fn merge_items(items: &[CartItemRequest]) -> ApiResult<Vec<(i64, i64)>>
  pub async fn resolve_cart(conn: &mut SqliteConnection, event_id: i64, merged: &[(i64, i64)]) -> ApiResult<Vec<ResolvedLine>>
  pub async fn load_lots(conn: &mut SqliteConnection, event_id: i64) -> ApiResult<Vec<LotDef>>
  pub fn price_cart(cart: &[CartLine], lots: &[LotDef]) -> ApiResult<PricedCart>
  ```
  `PricedCart::lines` 的顺序：Lot 内的行在前（按 Lot 实例顺序、实例内按商品 id 升序），散行在后（按商品 id 升序）。**Task 6 按这个顺序写 `order_lines`，所以它是契约不是巧合。**

- [ ] **Step 1: 加夹具**

`src-tauri/src/test_support.rs` 末尾追加：

```rust
/// 给展会加一个 Lot，返回 lot_id。`candidates` 是 `event_product_id` 列表。
///
/// 直接写 SQL 而不是打 API：Lot 的 CRUD 是 Task 4 才有的东西，而 Task 3 的
/// 测试现在就要用它。
pub async fn seed_lot(
    pool: &SqlitePool,
    event_id: i64,
    name: &str,
    pick_count: i64,
    total_price: i64,
    candidates: &[i64],
) -> i64 {
    let lot_id: i64 = sqlx::query_scalar(
        "INSERT INTO lots (event_id, name, pick_count, total_price) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(event_id)
    .bind(name)
    .bind(pick_count)
    .bind(total_price)
    .fetch_one(pool)
    .await
    .expect("seed lot");
    for c in candidates {
        sqlx::query("INSERT INTO lot_candidates (lot_id, event_product_id) VALUES (?, ?)")
            .bind(lot_id)
            .bind(c)
            .execute(pool)
            .await
            .expect("seed lot candidate");
    }
    lot_id
}
```

- [ ] **Step 2: 写失败的测试**

新建 `src-tauri/src/domain/pricing.rs`，只放测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{seed_event_and_product, seed_lot, test_pool};

    fn req(product_id: i64, quantity: i64) -> CartItemRequest {
        CartItemRequest { product_id, quantity }
    }

    fn line(id: i64, qty: i64, cents: i64) -> CartLine {
        CartLine { event_product_id: id, qty, unit_price: Money::from_cents(cents) }
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

        assert!(resolve_cart(&mut conn, event_id, &[(ep_a, 1)]).await.is_ok());
        let err = resolve_cart(&mut conn, 999, &[(ep_a, 1)]).await.unwrap_err();
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
            id: 7, name: "两本合购".into(), pick_count: 2,
            total_price: Money::from_cents(4000), candidates: vec![1, 2],
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
            id: 7, name: "任选2件60".into(), pick_count: 2,
            total_price: Money::from_cents(6000), candidates: vec![1],
        }];
        let priced = price_cart(&cart, &lots).unwrap();
        assert_eq!(priced.gross, Money::from_cents(12000));
        assert_eq!(priced.solved, Money::from_cents(10000));
        assert_eq!(priced.lines.len(), 2, "同一个商品拆成了两行");

        let in_lot: Vec<&PricedLine> = priced.lines.iter().filter(|l| l.lot_index.is_some()).collect();
        let loose: Vec<&PricedLine> = priced.lines.iter().filter(|l| l.lot_index.is_none()).collect();
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
        let cart = [line(1, 1, 3000), line(2, 1, 3500), line(3, 1, 4000), line(4, 2, 1700)];
        let lots = [LotDef {
            id: 7, name: "任选3件100".into(), pick_count: 3,
            total_price: Money::from_cents(10000), candidates: vec![1, 2, 3, 4],
        }];
        let priced = price_cart(&cart, &lots).unwrap();
        let sum: Money = priced.lines.iter().map(|l| l.allocated).sum();
        assert_eq!(sum, priced.solved);
        // 每一件货都有去向：各行 qty 之和 == 购物车件数之和
        let qty: i64 = priced.lines.iter().map(|l| l.qty).sum();
        assert_eq!(qty, 5);
    }
}
```

- [ ] **Step 3: 跑测试确认失败**

Run: `cd src-tauri && tauri-env linux cargo test --all-features domain::pricing`
Expected: 编译失败，`cannot find function merge_items`。

- [ ] **Step 4: 写实现**

在测试模块之前插入：

```rust
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
    pub name: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedLine {
    pub product: CartProduct,
    pub qty: i64,
}

/// 查 `event_products`，把 `(product_id, qty)` 变成带商品快照的行，顺序与 `merged` 一致。
pub async fn resolve_cart(
    conn: &mut SqliteConnection,
    event_id: i64,
    merged: &[(i64, i64)],
) -> ApiResult<Vec<ResolvedLine>> {
    let mut out = Vec::with_capacity(merged.len());
    for (product_id, qty) in merged {
        let product: CartProduct = sqlx::query_as(
            "SELECT ep.id, ep.unit_price, ep.name, mp.image_url
             FROM event_products ep
             JOIN master_products mp ON mp.id = ep.master_product_id
             WHERE ep.id = ? AND ep.event_id = ?",
        )
        .bind(*product_id)
        .bind(event_id)
        .fetch_optional(&mut *conn)
        .await?
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
            weights.push(price_of[id].checked_mul_qty(*q).ok_or_else(overflow)?.cents());
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

    Ok(PricedCart { lots: solution.lots, lines, gross, solved })
}
```

`src-tauri/src/domain/mod.rs` 加一行 `pub mod pricing;`。

- [ ] **Step 5: 跑测试确认通过**

Run: `cd src-tauri && tauri-env linux cargo test --all-features domain::pricing`
Expected: 8 个测试全绿。

- [ ] **Step 6: 过门禁并提交**

```bash
cd src-tauri
tauri-env linux cargo fmt --all
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
cd ..
git add src-tauri/src/domain/pricing.rs src-tauri/src/domain/mod.rs src-tauri/src/test_support.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: pricing 层——下单与报价共用同一份求解 + 分摊

购物车显示的价和真正落账的价不可能分叉，理由就是这一层：两条路径跑的是
同一份代码，而不是两份需要人去保持同步的实现。

里面有一件 spec 没写、但 order_lines 的形状逼出来的事：**订单行必须按 Lot
归属拆分**。order_lot_id 是每行一个可空外键，而求解器完全可能把「某商品
3 件」里的 2 件放进套装、1 件留在外面——一行里表达不了两种归属。于是同一个
商品在一张订单里会出现两次。spec 4.5 已同步补上这一条。

PricedCart::lines 的顺序（Lot 内的行在前、散行在后）是 Task 6 写 order_lines
时依赖的契约，不是巧合。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Lot CRUD + 权限检查收口

**Files:**
- Create: `src-tauri/src/api/lot.rs`
- Modify: `src-tauri/src/api/guard.rs`、`src-tauri/src/api/order.rs:517-531`（删本地权限函数）、`src-tauri/src/api/product.rs:44-62`（同上）、`src-tauri/src/api/mod.rs`
- Test: `src-tauri/src/api/lot.rs` 的 `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: Task 3 的 `pricing::load_lots`（**不在本 task 用**，Task 5 才用）
- Produces:
  ```rust
  // api/guard.rs
  pub fn check_read_permission(claims: &Claims, event_id: i64) -> ApiResult<()>
  pub fn check_write_permission(claims: &Claims, event_id: i64) -> ApiResult<()>
  // api/lot.rs
  pub fn router() -> Router<AppState>
  ```
  路由：`GET|POST /api/events/:event_id/lots`、`PUT|DELETE /api/events/:event_id/lots/:lot_id`。
  `LotResponse` 字段：`id, event_id, name, pick_count, total_price(分), candidate_ids, owner_society_id, owner_society_name`。

- [ ] **Step 1: 把权限检查收进 guard**

`src-tauri/src/api/guard.rs` 末尾追加（`AdminOnly` 之后）：

```rust
/// 读权限：admin 全通；vendor + `access = "all"` 全通；vendor + `access = "event"`
/// 只能碰 token 里钉着的那一场。
///
/// **这份实现原本在 `api/order.rs` 和 `api/product.rs` 里各抄了一份一模一样的**，
/// `api/lot.rs` 会是第三份。收到这里来，三处共用。
pub fn check_read_permission(claims: &Claims, event_id: i64) -> ApiResult<()> {
    if claims.role == "admin" {
        return Ok(());
    }
    if claims.role == "vendor" {
        if claims.access == "all" {
            return Ok(());
        }
        if let Some(eid) = claims.event_id {
            if eid == event_id {
                return Ok(());
            }
        }
    }
    Err(ApiError::Forbidden)
}

/// 写权限目前与读权限一致。保留两个名字是因为调用点读起来意图不同，
/// 将来要收紧写权限时也有地方下手。
pub fn check_write_permission(claims: &Claims, event_id: i64) -> ApiResult<()> {
    check_read_permission(claims, event_id)
}
```

`guard.rs` 顶部补 `use crate::error::{ApiError, ApiResult};`（若尚未 import）。

然后：
- `src-tauri/src/api/order.rs`：删掉文件末尾「权限检查辅助函数」整节（`check_read_permission` / `check_write_permission` 两个 fn），把 `use` 里加上 `crate::api::guard::{check_read_permission, check_write_permission}`。
- `src-tauri/src/api/product.rs`：删掉 `check_write_permission`，`use` 里加上 `crate::api::guard::check_write_permission`。
- 两个文件的调用点一个字都不用改（函数名与签名一致，只是返回 `ApiResult<()>` 而不是 `Result<(), ApiError>`——**这两个是同一个类型**）。

- [ ] **Step 2: 跑一遍确认没改坏**

Run: `cd src-tauri && tauri-env linux cargo test --all-features`
Expected: 全绿，测试数与改动前一致。

- [ ] **Step 3: 写失败的测试**

新建 `src-tauri/src/api/lot.rs`，只放测试：

```rust
#[cfg(test)]
mod tests {
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn creating_a_lot_stores_its_candidates_and_reports_the_owner() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/lots"),
                Some(&token),
                json!({"name": "本子任选1本25", "pick_count": 1, "total_price": 2500,
                       "candidate_ids": [ep_a]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = read_json(res).await;
        assert_eq!(body["candidate_ids"], json!([ep_a]));
        assert_eq!(body["total_price"], 2500, "金额是分，不是元");
        assert_eq!(body["owner_society_id"], 1, "ep_a 归本社团");
    }

    #[tokio::test]
    async fn a_lot_spanning_two_owners_is_refused() {
        // spec 4.1：Lot 是摊主配的，代卖社团没参与这个决定。把他们的本子圈进
        // 「任选 3 本 100」等于替他们让价。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/lots"),
                Some(&token),
                json!({"name": "跨社团套装", "pick_count": 2, "total_price": 4000,
                       "candidate_ids": [ep_a, ep_b]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lots")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "校验失败必须整体回滚，不能留下一个没有候选的 Lot");
    }

    #[tokio::test]
    async fn a_candidate_from_another_event_is_refused() {
        let (router, _dir, pool) = test_router_with().await;
        let (_event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("INSERT INTO events (id, name, event_date, status) VALUES (2, '另一场', '2026-11-01', '进行中')")
            .execute(&pool).await.unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/events/2/lots",
                Some(&token),
                json!({"name": "偷别场的商品", "pick_count": 1, "total_price": 100,
                       "candidate_ids": [ep_a]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn a_settled_event_refuses_new_lots() {
        // ②-1 交接段列了 4 个「当前完全不查 events.status」的既有敞口留给 ②-3。
        // 新增的写入路径不要再添第 5 个。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id).execute(&pool).await.unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/lots"),
                Some(&token),
                json!({"name": "迟到的套装", "pick_count": 1, "total_price": 100,
                       "candidate_ids": [ep_a]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn updating_a_lot_replaces_its_whole_candidate_set() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query(
            "INSERT INTO event_products (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
             VALUES (3, 1, 1, 1, 'C', '本子C', 2500)",
        ).execute(&pool).await.unwrap();
        let token = admin_token();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/lots"), Some(&token),
            json!({"name": "旧", "pick_count": 1, "total_price": 2000, "candidate_ids": [ep_a]}),
        )).await.unwrap();
        let lot_id = read_json(res).await["id"].as_i64().unwrap();

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/lots/{lot_id}"), Some(&token),
            json!({"name": "新", "pick_count": 2, "total_price": 4500, "candidate_ids": [ep_a, 3]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        assert_eq!(body["name"], "新");
        assert_eq!(body["candidate_ids"], json!([ep_a, 3]));

        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lot_candidates WHERE lot_id = ?")
            .bind(lot_id).fetch_one(&pool).await.unwrap();
        assert_eq!(rows, 2, "旧的候选必须被整组替换，不是追加");
    }

    #[tokio::test]
    async fn deleting_a_lot_leaves_past_orders_untouched() {
        // order_lots 存的是名字和价格的**快照**，lot_id 是 ON DELETE SET NULL。
        // 所以删 Lot 永远安全，不需要「被引用就不给删」的守卫。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/lots"), Some(&token),
            json!({"name": "会被删掉的套装", "pick_count": 1, "total_price": 2000, "candidate_ids": [ep_a]}),
        )).await.unwrap();
        let lot_id = read_json(res).await["id"].as_i64().unwrap();

        // 造一张引用了它的历史订单（Task 6 之前 create_order 还不会写 order_lots）
        sqlx::query("INSERT INTO orders (id, event_id, status, gross_amount, solved_amount, final_amount) VALUES (9, ?, 'pending', 3000, 2000, 2000)")
            .bind(event_id).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO order_lots (order_id, lot_id, name, price) VALUES (9, ?, '会被删掉的套装', 2000)")
            .bind(lot_id).execute(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "DELETE", &format!("/api/events/{event_id}/lots/{lot_id}"), Some(&token), json!({}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        let (lot_ref, name, price): (Option<i64>, String, i64) =
            sqlx::query_as("SELECT lot_id, name, price FROM order_lots WHERE order_id = 9")
                .fetch_one(&pool).await.unwrap();
        assert_eq!(lot_ref, None, "外键置空");
        assert_eq!(name, "会被删掉的套装", "名字快照还在");
        assert_eq!(price, 2000, "价格快照还在");
    }

    #[tokio::test]
    async fn listing_lots_needs_a_token() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let res = router
            .clone()
            .oneshot(json_request("GET", &format!("/api/events/{event_id}/lots"), None, json!({})))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
```

- [ ] **Step 4: 跑测试确认失败**

Run: `cd src-tauri && tauri-env linux cargo test --all-features api::lot`
Expected: 全部 404（路由不存在）或编译失败。

- [ ] **Step 5: 写实现**

在测试模块之前插入：

```rust
//! Lot = 候选商品集合 + 要选几件(N) + 总价（spec 4.1）。
//!
//! 「固定成分」是「任选 N」的特例（N = 候选集大小），模型里只有一个概念，不是两套并行。
//!
//! **单品不在这张表里。** 单品在求解器眼里是「候选集只有自己、N = 1、价格就是单价」的
//! 退化 Lot，那是求解器**输入归一化**的事，不是模型的事——管理员眼里仍然是
//! 「改这个商品的价格」（spec 4.1，路线图 D3 的「一切都是 Lot」按这个理解落地）。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;

use crate::{
    api::guard::{check_read_permission, check_write_permission},
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

/// 一个 Lot 最多几个候选商品。防的是「把全场商品一次性圈进来」这种请求，
/// 而不是业务需要——真实套装候选集十几个封顶。
const MAX_CANDIDATES: usize = 200;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events/:event_id/lots", get(list_lots).post(create_lot))
        .route(
            "/events/:event_id/lots/:lot_id",
            put(update_lot).delete(delete_lot),
        )
}

#[derive(Serialize)]
struct LotResponse {
    id: i64,
    event_id: i64,
    name: String,
    pick_count: i64,
    /// 单位：分。
    total_price: i64,
    candidate_ids: Vec<i64>,
    /// 候选集必然同一货主（下面的 `validate_candidates` 保证），所以取其一即可。
    owner_society_id: i64,
    owner_society_name: String,
}

#[derive(Deserialize)]
struct LotPayload {
    name: String,
    pick_count: i64,
    /// 单位：分。
    total_price: i64,
    candidate_ids: Vec<i64>,
}

/// 已结算的展会账已冻结，不能再改定价。
///
/// ②-1 交接段列了 4 个「当前完全不查 `events.status`」的既有敞口留给 ②-3 的冻结语义。
/// **新增的写入路径不要再添第 5 个。**
async fn ensure_event_open(conn: &mut SqliteConnection, event_id: i64) -> ApiResult<()> {
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM events WHERE id = ?")
        .bind(event_id)
        .fetch_optional(&mut *conn)
        .await?;
    match status.as_deref() {
        None => Err(ApiError::NotFound("展会不存在".into())),
        Some("已结算") => Err(ApiError::Conflict("展会已结算，不能再改套装配置".into())),
        Some(_) => Ok(()),
    }
}

/// 校验候选集：非空、去重、全部属于本展会、**全部同一货主**。返回 (去重后的 id, 货主 id, 货主名)。
///
/// 同一货主是 spec 4.1 的硬约束：Lot 是摊主配的，代卖社团一样没参与这个决定，
/// 把他们的本子圈进「任选 3 本 100」等于替他们让价。将来真有两家事先谈好的联合套装，
/// 加一个 per-Lot 的「折让由谁承担」再放开，是纯增量。
async fn validate_candidates(
    conn: &mut SqliteConnection,
    event_id: i64,
    candidate_ids: &[i64],
) -> ApiResult<(Vec<i64>, i64, String)> {
    let mut ids: Vec<i64> = candidate_ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return Err(ApiError::BadRequest("套装至少要有一个候选商品".into()));
    }
    if ids.len() > MAX_CANDIDATES {
        return Err(ApiError::BadRequest("候选商品过多".into()));
    }

    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!(
        "SELECT ep.id, ep.owner_society_id, s.name
         FROM event_products ep
         JOIN societies s ON s.id = ep.owner_society_id
         WHERE ep.event_id = ? AND ep.id IN ({placeholders})"
    );
    let mut q = sqlx::query_as::<_, (i64, i64, String)>(&sql).bind(event_id);
    for id in &ids {
        q = q.bind(*id);
    }
    let rows = q.fetch_all(&mut *conn).await?;

    if rows.len() != ids.len() {
        return Err(ApiError::BadRequest(
            "候选商品不存在或不属于本场展会".into(),
        ));
    }
    let owner = rows[0].1;
    if rows.iter().any(|(_, o, _)| *o != owner) {
        return Err(ApiError::BadRequest(
            "套装的候选商品必须属于同一个货主——替别的社团让价不是摊主能单方面决定的".into(),
        ));
    }
    let owner_name = rows[0].2.clone();
    Ok((ids, owner, owner_name))
}

fn validate_payload(payload: &LotPayload) -> ApiResult<String> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::BadRequest("套装名称不能为空".into()));
    }
    if payload.pick_count <= 0 {
        return Err(ApiError::BadRequest("「要选几件」必须是正数".into()));
    }
    if payload.total_price < 0 {
        return Err(ApiError::BadRequest("套装价格不能为负".into()));
    }
    // 候选集大小**不要求** >= pick_count：「任选 3 本 100」而候选只有一种书，
    // 意思是「买 3 本同款 100」，完全合法。
    Ok(name)
}

/// 把候选商品写进去。更新时先删后写，整组替换。
async fn write_candidates(
    conn: &mut SqliteConnection,
    lot_id: i64,
    ids: &[i64],
) -> ApiResult<()> {
    sqlx::query("DELETE FROM lot_candidates WHERE lot_id = ?")
        .bind(lot_id)
        .execute(&mut *conn)
        .await?;
    for id in ids {
        sqlx::query("INSERT INTO lot_candidates (lot_id, event_product_id) VALUES (?, ?)")
            .bind(lot_id)
            .bind(*id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

async fn list_lots(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<Vec<LotResponse>>> {
    check_read_permission(&claims, event_id)?;

    // 一条 JOIN 查完再在 Rust 里分组，靠 ORDER BY l.id 保证同一个 Lot 的候选连续。
    let rows: Vec<(i64, i64, String, i64, i64, i64, i64, String)> = sqlx::query_as(
        "SELECT l.id, l.event_id, l.name, l.pick_count, l.total_price,
                lc.event_product_id, ep.owner_society_id, s.name
         FROM lots l
         JOIN lot_candidates lc ON lc.lot_id = l.id
         JOIN event_products ep ON ep.id = lc.event_product_id
         JOIN societies s ON s.id = ep.owner_society_id
         WHERE l.event_id = ?
         ORDER BY l.id, lc.event_product_id",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    let mut out: Vec<LotResponse> = Vec::new();
    for (id, ev, name, pick, price, candidate, owner, owner_name) in rows {
        match out.last_mut() {
            Some(last) if last.id == id => last.candidate_ids.push(candidate),
            _ => out.push(LotResponse {
                id,
                event_id: ev,
                name,
                pick_count: pick,
                total_price: price,
                candidate_ids: vec![candidate],
                owner_society_id: owner,
                owner_society_name: owner_name,
            }),
        }
    }
    Ok(Json(out))
}

async fn create_lot(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<LotPayload>,
) -> ApiResult<impl IntoResponse> {
    check_write_permission(&claims, event_id)?;
    let name = validate_payload(&payload)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    ensure_event_open(&mut tx, event_id).await?;
    let (ids, owner, owner_name) =
        validate_candidates(&mut tx, event_id, &payload.candidate_ids).await?;

    let lot_id: i64 = sqlx::query_scalar(
        "INSERT INTO lots (event_id, name, pick_count, total_price)
         VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(event_id)
    .bind(&name)
    .bind(payload.pick_count)
    .bind(payload.total_price)
    .fetch_one(&mut *tx)
    .await?;
    write_candidates(&mut tx, lot_id, &ids).await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(LotResponse {
            id: lot_id,
            event_id,
            name,
            pick_count: payload.pick_count,
            total_price: payload.total_price,
            candidate_ids: ids,
            owner_society_id: owner,
            owner_society_name: owner_name,
        }),
    ))
}

async fn update_lot(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, lot_id)): Path<(i64, i64)>,
    Json(payload): Json<LotPayload>,
) -> ApiResult<Json<LotResponse>> {
    check_write_permission(&claims, event_id)?;
    let name = validate_payload(&payload)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    ensure_event_open(&mut tx, event_id).await?;

    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM lots WHERE id = ? AND event_id = ?")
        .bind(lot_id)
        .bind(event_id)
        .fetch_optional(&mut *tx)
        .await?;
    exists.ok_or_else(|| ApiError::NotFound("套装不存在".into()))?;

    let (ids, owner, owner_name) =
        validate_candidates(&mut tx, event_id, &payload.candidate_ids).await?;

    sqlx::query("UPDATE lots SET name = ?, pick_count = ?, total_price = ? WHERE id = ?")
        .bind(&name)
        .bind(payload.pick_count)
        .bind(payload.total_price)
        .bind(lot_id)
        .execute(&mut *tx)
        .await?;
    write_candidates(&mut tx, lot_id, &ids).await?;
    tx.commit().await?;

    Ok(Json(LotResponse {
        id: lot_id,
        event_id,
        name,
        pick_count: payload.pick_count,
        total_price: payload.total_price,
        candidate_ids: ids,
        owner_society_id: owner,
        owner_society_name: owner_name,
    }))
}

/// 删除永远放行。
///
/// `order_lots.lot_id` 是 `ON DELETE SET NULL`，而名字和价格在下单那一刻就
/// **快照**进了 `order_lots`——历史订单不受影响。所以这里不需要
/// 「被引用就不给删」那种守卫（`api/product.rs` 删商品时要，因为那边没有快照）。
async fn delete_lot(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, lot_id)): Path<(i64, i64)>,
) -> ApiResult<impl IntoResponse> {
    check_write_permission(&claims, event_id)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    ensure_event_open(&mut tx, event_id).await?;
    let affected = sqlx::query("DELETE FROM lots WHERE id = ? AND event_id = ?")
        .bind(lot_id)
        .bind(event_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    tx.commit().await?;

    if affected == 0 {
        return Err(ApiError::NotFound("套装不存在".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}
```

`src-tauri/src/api/mod.rs`：`mod lot;`（按字母序放在 `mod legacy;` 之后），并在 `router()` 里加 `.merge(lot::router())`（放在 `.merge(product::router())` 之后）。

- [ ] **Step 6: 跑测试确认通过**

Run: `cd src-tauri && tauri-env linux cargo test --all-features api::lot`
Expected: 7 个测试全绿。

- [ ] **Step 7: 过门禁并提交**

```bash
cd src-tauri
tauri-env linux cargo fmt --all
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
cd ..
git add src-tauri/src/api/lot.rs src-tauri/src/api/mod.rs src-tauri/src/api/guard.rs src-tauri/src/api/order.rs src-tauri/src/api/product.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: Lot 配置 CRUD——候选集必须同一货主，已结算展会拒改

spec 4.1。候选集同一货主是硬校验而不是建议：Lot 是摊主配的，代卖社团
没参与这个决定，把他们的本子圈进「任选 3 本 100」等于替他们让价。

删除永远放行——order_lots 存的是名字与价格的快照、lot_id 是
ON DELETE SET NULL，历史订单不受影响。这和 api/product.rs 删商品要查
「有没有流水」不一样，那边没有快照。

顺带把 check_read_permission / check_write_permission 收进 api/guard.rs。
order.rs 和 product.rs 里原本各抄了一份一模一样的，lot.rs 会是第三份。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: `/quote` 报价端点

**Files:**
- Modify: `src-tauri/src/api/lot.rs`、`src-tauri/src/domain/pricing.rs`（加 `ensure_event_selling`）
- Test: `src-tauri/src/api/lot.rs` 的测试模块

**Interfaces:**
- Consumes: Task 3 的 `pricing::{merge_items, resolve_cart, cart_lines, load_lots, price_cart, CartItemRequest}`
- Produces:
  - `POST /api/events/:event_id/quote`（**公开、无鉴权、零写入**）
  - 响应形状（前端 Task 11 按这个写）：
    ```json
    { "gross_amount": 19000, "solved_amount": 17000,
      "lots":  [{"lot_id":3, "name":"本子任选3本100", "price":10000, "original_amount":12000,
                 "members":[{"product_id":7,"qty":2},{"product_id":9,"qty":1}]}],
      "lines": [{"product_id":7, "qty":2, "lot_index":0, "allocated_amount":6667}] }
    ```
  - `pub async fn pricing::ensure_event_selling(conn: &mut SqliteConnection, event_id: i64) -> ApiResult<()>`

- [ ] **Step 1: 写失败的测试**

追加到 `src-tauri/src/api/lot.rs` 的 `mod tests`：

```rust
    /// 给 seed 出来的展会加一个只含 ep_a 的「任选 2 件 50」，返回 lot_id。
    async fn seed_pair_lot(router: &axum::Router, event_id: i64, ep_a: i64, token: &str) -> i64 {
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/lots"),
                Some(token),
                json!({"name": "任选2件50", "pick_count": 2, "total_price": 5000,
                       "candidate_ids": [ep_a]}),
            ))
            .await
            .unwrap();
        read_json(res).await["id"].as_i64().unwrap()
    }

    #[tokio::test]
    async fn quoting_without_any_lot_returns_the_gross_price() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/quote"),
                None, // 公开端点
                json!({"items": [{"product_id": ep_a, "quantity": 2},
                                 {"product_id": ep_b, "quantity": 1}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        assert_eq!(body["gross_amount"], 8000);
        assert_eq!(body["solved_amount"], 8000);
        assert_eq!(body["lots"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn quoting_applies_the_best_lot_and_says_how_much_it_saved() {
        // D3：顾客端价格必须可解释。购物车要能写出「已应用：任选2件50 −10.00」，
        // 所以响应里必须同时有套装价和成分原价合计。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let lot_id = seed_pair_lot(&router, event_id, ep_a, &token).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/quote"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 3}]}),
            ))
            .await
            .unwrap();
        let body = read_json(res).await;
        assert_eq!(body["gross_amount"], 9000, "3 × 30");
        assert_eq!(body["solved_amount"], 8000, "套装 50 + 散的 30");
        let lots = body["lots"].as_array().unwrap();
        assert_eq!(lots.len(), 1);
        assert_eq!(lots[0]["lot_id"], lot_id);
        assert_eq!(lots[0]["price"], 5000);
        assert_eq!(lots[0]["original_amount"], 6000, "省了 10 元");
        assert_eq!(lots[0]["members"], json!([{"product_id": ep_a, "qty": 2}]));

        // 同一个商品被拆成两行：2 件在套装里、1 件散着
        let lines = body["lines"].as_array().unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["lot_index"], 0);
        assert_eq!(lines[0]["allocated_amount"], 5000);
        assert!(lines[1]["lot_index"].is_null());
        assert_eq!(lines[1]["allocated_amount"], 3000);
    }

    #[tokio::test]
    async fn quoting_writes_nothing() {
        // 报价是只读的。留下订单或 journal 就是灾难——它是公开未鉴权端点。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool).await.unwrap();

        let _ = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/quote"), None,
            json!({"items": [{"product_id": ep_a, "quantity": 2}]}),
        )).await.unwrap();

        let orders: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders")
            .fetch_one(&pool).await.unwrap();
        let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(orders, 0);
        assert_eq!(after, before);
    }

    #[tokio::test]
    async fn quoting_ignores_stock_because_it_is_only_a_price_preview() {
        // 现场仓只有 10 件，报 50 件的价照样出——库存由下单把关。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/quote"), None,
            json!({"items": [{"product_id": ep_a, "quantity": 50}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(read_json(res).await["gross_amount"], 150_000);
    }

    #[tokio::test]
    async fn quoting_a_product_from_another_event_is_not_found() {
        let (router, _dir, pool) = test_router_with().await;
        let (_event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("INSERT INTO events (id, name, event_date, status) VALUES (2, '另一场', '2026-11-01', '进行中')")
            .execute(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "POST", "/api/events/2/quote", None,
            json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn quoting_a_settled_event_is_refused() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id).execute(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/quote"), None,
            json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd src-tauri && tauri-env linux cargo test --all-features api::lot::tests::quoting`
Expected: 全部 404（`/quote` 路由不存在）。

- [ ] **Step 3: 加 `ensure_event_selling`**

`src-tauri/src/domain/pricing.rs` 追加：

```rust
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
```

- [ ] **Step 4: 加 `/quote` handler**

`src-tauri/src/api/lot.rs`：`use` 里补

```rust
use crate::domain::pricing::{
    cart_lines, ensure_event_selling, load_lots, merge_items, price_cart, resolve_cart,
    CartItemRequest,
};
```

`router()` 加一行（`use axum::routing::post` 也要补进去）：

```rust
        .route("/events/:event_id/quote", post(quote))
```

以及结构与 handler：

```rust
#[derive(Deserialize)]
struct QuoteRequest {
    items: Vec<CartItemRequest>,
}

#[derive(Serialize)]
struct QuoteMember {
    product_id: i64,
    qty: i64,
}

#[derive(Serialize)]
struct QuoteLot {
    lot_id: i64,
    name: String,
    /// 套装价（分）。
    price: i64,
    /// 成分按原价的合计（分）。
    ///
    /// 前端显示「已应用：XX −YY」时 `YY = original_amount − price`。放在后端算，
    /// 是因为前端再做一遍单价乘法就等于把分摊逻辑抄了半份出去。
    original_amount: i64,
    members: Vec<QuoteMember>,
}

#[derive(Serialize)]
struct QuoteLine {
    product_id: i64,
    qty: i64,
    /// 进了 `lots` 数组的第几个，`null` = 没进套装。
    lot_index: Option<usize>,
    allocated_amount: i64,
}

#[derive(Serialize)]
struct QuoteResponse {
    gross_amount: i64,
    solved_amount: i64,
    lots: Vec<QuoteLot>,
    lines: Vec<QuoteLine>,
}

/// 给购物车报价。**公开、无鉴权、零写入。**
///
/// 存在的唯一理由是 D3「顾客端价格必须可解释」：购物车要明写
/// 「已应用：本子任选3本100 −20.00」，而不是默默给个低价。
///
/// **不查库存**——报价是定价预览，库存由下单把关（所以这里也不必开事务）。
///
/// **报价永远不被信任**：下单时服务端用同一份 `price_cart` 独立重算，顾客最终付的
/// 金额取自**下单响应**而不是这里。两次之间摊主完全可能刚改过 Lot 配置。
async fn quote(
    State(state): State<AppState>,
    Path(event_id): Path<i64>,
    Json(payload): Json<QuoteRequest>,
) -> ApiResult<Json<QuoteResponse>> {
    let merged = merge_items(&payload.items)?;
    let mut conn = state.db.acquire().await?;
    ensure_event_selling(&mut conn, event_id).await?;
    let resolved = resolve_cart(&mut conn, event_id, &merged).await?;
    let lots = load_lots(&mut conn, event_id).await?;
    let priced = price_cart(&cart_lines(&resolved), &lots)?;

    // 成分原价合计：从 priced.lines 里按 lot_index 聚合，而不是重新乘一遍，
    // 保证和分摊用的是同一组数。
    let mut original: Vec<i64> = vec![0; priced.lots.len()];
    for line in &priced.lines {
        if let Some(k) = line.lot_index {
            original[k] += line.unit_price.cents() * line.qty;
        }
    }

    Ok(Json(QuoteResponse {
        gross_amount: priced.gross.cents(),
        solved_amount: priced.solved.cents(),
        lots: priced
            .lots
            .iter()
            .enumerate()
            .map(|(k, l)| QuoteLot {
                lot_id: l.lot_id,
                name: l.name.clone(),
                price: l.price.cents(),
                original_amount: original[k],
                members: l
                    .members
                    .iter()
                    .map(|(id, q)| QuoteMember { product_id: *id, qty: *q })
                    .collect(),
            })
            .collect(),
        lines: priced
            .lines
            .iter()
            .map(|l| QuoteLine {
                product_id: l.event_product_id,
                qty: l.qty,
                lot_index: l.lot_index,
                allocated_amount: l.allocated.cents(),
            })
            .collect(),
    }))
}
```

- [ ] **Step 5: 跑测试确认通过**

Run: `cd src-tauri && tauri-env linux cargo test --all-features api::lot`
Expected: 13 个测试全绿（Task 4 的 7 个 + 本 task 的 6 个）。

- [ ] **Step 6: 过门禁并提交**

```bash
cd src-tauri
tauri-env linux cargo fmt --all
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
cd ..
git add src-tauri/src/api/lot.rs src-tauri/src/domain/pricing.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: /quote 报价端点——让顾客看得见优惠是怎么来的

D3「顾客端价格必须可解释」。购物车要明写「已应用：任选2件50 −10.00」，
而不是默默给个低价——自助点单场景下顾客看到总价被改过却不知道为什么会不信任。

公开、无鉴权、零写入，有一个专门的测试盯着「报完价数据库一个字没变」。
不查库存：报价是定价预览，库存由下单把关。展会状态照下单一样要求「进行中」，
省得顾客拿到一个下不了的报价。

original_amount 由后端从 priced.lines 聚合而不是让前端重乘一遍单价——
前端再算一遍就等于把分摊逻辑抄了半份出去。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: `create_order` 接上求解器

**Files:**
- Modify: `src-tauri/src/api/order.rs:51-61`（请求结构）、`:96-104`（删 `ProductRow`）、`:105-160`（`OrderItemRow` 与两条 SQL 常量加 `lot_name`）、`:179-330`（`create_order` 重写）
- Test: `src-tauri/src/api/order.rs` 的测试模块

**Interfaces:**
- Consumes: Task 3 的 `pricing::{merge_items, resolve_cart, cart_lines, load_lots, price_cart, ensure_event_selling, CartItemRequest}`
- Produces: `OrderItemResponse` 多一个字段 `lot_name: Option<String>`（前端 Task 12 的 `OrderCard` 读它）

- [ ] **Step 1: 写失败的测试**

追加到 `src-tauri/src/api/order.rs` 的 `mod tests`：

```rust
    /// 给展会加一个只含 `ep` 的「任选 2 件 50」，返回 lot_id。
    async fn seed_pair_lot(pool: &sqlx::SqlitePool, event_id: i64, ep: i64) -> i64 {
        crate::test_support::seed_lot(pool, event_id, "任选2件50", 2, 5000, &[ep]).await
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
                .bind(order_id).fetch_one(&pool).await.unwrap();
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

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders"), None,
            json!({"items": [{"product_id": ep_a, "quantity": 3}]}),
        )).await.unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        let legs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM stock_movements sm JOIN journals j ON j.id = sm.journal_id
             WHERE j.order_id = ?",
        ).bind(order_id).fetch_one(&pool).await.unwrap();
        assert_eq!(legs, 1, "3 件同一个商品只有一条货的腿，不跟着订单行拆");
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_a).await.unwrap(),
            7
        );
    }

    #[tokio::test]
    async fn changing_a_lot_after_the_order_does_not_change_the_order() {
        // 价格在下单那一刻快照进 order_lots / order_lines
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let lot_id = seed_pair_lot(&pool, event_id, ep_a).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders"), None,
            json!({"items": [{"product_id": ep_a, "quantity": 2}]}),
        )).await.unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        sqlx::query("UPDATE lots SET total_price = 100 WHERE id = ?")
            .bind(lot_id).execute(&pool).await.unwrap();

        let price: i64 = sqlx::query_scalar("SELECT price FROM order_lots WHERE order_id = ?")
            .bind(order_id).fetch_one(&pool).await.unwrap();
        let solved: i64 = sqlx::query_scalar("SELECT solved_amount FROM orders WHERE id = ?")
            .bind(order_id).fetch_one(&pool).await.unwrap();
        assert_eq!(price, 5000);
        assert_eq!(solved, 5000);
    }
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd src-tauri && tauri-env linux cargo test --all-features api::order`
Expected: 三个新测试失败（`items.len()` 是 1 不是 2、`lot_name` 字段不存在）。

- [ ] **Step 3: 响应结构加 `lot_name`**

`src-tauri/src/api/order.rs`：

```rust
#[derive(Serialize)]
struct OrderItemResponse {
    id: i64,
    product_id: i64,
    quantity: i64,
    product_name: String,
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
```

`OrderItemRow` 加 `lot_name: Option<String>`，`From` 实现里跟着加一行；两条 SQL 常量都加 `LEFT JOIN`：

```rust
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
```

请求结构改成复用 `pricing` 的：

```rust
#[derive(Deserialize)]
struct CreateOrderRequest {
    items: Vec<CartItemRequest>,
}
```
（删掉本地的 `CreateOrderItemRequest`。）

删掉 `ProductRow`——它被 `pricing::CartProduct` 取代了。

- [ ] **Step 4: 重写 `create_order`**

```rust
async fn create_order(
    State(state): State<AppState>,
    Path(event_id): Path<i64>,
    Json(payload): Json<CreateOrderRequest>,
) -> ApiResult<impl IntoResponse> {
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
            return Err(ApiError::Conflict(format!("「{}」库存不足", r.product.name)));
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
    let snapshot: HashMap<i64, &crate::domain::pricing::CartProduct> =
        resolved.iter().map(|r| (r.product.id, &r.product)).collect();

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

    Ok((StatusCode::CREATED, Json(OrderResponse { order, items })))
}
```

`use` 里补：

```rust
    domain::pricing::{
        cart_lines, ensure_event_selling, load_lots, merge_items, price_cart, resolve_cart,
        CartItemRequest,
    },
```

- [ ] **Step 5: 跑测试确认通过**

Run: `cd src-tauri && tauri-env linux cargo test --all-features api::order`
Expected: ②-1 的 10 个老测试 + 3 个新测试全绿。**老测试一个都不该改**——没有 Lot 的展会里求解器是恒等映射。

- [ ] **Step 6: 过门禁并提交**

```bash
cd src-tauri
tauri-env linux cargo fmt --all
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
cd ..
git add src-tauri/src/api/order.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 下单接上求解器——订单行按 Lot 归属拆分

下单和 /quote 共用同一份 price_cart，所以购物车显示的价和落账的价不可能
因为实现漂移而分叉。

订单行按 Lot 归属拆分：order_lot_id 是每行一个，「3 件里 2 件进套装」
在一行里表达不了。于是同一个商品会在一张订单里出现两行，摊主端靠新加的
lot_name 区分——否则配货时会以为系统重复计数了。

**货那一侧一个字没改**：stock_movements 仍然是每个商品一条腿，不跟着订单行拆。
这是 api/product.rs 的「有流水才禁止删除」守卫仍然完备的前提（②-1 交接段第 2 条）。

价格在下单那一刻快照进 order_lots / order_lines，之后改价甚至删掉 Lot
都不影响已下的单。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: 确认收款时的手工覆盖

**Files:**
- Modify: `src-tauri/src/api/order.rs:62-66`（`UpdateStatusRequest`）、`:426-487`（`("pending", "completed")` 分支）
- Test: `src-tauri/src/api/order.rs` 的测试模块

**Interfaces:**
- Consumes: Task 2 的 `allocation::apply_manual_adjustment`
- Produces: `PUT /api/events/:event_id/orders/:order_id/status` 的 body 多一个可选字段 `final_amount`（整数分）。不给就等于 `solved_amount`。

- [ ] **Step 1: 写失败的测试**

追加到 `src-tauri/src/api/order.rs` 的 `mod tests`：

```rust
    use crate::domain::ledger::{account_balance, Account};
    use crate::domain::money::Money;

    /// 下一张单并返回 order_id。
    async fn place(router: &axum::Router, event_id: i64, items: serde_json::Value) -> i64 {
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({ "items": items }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        read_json(res).await["id"].as_i64().unwrap()
    }

    #[tokio::test]
    async fn a_manual_discount_falls_entirely_on_the_home_society() {
        // spec 4.4 里最要命的那个 case：**整单都是代卖货**，摊主还是让了价。
        // 规则不依赖订单里有没有本社团的行——那 5 块一样落到本社团头上。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let order_id = place(&router, event_id, json!([{"product_id": ep_b, "quantity": 1}])).await;

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "现金", "final_amount": 1500}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // 代卖社团按**自己的定价**全额入账，一分不少
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::from_cents(-2000),
            "「我帮你卖货，我自己让的价不该由你买单」"
        );
        // 本社团承担了这 5 块：债权减少 500（余额为正）
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1)).await.unwrap(),
            Money::from_cents(500)
        );
        // 实收净入 = 顾客实付
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into())).await.unwrap(),
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
        let order_id = place(&router, event_id, json!([{"product_id": ep_a, "quantity": 1}])).await;

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "现金", "final_amount": 3500}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK, "加价不能 400");

        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into())).await.unwrap(),
            Money::from_cents(3500)
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1)).await.unwrap(),
            Money::from_cents(-3500),
            "本社团那头也跟着多出 5 块"
        );
    }

    #[tokio::test]
    async fn paid_amounts_always_add_up_to_the_final_amount() {
        // spec 4.5 的第二条不变量。Task 9 的统计口径完全押在它上面。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let order_id = place(&router, event_id,
            json!([{"product_id": ep_a, "quantity": 2}, {"product_id": ep_b, "quantity": 1}])).await;

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "微信", "final_amount": 7333}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let sum: i64 = sqlx::query_scalar("SELECT SUM(paid_amount) FROM order_lines WHERE order_id = ?")
            .bind(order_id).fetch_one(&pool).await.unwrap();
        assert_eq!(sum, 7333, "取整的余数不能凭空蒸发或多出来");

        // 货主那一侧完全不受影响（spec 4.3 第一条约束）
        let alloc: i64 = sqlx::query_scalar("SELECT SUM(allocated_amount) FROM order_lines WHERE order_id = ?")
            .bind(order_id).fetch_one(&pool).await.unwrap();
        assert_eq!(alloc, 8000);
    }

    #[tokio::test]
    async fn completing_without_a_final_amount_keeps_the_solved_price() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let order_id = place(&router, event_id, json!([{"product_id": ep_a, "quantity": 1}])).await;

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "微信"}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(read_json(res).await["final_amount"], 3000);

        // 没有手工折让就不该有那条腿
        let legs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM money_movements mm JOIN journals j ON j.id = mm.journal_id
             WHERE j.order_id = ?",
        ).bind(order_id).fetch_one(&pool).await.unwrap();
        assert_eq!(legs, 1, "只有按货主分组的那一条");
    }

    #[tokio::test]
    async fn a_negative_final_amount_is_refused() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let order_id = place(&router, event_id, json!([{"product_id": ep_a, "quantity": 1}])).await;

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "微信", "final_amount": -1}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn cancelling_a_discounted_order_reverses_the_discount_leg_too() {
        // 冲正走的是 reverse_order_journals，它把 journal 的每条腿都反向——
        // 手工折让那条腿不需要任何特殊处理，但必须有测试钉住这一点。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let order_id = place(&router, event_id,
            json!([{"product_id": ep_a, "quantity": 1}, {"product_id": ep_b, "quantity": 1}])).await;

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "现金", "final_amount": 4000}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "cancelled"}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        for account in [Account::SocietyDue(1), Account::SocietyDue(2), Account::Received("现金".into())] {
            assert_eq!(
                account_balance(&pool, event_id, &account).await.unwrap(),
                Money::ZERO,
                "取消之后所有资金账户必须回到 0：{account}"
            );
        }
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_a).await.unwrap(), 10);
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_b).await.unwrap(), 5);
    }
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd src-tauri && tauri-env linux cargo test --all-features api::order`
Expected: 新测试失败（`final_amount` 被 serde 忽略，账面仍按 solved 走）。

- [ ] **Step 3: 写实现**

`UpdateStatusRequest` 加字段：

```rust
#[derive(Deserialize)]
struct UpdateStatusRequest {
    status: String,
    channel: Option<String>,
    /// 摊主手工改的实收金额（分）。不给就等于 `solved_amount`。
    ///
    /// 落点是「确认收款」这一步（spec 4.3）：现场的手势本来就是
    /// 「报个数、收钱、点完成」，拆成两步只会多一次忘记。
    final_amount: Option<i64>,
}
```

把 `("pending", "completed")` 分支里「按货主分组」之前插入重算 `paid`，之后插入手工折让腿：

```rust
        ("pending", "completed") => {
            // completed 必须带 channel：钱那条腿 `社团往来 → 实收-<渠道>` 需要对手账户。
            let channel = payload
                .channel
                .as_deref()
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(str::to_string)
                .ok_or_else(|| ApiError::BadRequest("完成订单必须提供收款渠道".into()))?;

            let solved: i64 = query_scalar("SELECT solved_amount FROM orders WHERE id = ?")
                .bind(order_id)
                .fetch_one(&mut *tx)
                .await?;
            let final_amount = payload.final_amount.unwrap_or(solved);
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
            let allocated: Vec<Money> =
                lines.iter().map(|(_, a, _)| Money::from_cents(*a)).collect();
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
                let home: i64 = query_scalar("SELECT id FROM societies WHERE is_home = 1")
                    .fetch_one(&mut *tx)
                    .await?;
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
```

`use` 里补 `domain::allocation::apply_manual_adjustment`。

- [ ] **Step 4: 跑测试确认通过**

Run: `cd src-tauri && tauri-env linux cargo test --all-features api::order`
Expected: 全绿。②-1 的老测试仍然一个都不用改——不传 `final_amount` 时行为完全不变。

- [ ] **Step 5: 过门禁并提交**

```bash
cd src-tauri
tauri-env linux cargo fmt --all
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
cd ..
git add src-tauri/src/api/order.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 确认收款时可改总价——手工折让整笔落在本社团头上

spec 4.3 / 4.4。「这堆算你 150」是现场最常见的说法，落点就是确认收款这一步。

手工折让不按货主比例分摊：代卖社团按自己的定价全额入账，让的价全部由本社团
承担。这条规则不依赖订单里有没有本社团的行——整单都是代卖货、摊主说
「算你 15」，那 5 块一样落到本社团头上，有专门的测试钉住这个 case。

加价（spec 4.3 明说允许）把腿的方向反过来、金额取绝对值。照
「金额 = solved − final」直接传，加价时它是负数，post_journal 会用
「资金移动的金额必须为正」把每一张加价订单的完成打成 400——②-1 的交接段
预告过这个坑。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: 拆掉套装实例

**Files:**
- Modify: `src-tauri/src/api/order.rs`（`OrderResponse` 加 `lots`、`UpdateStatusRequest` 加 `unapply_lot_ids`、完成分支前置拆解、`create_order` / `load_order_response` / `list_orders` 三处响应组装）
- Test: `src-tauri/src/api/order.rs` 的测试模块

**Interfaces:**
- Consumes: Task 6 写进 `order_lots` 的快照、Task 7 的完成分支
- Produces:
  ```rust
  struct OrderLotResponse { id: i64, lot_id: Option<i64>, name: String, price: i64, original_amount: i64 }
  // OrderResponse 多一个字段
  struct OrderResponse { order: OrderRow, items: Vec<OrderItemResponse>, lots: Vec<OrderLotResponse> }
  ```
  `PUT .../orders/:order_id/status` 的 body 多一个可选字段 `unapply_lot_ids: Vec<i64>`，**装的是 `order_lots.id`（实例 id），不是 `lots.id`**——同一个套装可以套用多次，拆的时候必须能指到具体是哪一次。

**为什么要有这条通道**（spec 4.3 的 2026-09-23 修正）：光靠「改总价」拆不掉套装。改总价会把差额记成**手工折让**，按 spec 4.4 整笔落本社团；而「这个套装不该套用」是**纠错**，钱必须回到真正的货主头上。代卖货上的后果是货主少拿钱、差额挂在本社团头上——**金额总数对，归属错，而且不报错**。

- [ ] **Step 1: 写失败的测试**

追加到 `src-tauri/src/api/order.rs` 的 `mod tests`：

```rust
    /// 给代卖社团（黄昏堂，id=2）的商品配一个「任选2件30」（原价 40），返回 lot_id。
    ///
    /// 刻意用**代卖**货：自家货上拆不拆都对得上，只有代卖货能暴露归属错位。
    async fn seed_consignment_lot(pool: &sqlx::SqlitePool, event_id: i64, ep_b: i64) -> i64 {
        crate::test_support::seed_lot(pool, event_id, "代卖任选2件30", 2, 3000, &[ep_b]).await
    }

    #[tokio::test]
    async fn the_order_response_carries_its_lot_instances() {
        // 摊主端的收款弹窗要按实例列勾选框，所以响应里必须有实例 id、名字、
        // 套装价和成分原价合计——最后一个是「拆掉它应收会回到多少」。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_consignment_lot(&pool, event_id, ep_b).await;
        let _order_id = place(&router, event_id, json!([{"product_id": ep_b, "quantity": 2}])).await;

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
        let order_id = place(&router, event_id, json!([{"product_id": ep_b, "quantity": 2}])).await;

        let lot_instance: i64 = sqlx::query_scalar("SELECT id FROM order_lots WHERE order_id = ?")
            .bind(order_id).fetch_one(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "现金", "unapply_lot_ids": [lot_instance]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        assert_eq!(body["solved_amount"], 4000, "应收回到原价");
        assert_eq!(body["final_amount"], 4000, "没另外改实收，就跟着新的应收走");
        assert_eq!(body["gross_amount"], 4000, "原价合计从来没变过");
        assert_eq!(body["lots"].as_array().unwrap().len(), 0, "实例被拆掉了");

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::from_cents(-4000),
            "代卖社团拿全价——只改实收的话这里会是 -3000"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1)).await.unwrap(),
            Money::ZERO,
            "本社团完全不该被牵连——只改实收的话这里会是 -1000"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into())).await.unwrap(),
            Money::from_cents(4000)
        );

        // 货那一侧一个字没动（spec 4.3 第一条约束）
        assert_eq!(crate::domain::ledger::onsite_balance(&pool, ep_b).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn unapplying_one_instance_leaves_the_other_alone() {
        // 「任选2件30」买 4 件 = 两个实例。只拆一个。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_consignment_lot(&pool, event_id, ep_b).await;
        let token = admin_token();
        let order_id = place(&router, event_id, json!([{"product_id": ep_b, "quantity": 4}])).await;

        let instances: Vec<i64> =
            sqlx::query_scalar("SELECT id FROM order_lots WHERE order_id = ? ORDER BY id")
                .bind(order_id).fetch_all(&pool).await.unwrap();
        assert_eq!(instances.len(), 2, "8000 的货套两次 30，应收 6000");

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "现金", "unapply_lot_ids": [instances[0]]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        assert_eq!(body["solved_amount"], 7000, "一个实例回到 40，另一个还是 30");
        assert_eq!(body["lots"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn unapplying_a_lot_that_belongs_to_another_order_is_refused() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_consignment_lot(&pool, event_id, ep_b).await;
        let token = admin_token();
        let first = place(&router, event_id, json!([{"product_id": ep_b, "quantity": 2}])).await;
        let second = place(&router, event_id, json!([{"product_id": ep_b, "quantity": 2}])).await;

        let other: i64 = sqlx::query_scalar("SELECT id FROM order_lots WHERE order_id = ?")
            .bind(first).fetch_one(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{second}/status"), Some(&token),
            json!({"status": "completed", "channel": "现金", "unapply_lot_ids": [other]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        // 整体回滚：第一张单的实例还在，第二张单也没被完成
        let still: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM order_lots WHERE order_id = ?")
            .bind(first).fetch_one(&pool).await.unwrap();
        assert_eq!(still, 1);
        let status: String = sqlx::query_scalar("SELECT status FROM orders WHERE id = ?")
            .bind(second).fetch_one(&pool).await.unwrap();
        assert_eq!(status, "pending");
    }

    #[tokio::test]
    async fn unapplying_and_then_discounting_compose() {
        // 拆完之后摊主还想让价：那一步才走 4.4 的「全额落本社团」。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_consignment_lot(&pool, event_id, ep_b).await;
        let token = admin_token();
        let order_id = place(&router, event_id, json!([{"product_id": ep_b, "quantity": 2}])).await;

        let instance: i64 = sqlx::query_scalar("SELECT id FROM order_lots WHERE order_id = ?")
            .bind(order_id).fetch_one(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "现金",
                   "unapply_lot_ids": [instance], "final_amount": 3500}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::from_cents(-4000), "货主仍然按自己的定价全额入账");
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1)).await.unwrap(),
            Money::from_cents(500), "这 5 块是摊主自己让的，落本社团");
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into())).await.unwrap(),
            Money::from_cents(3500));
    }

    #[tokio::test]
    async fn unapply_is_refused_on_a_cancellation() {
        // 静默忽略一个字段是会咬人的：取消路径上拆套装没有意义，明确拒绝。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_consignment_lot(&pool, event_id, ep_b).await;
        let token = admin_token();
        let order_id = place(&router, event_id, json!([{"product_id": ep_b, "quantity": 2}])).await;
        let instance: i64 = sqlx::query_scalar("SELECT id FROM order_lots WHERE order_id = ?")
            .bind(order_id).fetch_one(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "cancelled", "unapply_lot_ids": [instance]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd src-tauri && tauri-env linux cargo test --all-features api::order`
Expected: 六个新测试失败（响应里没有 `lots`、`unapply_lot_ids` 被 serde 忽略）。

- [ ] **Step 3: 响应带上套装实例**

`src-tauri/src/api/order.rs`：

```rust
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

#[derive(Serialize)]
struct OrderResponse {
    #[serde(flatten)]
    order: OrderRow,
    items: Vec<OrderItemResponse>,
    lots: Vec<OrderLotResponse>,
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
```

（`OrderResponse` 原本就是 `#[serde(flatten)]` 铺平 `OrderRow` 的，保持不变，只加 `lots` 一个字段。）

三处组装都要跟着改：

**`load_order_response`** 末尾：

```rust
    let lots: Vec<OrderLotResponse> = query_as::<_, OrderLotRow>(LOTS_BY_ORDER)
        .bind(order_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(OrderResponse { order, items, lots })
```

**`list_orders`**：和 `items_map` 一样再来一份 `lots_map`：

```rust
    let lot_rows: Vec<OrderLotRow> = query_as(LOTS_BY_EVENT)
        .bind(event_id)
        .fetch_all(&state.db)
        .await?;
    let mut lots_map: HashMap<i64, Vec<OrderLotResponse>> = HashMap::new();
    for row in lot_rows {
        let oid = row.order_id;
        lots_map.entry(oid).or_default().push(row.into());
    }
```
组装时 `lots: lots_map.remove(&oid).unwrap_or_default()`。

**`create_order`**（Task 6 写的那段）末尾，把 `OrderResponse { order, items }` 换成：

```rust
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
                .map(|l| l.unit_price.cents() * l.qty)
                .sum(),
        })
        .collect();

    Ok((
        StatusCode::CREATED,
        Json(OrderResponse { order, items, lots: lots_response }),
    ))
```

- [ ] **Step 4: 加拆解逻辑**

`UpdateStatusRequest` 加字段：

```rust
#[derive(Deserialize)]
struct UpdateStatusRequest {
    status: String,
    channel: Option<String>,
    final_amount: Option<i64>,
    /// 要拆掉的套装实例（`order_lots.id`）。
    ///
    /// **拆 ≠ 改总价。** 改总价把差额记成手工折让、按 spec 4.4 整笔落本社团；
    /// 而「这个套装不该套用」是纠错，钱必须回到真正的货主头上。代卖货上
    /// 只改总价会让货主少拿钱、差额挂在本社团头上——金额总数对，归属错。
    unapply_lot_ids: Option<Vec<i64>>,
}
```

`update_order_status` 里，**紧跟在 `target` 校验之后、开事务之前**加一道：

```rust
    // 静默忽略一个用户传了的字段是会咬人的。取消路径上拆套装没有意义。
    if payload.unapply_lot_ids.is_some() && target != "completed" {
        return Err(ApiError::BadRequest(
            "只有在完成订单时才能拆套装".into(),
        ));
    }
```

`("pending", "completed")` 分支里，**在读 `solved_amount` 之前**插入：

```rust
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
                    belongs.ok_or_else(|| {
                        ApiError::BadRequest("要拆的套装不属于这张订单".into())
                    })?;

                    // 成分行金额恢复成「单价 × 件数」，并解除归属。
                    // **不重新求解**：纯拆解，结果唯一，也不依赖当前的 Lot 配置
                    // （摊主可能刚把那个 Lot 改了或删了）。
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
```

后面 Task 7 写的 `let solved: i64 = query_scalar("SELECT solved_amount ...")` 一个字都不用改——它读到的已经是拆完之后的值。

- [ ] **Step 5: 跑测试确认通过**

Run: `cd src-tauri && tauri-env linux cargo test --all-features api::order`
Expected: 全绿。Task 6 / Task 7 的测试也要跟着过——它们不传 `unapply_lot_ids`，行为完全不变。

- [ ] **Step 6: 过门禁并提交**

```bash
cd src-tauri
tauri-env linux cargo fmt --all
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
cd ..
git add src-tauri/src/api/order.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 收款时可以拆掉套装——让钱回到真正的货主头上

spec 4.3 的辅助通道，2026-09-23 修正后确定要做。

光靠「改总价」拆不掉套装：改总价把差额记成手工折让、按 4.4 整笔落本社团，
而「这个套装不该套用」是**纠错**，钱必须回到真正的货主头上。代卖货上的后果是
B 的三本漫画（原价 120）被套装算成 100、摊主改回 120，账上 B 仍然只拿 100，
多出的 20 挂在本社团头上——金额总数对，归属错，而且不报错。

拆是纯拆解不是重新求解：成分行金额恢复成「单价 × 件数」、实例删掉、应收重新
聚合。因此结果唯一，也不依赖当前的 Lot 配置（摊主可能刚把那个 Lot 改了或删了）。
gross 和货那一侧都不动。

响应带上套装实例（含成分原价合计），收款弹窗靠它在本地算出「拆掉之后应收是多少」，
不必多一次往返。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: 统计口径改用 `paid_amount`

**Files:**
- Modify: `src-tauri/src/api/stats.rs:122`、`:193`、`:403`
- Test: `src-tauri/src/api/stats.rs` 的测试模块

**Interfaces:**
- Consumes: Task 7 / Task 8 的不变量「同一订单 Σ paid = final_amount」
- Produces: 无新接口。`ProductSalesItem::total_revenue_per_item` 的**含义**从「原价合计」变成「顾客实付合计」。

**为什么必须改**（②-1 交接段第 4 条）：`unit_price × qty` 是折让**前**的原价。Lot 和手工折让一落地，仪表盘 / 趋势图 / CSV / xlsx 的单品销售额全部虚高，而同页的 `SUM(orders.final_amount)` 仍然正确——于是「总额」和「按商品汇总之和」对不上，摊主会当成 bug 报回来。

- [ ] **Step 1: 写失败的测试**

追加到 `src-tauri/src/api/stats.rs` 的 `mod tests`：

```rust
    #[tokio::test]
    async fn per_product_revenue_adds_up_to_the_order_total() {
        // ②-1 交接段第 4 条：Lot + 手工折让一落地，SUM(unit_price × qty) 就是
        // 「折让前的原价」，仪表盘的两个数字会对不上。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();
        // ep_a 上挂一个「任选 2 件 50」（原价 60）
        crate::test_support::seed_lot(&pool, event_id, "任选2件50", 2, 5000, &[ep_a]).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders"), None,
            json!({"items": [{"product_id": ep_a, "quantity": 2},
                             {"product_id": ep_b, "quantity": 1}]}),
        )).await.unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        // solved = 5000 + 2000 = 7000，摊主再抹到 6666
        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "微信", "final_amount": 6666}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = router.clone().oneshot(json_request(
            "GET", &format!("/api/events/{event_id}/stats"), Some(&token), json!({}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;

        let total = body["summary"]["total_revenue"].as_i64().unwrap();
        assert_eq!(total, 6666);

        let per_item: i64 = body["product_details"].as_array().unwrap().iter()
            .map(|d| d["total_revenue_per_item"].as_i64().unwrap())
            .sum();
        assert_eq!(
            per_item, total,
            "按商品汇总之和必须等于总额——这条由「同一订单 Σ paid = final_amount」结构上保证"
        );
    }
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd src-tauri && tauri-env linux cargo test --all-features api::stats::tests::per_product_revenue`
Expected: FAIL，`per_item` 是 8000（原价合计）而 `total` 是 6666。

- [ ] **Step 3: 改三处 SQL**

`stats.rs` 的 `:122`、`:193`、`:403` 三处**完全相同**的一行：

```sql
            SUM(ol.unit_price * ol.qty) as total_revenue_per_item
```

一律改成：

```sql
            -- 顾客实付，不是原价。Σ paid = final_amount（spec 4.5）保证
            -- 「按商品汇总之和 == SUM(orders.final_amount)」，仪表盘两个数字永远对得上。
            -- 货主该得多少是结算单的事（②-3），那边用 allocated_amount。
            SUM(ol.paid_amount) as total_revenue_per_item
```

`ORDER BY total_revenue_per_item DESC` 和 `GROUP BY ... ol.unit_price` 都不用动。

同时改 `ProductSalesItem::total_revenue_per_item` 的文档注释：

```rust
    /// 单位：分。**顾客实付合计**（`Σ paid_amount`），不是原价合计——
    /// Lot 分摊与手工折让都已经摊进去了。
    total_revenue_per_item: i64,
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cd src-tauri && tauri-env linux cargo test --all-features api::stats`
Expected: ②-1 的 3 个冒烟测试 + 本 task 的 1 个全绿。老测试不受影响——没有折让时 `paid == unit_price × qty`。

- [ ] **Step 5: 过门禁并提交**

```bash
cd src-tauri
tauri-env linux cargo fmt --all
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
cd ..
git add src-tauri/src/api/stats.rs
git commit -m "$(cat <<'EOF'
fix: :bug: 单品销售额改用 paid_amount，否则仪表盘两个数字对不上

②-1 交接段第 4 条点名的事。三处按商品汇总用的是 SUM(unit_price × qty)，
那是折让**前**的原价——②-1 里三个金额恒等所以没暴露，Lot 和手工折让一落地
就会让仪表盘、趋势图、CSV、xlsx 的单品销售额全部虚高，而同页的
SUM(orders.final_amount) 仍然正确，于是「总额」和「按商品汇总之和」对不上。

改成 SUM(paid_amount) 之后，这两个数字的相等由「同一订单 Σ paid = final_amount」
这条不变量**结构上**保证，测试直接断言它们相等。

货主该得多少是结算单的事（②-3），那边用 allocated_amount，不受影响。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: 套装配置页（前端）

**Files:**
- Create: `frontend/src/stores/lotStore.js`、`frontend/src/views/AdminEventLots.vue`
- Modify: `frontend/src/router/index.js`、`frontend/src/views/AdminLayout.vue:224-251`

**Interfaces:**
- Consumes: Task 4 的 `GET|POST /events/:id/lots`、`PUT|DELETE /events/:id/lots/:lot_id`
- Produces: `useLotStore()` → `{ lots, isLoading, error, fetchLots, createLot, updateLot, deleteLot, resetStore }`；路由 `/admin/events/:id/lots`（name `admin-event-lots`）

- [ ] **Step 1: 建 store**

`frontend/src/stores/lotStore.js`：

```js
import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/services/api'

/**
 * 套装（Lot）= 候选商品集合 + 要选几件 + 总价。
 *
 * 金额一律是**分**：后端收分、返回分，页面上的「元」只在输入框和显示处换算
 * （`@/utils/money`）。这里不做任何换算，免得两边各转一次。
 *
 * 候选集必须同一货主是后端的硬校验（spec 4.1），前端只负责把错误原文显示出来——
 * 不在前端重复这条规则，否则两边的判断迟早不一致。
 */
export const useLotStore = defineStore('lot', () => {
  const lots = ref([])
  const isLoading = ref(false)
  const error = ref(null)

  async function fetchLots(eventId) {
    isLoading.value = true
    error.value = null
    try {
      const response = await api.get(`/events/${eventId}/lots`)
      lots.value = Array.isArray(response.data) ? response.data : []
    } catch (err) {
      error.value = '无法加载套装列表。'
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }

  async function createLot(eventId, payload) {
    try {
      const response = await api.post(`/events/${eventId}/lots`, payload)
      lots.value.push(response.data)
      return response.data
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '新建套装失败。')
    }
  }

  async function updateLot(eventId, lotId, payload) {
    try {
      const response = await api.put(`/events/${eventId}/lots/${lotId}`, payload)
      const index = lots.value.findIndex((l) => l.id === lotId)
      if (index !== -1) lots.value[index] = response.data
      return response.data
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '更新套装失败。')
    }
  }

  async function deleteLot(eventId, lotId) {
    try {
      await api.delete(`/events/${eventId}/lots/${lotId}`)
      lots.value = lots.value.filter((l) => l.id !== lotId)
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '删除套装失败。')
    }
  }

  function resetStore() {
    lots.value = []
    error.value = null
  }

  return { lots, isLoading, error, fetchLots, createLot, updateLot, deleteLot, resetStore }
})
```

- [ ] **Step 2: 建页面**

`frontend/src/views/AdminEventLots.vue`：

```vue
<template>
  <div class="page">
    <header class="page-header">
      <h1>套装与优惠</h1>
      <p>
        套装 = 从一组候选商品里任选 N 件，按一个总价卖。「全套 5 本 100」是候选集正好 5 件的特例。
        <strong>候选商品必须属于同一个货主</strong>——替别的社团让价不是摊主能单方面决定的。
      </p>
    </header>

    <main class="page-body">
      <CollapsibleSection title="新建套装" v-model:collapsed="isFormCollapsed" class="form-section">
        <div class="form-grid">
          <n-input v-model:value="form.name" placeholder="套装名称，如「本子任选3本100」" />
          <n-input-number v-model:value="form.pickCount" :min="1" :precision="0" placeholder="要选几件" />
          <n-input-number v-model:value="form.priceYuan" :min="0" :precision="2" placeholder="总价（元）" />
          <n-select
            v-model:value="form.candidateIds"
            multiple
            filterable
            :options="candidateOptions"
            placeholder="候选商品"
            class="candidates"
          />
          <n-button type="primary" :disabled="isBusy" @click="handleSubmit">
            {{ editingId ? '保存修改' : '新建' }}
          </n-button>
          <n-button v-if="editingId" quaternary @click="resetForm">取消编辑</n-button>
        </div>
      </CollapsibleSection>

      <div v-if="store.isLoading" class="loading-message">正在加载套装列表...</div>
      <div v-else-if="store.error" class="error-message">{{ store.error }}</div>
      <EmptyGuide v-else-if="!store.lots.length" title="还没有套装" hint="配一个套装，顾客的购物车就会自动套用最省的那一种。" />

      <div v-else class="table-wrapper">
        <table class="lot-table">
          <thead>
            <tr>
              <th>名称</th>
              <th>任选</th>
              <th>总价</th>
              <th>货主</th>
              <th>候选商品</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="lot in store.lots" :key="lot.id">
              <td>{{ lot.name }}</td>
              <td>{{ lot.pick_count }} 件</td>
              <td>{{ formatYuan(lot.total_price) }}</td>
              <td>{{ lot.owner_society_name }}</td>
              <td class="candidates-cell">{{ candidateNames(lot) }}</td>
              <td>
                <n-space size="small" justify="end">
                  <n-button size="small" :disabled="isBusy" @click="startEdit(lot)">编辑</n-button>
                  <n-button size="small" type="error" quaternary :disabled="isBusy" @click="handleDelete(lot)">
                    删除
                  </n-button>
                </n-space>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </main>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { NInput, NInputNumber, NSelect, NButton, NSpace, useDialog, useMessage } from 'naive-ui'
import CollapsibleSection from '@/components/shared/CollapsibleSection.vue'
import EmptyGuide from '@/components/shared/EmptyGuide.vue'
import { useLotStore } from '@/stores/lotStore'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import { formatYuan, toCents, fromCents } from '@/utils/money'

const props = defineProps({ id: { type: [String, Number], required: true } })

const store = useLotStore()
const eventDetailStore = useEventDetailStore()
const dialog = useDialog()
const message = useMessage()

const isFormCollapsed = ref(false)
const isBusy = ref(false)
const editingId = ref(null)
const form = ref({ name: '', pickCount: 1, priceYuan: null, candidateIds: [] })

// 选项标签带上货主名：候选集必须同一货主是后端硬校验，把货主写在标签上
// 能让摊主在点选时就看出来，而不是提交后才吃一个 400。
const candidateOptions = computed(() =>
  eventDetailStore.products.map((p) => ({
    label: `${p.name}（${p.owner_society_name} · ${formatYuan(p.unit_price)}）`,
    value: p.id,
  }))
)

function candidateNames(lot) {
  const byId = new Map(eventDetailStore.products.map((p) => [p.id, p.name]))
  return lot.candidate_ids.map((id) => byId.get(id) || `#${id}`).join('、')
}

function resetForm() {
  editingId.value = null
  form.value = { name: '', pickCount: 1, priceYuan: null, candidateIds: [] }
}

function startEdit(lot) {
  editingId.value = lot.id
  form.value = {
    name: lot.name,
    pickCount: lot.pick_count,
    priceYuan: fromCents(lot.total_price),
    candidateIds: [...lot.candidate_ids],
  }
  isFormCollapsed.value = false
}

async function handleSubmit() {
  const name = form.value.name.trim()
  if (!name) return message.warning('请填写套装名称')
  if (!form.value.candidateIds.length) return message.warning('请至少选一个候选商品')
  if (form.value.priceYuan === null) return message.warning('请填写总价')

  const payload = {
    name,
    pick_count: form.value.pickCount,
    total_price: toCents(form.value.priceYuan),
    candidate_ids: form.value.candidateIds,
  }
  isBusy.value = true
  try {
    if (editingId.value) {
      await store.updateLot(props.id, editingId.value, payload)
      message.success('套装已更新')
    } else {
      await store.createLot(props.id, payload)
      message.success('套装已新建')
    }
    resetForm()
  } catch (error) {
    message.error(error.message || '操作失败')
  } finally {
    isBusy.value = false
  }
}

function handleDelete(lot) {
  dialog.warning({
    title: '确认删除',
    // 快照的存在是这句话成立的理由，不是安慰剧。
    content: `删除套装「${lot.name}」？已经下过的订单不受影响——它们存的是名字和价格的快照。`,
    positiveText: '确认删除',
    negativeText: '取消',
    async onPositiveClick() {
      isBusy.value = true
      try {
        await store.deleteLot(props.id, lot.id)
        message.success('套装已删除')
      } catch (error) {
        message.error(error.message || '删除失败')
      } finally {
        isBusy.value = false
      }
    },
  })
}

onMounted(async () => {
  await eventDetailStore.fetchProductsForEvent(props.id)
  await store.fetchLots(props.id)
})
</script>

<style scoped>
.page {
  max-width: 960px;
}
.page-header {
  margin-bottom: 1.5rem;
}
.page-header h1 {
  margin: 0 0 0.25rem;
  font-size: var(--font-xl);
  color: var(--accent-color);
}
.page-header p {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--font-base);
  line-height: 1.6;
}

.form-section {
  margin-bottom: 1.5rem;
}
.form-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: center;
}
.form-grid > * {
  flex: 1 1 160px;
  min-width: 0;
}
.candidates {
  flex: 2 1 320px;
}

.table-wrapper {
  width: 100%;
  overflow-x: auto;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
}
.lot-table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;
  font-size: var(--font-base);
}
.lot-table th {
  padding: 12px 16px;
  background-color: var(--card-bg-color);
  color: var(--primary-text-color);
  font-weight: 600;
  border-bottom: 2px solid var(--accent-color);
  white-space: nowrap;
}
.lot-table td {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
  color: var(--text-placeholder);
  vertical-align: middle;
}
.lot-table th:last-child,
.lot-table td:last-child {
  text-align: right;
}
.candidates-cell {
  max-width: 320px;
}

.loading-message,
.error-message {
  padding: 1rem;
  text-align: center;
}
.error-message {
  color: var(--error-color);
}
</style>
```

- [ ] **Step 3: 挂路由与侧栏**

`frontend/src/router/index.js`：`import AdminEventLots from '@/views/AdminEventLots.vue'`，并在 `events/:id/products` 之后加

```js
      {
        path: 'events/:id/lots',
        name: 'admin-event-lots',
        component: AdminEventLots,
        props: true,
      },
```

`frontend/src/views/AdminLayout.vue` 的展会分组 `children` 里，「商品管理」与「订单管理」之间插入：

```js
          {
            label: () =>
              h(
                RouterLink,
                { to: `/admin/events/${event.value.id}/lots` },
                { default: () => '套装与优惠' }
              ),
            key: `/admin/events/${event.value.id}/lots`,
          },
```

- [ ] **Step 4: 过门禁**

```bash
npm run lint --prefix frontend
npm run format:check --prefix frontend
npm run test:unit --prefix frontend
npm run build --prefix frontend
```
Expected: 四条全绿。

- [ ] **Step 5: 在 VNC 上看一眼**

Run: `tauri-env vnc npx tauri dev`
手动确认：侧栏多了「套装与优惠」；建一个候选集跨两个货主的套装会被拒绝并显示后端那句中文；建一个单货主的能成功、能编辑、能删除。

- [ ] **Step 6: 提交**

```bash
git add frontend/src/stores/lotStore.js frontend/src/views/AdminEventLots.vue frontend/src/router/index.js frontend/src/views/AdminLayout.vue
git commit -m "$(cat <<'EOF'
feat: :sparkles: 套装配置页——独立路由，不往 1005 行的商品管理页里塞

新开 /admin/events/:id/lots，体量对标 ②-1 的社团管理页。AdminEventProducts.vue
已经 1005 行、56% 是 CSS，往里再加一块只会让 ④ 的组件层收口更难做。

候选商品的下拉标签带上货主名：候选集必须同一货主是后端硬校验，写在标签上
能让摊主在点选时就看出来，而不是提交后才吃一个 400。前端不重复这条规则——
两边各判一次迟早不一致。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: 顾客购物车接上报价

**Files:**
- Create: `frontend/src/utils/quote.js`、`frontend/src/utils/quote.spec.js`
- Modify: `frontend/src/stores/customerStore.js`、`frontend/src/components/customer/ShoppingCart.vue`、`frontend/src/views/CustomerView.vue:202-220`、`:541`

**Interfaces:**
- Consumes: Task 5 的 `POST /events/:id/quote`
- Produces:
  ```js
  // utils/quote.js
  summarizeQuote(quote, grossFallback) -> { gross, payable, discounts: [{ name, saved, count }] }
  // customerStore 新增导出
  { quote, quoteError, cartSummary }
  // ShoppingCart 新增 props
  { payable: Number, discounts: Array, quoteNotice: String|null }
  ```

- [ ] **Step 1: 写失败的测试**

`frontend/src/utils/quote.spec.js`：

```js
import { describe, it, expect } from 'vitest'
import { summarizeQuote } from './quote'

describe('summarizeQuote', () => {
  it('没有报价时退回原价合计，且不编造优惠', () => {
    expect(summarizeQuote(null, 8000)).toEqual({ gross: 8000, payable: 8000, discounts: [] })
  })

  it('把一个套装折算成一条「省了多少」', () => {
    const quote = {
      gross_amount: 9000,
      solved_amount: 8000,
      lots: [{ lot_id: 1, name: '任选2件50', price: 5000, original_amount: 6000 }],
    }
    expect(summarizeQuote(quote, 9000)).toEqual({
      gross: 9000,
      payable: 8000,
      discounts: [{ name: '任选2件50', saved: 1000, count: 1 }],
    })
  })

  it('同一个套装套用两次合并成一条，带次数', () => {
    // 「任选 3 本 100」买 6 本 = 两个实例。列两行同名的会让人以为系统算重了。
    const quote = {
      gross_amount: 24000,
      solved_amount: 20000,
      lots: [
        { lot_id: 1, name: '任选3本100', price: 10000, original_amount: 12000 },
        { lot_id: 1, name: '任选3本100', price: 10000, original_amount: 12000 },
      ],
    }
    const out = summarizeQuote(quote, 24000)
    expect(out.discounts).toEqual([{ name: '任选3本100', saved: 4000, count: 2 }])
  })

  it('一分钱都没省的套装不显示', () => {
    const quote = {
      gross_amount: 5000,
      solved_amount: 5000,
      lots: [{ lot_id: 1, name: '不划算的套装', price: 5000, original_amount: 5000 }],
    }
    expect(summarizeQuote(quote, 5000).discounts).toEqual([])
  })
})
```

Run: `npm run test:unit --prefix frontend`
Expected: FAIL，`Failed to resolve import "./quote"`。

- [ ] **Step 2: 写 `utils/quote.js`**

```js
/**
 * 把 `/quote` 的响应整理成购物车要显示的三样东西：原价、应付、逐条优惠。
 *
 * 纯函数——不碰网络也不碰 store，所以能直接单测。
 *
 * `quote` 为 null（还没报价，或报价失败）时退回原价合计。**这不是静默降级**：
 * 调用方必须另外把 `quoteError` 显示出来，否则顾客会以为原价就是应付价。
 */
export function summarizeQuote(quote, grossFallback) {
  if (!quote) {
    return { gross: grossFallback, payable: grossFallback, discounts: [] }
  }
  // 同一个套装可以套用多次（「任选 3 本 100」买 6 本 = 两个实例）。
  // 按名字合并再带一个次数——列两行一模一样的文字会让人以为系统算重了。
  const byName = new Map()
  for (const lot of quote.lots || []) {
    const saved = (lot.original_amount || 0) - (lot.price || 0)
    const entry = byName.get(lot.name) || { name: lot.name, saved: 0, count: 0 }
    entry.saved += saved
    entry.count += 1
    byName.set(lot.name, entry)
  }
  return {
    gross: quote.gross_amount,
    payable: quote.solved_amount,
    discounts: [...byName.values()].filter((d) => d.saved > 0),
  }
}
```

Run: `npm run test:unit --prefix frontend` → PASS。

- [ ] **Step 3: store 接线**

`frontend/src/stores/customerStore.js`：

```js
import { summarizeQuote } from '@/utils/quote'
```

在 `cart` 附近加状态、在 `cartTotal` 之后加 computed：

```js
  // --- 报价 ---
  // 折扣由服务端算（`domain/solver.rs`）。前端不重写一份求解器：同一套集合覆盖
  // 两份实现，取整规则一漂移就是「显示 145 实收 150」。
  const quote = ref(null)
  const quoteError = ref(null)
  let quoteSeq = 0
  let quoteTimer = null

  /**
   * 购物车一变就重新报价。300ms debounce + 请求序号两道都需要：
   * 顾客连点加号会发好几次，而乱序返回会让价格来回跳。
   */
  function scheduleQuote() {
    if (quoteTimer) clearTimeout(quoteTimer)
    if (!activeEventId.value || cart.value.length === 0) {
      quoteSeq += 1 // 作废在途的请求，免得它回来给空车填上价
      quote.value = null
      quoteError.value = null
      return
    }
    quoteTimer = setTimeout(fetchQuote, 300)
  }

  async function fetchQuote() {
    const seq = (quoteSeq += 1)
    const items = cart.value.map((item) => ({ product_id: item.id, quantity: item.quantity }))
    try {
      const response = await api.post(`/events/${activeEventId.value}/quote`, { items })
      if (seq !== quoteSeq) return // 旧请求，结果丢掉
      quote.value = response.data
      quoteError.value = null
    } catch (err) {
      if (seq !== quoteSeq) return
      // **不静默**：退回原价，同时明说没套用优惠。后端在超限时给的就是人话。
      quote.value = null
      quoteError.value = err.response?.data?.error || '优惠暂时算不出来，按原价显示'
    }
  }

  const cartSummary = computed(() => summarizeQuote(quote.value, cartTotal.value))
```

`addToCart` / `removeFromCart` / `clearCart` 的末尾各加一行 `scheduleQuote()`。
`return {}` 里加上 `quote, quoteError, cartSummary`。

- [ ] **Step 4: 购物车显示优惠明细**

`frontend/src/components/customer/ShoppingCart.vue`：

props 加三个：

```js
const props = defineProps({
  cart: { type: Array, required: true },
  /** 原价合计（分） */
  total: { type: Number, required: true },
  /** 折后应付（分）。报价失败时等于 total。 */
  payable: { type: Number, required: true },
  /** [{ name, saved, count }] */
  discounts: { type: Array, default: () => [] },
  /** 报价失败时的提示。非空就必须显示——不能让顾客以为原价就是应付价。 */
  quoteNotice: { type: String, default: null },
  isCheckingOut: { type: Boolean, default: false },
})
```

顶部触发栏的 `{{ formatYuan(total) }}` 改成 `{{ formatYuan(payable) }}`。

底部结算区替换成：

```html
        <div class="cart-footer">
          <!-- D3：顾客端价格必须可解释。被优化过的总价必须说清楚是怎么来的，
               否则自助点单的顾客不会信任它。 -->
          <div v-if="payable !== total" class="footer-row subtle">
            <span>原价</span>
            <span class="struck">{{ formatYuan(total) }}</span>
          </div>
          <div v-for="d in discounts" :key="d.name" class="footer-row discount">
            <span>已应用：{{ d.name }}<template v-if="d.count > 1"> ×{{ d.count }}</template></span>
            <span>−{{ formatYuan(d.saved) }}</span>
          </div>
          <p v-if="quoteNotice" class="quote-notice">{{ quoteNotice }}</p>
          <div class="footer-row">
            <span>应付</span>
            <span class="big-total">{{ formatYuan(payable) }}</span>
          </div>
          <n-button
            type="primary"
            block
            round
            size="large"
            :disabled="!cart.length || isCheckingOut"
            :loading="isCheckingOut"
            @click="$emit('checkout')"
            class="checkout-btn"
          >
            {{ isCheckingOut ? '提交中...' : '去结算' }}
          </n-button>
        </div>
```

样式补三条：

```css
.footer-row.subtle {
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.footer-row.subtle .struck {
  text-decoration: line-through;
}
.footer-row.discount {
  color: var(--success-color);
  font-size: var(--font-sm);
}
.quote-notice {
  margin: 0.25rem 0;
  font-size: var(--font-sm);
  color: var(--warning-color);
  line-height: 1.4;
}
```

- [ ] **Step 5: `CustomerView` 传参，并修掉付款金额取错源的问题**

两处 `<ShoppingCart>` 都加：

```html
        :payable="store.cartSummary.payable"
        :discounts="store.cartSummary.discounts"
        :quote-notice="store.quoteError"
```

`handleCheckout` 里两处改动：

```js
  const itemCount = store.cartItemCount
  const totalAmount = store.cartSummary.payable   // 确认框里也该是折后价
```

```js
        const newOrder = await store.submitOrder()
        if (newOrder) {
          // **金额取自下单响应，不是购物车的报价。** 报价只是预览，两次之间
          // 摊主完全可能刚改过 Lot 配置——顾客扫码付的数必须是服务端落账的那个数。
          orderTotal.value = newOrder.final_amount
          showPaymentModal.value = true
          store.clearCart()
          store.fetchProductsForEvent()
        }
```

- [ ] **Step 6: 过门禁**

```bash
npm run lint --prefix frontend
npm run format:check --prefix frontend
npm run test:unit --prefix frontend
npm run build --prefix frontend
```

- [ ] **Step 7: 在 VNC 上看一眼**

Run: `tauri-env vnc npx tauri dev`
手动确认：给某个商品配一个「任选2件」的套装，购物车加到 2 件时出现「原价 / 已应用：… −xx.xx / 应付」三行；加到 3 件时应付 = 套装价 + 一件原价；把套装删掉后购物车回到原价且没有残留的优惠行。

- [ ] **Step 8: 提交**

```bash
git add frontend/src/utils/quote.js frontend/src/utils/quote.spec.js frontend/src/stores/customerStore.js frontend/src/components/customer/ShoppingCart.vue frontend/src/views/CustomerView.vue
git commit -m "$(cat <<'EOF'
feat: :sparkles: 购物车显示折后价与优惠明细，付款金额改取下单响应

D3「顾客端价格必须可解释」：自助点单场景下顾客看到总价被优化过却不知道
为什么会不信任，所以购物车明写「原价 / 已应用：任选2件50 −10.00 / 应付」。

折扣由服务端算，前端不重写一份求解器——同一套集合覆盖两份实现，取整规则
一漂移就是「显示 145 实收 150」。300ms debounce + 请求序号两道都需要：
顾客连点加号会发好几次，乱序返回会让价格来回跳。报价失败不静默降级，
退回原价的同时必须显示提示。

顺手修掉一处真问题：付款弹窗的金额原本取自 store.cartTotal，现在取自下单
响应的 final_amount。报价只是预览，两次之间摊主完全可能刚改过 Lot 配置——
顾客扫码付的数必须是服务端落账的那个数。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 12: 收款确认弹窗（前端）

**Files:**
- Create: `frontend/src/components/vendor/ReceiptModal.vue`
- Delete: `frontend/src/components/vendor/ChannelPicker.vue`
- Modify: `frontend/src/stores/orderStore.js:83-105`、`frontend/src/views/VendorView.vue:81-85`、`:167-190`、`frontend/src/components/order/OrderCard.vue`

**Interfaces:**
- Consumes: Task 6 的 `OrderItemResponse::lot_name`、Task 7 的 `final_amount` 入参、Task 8 的 `OrderResponse::lots` 与 `unapply_lot_ids` 入参
- Produces: `ReceiptModal` 的 `@confirm` 载荷是 `{ channel: String, finalAmount: Number, unapplyLotIds: Number[] }`（`finalAmount` 单位分，`unapplyLotIds` 是 `order_lots.id`）

- [ ] **Step 1: 建 `ReceiptModal.vue`**

```vue
<template>
  <AppModal :show="show" @close="$emit('cancel')">
    <template #header><h3>确认收款</h3></template>
    <template #body>
      <div class="price-block">
        <div v-if="grossAmount !== effectiveSolved" class="price-row subtle">
          <span>原价</span>
          <span class="struck">{{ formatYuan(grossAmount) }}</span>
        </div>
        <div class="price-row total">
          <span>应收</span>
          <span>{{ formatYuan(effectiveSolved) }}</span>
        </div>
      </div>

      <!-- 已套用的套装，逐个可拆。取消勾选 = 这一单不套用它，成分回到原价。
           这和「直接改实收」不是一回事：改实收把差额记成手工折让、整笔落本社团，
           而拆套装是纠错，钱回到真正的货主头上（spec 4.3 的 2026-09-23 修正）。 -->
      <div v-if="lots.length" class="lot-block">
        <p class="lot-title">已套用的套装</p>
        <label v-for="lot in lots" :key="lot.id" class="lot-row">
          <n-checkbox :checked="!unapplied.includes(lot.id)" @update:checked="toggle(lot.id)" />
          <span class="lot-name">{{ lot.name }}</span>
          <span class="lot-saved">−{{ formatYuan(lot.original_amount - lot.price) }}</span>
        </label>
        <p v-if="unapplied.length" class="lot-note">
          已拆掉 {{ unapplied.length }} 个套装，这些商品按原价计算。
        </p>
      </div>

      <label class="field">
        <span class="field-label">实收（元）</span>
        <n-input-number v-model:value="finalYuan" :min="0" :precision="2" class="field-input" />
      </label>
      <p v-if="adjustment !== 0" class="adjustment">
        {{ adjustment > 0 ? '手工折让' : '手工加价' }} {{ formatYuan(Math.abs(adjustment)) }}
        <span class="adjustment-note">——全部算在本社团头上，代卖社团按自己的定价结算</span>
      </p>

      <n-radio-group v-model:value="channel" name="payment-channel">
        <n-space>
          <n-radio v-for="c in CHANNELS" :key="c" :value="c">{{ c }}</n-radio>
        </n-space>
      </n-radio-group>

      <!-- spec 第 11 节的不可破坏项：不得让复式记账制造出「钱已到账」的错觉。
           系统始终不知道顾客有没有真付，摊主点的是「我看到到账提示了」。
           这个弹窗现在还能改金额、还能拆套装，**看起来**更像在处理真钱——
           所以这句话比以前更不能删。 -->
      <p class="disclosure">这只是记账。请先确认手机上真的收到了到账提示，再点确认。</p>
    </template>
    <template #footer>
      <n-space>
        <n-button @click="$emit('cancel')">取消</n-button>
        <n-button type="primary" @click="handleConfirm">确认</n-button>
      </n-space>
    </template>
  </AppModal>
</template>

<script setup>
import { ref, computed, watch } from 'vue'
import { NRadioGroup, NRadio, NSpace, NButton, NInputNumber, NCheckbox } from 'naive-ui'
import AppModal from '@/components/shared/AppModal.vue'
import { formatYuan, toCents, fromCents } from '@/utils/money'

// 现场一场展会里收款渠道基本不变，上次选的记 localStorage 做默认值。
const CHANNELS = ['现金', '微信', '支付宝']
const CHANNEL_STORAGE_KEY = 'last_payment_channel'

const props = defineProps({
  show: { type: Boolean, default: false },
  /** 原价合计（分） */
  grossAmount: { type: Number, default: 0 },
  /** 服务端算出的应收（分） */
  solvedAmount: { type: Number, default: 0 },
  /** 这一单套用的套装实例：[{ id, name, price, original_amount }] */
  lots: { type: Array, default: () => [] },
})
const emit = defineEmits(['confirm', 'cancel'])

const channel = ref('微信')
const finalYuan = ref(0)
/** 被取消勾选的套装实例 id */
const unapplied = ref([])

/**
 * 拆掉几个套装之后的应收。
 *
 * 在本地算而不是再问一次服务端：`original_amount` 就是「这个实例的成分按原价合计」，
 * 拆掉它应收就回到那个数。服务端在确认时会用同一套规则重算一遍，本地这个数只用于显示。
 */
const effectiveSolved = computed(() =>
  props.lots.reduce(
    (sum, lot) => (unapplied.value.includes(lot.id) ? sum + lot.original_amount - lot.price : sum),
    props.solvedAmount
  )
)

const adjustment = computed(() => effectiveSolved.value - toCents(finalYuan.value))

function reset() {
  const saved = localStorage.getItem(CHANNEL_STORAGE_KEY)
  channel.value = CHANNELS.includes(saved) ? saved : '微信'
  unapplied.value = []
  finalYuan.value = fromCents(props.solvedAmount)
}

watch(
  () => props.show,
  (val) => {
    // 每次打开都重置：上一单改过的数字或拆过的勾留在这里是真事故。
    if (val) reset()
  }
)

function toggle(lotId) {
  unapplied.value = unapplied.value.includes(lotId)
    ? unapplied.value.filter((id) => id !== lotId)
    : [...unapplied.value, lotId]
  // 勾选一变就把实收拉回新的应收。摊主的手势是「先决定套不套装，再决定让不让价」，
  // 反过来保留旧数字只会让人对着一个过期的金额点确认。
  finalYuan.value = fromCents(effectiveSolved.value)
}

function handleConfirm() {
  localStorage.setItem(CHANNEL_STORAGE_KEY, channel.value)
  emit('confirm', {
    channel: channel.value,
    finalAmount: toCents(finalYuan.value),
    unapplyLotIds: [...unapplied.value],
  })
}
</script>

<style scoped>
.price-block {
  margin-bottom: 1rem;
}
.price-row {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  padding: 2px 0;
}
.price-row.subtle {
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.price-row.subtle .struck {
  text-decoration: line-through;
}
.price-row.total {
  font-weight: 600;
  font-size: var(--font-lg);
}

.lot-block {
  margin-bottom: 1rem;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
}
.lot-title {
  margin: 0 0 0.25rem;
  font-size: var(--font-sm);
  color: var(--text-muted);
}
.lot-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 4px 0;
  cursor: pointer;
}
.lot-name {
  flex: 1;
  min-width: 0;
}
.lot-saved {
  color: var(--success-color);
  white-space: nowrap;
}
.lot-note {
  margin: 0.25rem 0 0;
  font-size: var(--font-sm);
  color: var(--warning-color);
}

.field {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.5rem;
}
.field-label {
  white-space: nowrap;
  color: var(--text-muted);
}
.field-input {
  flex: 1;
}
.adjustment {
  margin: 0 0 1rem;
  font-size: var(--font-sm);
  color: var(--accent-color);
  line-height: 1.5;
}
.adjustment-note {
  color: var(--text-muted);
}
.disclosure {
  margin: 16px 0 0;
  font-size: var(--font-sm);
  line-height: 1.5;
  color: var(--text-muted);
}
</style>
```

然后 `git rm frontend/src/components/vendor/ChannelPicker.vue`。

- [ ] **Step 2: `orderStore` 带上金额与要拆的套装**

`frontend/src/stores/orderStore.js` 的 `markOrderAsCompleted`：

```js
  async function markOrderAsCompleted(orderId, channel, finalAmount, unapplyLotIds) {
    if (!activeEventId.value) return
    // 渠道是账本「钱」那条腿的对手账户，缺了后端会 400，前端先拦一道给出人话。
    if (!channel) throw new Error('请选择收款渠道')
    try {
      const payload = { status: 'completed', channel }
      // 不传就等于 solved_amount（后端决定），所以只在真拿到数字时才带上。
      if (Number.isFinite(finalAmount)) payload.final_amount = finalAmount
      // 空数组也不传：后端对非 completed 的转换会拒绝这个字段，少传少一处可能。
      if (unapplyLotIds?.length) payload.unapply_lot_ids = unapplyLotIds
      await api.put(`/events/${activeEventId.value}/orders/${orderId}/status`, payload)
      // 更新成功后，将该订单从 pending 移到 completed
      const completedOrder = pendingOrders.value.find((order) => order.id === orderId)
      if (completedOrder) {
        completedOrders.value.unshift(completedOrder)
      }
      pendingOrders.value = pendingOrders.value.filter((order) => order.id !== orderId)
    } catch (err) {
      console.error(err)
      // 后端的错误原文比「更新订单状态失败」有用得多：实收为负、要拆的套装不属于
      // 这张订单、展会已结算，摊主看到原文才知道下一步该干什么。
      throw new Error(err.response?.data?.error || '更新订单状态失败。')
    }
  }
```

**注意**：`finalAmount` 是弹窗里那个框的值，而拆套装会改变应收。弹窗已经保证了这两者一致（`toggle()` 里把实收拉回新的应收），所以这里不需要再算一遍——**服务端是权威**，它先拆再按传来的 `final_amount` 摊折让。

- [ ] **Step 3: `VendorView` 换弹窗**

`import ChannelPicker` → `import ReceiptModal from '@/components/vendor/ReceiptModal.vue'`，模板里：

```html
    <!-- 完成配货前先确认收款：显示原价/应收/已套用的套装（可逐个拆），实收可改（spec 4.3） -->
    <ReceiptModal
      :show="showReceiptModal"
      :gross-amount="pendingOrder?.gross_amount ?? 0"
      :solved-amount="pendingOrder?.solved_amount ?? 0"
      :lots="pendingOrder?.lots ?? []"
      @confirm="onReceiptConfirm"
      @cancel="closeReceipt"
    />
```

script：

```js
// 点「完成配货」先确认收款，确认了才真正调接口。
const showReceiptModal = ref(false)
const pendingOrder = ref(null)

function completeOrder(orderId) {
  pendingOrder.value = store.pendingOrders.find((o) => o.id === orderId) || null
  showReceiptModal.value = true
}

function closeReceipt() {
  showReceiptModal.value = false
  pendingOrder.value = null
}

async function onReceiptConfirm({ channel, finalAmount, unapplyLotIds }) {
  const order = pendingOrder.value
  showReceiptModal.value = false
  if (!order) return
  try {
    await store.markOrderAsCompleted(order.id, channel, finalAmount, unapplyLotIds)
    await eventDetailStore.fetchProductsForEvent(props.id)
    await store.fetchCompletedOrders()
    message.success('已记录收款')
  } catch (error) {
    message.error(error?.message || '操作失败')
  } finally {
    pendingOrder.value = null
  }
}
```

（删掉原来的 `showChannelPicker` / `pendingOrderId` / `onChannelConfirm`。）

- [ ] **Step 4: `OrderCard` 显示套装归属与原价**

行上加一个套装标签：

```html
        <div class="item-details">
          <span class="item-name">{{ item.product_name }}</span>
          <!-- 同一个商品可能在一张订单里出现两行（2 件进套装、1 件散着，spec 4.5）。
               不标出来，摊主配货时会以为系统重复计数了。 -->
          <span v-if="item.lot_name" class="item-lot">{{ item.lot_name }}</span>
          <span class="item-price">{{ formatYuan(item.product_price) }}</span>
        </div>
```

页脚：

```html
      <span class="total-amount">
        <span v-if="order.final_amount !== order.gross_amount" class="struck">
          {{ formatYuan(order.gross_amount) }}
        </span>
        总计: {{ formatYuan(order.final_amount) }}
      </span>
```

样式：

```css
.item-lot {
  align-self: flex-start;
  padding: 0 6px;
  border-radius: var(--radius-sm);
  background-color: var(--accent-color-light);
  color: var(--accent-color);
  font-size: var(--font-xs);
  line-height: 1.6;
}
.total-amount .struck {
  margin-right: 0.5rem;
  color: var(--text-disabled);
  text-decoration: line-through;
  font-weight: 400;
}
```

- [ ] **Step 5: 过门禁**

```bash
npm run lint --prefix frontend
npm run format:check --prefix frontend
npm run test:unit --prefix frontend
npm run build --prefix frontend
grep -rn "ChannelPicker" frontend/src   # 必须没有任何命中
```

- [ ] **Step 6: 在 VNC 上走一遍**

Run: `tauri-env vnc npx tauri dev`

四条都要试到：

1. 顾客下一张带套装的单 → 摊主端 `OrderCard` 上原价被划掉、套装行有标签。
2. 点「完成配货」→ 弹窗列出套装、实收默认等于应收 → 改成更低的数，提示「手工折让 xx」。
3. **改成更高的数必须成功**，提示「手工加价 xx」，不是报错。
4. **取消勾选一个套装** → 应收当场回到原价、实收跟着变 → 确认。然后在「销售统计」里确认总额与按商品汇总之和一致。

- [ ] **Step 7: 提交**

```bash
git add -A frontend/src/components/vendor frontend/src/stores/orderStore.js frontend/src/views/VendorView.vue frontend/src/components/order/OrderCard.vue
git commit -m "$(cat <<'EOF'
feat: :lipstick: 收款确认弹窗——三个价一眼看全，套装可逐个拆

spec 4.3 的手工覆盖与辅助通道都落在这一个弹窗里。ChannelPicker 扩成
ReceiptModal：原价 / 应收 / 已套用的套装（每个一个勾）/ 默认填应收的实收框 / 渠道，
一次提交。不单开改价动作——现场的手势本来就是「报个数、收钱、点完成」。

拆套装的应收在本地算（original_amount 就是「拆掉它应收回到多少」），不多一次
往返；服务端在确认时用同一套规则重算，本地那个数只用于显示。勾选一变就把实收
拉回新的应收——摊主的手势是「先决定套不套装，再决定让不让价」。

那句「这只是记账。请先确认手机上真的收到了到账提示，再点确认。」原样保留。
弹窗现在既能改金额又能拆套装，看起来更像在处理真钱，这句话因此比以前更不能删
（spec 第 11 节，三条不可破坏项之一）。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 13: 改掉两处现在已经做不到的文档

**Files:**
- Modify: `frontend/src/views/Help.vue:373-380`、`docs/guide/workflow.md:38-45`

**为什么是 bug 而不是润色**：两处都在教用户「创建一个价格为**负数**的商品」，而 ②-1 的新 schema 有 `CHECK (unit_price >= 0)`、`api/product.rs` 也会 400。**照着帮助文档做，现在会直接失败。** 路线图 D4 点名了这两处。

- [ ] **Step 1: 改 `Help.vue`**

把「如何设置打折/满减优惠？」和「支持"捆绑销售"或"套装"吗？」两条换成：

```js
      {
        q: '如何给顾客打折？',
        a: '两条路，覆盖现场绝大多数说法：<br/><strong>① 套装</strong>——在「套装与优惠」里配好「从这几样里任选 N 件，总价 XX」，顾客的购物车会自动套用最省的那一种，并写明省了多少。<br/><strong>② 一口价</strong>——点「完成配货」时直接改「实收」。差额记成手工折让，全部算在本社团头上，帮别的社团代卖的商品仍按它们自己的定价结算。<br/>改高也可以，用于往上凑整或顺手搭了个没录入的小东西。',
      },
      {
        q: '支持“捆绑销售”或“套装”吗？',
        a: '看拆得开拆不开：<br/><strong>拆得开</strong>（5 本书装一个袋子）——在「套装与优惠」里配一个套装。账上仍然是 5 件散货，库存精准扣减，顾客买单件也不受影响。<br/><strong>拆不开</strong>（塑封礼盒，拆了就废）——当成一个独立商品录入，进出库按它自己算。',
      },
```

- [ ] **Step 2: 改 `docs/guide/workflow.md` 的「方式 B」**

```markdown
*   **方式 B：现场凑单 + 套装优惠**
    *   *场景*：您没有提前打包，是顾客买了套装后，您现场拿一本本子、拿一个挂件。
    *   *操作*：分别录入单品“新刊”和“挂件”，再到 **「套装与优惠」** 里配一个套装——候选商品选这两样、要选 2 件、填上打包价。
    *   *逻辑*：顾客在点单界面分别点“新刊”和“挂件”，购物车会自动套用这个套装并写明省了多少。单品的库存仍然精准扣减。
    *   *注意*：**套装的候选商品必须属于同一个货主。** 帮别的社团代卖的商品不能和自己的本子凑进同一个套装——那等于替他们让价。
```

- [ ] **Step 3: 确认没有残留**

```bash
grep -rn "价格设为\|价格为.*负数\|优惠/抹零" frontend/src docs/guide docs/faq docs/en docs/ja
```
Expected: 零命中。**若 `docs/en` / `docs/ja` 里有对应的英日文版本，一并改掉**——它们是线上用户看得到的页面。

- [ ] **Step 4: 过门禁并提交**

```bash
npm run lint --prefix frontend
npm run format:check --prefix frontend
npm run build --prefix frontend
npm run docs:build
git add frontend/src/views/Help.vue docs/guide/workflow.md
git commit -m "$(cat <<'EOF'
docs: :memo: 删掉「建一个负价格商品」——这招现在直接做不到了

Help.vue 和 guide/workflow.md 两处都在教用户「创建一个价格为 -5 元的
『优惠/抹零』商品」。那是 v1.1 的官方方案，而 ②-1 的新 schema 有
CHECK (unit_price >= 0)，照着帮助文档做现在会吃一个 400。路线图 D4 点名了这两处。

换成真正的两条路：套装（拆得开的包装）和收款时改实收（一口价）。
并补上「套装候选商品必须同一货主」这条硬约束——摊主在配置页吃 400 之前
应该先在文档里看到它。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## 完成标准

全部打勾才算 ②-2 完成：

- [ ] `cd src-tauri && tauri-env linux cargo fmt --all --check` 无输出
- [ ] `tauri-env linux cargo clippy --all-targets --all-features -- -D warnings` 零警告
- [ ] `tauri-env linux cargo test --all-features` 全绿，且**测试数从约 50 涨到约 92**
- [ ] `npm run lint --prefix frontend` 零警告；`format:check` / `test:unit` / `build` 全绿
- [ ] `npm run docs:build` 通过
- [ ] `tauri-env vnc npx tauri dev` 起得来，且能完整走通：
      配一个「任选 2 件」的套装 → 顾客加到 3 件、购物车显示「原价 / 已应用 −xx / 应付」 →
      下单 → 摊主端看到拆成两行、套装行有标签 → 收款弹窗改实收（**一次改低、一次改高**）→
      「销售统计」里总额与按商品汇总之和一致 → 取消一单、所有资金账户回到 0
- [ ] **拆套装那条路单独验一遍，而且必须用代卖社团的货**：给代卖社团的商品配套装 →
      顾客下单 → 收款弹窗里取消勾选 → 应收当场回到原价 → 确认后
      `社团往来:<代卖社团>` 的余额等于**原价**而不是套装价，`社团往来:<本社团>` 是 0
- [ ] `grep -rn "unit_price \* ol.qty\|unit_price\*ol.qty" src-tauri/src` 零命中
- [ ] `grep -rn "ChannelPicker" frontend/src` 零命中
- [ ] `grep -rn "价格设为\|优惠/抹零" frontend/src docs` 零命中
- [ ] `grep -rn "f64" src-tauri/src/domain` 零命中
- [ ] 这四条不变量各有一个直接断言它的测试：
      `同一 Lot 实例 Σ allocated = order_lots.price`、`同一订单 Σ allocated = solved_amount`、
      `同一订单 Σ paid = final_amount`、`按商品汇总之和 = SUM(orders.final_amount)`
- [ ] 混货主 + 手工折让那条规则有测试钉住：**整单都是代卖货时，折让仍然全额落本社团**
- [ ] 拆套装与手工折让的**区别**有测试钉住：拆 → 钱回货主；改实收 → 钱落本社团

---

## 交给 ②-3 的接口契约与已知不变量

不要改的边界是 `domain/solver.rs` 与 `domain/allocation.rs` 的任何签名、
`PricedCart::lines` 的顺序约定、`order_lines` / `order_lots` 的表结构。

### 1. 结算单用 `allocated_amount`，**不是** `paid_amount`

Task 9 把 `stats.rs` 三处「按商品汇总」改成了 `paid_amount`（顾客实付），这是**仪表盘**的口径。
**结算单（spec 第 7 节）必须用 `allocated_amount`**——货主该得多少不受摊主当天做了什么人情的影响，
这正是 4.5 要存两个金额的全部理由。两个口径在同一场展会里合法地对不上，差额恰好是手工折让，
`社团往来:<本社团>` 账户的余额就是它。

### 2. 退货（6.3）要同时用两个金额

退给顾客的默认金额按 `paid_amount`，冲销货主按 `allocated_amount`。
只用一个数，要么多退给顾客、要么让货主吃了折让——spec 4.5 明写这是「早先版本的一个真漏洞」。

### 3. 订单行会拆，同一个商品可能出现多行

`order_lines` 现在是「按 Lot 归属」拆的，不是「一个商品一行」。影响 ②-3 的两处：

- **退货 UI** 要么允许两行分别退，要么在界面上合并显示、在写入时按行分配。前者更简单也更诚实。
- 任何 `SELECT ... FROM order_lines WHERE event_product_id = ?` 的统计都必须 `SUM`，不能假设只有一行。

### 4. 拆套装（`unapply_lot_ids`）和手工折让是两回事，别合并

`PUT .../status` 上这两个字段看起来都在「改这单要多少钱」，但**记账语义完全相反**：

| | 拆套装 | 改实收 |
|---|---|---|
| 性质 | 纠错——这个套装不该套用 | 摊主自己决定的让价 |
| 钱归谁 | 回到**真正的货主**（`allocated` 恢复成 `unit_price × qty`） | **本社团全额承担**（spec 4.4） |
| 动了什么 | `order_lines.allocated` / `order_lots` / `orders.solved_amount` | 只动 `order_lines.paid` 和一条资金腿 |

执行顺序是**先拆后摊**，手工折让相对的是拆完之后的 `solved_amount`。
②-3 做退货时如果要让摊主「退货的同时拆掉套装」，得按同样的顺序走，
不能套用退货自己的那套比例。

### 5. 「订单是 completed」仍然不蕴含「存在收款 journal」

②-1 交接段第 1 条原样成立，而且 ②-2 之后多了一种情形：**整单被抹成 0 元**时
（`final_amount = 0`），各货主的腿和折让腿金额相抵，实收净入为 0——但 journal 本身存在。
反之全赠品单（`solved = 0` 且 `final = 0`）连 journal 都没有。
**按 `orders.status` 判定已收款订单，不要按 journal 存在性。**

### 6. `events.status` 的写入守卫：新增的补了，既有的 4 个仍然敞着

`api/lot.rs` 的四个写入口（含 `/quote`）都查了状态。但 ②-1 列出的 4 个既有敞口
**一个都没补**，冻结语义仍然归 ②-3：

- `api/order.rs` 的 `update_order_status`（完成 / 取消）
- `api/product.rs` 的 `add_product_to_event`
- `api/product.rs` 的 `restock_product`
- `api/product.rs` 的 `update_product`

### 7. 求解器的三条上限是硬编码常量

`MAX_UNITS = 300` / `MAX_STATE_SPACE = 200_000` / `MAX_STEPS = 2_000_000`
（`domain/solver.rs` 顶部）。超限返回 `400「购物车商品过多，无法自动计算优惠，请分单结算」`。
②-3 若做「摊主代客下大单」或批量补录，要么分单、要么先动这三个数并补一个压力测试——
**不要改成「预算内尽力搜」**，spec 第 10 节明确把那条路砍掉了。

### 8. `channel` 的约束（②-1 交接段原样成立）

`channel` 仍是无约束自由文本，而它会成为账户名的一部分（`实收-<渠道>`）。
②-3 做「自定义渠道」时必须同时做三件事：写库前规范化、长度上限、
**给前端一个 `SELECT DISTINCT` 的已用渠道列表让摊主从已有的里挑**——第三条才是真正防
「微信」和「微信支付」分裂成两个账户的那一条。不要加白名单。

### 9. 留给 ④（外观与信息架构）的一件事：从上一场展会复制套装配置

套装的作用域是展会（spec 4.1），所以每开一场新展会都要重配一遍。技术上复用是通的——
`event_products` 带 `master_product_id`，上一场的候选集能按全局商品映射到这一场。

**但不要单给套装长一个「从上一场复制」按钮。** 它会牵出「上一场的价格变了怎么提示」
「商品这一场没上架怎么办」这类问题，而这些问题在「选品」那一步是同一批问题。
④ 本来就要重做展会的准备流程，这两件事该一起设计。

### 10. 本 plan 知情留下的三件事

1. **前端仍然没有 store / 组件测试。** ②-2 只加了 `utils/quote.spec.js` 一个纯函数 spec。
   `customerStore` 的 debounce + 请求序号那段逻辑目前只靠真机验证，没有自动化覆盖。
2. **`/quote` 没有速率限制**，只有规模上限。公开未鉴权端点上单次请求的 CPU 被封住了，
   但「每秒几百个合法请求」这一维没管。离线局域网场景下可接受，接公网就不行——
   而接公网本来就违反产品定位（路线图「不能破坏的三件事」第 1 条）。
3. **`AdminEventOrders.vue` 没改。** 它仍然只显示 `final_amount`，看不到原价和套装。
   ④ 的 IA 重设计会整体重做订单管理页，现在改一半是浪费。
