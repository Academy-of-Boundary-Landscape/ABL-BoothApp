# ②-1 账本内核与「卖一单」 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把库存与资金从单边扣减换成移动账本——落全量新 schema，建 journal + 两张移动表的写入内核，把「下单·完成收款·取消冲正」三条路径迁到新模型上，前端接线到金额用分、订单要录收款渠道，交付一个**可用但还没有 Lot 和折扣**的应用。

**Architecture:** 账本是唯一真相，`current_stock` 这个缓存字段被删除，余额改为对 `stock_movements` 实时聚合。写入统一走 `domain::ledger::post_journal`，每条移动的 `from`/`to` 都非空，「不平衡的分录」在结构上写不出来。数据库层用 CHECK 约束把「completed 必须有渠道」「一个 journal 只能被冲正一次」「from ≠ to」这类不变量钉死，不指望应用层自觉。金额全部改成**整数分**（`Money(i64)`），因为本子项目后续两份 plan 的核心断言全是「各行之和必须等于总价」，浮点下写不出来。

**Tech Stack:** Rust 1.98.1 / axum 0.7 / sqlx 0.9（sqlite，运行时 `migrate!()`，无编译期宏）/ tower 0.4 / Vue 3 + Pinia + Vite / vitest

**Spec:** `docs/superpowers/specs/2026-09-22-double-entry-domain-design.md`
**Umbrella:** `docs/superpowers/specs/2026-09-22-v1.2-roadmap.md`（含 ① 执行后的附录）

---

## 这份 plan 在 ② 里的位置

② 被纵切成三份 plan，每份各自产出可运行、可测的软件：

| plan | 内容 | 状态 |
|---|---|---|
| **②-1（本文）** | 全量新 schema、账本内核、社团与归属、进货、下单·收款·取消、前端接线 | 本次执行 |
| ②-2 | Lot、最优折扣求解器、分摊（`allocated`/`paid`）、手工覆盖、混货主规则 + 配置与购物车 UI | 之后 |
| ②-3 | 补货之外的展会闭环（盘点/带回/冻结/清 pending）、赠品损耗、退货 UI、垫付调整、结算单与导出 | 之后 |

**schema 一次到位，不分三批。** 12 张表在本 plan 的 Task 2 全部建好（含 ②-2 的 `lots`/`lot_candidates`/`order_lots`、②-3 的 `refunds`/`advances`/`settlement_adjustments`），后两份 plan 只往里写数据、不改表结构。理由：SQLite 改表昂贵且易错，而 spec 附录已经把 schema 设计完了，分批建表只会带来三次迁移和两次返工。

### 与 ③a 的关系（对路线图执行顺序的一处偏离）

路线图把「③a handler 类型化」列为 ② 的前置。本 plan **只提取 ③a 的基建部分**（`ApiError` + 统一响应信封，Task 1），② 写的每个新 handler 一律用它；`auth` / `master_product` / `admin` / `sync` / `info` / `vision` 这些 ② 不改的 route 维持现状，等 ③a 本体统一收口。

理由：③a 原本要收口的 route 里，`order.rs` / `product.rs` / `stats.rs` 会被 ② 整个重写，先对它们做 ③a 等于白做。路线图给出的「③a 必须在 ② 之前」的真实理由是「不先收口，②的新领域概念会继续以无类型 JSON 的形式漏出去」——这个理由被 Task 1 完整满足了。

---

## Global Constraints

- **分支**：全部工作在 `1.2-dev` 上。不要合并到 `main`。
- **本机构建必须走 `tauri-env`**（见 `CLAUDE.md`）：`tauri-env linux cargo <...>`。裸跑 `cargo` 会因为缺 webkit2gtk 失败。
- **`cargo` 命令一律在 `src-tauri/` 下跑**。仓库根没有 `Cargo.toml`。
- **rustc 钉死 1.98.1**（`rust-toolchain.toml`）。
- **CI 的三条门禁命令必须全绿**（`.github/workflows/ci.yml` 的 rust job，`working-directory: src-tauri`）：
  ```
  cargo fmt --all --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --all-features
  ```
  注意 clippy 带 `--all-targets`，**测试代码也吃 `-D warnings`**。
- **前端门禁**：`npm run lint --prefix frontend`（eslint `--max-warnings 0`）、`format:check`、`test:unit`、`build`。
- **金额一律整数分**（`Money(i64)`）。数据库列是 `INTEGER`，API 传的 JSON 是整数分，前端显示时才除以 100。**任何地方不得出现 `f64` 表示金额。**
- **错误响应体必须保持 `{"error": "..."}` 形状**。`frontend/src/services/api.js` 全仓在读 `err.response?.data?.error`，换形状会让所有错误提示变成 undefined。
- **全仓没有编译期 sqlx 宏**（已验证 `query!` / `query_as!` 零命中），所以 **schema 改动不会引发编译错误，只会在运行时炸**。每一处引用旧表旧列的 SQL 字符串都必须手工找出来改掉，Task 7 给了完整清单。
- **注释写中文，解释 why 而不是 what**，匹配仓库既有风格（`//!` 模块级说明设计取舍、`///` 说明为什么必须这么做、行内注释记录踩过的坑和「改这里要同步改哪里」）。
- **不碰这两个文件**：`my-release-key.jks`、`src-tauri/updater-key.key`。
- **不引入新的运行时依赖**。`thiserror` 已在 `Cargo.toml`（目前只有 `utils/cert.rs` 用了一处）。
- **不做**（留给 ②-2 / ②-3）：Lot、折扣求解器、分摊算法、手工覆盖、退货、盘点、带回、冻结、清 pending 阻断、赠品、损耗、拆封/转换、垫付、结算调整、结算单、导出重做。
  这些概念对应的**表和枚举值在 Task 2 / Task 3 就建好**（`Location::Conversion`、`JournalKind::Scrap` 等），只是没有写入路径。
- **不得让复式记账制造出「钱已到账」的错觉**（spec 第 11 节第 2 条，是三条不可破坏项之一）。
  本 plan 让「确认收款」开始要求选渠道并记一笔资金移动，这**看起来**比以前更像钱真的到账了——
  而支付依然是非闭环的，系统从来不知道顾客有没有真付。Task 8 Step 3 必须同步改文案，
  这不是可选的润色。
- 提交信息用仓库现有的 gitmoji 风格，结尾带
  `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>`。

### 三条本 plan 期间知情接受的状态

1. **Task 2 之后到 Task 7 之前，`/api/events/:id/products`、`/api/events/:id/orders`、`/api/events/:id/sales_summary` 在运行时是坏的**（SQL 引用了被删的表）。`cargo test` 全程必须绿——坏的是那些没有测试覆盖的老路径。Task 7 结束时应用重新完整可用。这是大爆炸式 schema 替换不可避免的窗口，不是疏忽。
2. **`cargo test --no-default-features` 本来就是坏的**（`test_support.rs` 无条件 `use crate::vision::VisionRuntime`，而 `lib.rs:195` 也有同类问题）。这是 ① 留下的、路线图附录预言过的腐烂，**不在本 plan 范围内**，不要顺手修。CI 跑的是 `--all-features`。
3. **`socketService.js` 是死代码**（全仓零 import），实时性靠 3s + 5s 双轮询。本 plan 不碰它。

---

## File Structure

**新建**

| 文件 | 职责 |
|---|---|
| `src-tauri/migrations/202609230001_double_entry_schema.sql` | 一次性 schema 切换：建 12 张新表，删旧业务表 |
| `src-tauri/src/error.rs` | `ApiError` + `ApiResult`，③a 基建 |
| `src-tauri/src/domain/mod.rs` | 领域层模块根 |
| `src-tauri/src/domain/money.rs` | `Money(i64)`，单位分 |
| `src-tauri/src/domain/ledger.rs` | `Location` / `Account` / `JournalKind` 枚举，`post_journal` / `reverse_journal`，余额聚合 |
| `src-tauri/src/api/society.rs` | 社团 CRUD |
| `frontend/src/utils/money.js` | `fromCents` / `toCents` / `formatCents`，唯一的金额显示入口 |
| `frontend/src/utils/money.spec.js` | 上者的单测 |
| `frontend/src/stores/societyStore.js` | 社团列表 |
| `frontend/src/views/AdminSocieties.vue` | 社团管理页（最小实现） |

**修改**

| 文件 | 改什么 |
|---|---|
| `src-tauri/src/lib.rs` | 挂 `mod error;` `mod domain;` |
| `src-tauri/src/test_support.rs` | 抽 `TEST_JWT_SECRET` 常量；加 `admin_token()` / `vendor_token()` / `json_request()` / `read_json()` helper |
| `src-tauri/src/db/models.rs` | 删 `Product` / `Order` / `OrderItem` / `CreateOrder*DTO`；加新 model |
| `src-tauri/src/db/mod.rs` | `init_db` 里加一次性 v1 备份 |
| `src-tauri/src/api/mod.rs` | 挂 `society::router()` |
| `src-tauri/src/api/product.rs` | 全量重写到 `event_products` + 进货移动 |
| `src-tauri/src/api/order.rs` | 全量重写到移动账本 |
| `src-tauri/src/api/stats.rs` | 查询迁到新表；金额改分 |
| `src-tauri/src/api/event.rs` | 状态值改 `筹备/进行中/已结算`；`delete_event` 的级联删除改表名 |
| `src-tauri/src/api/admin.rs` | `tables_to_clear` 清单改成新表 |
| `src-tauri/src/api/sync.rs` | `.boothpack` 带社团归属 |
| `src-tauri/src/vision/store.rs` | `JOIN products` → `JOIN event_products` |
| `frontend/src/stores/customerStore.js` | 库存字段改名；金额改分 |
| `frontend/src/stores/orderStore.js` | 完成订单要带渠道；金额字段改名 |
| `frontend/src/stores/eventDetailStore.js` | event product 字段改名；进货入口 |
| `frontend/src/views/CustomerView.vue`、`components/customer/{ProductGrid,ShoppingCart,PaymentModal}.vue`、`views/VendorView.vue`、`components/vendor/LiveStats.vue`、`components/order/OrderCard.vue`、`views/AdminEventProducts.vue`、`views/AdminEventOrders.vue`、`components/event/EventList.vue` 等 | 字段改名 + 金额格式化 + 展会状态文案，逐处清单见 Task 8 |

---

## Task 1: `ApiError` 基建、`Money` 类型、测试夹具补强

这一步不改任何现有行为，只把后面每个 task 都要用的三样东西准备好。**先做它的理由**：后面 6 个 task 每一个都要写 HTTP 测试，而现在「造一个合法 token」这个能力在仓库里根本不存在（唯一的鉴权测试走的是「不给 token → 401」的负路径）。

**Files:**
- Create: `src-tauri/src/error.rs`
- Create: `src-tauri/src/domain/mod.rs`, `src-tauri/src/domain/money.rs`
- Modify: `src-tauri/src/lib.rs`（挂模块）
- Modify: `src-tauri/src/test_support.rs`

**Interfaces（后续 task 全部依赖这里，签名不要改）:**
- Produces: `crate::error::{ApiError, ApiResult}`；`ApiError::{BadRequest, NotFound, Conflict, Forbidden, Db}`，`ApiError: From<sqlx::Error>`
- Produces: `crate::domain::money::Money`，`Money::from_cents(i64) -> Money`、`Money::cents(&self) -> i64`、`Money::ZERO`、`impl Add/Sub/Neg/Sum/Display`、`Money::checked_mul_qty(i64) -> Option<Money>`
- Produces: `crate::test_support::{TEST_JWT_SECRET, admin_token, vendor_token, json_request, read_json}`

- [ ] **Step 1: 写 `src-tauri/src/error.rs`**

```rust
//! 统一的 API 错误类型。
//!
//! 这是 ③a「48 个 route 的响应与错误收口成具名类型」的**基建部分**，提前到 ② 来做：
//! ② 写的每个新 handler 一律用它，② 不改的老 route 维持现状，等 ③a 本体统一收。
//! 为什么这么切，见 plan 头部「与 ③a 的关系」。
//!
//! **响应体形状必须是 `{"error": "..."}`**：前端 `frontend/src/services/api.js` 的
//! 自定义 adapter 会把非 2xx 响应手工包成 axios 风格的 `error.response`，而全仓到处在读
//! `err.response?.data?.error`。换形状会让所有错误提示静默变成 undefined。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// 请求本身不合法：字段缺失、数量为负、枚举值不认识。
    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    NotFound(String),

    /// 业务状态冲突：库存不足、展会已冻结、订单已取消。
    /// 与 BadRequest 的区别是「请求没毛病，是世界的状态不允许」。
    #[error("{0}")]
    Conflict(String),

    #[error("权限不足")]
    Forbidden,

    /// sqlx 的原始错误**绝不进响应体**——它可能带表名、列名甚至具体值。
    /// 只写 stderr，客户端统一看到「数据库错误」。
    #[error("数据库错误")]
    Db(#[from] sqlx::Error),
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // 数据库错误在这里落日志——handler 里不需要再各自 eprintln! 一遍。
        if let ApiError::Db(ref e) = self {
            eprintln!("[api] database error: {e}");
        }
        (self.status(), Json(json!({ "error": self.to_string() }))).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
```

- [ ] **Step 2: 写 `src-tauri/src/domain/money.rs`**

```rust
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

    pub fn abs(self) -> Self {
        Money(self.0.abs())
    }

    /// 单价 × 数量。溢出返回 None——摊位场景溢不了，但把它变成一个
    /// 调用方必须处理的 Option，好过某天真的溢出时悄悄绕回负数。
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
```

- [ ] **Step 3: 写 `src-tauri/src/domain/mod.rs`，并在 `lib.rs` 挂上模块**

`src-tauri/src/domain/mod.rs`——**本 task 只写这些**，`pub mod ledger;` 由 Task 3 追加（`ledger.rs` 那时才存在，现在写上去编译不过）：
```rust
//! 领域层：账本、金额、位置与账户。
//!
//! 这一层**不认识 axum，也不认识 Tauri**，只认识 sqlx 的 Executor/Transaction。
//! 把它单独拎出来是为了 ②-2 的折扣求解器能作为纯函数测试。

pub mod money;
```

`src-tauri/src/lib.rs`：在现有 `mod` 声明区（`#[cfg(test)] mod test_support;` 附近）加：
```rust
mod domain;
mod error;
```

- [ ] **Step 4: 给 `money.rs` 写单测（追加在文件末尾）**

```rust
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
        assert_eq!(serde_json::to_string(&Money::from_cents(1990)).unwrap(), "1990");
        let back: Money = serde_json::from_str("1990").unwrap();
        assert_eq!(back, Money::from_cents(1990));
    }

    #[test]
    fn checked_mul_qty_reports_overflow_instead_of_wrapping() {
        assert_eq!(Money::from_cents(3000).checked_mul_qty(2), Some(Money::from_cents(6000)));
        assert_eq!(Money::from_cents(i64::MAX).checked_mul_qty(2), None);
    }
}
```

- [ ] **Step 5: 补强 `src-tauri/src/test_support.rs`**

把写死的 `"test-secret"` 抽成常量，并加四个 helper。**改 `test_state()` 里那行字面量为常量引用。**

在文件顶部 `use` 之后加：
```rust
/// `test_state()` 注入 AppState 的 JWT 密钥。造 token 的 helper 必须用同一个值，
/// 所以抽成常量而不是各处抄字面量。
pub const TEST_JWT_SECRET: &str = "test-secret";
```
把 `test_state()` 里的 `jwt_secret: "test-secret".to_string(),` 改成 `jwt_secret: TEST_JWT_SECRET.to_string(),`。

在文件末尾追加：
```rust
use crate::utils::security::create_jwt;
use axum::body::Body;
use axum::http::Request;

/// 管理员 token（`role = "admin"`，全局访问）。
pub fn admin_token() -> String {
    create_jwt("admin", "all", None, TEST_JWT_SECRET).expect("sign admin jwt")
}

/// 摊主 token，限定在某一场展会上。
///
/// 注意 `role`/`access` 的组合语义（见 `api/order.rs` 的 `check_read_permission`）：
/// `vendor` + `all` 是全局摊主，`vendor` + `event` 才受 `event_id` 限制。
pub fn vendor_token(event_id: i64) -> String {
    create_jwt("vendor", "event", Some(event_id), TEST_JWT_SECRET).expect("sign vendor jwt")
}

/// 构造一个带 JSON body 的请求。`token` 为 None 时不加 Authorization 头。
///
/// 既有的两个测试是手抄 `Request::builder()` 的；新模型下每个 task 都要发好几个
/// 请求，抄六七遍 builder 只会让 diff 难读。
pub fn json_request(method: &str, uri: &str, token: Option<&str>, body: serde_json::Value) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(t) = token {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    b.body(Body::from(body.to_string())).expect("build request")
}

/// 把响应 body 读成 JSON。axum 0.7 的 `to_bytes` 必须带 limit 参数。
pub async fn read_json(res: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("read body");
    if bytes.is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::from_slice(&bytes).expect("parse json body")
}

/// 同 `test_router()`，但把 pool 也还回来。
///
/// Task 5 / Task 6 的测试要直接查账本余额做断言——拿不到 pool 就只能靠 HTTP 反推，
/// 断言力弱很多（「响应里写着 8」和「账本聚合出来确实是 8」不是一回事）。
pub async fn test_router_with() -> (Router, TempDir, SqlitePool) {
    let (state, dir) = test_state().await;
    let pool = state.db.clone();
    let router = Router::new()
        .nest("/api", crate::api::router())
        .with_state(state);
    (router, dir, pool)
}

/// 种一场「进行中」的展会 + 两个商品，各带 10 / 5 件进货，返回
/// `(event_id, event_product_a, event_product_b)`。
///
/// - A 归**本社团**（id 1），单价 3000 分
/// - B 归**代卖社团「黄昏堂」**（id 2），单价 2000 分
///
/// 混货主是刻意的：按货主拆分收款是新模型的核心行为之一，单货主的夹具测不出来。
/// 直接写 SQL 而不是打 API，是因为建全局商品走的是 multipart 接口，
/// 构造成本高且不是这些测试的被测对象。
pub async fn seed_event_and_product(pool: &SqlitePool) -> (i64, i64, i64) {
    sqlx::query("INSERT INTO societies (id, name, is_home) VALUES (2, '黄昏堂', 0)")
        .execute(pool)
        .await
        .expect("seed society");
    sqlx::query(
        "INSERT INTO master_products (id, product_code, name, default_price, owner_society_id)
         VALUES (1, 'A', '本子A', 30.0, 1), (2, 'B', '本子B', 20.0, 2)",
    )
    .execute(pool)
    .await
    .expect("seed master products");
    sqlx::query(
        "INSERT INTO events (id, name, event_date, status)
         VALUES (1, 'ABC漫展', '2026-10-01', '进行中')",
    )
    .execute(pool)
    .await
    .expect("seed event");
    sqlx::query(
        "INSERT INTO event_products
           (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
         VALUES (1, 1, 1, 1, 'A', '本子A', 3000),
                (2, 1, 2, 2, 'B', '本子B', 2000)",
    )
    .execute(pool)
    .await
    .expect("seed event products");

    // 开场带货：A 10 件、B 5 件。走账本而不是直接塞 stock_movements，
    // 这样夹具本身也在守 post_journal 的行为。
    let mut tx = pool.begin().await.expect("begin");
    crate::domain::ledger::post_journal(
        &mut tx,
        1,
        crate::domain::ledger::JournalKind::Restock,
        None,
        None,
        Some("夹具：开场带货"),
        &[
            crate::domain::ledger::StockLeg {
                event_product_id: 1,
                from: crate::domain::ledger::Location::External,
                to: crate::domain::ledger::Location::OnSite,
                qty: 10,
            },
            crate::domain::ledger::StockLeg {
                event_product_id: 2,
                from: crate::domain::ledger::Location::External,
                to: crate::domain::ledger::Location::OnSite,
                qty: 5,
            },
        ],
        &[],
    )
    .await
    .expect("seed restock");
    tx.commit().await.expect("commit");

    (1, 1, 2)
}
```

> **`test_router_with` 和 `seed_event_and_product` 在 Task 1 就写好，但 `seed_event_and_product`
> 依赖 Task 3 的 `domain::ledger`。** 所以本 task 先只加 `test_router_with`，
> `seed_event_and_product` 在 **Task 3 结束时补上**（那时 `ledger` 已存在）。
> Task 5 / Task 6 直接用。

> **`oneshot` 消费 router。** 一个 `Router` 只能发一个请求。后续所有多步测试必须
> `let (router, _dir) = test_router().await;` 之后对每个请求用 `router.clone()`。
> 这是既有那两个单请求测试没暴露的限制，**每个 task 的测试都会踩**。
> `TempDir` 必须绑到 `_dir` 这种有名变量，写 `_` 会立即 drop 把目录删掉。

- [ ] **Step 6: 跑门禁，必须全绿**

```bash
cd src-tauri
tauri-env linux cargo fmt --all --check
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
```
Expected: 原有 20 个测试 + 新增 4 个 money 测试 = 24 passed。

> 若 clippy 报 `test_support` 里的 helper 未使用：它们是 `pub`，在 `#[cfg(test)] mod` 里
> `pub` 项不会触发 dead_code。真报了就说明模块没挂对。

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/error.rs src-tauri/src/domain src-tauri/src/lib.rs src-tauri/src/test_support.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: ApiError 基建、Money 整数分类型、测试夹具补 token helper

③a 的基建部分提前到 ② 做：② 写的新 handler 一律用 ApiError，老 route 维持现状。
Money 用整数分而不是 f64——②-2 的分摊断言「各行之和 = 总价」在浮点下写不出来。
test_support 加 admin_token/vendor_token/json_request/read_json，此前仓库里
根本没有「造一个合法 token」的能力（唯一的鉴权测试走的是无 token 的负路径）。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: 全量新 schema 迁移 + 一次性 v1 备份

**Files:**
- Create: `src-tauri/migrations/202609230001_double_entry_schema.sql`
- Modify: `src-tauri/src/db/mod.rs`（`init_db` 加 v1 备份）
- Modify: `src-tauri/src/db/models.rs`（删旧 model，加新 model）

**Interfaces:**
- Consumes: `crate::domain::money::Money`（Task 1）
- Produces: 表 `societies` / `event_products` / `lots` / `lot_candidates` / `orders` / `order_lots` / `order_lines` / `journals` / `stock_movements` / `money_movements` / `refunds` / `advances` / `settlement_adjustments`
- Produces: `crate::db::models::{Society, EventProduct, OrderRow, OrderLineRow}`

**这份 DDL 已经在真实老库上跑通过**（含 5 个既有迁移建出的表 + 真实数据行），`PRAGMA foreign_key_check` 干净，`master_products` 保留、`events` 清空。下面三条是实测出来的硬约束，照抄时不要"优化"掉：

1. **`ALTER TABLE ... ADD COLUMN ... REFERENCES` 在 `foreign_keys=ON` 下必须默认 NULL**
   （实测报错：`Cannot add a REFERENCES column with non-NULL default value`）。
   所以 `master_products.owner_society_id` 只能写 `INTEGER NOT NULL DEFAULT 1`，**不带 REFERENCES**。
   `init_db` 里 `pragma("foreign_keys","ON")`，sqlx 默认也是 ON，躲不掉。
2. **`is_home` 用偏唯一索引** `WHERE is_home = 1` 限制「本社团有且只有一个」。
   普通唯一索引会让第二个非本社团也插不进去。
3. **删表顺序**：`order_items` → `orders` → `products`（子表在前），否则 FK 拦截。

- [ ] **Step 1: 写迁移文件 `src-tauri/migrations/202609230001_double_entry_schema.sql`**

```sql
-- ② 领域模型：库存与资金的复式账
--
-- 这是一次性的大爆炸切换，不是增量加列。依据是路线图 D1「历史数据知情清零」：
-- 老用户的展会、订单、order_items、当前库存**不迁移**到新模型。
--
-- 保留什么、丢什么：
--   保留 settings（管理员/摊主密码）、master_products（全局商品库，加一列归属）、
--        master_product_images / image_embeddings / vision_index_meta（AI 识别资产）
--   丢弃 events 的行（表结构保留并加列）、products、orders、order_items
--
-- 用户侧的兜底由 src-tauri/src/db/mod.rs 的一次性 v1 备份负责（sale_system.db.v1-backup），
-- 外加 ① 建好的迁移前快照 + 失败回滚（db/snapshot.rs）。

-- ========== 社团：货主的单位 ==========
-- 账户名和外键都用 id 而不是名字，因为社团会改名（spec 3.1）。
CREATE TABLE societies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    is_home INTEGER NOT NULL DEFAULT 0 CHECK (is_home IN (0, 1)),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 「本社团」有且只有一个。偏索引是关键：普通 UNIQUE 会让第二个 is_home=0 也插不进去。
CREATE UNIQUE INDEX idx_societies_home ON societies(is_home) WHERE is_home = 1;

INSERT INTO societies (id, name, is_home) VALUES (1, '本社团', 1);

-- ========== 商品库加归属（默认值，选品时会被快照到 event_products）==========
-- 故意不写 REFERENCES societies(id)：SQLite 在 foreign_keys=ON 下拒绝
-- 「带 REFERENCES 且默认值非 NULL」的 ADD COLUMN（实测报错）。改成可空 + UPDATE
-- 又会让 NOT NULL 语义丢失，权衡之后这一列不做 FK 约束，由应用层保证。
ALTER TABLE master_products ADD COLUMN owner_society_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX idx_master_products_owner ON master_products(owner_society_id);

-- ========== 丢弃旧业务表（子表在前，否则 FK 拦截）==========
DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS products;
DELETE FROM events;

-- ========== 展会 ==========
-- 状态值从 未进行/进行中/已结束 改成 筹备/进行中/已结算（spec 6.4）。
-- events 的行刚被清空，所以这里不需要数据转换，只需要把默认值改对。
ALTER TABLE events ADD COLUMN stocktake_skipped INTEGER NOT NULL DEFAULT 0
    CHECK (stocktake_skipped IN (0, 1));

-- ========== 摊位商品 ==========
-- 没有 current_stock —— 可售余额是 SUM(到现场仓) − SUM(离现场仓)，
-- 不存在第二个可以漂移的数字。这是整个设计最直接的体现。
-- owner_society_id 是选品时从 master_products 抄来的**快照**：否则展会结算完
-- 之后有人改了全局商品库的归属，冻结的账就跟着变。
CREATE TABLE event_products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    master_product_id INTEGER NOT NULL,
    owner_society_id INTEGER NOT NULL,
    product_code TEXT NOT NULL,          -- 冗余快照，防商品库改名后账面混乱
    name TEXT NOT NULL,                  -- 同上
    unit_price INTEGER NOT NULL CHECK (unit_price >= 0),  -- 分
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE,
    FOREIGN KEY (master_product_id) REFERENCES master_products(id),
    FOREIGN KEY (owner_society_id) REFERENCES societies(id),
    UNIQUE (event_id, master_product_id)
);
CREATE INDEX idx_event_products_event ON event_products(event_id);

-- ========== Lot（②-2 才写数据，表先建好）==========
-- 候选集必须同一货主，由应用层在配置时校验（spec 4.1）；SQL 层表达不了。
CREATE TABLE lots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    pick_count INTEGER NOT NULL CHECK (pick_count > 0),
    total_price INTEGER NOT NULL CHECK (total_price >= 0),
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
);
CREATE TABLE lot_candidates (
    lot_id INTEGER NOT NULL,
    event_product_id INTEGER NOT NULL,
    PRIMARY KEY (lot_id, event_product_id),
    FOREIGN KEY (lot_id) REFERENCES lots(id) ON DELETE CASCADE,
    FOREIGN KEY (event_product_id) REFERENCES event_products(id) ON DELETE CASCADE
);

-- ========== 订单 ==========
-- 没有 journal_id —— 反过来，journals.order_id 指向订单。一个订单从下单到退货
-- 有多个 journal（下单记货、完成记钱、取消冲正），单向外键放订单上装不下。
CREATE TABLE orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'completed', 'cancelled')),
    channel TEXT,                                   -- 收款渠道，完成时才有
    gross_amount INTEGER NOT NULL,                  -- 原价合计（分）
    solved_amount INTEGER NOT NULL,                 -- 求解器价；②-1 恒等于 gross
    final_amount INTEGER NOT NULL,                  -- 手工覆盖后；②-1 恒等于 solved
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE,
    -- spec 6.2：结算时必须录收款渠道，否则钱那条腿没有对手账户。
    -- 放在 DB 层是因为这条不变量一旦漏掉，账面要到收摊对账才发现。
    CHECK (status <> 'completed' OR channel IS NOT NULL)
);
CREATE INDEX idx_orders_event_status ON orders(event_id, status);

-- Lot 在一单里的**一次**套用。同一个 Lot 套两次 = 两行，各自分摊。
-- lot_id 可空且 ON DELETE SET NULL：Lot 配置被删掉不该让历史订单跟着没。
CREATE TABLE order_lots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    lot_id INTEGER,
    name TEXT NOT NULL,                             -- Lot 名字的快照
    price INTEGER NOT NULL CHECK (price >= 0),      -- Lot 总价的快照
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (lot_id) REFERENCES lots(id) ON DELETE SET NULL
);
CREATE INDEX idx_order_lots_order ON order_lots(order_id);

-- 订单行。粒度是「商品 × Lot 实例」：同一商品 5 件里 3 件进 Lot、2 件原价 ⇒ 两行。
-- 两个金额的分工见 spec 4.5：
--   allocated_amount = 这一行的**货主应得**（Lot 分摊后、手工折让前）
--   paid_amount      = **顾客为这一行实付**（手工折让摊入后）
-- ②-1 里没有 Lot 也没有折让，两者恒等于 unit_price × qty；②-2 才会让它们分道。
CREATE TABLE order_lines (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    event_product_id INTEGER NOT NULL,
    order_lot_id INTEGER,
    qty INTEGER NOT NULL CHECK (qty > 0),
    unit_price INTEGER NOT NULL CHECK (unit_price >= 0),
    allocated_amount INTEGER NOT NULL CHECK (allocated_amount >= 0),
    paid_amount INTEGER NOT NULL CHECK (paid_amount >= 0),
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (event_product_id) REFERENCES event_products(id),
    FOREIGN KEY (order_lot_id) REFERENCES order_lots(id) ON DELETE SET NULL
);
CREATE INDEX idx_order_lines_order ON order_lines(order_id);

-- ========== 账本：一次业务操作 ==========
CREATE TABLE journals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    kind TEXT NOT NULL,   -- 进货|销售|收款|退货|取消|赠送|报废|盘点|带回|拆封|垫付|调整
    occurred_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    order_id INTEGER,                    -- 非空 = 属于某个订单
    reverses_journal_id INTEGER,         -- 冲正时指向原 journal
    note TEXT,
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (reverses_journal_id) REFERENCES journals(id)
);
CREATE INDEX idx_journals_event ON journals(event_id, kind);
CREATE INDEX idx_journals_order ON journals(order_id);

-- 一个 journal 只能被冲正一次。没有这条约束，一次网络重试就能把库存退两遍。
CREATE UNIQUE INDEX idx_journals_reverses
    ON journals(reverses_journal_id) WHERE reverses_journal_id IS NOT NULL;

-- ========== 货的移动：from/to 均非空 ⇒ 结构上不可能不平衡 ==========
CREATE TABLE stock_movements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    journal_id INTEGER NOT NULL,
    event_product_id INTEGER NOT NULL,
    from_location TEXT NOT NULL,
    to_location TEXT NOT NULL,
    qty INTEGER NOT NULL CHECK (qty > 0),
    FOREIGN KEY (journal_id) REFERENCES journals(id) ON DELETE CASCADE,
    FOREIGN KEY (event_product_id) REFERENCES event_products(id),
    CHECK (from_location <> to_location)
);
CREATE INDEX idx_stock_movements_product ON stock_movements(event_product_id);
CREATE INDEX idx_stock_movements_journal ON stock_movements(journal_id);

-- ========== 钱的移动：同上 ==========
-- 账户是字符串：'摊主自有' | '实收-<渠道>' | '社团往来:<society_id>' | '结算调整' | '对账差异'
-- 不做外键，因为 '实收-微信'、'摊主自有' 这些不挂任何社团。
CREATE TABLE money_movements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    journal_id INTEGER NOT NULL,
    from_account TEXT NOT NULL,
    to_account TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK (amount > 0),
    FOREIGN KEY (journal_id) REFERENCES journals(id) ON DELETE CASCADE,
    CHECK (from_account <> to_account)
);
CREATE INDEX idx_money_movements_journal ON money_movements(journal_id);
CREATE INDEX idx_money_movements_from ON money_movements(from_account);
CREATE INDEX idx_money_movements_to ON money_movements(to_account);

-- ========== 退货（②-3 才写数据）==========
CREATE TABLE refunds (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_line_id INTEGER NOT NULL,
    journal_id INTEGER NOT NULL,
    qty INTEGER NOT NULL CHECK (qty > 0),
    allocated_amount INTEGER NOT NULL CHECK (allocated_amount >= 0),
    paid_amount INTEGER NOT NULL CHECK (paid_amount >= 0),
    refund_amount INTEGER NOT NULL CHECK (refund_amount >= 0),
    channel TEXT NOT NULL,
    destination TEXT NOT NULL,           -- 现场仓|损耗
    FOREIGN KEY (order_line_id) REFERENCES order_lines(id) ON DELETE CASCADE,
    FOREIGN KEY (journal_id) REFERENCES journals(id) ON DELETE CASCADE,
    -- spec 6.3：退多于实付不允许，那是白送钱，走结算调整
    CHECK (refund_amount <= paid_amount)
);
CREATE INDEX idx_refunds_line ON refunds(order_line_id);

-- ========== 垫付 / 结算调整（②-3 才写数据）==========
-- 两张表都带 journal_id：结算单 = 往来账户的余额（spec 5.2），
-- 前提是所有影响往来的东西都在账本里。
CREATE TABLE advances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    owner_society_id INTEGER NOT NULL,
    journal_id INTEGER NOT NULL,
    label TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK (amount > 0),
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE,
    FOREIGN KEY (owner_society_id) REFERENCES societies(id),
    FOREIGN KEY (journal_id) REFERENCES journals(id) ON DELETE CASCADE
);
CREATE INDEX idx_advances_event ON advances(event_id);

CREATE TABLE settlement_adjustments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    owner_society_id INTEGER NOT NULL,
    journal_id INTEGER NOT NULL,
    label TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK (amount <> 0),   -- 可正可负，但不能是 0
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE,
    FOREIGN KEY (owner_society_id) REFERENCES societies(id),
    FOREIGN KEY (journal_id) REFERENCES journals(id) ON DELETE CASCADE
);
CREATE INDEX idx_settlement_adjustments_event ON settlement_adjustments(event_id);
```

- [ ] **Step 2: 先跑测试，确认迁移能跑通**

```bash
cd src-tauri && tauri-env linux cargo test --all-features
```
Expected: 24 passed。`test_pool()` 会跑全部迁移，跑不通的话所有 HTTP 测试都会炸。

> 此时 `db/models.rs` 里的 `Product`/`Order`/`OrderItem` 还指着已删的表，但因为
> 全仓是运行时 SQL，**编译不会报错**。下一步才处理它们。

- [ ] **Step 3: 在 `db/models.rs` 里删旧 model、加新 model**

**本 task 一个旧 model 都不删，只加新的。**

旧 model（`Product` / `Order` / `OrderItem` / …）只是 struct，**不在编译期引用任何表**——
表被删了它们照样编译，只是没人能用它们查到数据。而 `api/order.rs`、`api/product.rs`、
`api/stats.rs` 都 `use` 着它们，本 task 删掉就会让这三个文件编译失败，逼得本 task
去改三个它不拥有的 handler、写一堆 Task 5/6/7 马上要推翻的过渡代码。

所以删除权跟着重写走：**Task 5 删 `Product`，Task 6 删 `Order` / `OrderWithItems` /
`OrderItem` / `CreateOrderItemDTO` / `CreateOrderDTO`，Task 7 删四个僵尸 DTO**
（`ProductSalesDetail` / `SalesTimeSeries` / `SalesSummary` / `SalesReport`——
`#[allow(dead_code)]` 的，`api/stats.rs` 从来没用过，注释里写着「②/③a 重做统计响应类型时
大概率会把 stats.rs 里的私有结构体换成它们」，现在的结论是不换，直接删）。

`MasterProduct` 加一个字段：
```rust
    /// 归属社团的**默认值**。选品时会被快照到 `event_products.owner_society_id`，
    /// 之后改这里不影响已有展会的账（spec 3.1）。
    #[serde(default = "default_home_society")]
    pub owner_society_id: i64,
```
并在文件里加 `fn default_home_society() -> i64 { 1 }`（`.boothpack` 从老版本导入时这一列可能缺失）。

新增：
```rust
// ==========================================
// 社团（货主的单位）
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Society {
    pub id: i64,
    pub name: String,
    pub is_home: bool,
}

// ==========================================
// 摊位商品
// ==========================================
// 注意**没有 current_stock / initial_stock**：余额是 stock_movements 的聚合，
// 由 handler 组装进响应（见 api/product.rs 的 EventProductResponse）。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EventProduct {
    pub id: i64,
    pub event_id: i64,
    pub master_product_id: i64,
    pub owner_society_id: i64,
    pub product_code: String,
    pub name: String,
    /// 单位：分
    pub unit_price: i64,
    // JOIN master_products 得到，SELECT 里没有这几列时 sqlx(default) 返回 None
    #[sqlx(default)]
    pub image_url: Option<String>,
    #[sqlx(default)]
    pub category: Option<String>,
    #[sqlx(default)]
    pub tags: Option<String>,
}

// ==========================================
// 订单
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrderRow {
    pub id: i64,
    pub event_id: i64,
    pub status: String,
    pub channel: Option<String>,
    /// 以下三个单位都是分。②-1 里恒相等；②-2 引入 Lot 和手工覆盖后才会分开。
    pub gross_amount: i64,
    pub solved_amount: i64,
    pub final_amount: i64,
    /// 前端读的是 `timestamp` —— 这个 rename 是个隐形契约，
    /// `frontend/src/components/order/OrderCard.vue:55` 和
    /// `frontend/src/views/AdminEventOrders.vue:114` 都依赖它。改名会静默白屏。
    #[serde(rename = "timestamp")]
    pub created_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrderLineRow {
    pub id: i64,
    pub order_id: i64,
    pub event_product_id: i64,
    pub order_lot_id: Option<i64>,
    pub qty: i64,
    pub unit_price: i64,
    pub allocated_amount: i64,
    pub paid_amount: i64,
}
```

- [ ] **Step 4: 在 `db/mod.rs` 的 `init_db` 里加一次性 v1 备份**

spec 8：升级时老库**改名留存** `sale_system.db.v1-backup`（不删）。这和 ① 的迁移前快照是两回事——快照按 `KEEP=3` 轮转会被挤掉（路线图附录第 3 条），而这份是永久的。

在 `migrate_with_snapshot(&pool, &db_path, db_existed, &migrator).await?;` **之前**插入：

```rust
    // spec 8：v1 → v2 是知情清零，老库必须永久留一份。
    // 和 ① 的 premigrate 快照不是一回事——那些按 KEEP=3 轮转会被后续迁移挤掉
    // （见路线图附录第 3 条），这一份不参与轮转、永不删除。
    if db_existed {
        backup_v1_once(&pool, &db_path).await;
    }
```

在文件末尾（`#[cfg(test)] mod tests` 之前）加：

```rust
/// v1 库的一次性永久备份。只在「确实是 v1 库」且「备份还不存在」时落一份。
///
/// 判据是 `products` 表还在 —— 那是 v1 独有、v2 迁移会删掉的表。用它而不是版本号，
/// 是因为 `_sqlx_migrations` 在损坏库上读不出来，而这种库恰恰最需要备份。
/// 失败不阻断启动，但要吼出来。
async fn backup_v1_once(pool: &SqlitePool, db_path: &Path) {
    let dest = db_path.with_extension("db.v1-backup");
    if dest.exists() {
        return;
    }

    let is_v1: Result<Option<String>, sqlx::Error> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'products'",
    )
    .fetch_optional(pool)
    .await;

    if !matches!(is_v1, Ok(Some(_))) {
        return;
    }

    // VACUUM INTO 而不是 fs::copy：库跑在 WAL 模式下，直接拷 .db 会漏掉
    // -wal 中尚未 checkpoint 的数据。理由同 db/snapshot.rs。
    let escaped = dest.to_string_lossy().replace('\'', "''");
    match sqlx::query(sqlx::AssertSqlSafe(format!("VACUUM INTO '{escaped}'")))
        .execute(pool)
        .await
    {
        Ok(_) => println!("[Booth Tool] v1 database preserved at {}", dest.display()),
        Err(e) => eprintln!("[Booth Tool] WARNING: v1 backup failed: {e}"),
    }
}
```

- [ ] **Step 5: 给 v1 备份写测试（追加到 `db/mod.rs` 的 `mod tests`）**

```rust
    /// spec 8 的知情清零要求老库永久留存。这条守的是「只对 v1 库落、只落一次」。
    #[tokio::test]
    async fn backup_v1_once_preserves_old_db_exactly_once() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        let pool = empty_wal_db(&db_path).await;

        // 造一个 v1 特征：products 表存在
        sqlx::query("CREATE TABLE products (id INTEGER PRIMARY KEY, marker TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO products (marker) VALUES ('v1-data')")
            .execute(&pool)
            .await
            .unwrap();

        let dest = db_path.with_extension("db.v1-backup");
        backup_v1_once(&pool, &db_path).await;
        assert!(dest.exists(), "v1 库应被备份");

        // 备份内容必须读得到（VACUUM INTO 的事务一致性，fs::copy 在 WAL 下会漏）
        let snap = SqlitePool::connect(&format!("sqlite://{}", dest.display()))
            .await
            .unwrap();
        let marker: String = sqlx::query_scalar("SELECT marker FROM products")
            .fetch_one(&snap)
            .await
            .unwrap();
        assert_eq!(marker, "v1-data");
        snap.close().await;

        // 第二次调用必须是 no-op：不能把用户已经看过的备份覆盖掉
        let before = std::fs::metadata(&dest).unwrap().modified().unwrap();
        backup_v1_once(&pool, &db_path).await;
        let after = std::fs::metadata(&dest).unwrap().modified().unwrap();
        assert_eq!(before, after, "备份已存在时必须 no-op");
    }

    /// 全新安装（没有 products 表）不该产生 v1 备份文件。
    #[tokio::test]
    async fn backup_v1_once_skips_fresh_v2_database() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        let pool = empty_wal_db(&db_path).await;
        sqlx::migrate!().run(&pool).await.unwrap();

        backup_v1_once(&pool, &db_path).await;
        assert!(
            !db_path.with_extension("db.v1-backup").exists(),
            "v2 库不该产生 v1 备份"
        );
    }
```

- [ ] **Step 6: 跑门禁**

```bash
cd src-tauri
tauri-env linux cargo fmt --all --check
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
```
Expected: 26 passed（24 + 2 个 v1 备份测试）。

> clippy 可能会对删掉 model 后 `db/models.rs` 顶部多余的 `use` 报 unused import，一并清掉。

- [ ] **Step 7: Commit**

```bash
git add src-tauri/migrations src-tauri/src/db
git commit -m "$(cat <<'EOF'
feat: :card_file_box: 复式账全量 schema + v1 库一次性永久备份

12 张新表一次建齐（含 ②-2 的 Lot、②-3 的退货/垫付/调整），后两份 plan 只写数据
不改表。金额列全部 INTEGER（分）。

三条实测出来的 SQLite 约束写进注释了：ADD COLUMN + REFERENCES 在 foreign_keys=ON
下必须默认 NULL；is_home 的「有且只有一个」要用偏唯一索引；删表须子表在前。

v1 备份不参与 ① 的 KEEP=3 快照轮转（路线图附录第 3 条），用 VACUUM INTO 保证
WAL 下的事务一致性，只对有 products 表的老库落、且只落一次。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: 账本内核

纯领域层，不挂任何 route。**这是整个 ② 最值得写扎实的一块**——后面所有货和钱的变动都只能从这里走。

**Files:**
- Create: `src-tauri/src/domain/ledger.rs`
- Modify: `src-tauri/src/domain/mod.rs`（补 `pub mod ledger;`）

**Interfaces:**
- Consumes: `crate::error::{ApiError, ApiResult}`、`crate::domain::money::Money`
- Produces（后续三个 task 全部依赖，签名不要改）:
  - `Location::{External, OnSite, Customer, Loss, Gift, Variance, Conversion}`，`as_str()`、`FromStr`
  - `Account::{VendorOwn, Received(String), SocietyDue(i64), SettlementAdj, ReconDiff}`，`Display`、`FromStr`
  - `JournalKind::{Restock, Sale, Receipt, Refund, Cancel, Gift, Scrap, Stocktake, TakeBack, Convert, Advance, Adjust}`，`as_str()`
  - `StockLeg { event_product_id, from, to, qty }`、`MoneyLeg { from, to, amount }`
  - `async fn post_journal(tx, event_id, kind, order_id, reverses, note, stock, money) -> ApiResult<i64>`
  - `async fn reverse_journal(tx, journal_id, kind, note) -> ApiResult<i64>`
  - `async fn reverse_order_journals(tx, order_id, note) -> ApiResult<usize>`
  - `async fn onsite_balance(tx, event_product_id) -> ApiResult<i64>`
  - `async fn onsite_balances(pool, event_id) -> ApiResult<HashMap<i64, i64>>`
  - `async fn account_balance(pool, event_id, &Account) -> ApiResult<Money>`

- [ ] **Step 1: 先写测试（TDD，这个 task 的测试比实现更重要）**

新建 `src-tauri/src/domain/ledger.rs`，**先只写测试模块**，让它编译失败：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_pool;
    use sqlx::SqlitePool;

    /// 造一场展会 + 两个社团 + 两个商品，返回 (event_id, ep_a, ep_b)。
    /// ep_a 归本社团(1)，ep_b 归代卖社团(2)。
    async fn fixture(pool: &SqlitePool) -> (i64, i64, i64) {
        sqlx::query("INSERT INTO societies (id, name, is_home) VALUES (2, '黄昏堂', 0)")
            .execute(pool).await.unwrap();
        sqlx::query(
            "INSERT INTO master_products (id, product_code, name, default_price, owner_society_id)
             VALUES (1, 'A', '本子A', 0, 1), (2, 'B', '本子B', 0, 2)",
        ).execute(pool).await.unwrap();
        sqlx::query(
            "INSERT INTO events (id, name, event_date, status) VALUES (1, 'ABC漫展', '2026-10-01', '进行中')",
        ).execute(pool).await.unwrap();
        sqlx::query(
            "INSERT INTO event_products
               (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
             VALUES (1, 1, 1, 1, 'A', '本子A', 3000), (2, 1, 2, 2, 'B', '本子B', 2000)",
        ).execute(pool).await.unwrap();
        (1, 1, 2)
    }

    #[tokio::test]
    async fn stock_balance_is_the_aggregate_of_movements() {
        let pool = test_pool().await;
        let (event_id, ep_a, ep_b) = fixture(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        post_journal(
            &mut tx, event_id, JournalKind::Restock, None, None, Some("开场带货"),
            &[
                StockLeg { event_product_id: ep_a, from: Location::External, to: Location::OnSite, qty: 10 },
                StockLeg { event_product_id: ep_b, from: Location::External, to: Location::OnSite, qty: 5 },
            ],
            &[],
        ).await.unwrap();
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
        post_journal(&mut tx, event_id, JournalKind::Restock, None, None, None,
            &[StockLeg { event_product_id: ep_a, from: Location::External, to: Location::OnSite, qty: 10 }],
            &[]).await.unwrap();
        let sale = post_journal(&mut tx, event_id, JournalKind::Sale, None, None, None,
            &[StockLeg { event_product_id: ep_a, from: Location::OnSite, to: Location::Customer, qty: 3 }],
            &[MoneyLeg { from: Account::SocietyDue(1), to: Account::Received("微信".into()), amount: Money::from_cents(9000) }],
        ).await.unwrap();
        tx.commit().await.unwrap();

        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 7);
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into())).await.unwrap(),
            Money::from_cents(9000)
        );

        let mut tx = pool.begin().await.unwrap();
        reverse_journal(&mut tx, sale, JournalKind::Cancel, Some("取消订单")).await.unwrap();
        tx.commit().await.unwrap();

        // 货和钱必须同时归位——只回滚一半是这个模型最该防住的事故
        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 10);
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into())).await.unwrap(),
            Money::ZERO
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1)).await.unwrap(),
            Money::ZERO
        );
    }

    #[tokio::test]
    async fn a_journal_cannot_be_reversed_twice() {
        let pool = test_pool().await;
        let (event_id, ep_a, _) = fixture(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        let j = post_journal(&mut tx, event_id, JournalKind::Restock, None, None, None,
            &[StockLeg { event_product_id: ep_a, from: Location::External, to: Location::OnSite, qty: 10 }],
            &[]).await.unwrap();
        reverse_journal(&mut tx, j, JournalKind::Cancel, None).await.unwrap();
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
        let r = post_journal(&mut tx, event_id, JournalKind::Restock, None, None, None,
            &[StockLeg { event_product_id: ep_a, from: Location::OnSite, to: Location::OnSite, qty: 1 }],
            &[]).await;
        assert!(r.is_err(), "from == to 必须被 CHECK 约束拦住");
    }
}
```

- [ ] **Step 2: 跑测试，确认因「符号未定义」而失败**

```bash
cd src-tauri && tauri-env linux cargo test --all-features ledger
```
Expected: 编译失败，`cannot find type/function ...`。这一步是确认测试真的连上了实现，不是空跑。

- [ ] **Step 3: 写实现（在同一文件、测试模块之前）**

```rust
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
                    Account::SocietyDue(
                        id.parse()
                            .map_err(|_| ApiError::BadRequest(format!("社团往来账户 id 不是数字: {s}")))?,
                    )
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
    if stock.is_empty() && money.is_empty() {
        return Err(ApiError::BadRequest("空的 journal：既没有货也没有钱".into()));
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
        // 金额为 0 的腿直接跳过而不是报错：②-2 的手工折让为 0 时这条腿本就不该存在，
        // 让调用方无脑传、这里过滤，比每个调用方各自判一遍可靠。
        if leg.amount.is_zero() {
            continue;
        }
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

    post_journal(tx, event_id, kind, order_id, Some(journal_id), note, &stock, &money).await
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
```

在 `src-tauri/src/domain/mod.rs` 补上 `pub mod ledger;`。

- [ ] **Step 4: 跑测试，必须全过**

```bash
cd src-tauri && tauri-env linux cargo test --all-features ledger
```
Expected: 5 passed。

> 这 5 条的结果已经用等价的 SQL 在真实 schema 上预演过：带货 10/5 → 下单后 8/4 →
> 收款后往来 −6000/−2000、实收 +8000 → 全部冲正后回到 10/5 和 0/0/0。
> 数字对不上说明实现有偏差，不要改测试去迁就。

- [ ] **Step 5: 跑全量门禁并提交**

```bash
cd src-tauri
tauri-env linux cargo fmt --all --check
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
```
Expected: 31 passed。

```bash
git add src-tauri/src/domain
git commit -m "$(cat <<'EOF'
feat: :sparkles: 账本内核——journal + 货/钱两张移动表

货和钱唯一的写入口是 post_journal，每条腿的 from/to 都非空，「不平衡的分录」
从需要主动校验的不变量变成写不出来的错误。

余额改为对流水实时聚合，不再有 current_stock 这种可以漂移的缓存。
冲正走 reverse_journal（腿反向 + reverses_journal_id 关联），重复冲正由
偏唯一索引在 DB 层拦住——没有它，一次网络重试就能把库存退两遍。

Account 用 society_id 而不是社团名，因为社团会改名；渠道名带横线也能往返解析。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 6: 把 `seed_event_and_product` 补进 `test_support.rs`**

Task 1 Step 5 给了完整代码，但它依赖 `domain::ledger`，所以留到这里加。加完跑一次
`tauri-env linux cargo test --all-features`，确认仍是 31 passed（这个 helper 此时还没有调用方，
但它是 `pub` 且在 `#[cfg(test)] mod` 里，不会触发 dead_code）。

---

## Task 4: 社团与归属

**Files:**
- Create: `src-tauri/src/api/society.rs`
- Modify: `src-tauri/src/api/mod.rs`（挂 router）
- Modify: `src-tauri/src/api/sync.rs`（`.boothpack` 带归属）

**Interfaces:**
- Consumes: `ApiResult`、`db::models::Society`
- Produces: `GET /api/societies`、`POST /api/societies`、`PUT /api/societies/:id`、`DELETE /api/societies/:id`

**路由与语义：**

| 方法 | 路径 | 权限 | 行为 |
|---|---|---|---|
| GET | `/api/societies` | `Claims` | 列出全部，本社团排第一 |
| POST | `/api/societies` | `AdminOnly` | `{name}` → 新建（`is_home` 恒为 0） |
| PUT | `/api/societies/:id` | `AdminOnly` | `{name?, is_home?}`；`is_home: true` 会把原本社团降为普通 |
| DELETE | `/api/societies/:id` | `AdminOnly` | 本社团不可删；被商品或账引用时 409 |

- [ ] **Step 1: 先写测试**

在 `src-tauri/src/api/society.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use crate::test_support::{admin_token, json_request, read_json, test_router};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tower::ServiceExt; // for oneshot

    #[tokio::test]
    async fn fresh_database_has_exactly_one_home_society() {
        let (router, _dir) = test_router().await;
        let res = router
            .oneshot(
                Request::builder()
                    .uri("/api/societies")
                    .header("authorization", format!("Bearer {}", admin_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body = read_json(res).await;
        let arr = body.as_array().expect("数组");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "本社团");
        assert_eq!(arr[0]["is_home"], true);
    }

    #[tokio::test]
    async fn promoting_a_society_to_home_demotes_the_previous_one() {
        // 这条守的是 idx_societies_home 那个偏唯一索引不会被绕过：
        // 直接 UPDATE 新的为 is_home=1 会撞唯一约束，必须先降级旧的。
        let (router, _dir) = test_router().await;
        let token = admin_token();

        let res = router
            .clone() // oneshot 消费 router，多请求必须 clone
            .oneshot(json_request("POST", "/api/societies", Some(&token), json!({"name": "黄昏堂"})))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let new_id = read_json(res).await["id"].as_i64().unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/societies/{new_id}"),
                Some(&token),
                json!({"is_home": true}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = router
            .oneshot(
                Request::builder()
                    .uri("/api/societies")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = read_json(res).await;
        let homes: Vec<_> = body
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["is_home"] == true)
            .collect();
        assert_eq!(homes.len(), 1, "本社团必须有且只有一个");
        assert_eq!(homes[0]["id"], new_id);
    }

    #[tokio::test]
    async fn home_society_cannot_be_deleted() {
        let (router, _dir) = test_router().await;
        let res = router
            .oneshot(json_request("DELETE", "/api/societies/1", Some(&admin_token()), json!({})))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn creating_a_society_requires_admin() {
        let (router, _dir) = test_router().await;
        let res = router
            .oneshot(json_request("POST", "/api/societies", None, json!({"name": "X"})))
            .await
            .unwrap();
        // 无 token 走 Claims 提取器的 WrongCredentials → 401（不是 403，
        // 403 是「有 token 但不是 admin」）
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
```

- [ ] **Step 2: 跑测试确认失败**

```bash
cd src-tauri && tauri-env linux cargo test --all-features society
```
Expected: 编译失败（模块还没挂）或 4 个测试全红。

- [ ] **Step 3: 写 `src-tauri/src/api/society.rs` 的实现**

```rust
//! 社团 = 货主的单位。没有「个人」这一档（spec 3.1）。
//!
//! 「本社团」（`is_home`）有且只有一个，由 `idx_societies_home` 这个偏唯一索引
//! 在 DB 层保证。因此**升格一个新的本社团必须先把旧的降级**，顺序反了会撞唯一约束。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, put},
    Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    api::guard::AdminOnly,
    db::models::Society,
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_societies).post(create_society))
        .route("/:id", put(update_society).delete(delete_society))
}

#[derive(Deserialize)]
struct CreateSocietyRequest {
    name: String,
}

#[derive(Deserialize)]
struct UpdateSocietyRequest {
    name: Option<String>,
    is_home: Option<bool>,
}

/// 本社团排第一，其余按名字。选品下拉框里本社团永远在最上面。
async fn list_societies(State(state): State<AppState>, _: Claims) -> ApiResult<Json<Vec<Society>>> {
    let rows: Vec<Society> =
        sqlx::query_as("SELECT id, name, is_home FROM societies ORDER BY is_home DESC, name")
            .fetch_all(&state.db)
            .await?;
    Ok(Json(rows))
}

async fn create_society(
    State(state): State<AppState>,
    _: AdminOnly,
    Json(payload): Json<CreateSocietyRequest>,
) -> ApiResult<impl IntoResponse> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("社团名不能为空".into()));
    }

    let existing: Option<i64> = sqlx::query_scalar("SELECT id FROM societies WHERE name = ?")
        .bind(name)
        .fetch_optional(&state.db)
        .await?;
    if existing.is_some() {
        return Err(ApiError::Conflict(format!("社团「{name}」已存在")));
    }

    let row: Society = sqlx::query_as(
        "INSERT INTO societies (name, is_home) VALUES (?, 0) RETURNING id, name, is_home",
    )
    .bind(name)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(row)))
}

async fn update_society(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateSocietyRequest>,
) -> ApiResult<Json<Society>> {
    let mut tx = state.db.begin().await?;

    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM societies WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    if exists.is_none() {
        return Err(ApiError::NotFound("社团不存在".into()));
    }

    if let Some(name) = payload.name.as_deref() {
        let name = name.trim();
        if name.is_empty() {
            return Err(ApiError::BadRequest("社团名不能为空".into()));
        }
        sqlx::query("UPDATE societies SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }

    if payload.is_home == Some(true) {
        // 顺序不能反：偏唯一索引 idx_societies_home 会让「先升格新的」撞唯一约束。
        sqlx::query("UPDATE societies SET is_home = 0 WHERE is_home = 1")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE societies SET is_home = 1 WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    } else if payload.is_home == Some(false) {
        // 不允许把本社团降级成「没有本社团」——垫付、手工折让都要往它头上记。
        return Err(ApiError::BadRequest(
            "不能取消本社团标记，只能把另一个社团设为本社团".into(),
        ));
    }

    let row: Society = sqlx::query_as("SELECT id, name, is_home FROM societies WHERE id = ?")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(row))
}

async fn delete_society(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
) -> ApiResult<impl IntoResponse> {
    let is_home: Option<bool> = sqlx::query_scalar("SELECT is_home FROM societies WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;

    match is_home {
        None => return Err(ApiError::NotFound("社团不存在".into())),
        Some(true) => return Err(ApiError::Conflict("本社团不能删除".into())),
        Some(false) => {}
    }

    // 被引用就不给删。硬删会让历史账里的 owner_society_id 变成悬空外键，
    // 而 SQLite 的 FK 在这里是 NO ACTION，报出来的错对用户毫无意义。
    let refs: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM master_products WHERE owner_society_id = ?1)
              + (SELECT COUNT(*) FROM event_products  WHERE owner_society_id = ?1)",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    if refs > 0 {
        return Err(ApiError::Conflict(format!(
            "还有 {refs} 个商品归属这个社团，不能删除"
        )));
    }

    sqlx::query("DELETE FROM societies WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok((StatusCode::OK, Json(json!({"message": "社团已删除"}))))
}
```

在 `src-tauri/src/api/mod.rs` 加 `mod society;` 并在 `router()` 里加 `.nest("/societies", society::router())`。

- [ ] **Step 4: 让 `.boothpack` 带上归属（`api/sync.rs`）**

`CatalogExport.products` 是 `Vec<MasterProduct>`，Task 2 已经给 `MasterProduct` 加了 `owner_society_id` 且带 `#[serde(default = "default_home_society")]`，所以**导出自动带上、导入老包自动回落到本社团**，不需要改结构。

要改的只有导入时的 upsert 语句（`sync.rs:478` 附近）——按社团**名字**匹配，不是 id：不同设备上同一个社团的 id 必然不同。

导出结构加一个社团名列表，在 `CatalogExport` 上加：
```rust
    /// 社团名单（v1.2+）。导入时按**名字**匹配/新建，不能用 id——
    /// 不同设备上同一个社团的 id 必然不同。
    #[serde(default)]
    societies: Vec<String>,
```
导出时填 `SELECT name FROM societies ORDER BY id`，并把每个 `MasterProduct` 的 `owner_society_id` 在序列化前替换成「该社团在 `societies` 数组里的下标」——或更简单：给 `MasterProduct` 的导出加一个并行数组 `product_owners: Vec<(String /*product_code*/, String /*society_name*/)>`。

**选后者**，理由是不改 `MasterProduct` 的序列化语义（它同时被 `GET /master-products` 用着）：

```rust
    /// product_code → 归属社团名（v1.2+）。导入时按名字解析成本地 id，
    /// 缺失或找不到时回落到本社团。
    #[serde(default)]
    product_owners: Vec<(String, String)>,
```

导入侧在 upsert 商品之前，先把 `societies` 里没有的社团建出来（按名字，已存在就复用），再建一张 `name -> id` 的映射，然后按 `product_owners` 给每个商品设 `owner_society_id`。

> **导入时必须无条件覆盖 `owner_society_id`，不能相信包里 `MasterProduct` 自带的那个值。**
> `MasterProduct` 现在带这一列，所以导出的 JSON 里它**一定存在**，而那是**源设备的 id**——
> 直接 upsert 进去会把商品挂到本机一个毫不相干的社团上，或者挂到一个不存在的 id 上。
> `#[serde(default)]` 只救得了老版本的包（字段缺失），救不了新版本的包（字段存在但值是外来的）。
> 正确顺序是：先按 `product_owners` 里的**社团名**解析出本机 id，解析不到就回落到本社团，
> 然后用这个 id 覆盖。

- [ ] **Step 5: 跑门禁并提交**

```bash
cd src-tauri
tauri-env linux cargo fmt --all --check
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
```
Expected: 35 passed。

```bash
git add src-tauri/src/api/society.rs src-tauri/src/api/mod.rs src-tauri/src/api/sync.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 社团（货主）CRUD，.boothpack 带归属

本社团有且只有一个，靠偏唯一索引在 DB 层保证——所以升格新本社团必须先降级旧的，
顺序反了会撞唯一约束，这一点写进注释了。

.boothpack 按社团**名字**而不是 id 传递归属：不同设备上同一个社团的 id 必然不同。
老包缺这个字段时回落到本社团。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: 摊位商品与进货

把 `api/product.rs` 从「`products` 表 + `current_stock` 加减」整个换成「`event_products` + 进货移动」。

**Files:**
- Modify: `src-tauri/src/api/product.rs`（全量重写）

**Interfaces:**
- Consumes: `domain::ledger::{post_journal, onsite_balances, onsite_balance, JournalKind, Location, StockLeg}`、`ApiResult`、`db::models::EventProduct`
- Produces: 响应结构 `EventProductResponse`，字段 `id, event_id, master_product_id, owner_society_id, owner_society_name, product_code, name, unit_price, stocked_qty, onsite_qty, image_url, category, tags`

**路由与语义变化：**

| 方法 | 路径 | 旧行为 | 新行为 |
|---|---|---|---|
| GET | `/events/:event_id/products` | 返回 `price`/`initial_stock`/`current_stock` | 返回 `unit_price`（分）/`stocked_qty`（累计进货）/`onsite_qty`（聚合余额） |
| POST | `/events/:event_id/products` | 插 `products` 行，`current_stock = initial_stock` | 插 `event_products` 行（归属从 master 快照）+ 一条 `外部 → 现场仓` 进货 journal |
| **POST** | **`/events/:event_id/products/:id/restock`** | 不存在 | **新增**：补货，`{qty, note?}` → 一条进货 journal。spec 6.4：补货在「进行中」随时可录，不是开场专属 |
| PUT | `/products/:id` | 改 `price` + `initial_stock`（连带调 `current_stock`） | 只改 `unit_price`。**改库存必须走进货/盘点，不再能直接改数字** |
| DELETE | `/products/:id` | 直接删 | 有移动流水时拒绝（409），否则删 |

> **`PUT /products/:id` 不再接受 `initial_stock` 是有意的行为变化。** 旧接口用
> `current_stock += (new_initial - old_initial)` 硬调数字，正是 spec 第 1 节点名的
> 「三个互不共享的写入方」之一（而且它还不在事务里）。新模型下「我多带了 5 本来」
> 是一次进货，「点了一下发现少了 2 本」是一次盘点（②-3），两者语义不同、都留痕。
> 前端对应的编辑框在 Task 8 改掉。

- [ ] **Step 1: 先写测试**

在 `product.rs` 末尾：

```rust
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
}
```

- [ ] **Step 2: 跑测试确认失败**

```bash
cd src-tauri && tauri-env linux cargo test --all-features product
```

- [ ] **Step 3: 重写 `api/product.rs`**

要点（逐条都是必须的）：

1. **响应组装**：`list_event_products` 用 `ledger::onsite_balances(&state.db, event_id)` 一次拿全部余额，再和 `event_products` JOIN `master_products` 的结果拼起来。**不要在循环里逐个查余额**（N+1）。
   `EventProductResponse` 必须带 **`owner_society_name`**（`JOIN societies s ON s.id = ep.owner_society_id` 取 `s.name`）——
   前端选品界面要显示「这是谁的货」，只给 id 等于让前端再拉一次社团表。
   **`add_product_to_event` / `update_product` / `restock` 四个 handler 返回的都是同一个
   `EventProductResponse`**，抽一个 `fn load_one(pool, id) -> ApiResult<EventProductResponse>` 复用，
   不要四处各拼一遍（字段一多必然漏掉某个）。
2. `stocked_qty`（累计进货）= `SUM(qty) WHERE from_location='外部' AND to_location='现场仓'`，给前端画库存条的分母用（`ProductGrid.vue:77` 现在用 `initial_stock` 做分母）。
3. **`add_product_to_event` 的归属快照**：
   ```rust
   // owner_society_id 从 master_products 抄一份**快照**。之后改全局商品库的归属
   // 不影响已有展会的账——否则展会结算完之后有人改了归属，冻结的账就跟着变
   // （spec 3.1）。
   ```
4. **建商品 + 进货在同一个事务里**：`let mut tx = state.db.begin().await?;` → 插 `event_products` → `post_journal(&mut tx, ..., JournalKind::Restock, ...)` → `tx.commit()`。`initial_stock` 为 0 时**不记 journal**（`post_journal` 会拒绝空 journal）。
5. **`unit_price` 请求字段是分（整数）**。缺省时用 `master_products.default_price`——注意那一列还是 `REAL`（元），要换算：`(default_price * 100.0).round() as i64`。加注释说明为什么这里有一次 f64 换算：商品库的列没跟着改，因为 `.boothpack` 的跨版本兼容要靠它。
6. **删除前检查流水**：
   ```rust
   let moves: i64 = sqlx::query_scalar(
       "SELECT COUNT(*) FROM stock_movements WHERE event_product_id = ?")
       .bind(product_id).fetch_one(&state.db).await?;
   if moves > 0 { return Err(ApiError::Conflict("这个商品已经有进出记录，不能删除".into())); }
   ```
7. 权限 helper `check_write_permission` 保留现有逻辑，但返回类型改成 `Result<(), ApiError>`，用 `ApiError::Forbidden`。**顺手修掉旧实现「权限失败返回纯文本而不是 JSON」的不一致**（`product.rs:37-53`）。
8. handler 签名顺序照仓库惯例：`State` → 鉴权 → `Path` → `Query` → `Json`。返回类型用 `ApiResult<Json<T>>` 或 `ApiResult<impl IntoResponse>`。

- [ ] **Step 4: 跑门禁并提交**

Expected: 39 passed。

```bash
git add src-tauri/src/api/product.rs src-tauri/src/test_support.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 摊位商品迁到 event_products，库存变动只能走进货移动

current_stock 字段消失，余额改为聚合 stock_movements。新增 restock 路由——
spec 6.4 说补货在「进行中」随时可录，不是开场专属。

PUT /products/:id 不再接受 initial_stock：旧接口用 current_stock += (new - old)
硬调数字（而且不在事务里），正是 spec 点名的三个互不共享写入方之一。现在
「多带了几本」是进货、「点了下发现少了」是盘点，语义不同且都留痕。

归属在选品时从 master_products 快照到 event_products，之后改商品库不影响已有账。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: 订单全生命周期

**Files:**
- Modify: `src-tauri/src/api/order.rs`（全量重写）

**Interfaces:**
- Consumes: Task 3 的全部账本 API、Task 1 的 `Money`/`ApiError`
- Produces: `POST /events/:event_id/orders`（公开）、`GET /events/:event_id/orders`、`PUT /events/:event_id/orders/:order_id/status`

**三条路径的账本动作：**

```
下单  orders(pending) + order_lines + journal「销售」: 现场仓 → 顾客仓
收款  orders(completed, channel) + journal「收款」:
          对每个货主 X：社团往来:X → 实收-<渠道>   Σ 该货主各行的 allocated_amount
取消  对该订单名下全部未冲正的 journal 各冲正一次（货和钱一起回去）
```

②-1 没有 Lot 也没有折让，所以 `gross = solved = final = Σ unit_price × qty`，
每行 `allocated = paid = unit_price × qty`。**但代码要按「可能不等」写**——
②-2 只改计算这些数的地方，不该再动 handler 的结构。

- [ ] **Step 1: 先写测试**

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

        let left = crate::domain::ledger::onsite_balance(&pool, ep_a).await.unwrap();
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
            .fetch_one(&pool).await.unwrap();
        let journals: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(orders, 0);
        assert_eq!(journals, 1, "只该有 seed 时那一条进货 journal");
        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_a).await.unwrap(),
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
                "POST", &format!("/api/events/{event_id}/orders"), None,
                json!({"items": [
                    {"product_id": ep_a, "quantity": 2},
                    {"product_id": ep_b, "quantity": 1}
                ]}),
            ))
            .await.unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        // 不带渠道必须被拒——否则钱那条腿没有对手账户（spec 6.2）
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token), json!({"status": "completed"}),
            ))
            .await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token), json!({"status": "completed", "channel": "微信"}),
            ))
            .await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        use crate::domain::ledger::{account_balance, Account};
        use crate::domain::money::Money;
        // 我欠本社团 6000、欠代卖社团 2000，手上多了 8000
        assert_eq!(account_balance(&pool, event_id, &Account::SocietyDue(1)).await.unwrap(),
                   Money::from_cents(-6000));
        assert_eq!(account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
                   Money::from_cents(-2000));
        assert_eq!(account_balance(&pool, event_id, &Account::Received("微信".into())).await.unwrap(),
                   Money::from_cents(8000));
    }

    #[tokio::test]
    async fn cancelling_a_completed_order_reverses_both_goods_and_money() {
        // 旧模型只退库存、不碰钱（那时也没有钱的账）。新模型下只回滚一半
        // 正是最该防住的事故。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders"), None,
            json!({"items": [{"product_id": ep_a, "quantity": 2}]}),
        )).await.unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"),
            Some(&token), json!({"status": "completed", "channel": "现金"}),
        )).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"),
            Some(&token), json!({"status": "cancelled"}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        use crate::domain::ledger::{account_balance, onsite_balance, Account};
        use crate::domain::money::Money;
        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 10, "货回来了");
        assert_eq!(account_balance(&pool, event_id, &Account::SocietyDue(1)).await.unwrap(),
                   Money::ZERO, "钱也回去了");
        assert_eq!(account_balance(&pool, event_id, &Account::Received("现金".into())).await.unwrap(),
                   Money::ZERO);
    }

    #[tokio::test]
    async fn cancelling_twice_does_not_refund_the_stock_twice() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders"), None,
            json!({"items": [{"product_id": ep_a, "quantity": 2}]}),
        )).await.unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        let uri = format!("/api/events/{event_id}/orders/{order_id}/status");
        let r1 = router.clone().oneshot(json_request("PUT", &uri, Some(&token), json!({"status":"cancelled"}))).await.unwrap();
        assert_eq!(r1.status(), StatusCode::OK);
        let r2 = router.clone().oneshot(json_request("PUT", &uri, Some(&token), json!({"status":"cancelled"}))).await.unwrap();
        assert_eq!(r2.status(), StatusCode::CONFLICT, "已取消的订单不能再取消");

        assert_eq!(
            crate::domain::ledger::onsite_balance(&pool, ep_a).await.unwrap(),
            10, "库存只能退一次"
        );
    }
}
```

> 需要在 `test_support.rs` 加 **`test_router_with()`**，返回 `(Router, TempDir, SqlitePool)`
> ——测试要直接查账本余额做断言，拿不到 pool 就只能靠 HTTP 反推，断言力弱很多。
> 实现就是把 `test_router()` 里的 `state.db.clone()` 一并返回。

- [ ] **Step 2: 跑测试确认失败**

- [ ] **Step 3: 重写 `api/order.rs`**

要点：

1. **`create_order` 用 `BEGIN IMMEDIATE`**：
   ```rust
   // 防超卖的检查方式跟着模型变了：旧代码靠
   //   UPDATE products SET current_stock = current_stock - ? WHERE ... AND current_stock >= ?
   // 一条语句原子完成；新模型是「查余额 → 插移动」两步。SQLite 单写者模型下这仍然
   // 安全，**前提是两步在同一个 BEGIN IMMEDIATE 事务里**——默认的 deferred 事务
   // 在第一次写之前不持写锁，两台平板同时抢最后一本会双双通过检查。
   //
   // sqlx 0.9 的 Pool::begin_with 接受 &'static str（impl SqlSafeStr），已验证可用。
   let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
   ```
2. **逐个商品**：查 `event_products` 行（带 `unit_price`、`owner_society_id`、`name`）→ `ledger::onsite_balance(&mut *tx, ep_id)` → 不足则 `ApiError::Conflict(format!("「{}」库存不足", name))`。
   **同一商品在 items 里出现多次要先合并**，否则两行各自过检查、合起来超卖。
3. 算 `gross`，`solved = gross`，`final = solved`（②-1 无折扣）；插 `orders`；插 `order_lines`（`allocated = paid = unit_price × qty`）；一次 `post_journal(JournalKind::Sale)` 带上全部 `StockLeg`（`OnSite → Customer`）。
4. **`update_order_status` 的状态机**：
   | 当前 | 目标 | 结果 |
   |---|---|---|
   | pending | completed | 要 `channel`；记收款 journal；`completed_at = CURRENT_TIMESTAMP` |
   | pending | cancelled | `ledger::reverse_order_journals(&mut tx, order_id, Some("取消订单"))` |
   | completed | cancelled | 同上——**同一个调用**就会把货和钱两个 journal 一起冲掉 |
   | completed | completed | 409 |
   | cancelled | 任意 | 409「已取消的订单不能再改状态」 |
   | 任意 | pending | 400「不能退回待处理」 |

   **不要自己遍历 journal 反向插移动**——`reverse_order_journals` 已经处理了
   「跳过已被冲正的」和「重复冲正被偏唯一索引拦住」这两件事，手写一遍必然漏掉其一。
5. **收款按货主分组**：
   ```sql
   SELECT ep.owner_society_id, SUM(ol.allocated_amount)
   FROM order_lines ol JOIN event_products ep ON ep.id = ol.event_product_id
   WHERE ol.order_id = ? GROUP BY ep.owner_society_id
   ```
   每组一条 `MoneyLeg { from: SocietyDue(id), to: Received(channel), amount }`。
   > ②-2 会在这里追加一条 `实收 → 社团往来:本社团` 的手工折让腿（spec 4.4）。
   > **现在就把这条留成一个明确的 TODO 注释**，不要让 ②-2 的人重新推导。
6. **`list_orders` 的响应**：保持 `{...order, items: [...]}` 形状。`items` 元素字段名保持前端在用的 `product_name` / `product_price` / `quantity` / `product_image_url`，**但 `product_price` 改成分**。加 `allocated_amount` / `paid_amount` 备用。
   `total_amount` 这个字段名前端有 5 处在读（`orderStore.js:129`、`OrderCard.vue:31`、`AdminEventOrders.vue:123,205,208`），**响应里保留 `final_amount` 并额外输出一个 `total_amount` 别名会埋坑**——Task 8 统一改成 `final_amount`，这里只输出 `final_amount`。
7. `check_read_permission` / `check_write_permission` 改成返回 `ApiError`。

- [ ] **Step 4: 跑门禁并提交**

Expected: 44 passed。

```bash
git add src-tauri/src/api/order.rs src-tauri/src/test_support.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 订单三条路径迁到移动账本——下单 / 收款 / 取消冲正

下单即移动（货在那一刻就离开现场仓，天然防超卖）；完成订单 = 记一条按货主拆分的
收款 journal，必须带渠道否则钱那条腿没有对手账户；取消走冲正而不是删除，
已完成的订单取消时货和钱一起回滚——旧模型只退库存不碰钱。

防超卖从「一条原子 UPDATE」变成「查余额 + 插移动」两步，所以必须 BEGIN IMMEDIATE：
默认的 deferred 事务在第一次写之前不持写锁，两台平板抢最后一本会双双通过检查。

同一商品在 items 里出现多次要先合并，否则两行各自过检查、合起来超卖。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: 旁支收口——让应用重新完整可用

Task 2 删表之后，**全仓还有 5 处 SQL 字符串指着已经不存在的表和列**。因为项目用的是运行时 SQL（全仓零编译期 `query!` 宏），编译器一个都抓不到，只会在运行时炸。这是完整清单，一个都不能漏：

| 文件:行 | 现在引用 | 改成 |
|---|---|---|
| `src-tauri/src/api/stats.rs` 多处（82,86,106-113,169-197,217,228-247,378-385,501） | `order_items` / `products.initial_stock` / `orders.total_amount` | `order_lines` / 进货聚合 / `orders.final_amount`；金额全改分 |
| `src-tauri/src/api/admin.rs:206` | `tables_to_clear` 列的是旧表名 | 新表名，按 FK 依赖顺序 |
| `src-tauri/src/api/event.rs:459,486` | `delete_event` 删 `order_items` / `products` | 靠 `ON DELETE CASCADE`，整段可删 |
| `src-tauri/src/api/event.rs:386,414` | 状态枚举 `未进行/进行中/已结束` | `筹备/进行中/已结算` |
| `src-tauri/src/vision/store.rs:341` | `JOIN products p ON p.master_product_id = mp.id` | `JOIN event_products p ON ...` |

- [ ] **Step 1: `api/stats.rs` 的查询迁移**

字段映射：

| 旧 | 新 |
|---|---|
| `order_items oi` | `order_lines ol` |
| `oi.product_id` | `ol.event_product_id` |
| `oi.product_name` | `ep.name`（JOIN `event_products ep`） |
| `oi.product_price`（元 REAL） | `ol.unit_price`（分 INTEGER） |
| `oi.quantity` | `ol.qty` |
| `o.total_amount`（元） | `o.final_amount`（分） |
| `p.initial_stock` | `SUM(sm.qty) WHERE from='外部' AND to='现场仓'` 的子查询 |
| `p.product_code` | `ep.product_code` |

**金额从元变成分，Excel 导出（`stats.rs:427,438` 的 `currency_format`）要除以 100 再写。**
在那里加注释说明：Excel 单元格给人看，所以这里换算成元；API 响应保持分。

**统计口径不变**：仍然是「非 cancelled 的订单」。加一句注释说明为什么不改用账本聚合——
销售统计看的是订单视图（谁买了什么），账本看的是货和钱的流向，两者都对但回答的问题不同；
结算单（②-3）才是账本视图。

- [ ] **Step 2: `api/admin.rs` 的重置表清单**

```rust
    // 按外键依赖顺序清空（子表在前）。
    // ⚠️ 加新表时必须同步这里 —— 漏一张表会让「重置」留下孤儿数据，
    // 而 reset 是用户在「数据乱了」时的最后一根稻草。
    let tables_to_clear = vec![
        "stock_movements",
        "money_movements",
        "refunds",
        "order_lines",
        "order_lots",
        "advances",
        "settlement_adjustments",
        "journals",
        "orders",
        "lot_candidates",
        "lots",
        "event_products",
        "events",
        "master_products",
        // societies 不清：本社团那一行是迁移种进去的，清掉之后
        // owner_society_id 全部悬空，而且没有任何界面能把它建回来。
    ];
```

> 路线图附录第 3 条：`prereset` 与 `premigrate` 快照共用 `KEEP=3` 配额，
> `db::reset_database` 是 dead code 而 `/reset-database` 路由活着（第 4 条）。
> **这两条都不在本 plan 范围内**，不要顺手改；留给 ②-3 或单独处理。

- [ ] **Step 3: `api/event.rs`**

- `delete_event` 里删 `order_items` 和 `products` 的两大段整体删掉：新 schema 里
  `event_products` / `orders` / `journals` 对 `events` 都是 `ON DELETE CASCADE`，
  `stock_movements` / `money_movements` / `order_lines` 对各自父表也是 CASCADE。
  **保留 `DELETE FROM events` 和事务，以及事务提交后才清理二维码文件那段。**
  加注释说明改动理由。
- `update_status` 的枚举校验改成 `筹备` / `进行中` / `已结算`，错误消息同步。
- `create_event` 的默认状态：迁移里 `events.status` 的 DEFAULT 还是 `'未进行'`（老表结构），
  **`create_event` 是显式 bind status 的**（`event.rs:239`），把那里的默认值改成 `'筹备'` 即可。

- [ ] **Step 4: `vision/store.rs:341`**

```rust
                    JOIN event_products p ON p.master_product_id = mp.id
```
只改表名，`WHERE p.event_id = ?` 不变。

- [ ] **Step 5: 给收口加一条冒烟测试**

在 `api/stats.rs` 末尾加：

```rust
#[cfg(test)]
mod tests {
    use crate::test_support::{admin_token, test_router};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    /// 这条守的不是统计正确性，而是「查询还认识新 schema」。
    /// 全仓是运行时 SQL，schema 改了编译器一个都抓不到，只会在运行时炸——
    /// 而 stats 是没人写过测试的路径，正是最容易烂掉的地方。
    #[tokio::test]
    async fn sales_summary_runs_against_the_new_schema() {
        let (router, _dir) = test_router().await;
        let res = router
            .oneshot(
                Request::builder()
                    .uri("/api/events/1/sales_summary")
                    .header("authorization", format!("Bearer {}", admin_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_ne!(
            res.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "SQL 引用了不存在的表/列"
        );
    }
}
```

- [ ] **Step 6: 手工验一遍应用真的起得来**

```bash
cd /data/sunyunbo/www/ABL-BoothApp
tauri-env vnc npx tauri dev
```
在 VNC 桌面 :2 上：建展会 → 选品并录进货 → 顾客端下单 → 摊主端选渠道确认收款 → 取消一单。
**这一步不能省**——前面全是 HTTP 层测试，没有任何东西验证过前后端真的能对上话。
（前端此时还没改，会看到金额显示成分、库存字段读不到；**只确认没有 500 和白屏**，
显示问题是 Task 8 的事。）

- [ ] **Step 7: 跑门禁并提交**

Expected: 45 passed。

```bash
git add src-tauri/src
git commit -m "$(cat <<'EOF'
fix: :wrench: 把 stats / admin / event / vision 四处旁支接回新 schema

全仓是运行时 SQL（零编译期 query! 宏），schema 改了编译器一个都抓不到，
只会在运行时炸。这是 Task 2 删表后遗留的 5 处引用的完整收口，应用重新完整可用。

stats 的金额从元改到分，Excel 导出那一侧除以 100（单元格给人看）。
delete_event 的手工级联删除整段删掉，改靠 ON DELETE CASCADE。
展会状态枚举改成 筹备/进行中/已结算。
admin 的重置表清单加了注释：漏一张表会让「重置」留下孤儿数据。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: 前端接线

三件事：**金额从元改成分**、**库存字段改名**、**确认收款要选渠道**。外加一个最小的社团管理页。

**Files:** 见下面逐处清单。

- [ ] **Step 1: 建 `frontend/src/utils/money.js` 和它的单测**

```js
/**
 * 金额工具。**后端传的一律是分（整数）**，这里是唯一的换算与显示入口。
 *
 * 为什么后端不直接传元：分摊规则的核心断言是「各行金额之和必须精确等于总价」，
 * 浮点下写不出来（0.1 + 0.2 !== 0.3，而 19.9 这种价格摊位上遍地都是）。
 * 权威值在后端是整数分，前端只在显示的最后一刻除以 100。
 */

/** 分 → 元（数字）。只用于需要参与计算的场合，显示一律走 formatCents。 */
export function fromCents(cents) {
  return Number(cents || 0) / 100
}

/** 元（用户输入）→ 分。四舍五入到整数分，避免 19.99 * 100 === 1998.9999 这类浮点残渣。 */
export function toCents(yuan) {
  const n = Number(yuan)
  return Number.isFinite(n) ? Math.round(n * 100) : 0
}

/** 分 → 显示字符串，带两位小数。非法值回落到 '--'，不要让 NaN 上屏。 */
export function formatCents(cents) {
  const n = Number(cents)
  if (!Number.isFinite(n)) return '--'
  const sign = n < 0 ? '-' : ''
  const a = Math.abs(n)
  return `${sign}${Math.floor(a / 100)}.${String(a % 100).padStart(2, '0')}`
}

/** 分 → 带 ¥ 前缀的显示字符串。 */
export function formatYuan(cents) {
  const s = formatCents(cents)
  return s === '--' ? s : `¥${s}`
}
```

`frontend/src/utils/money.spec.js`：
```js
import { describe, it, expect } from 'vitest'
import { fromCents, toCents, formatCents, formatYuan } from './money.js'

describe('money', () => {
  it('formats cents with two decimals', () => {
    expect(formatCents(1990)).toBe('19.90')
    expect(formatCents(5)).toBe('0.05')
    expect(formatCents(0)).toBe('0.00')
    expect(formatCents(-1990)).toBe('-19.90')
  })

  it('never puts NaN on screen', () => {
    // 后端字段改名时漏掉某处，宁可显示 -- 也不要 NaN
    expect(formatCents(undefined)).toBe('--')
    expect(formatCents(null)).toBe('--')
    expect(formatCents('abc')).toBe('--')
  })

  it('rounds yuan input to whole cents', () => {
    // 19.99 * 100 === 1998.9999... 在 JS 里是真的
    expect(toCents(19.99)).toBe(1999)
    expect(toCents('30')).toBe(3000)
    expect(toCents('')).toBe(0)
  })

  it('round-trips through cents', () => {
    expect(toCents(fromCents(1999))).toBe(1999)
  })

  it('prefixes yuan sign', () => {
    expect(formatYuan(1990)).toBe('¥19.90')
    expect(formatYuan(undefined)).toBe('--')
  })
})
```

- [ ] **Step 2: 字段改名与金额格式化，逐处清单**

**`current_stock` → `onsite_qty`（17 处）**

| 文件:行 | 改法 |
|---|---|
| `stores/customerStore.js:80` | `existingItem.quantity < product.onsite_qty` |
| `stores/customerStore.js:87` | `product.onsite_qty > 0` |
| `views/CustomerView.vue:323` | `product.onsite_qty <= 0` |
| `views/CustomerView.vue:434,435` | `p.onsite_qty > 0` / `<= 0` |
| `views/AdminEventProducts.vue:142` | `{{ product.onsite_qty }}` |
| `components/customer/ProductGrid.vue:22,23,67,69,74,84,102,152` | 全部 `product.onsite_qty` |
| `components/vendor/LiveStats.vue:50,66,92,96,97` | 全部 `product.onsite_qty` |

**`initial_stock` → `stocked_qty`（显示处）/ 请求体字段保留 `initial_stock`（建商品时的首批进货量）**

| 文件:行 | 改法 |
|---|---|
| `components/customer/ProductGrid.vue:77` | `product.stocked_qty`，**并补除零保护**（现在没有，`LiveStats.vue:91` 有）：`product.stocked_qty ? Math.min(...) : '0%'` |
| `components/vendor/LiveStats.vue:66,91,92` | `product.stocked_qty` |
| `views/AdminEventProducts.vue:141` | `{{ product.stocked_qty }}` |
| `views/AdminEventProducts.vue:182` 及 `handleUpdate`(`:334-365`) | **删掉编辑弹窗里的「初始库存」输入框**——新模型下库存不能直接改（Task 5）。改成只读展示 + 一个「补货」按钮打 `POST /events/:id/products/:pid/restock` |

**`total_amount` → `final_amount`（5 处）**

| 文件:行 | 改法 |
|---|---|
| `stores/orderStore.js:129` | `total + order.final_amount` |
| `components/order/OrderCard.vue:31` | `{{ formatYuan(order.final_amount) }}` |
| `views/AdminEventOrders.vue:123` | `{{ formatYuan(order.final_amount) }}` |
| `views/AdminEventOrders.vue:205,208` | 金额筛选的阈值要 `toCents(minAmount)` 再比 |

**`price` → `unit_price`，且全部走 `formatYuan`**

| 文件:行 | 改法 |
|---|---|
| `stores/customerStore.js:132` | `total + item.unit_price * item.quantity`（分，整数运算） |
| `components/customer/ShoppingCart.vue:31,56,82` | `formatYuan(...)`。**`:56` 现在是唯一没有 `toFixed(2)` 的地方，`19.9` 会显示成 `¥19.9`——顺手修掉** |
| `components/customer/ProductGrid.vue:99,170-173` | `formatPrice` 整个换成 `formatYuan(product.unit_price)` |
| `components/customer/PaymentModal.vue:7` | `formatYuan(total)` |
| `views/AdminEventProducts.vue:140` | `formatYuan(product.unit_price)`。**现在是 `product.price.toFixed(2)`，无空值保护，后端返 null 就白屏** |
| `views/AdminEventProducts.vue:91,172,262,305,306,310,339,342,355` | 表单里用户输入的是**元**，提交前 `toCents()` |
| `components/order/OrderCard.vue:23` | `formatYuan(item.product_price)` |

**`master_products.default_price` 保持元（后端那一列没改）**——`CreateMasterProductForm.vue` / `EditMasterProductModal.vue` 不用动。在 `AdminEventProducts.vue:267` 回填时要 `toCents(product.default_price)`。顺手修掉 `:267` 的 `if (product.default_price)` truthy 判断（`default_price === 0` 不回填的 bug），改成 `!= null`。

**展会状态文案（`未进行` → `筹备`、`已结束` → `已结算`）**

`views/AdminLayout.vue:182`、`views/VendorEventSelection.vue:61`、`components/event/EventList.vue:67-81,184-186`、`views/EventPortalView.vue:111`、`config/helpContent.js:30-31`、`views/AdminControlPanel.vue:320,366`、`views/AdminEventOrders.vue:147`。
`进行中` 不变，只改另外两个。

- [ ] **Step 3: 确认收款要选渠道**

`stores/orderStore.js:83-102` 的 `markOrderAsCompleted` 加参数：
```js
async function markOrderAsCompleted(orderId, channel) {
  if (!channel) throw new Error('请选择收款渠道')
  await api.put(`/events/${activeEventId.value}/orders/${orderId}/status`, {
    status: 'completed',
    channel,
  })
  // ...（本地挪动逻辑不变）
}
```

`views/VendorView.vue:157-166` 的 `completeOrder`：点「完成配货」先弹一个渠道选择（`n-radio-group`，选项来自本地常量 `['现金','微信','支付宝']`），选完再调。
**把上次选的渠道记在 `localStorage`** 做默认值——现场一场展会里渠道基本不变，每单重选是纯摩擦。

`stores/eventDetailStore.js:97-113` 的 `adminUpdateOrderStatus` 同样要能带 channel。

**同时必须改文案——这是 spec 第 11 节的不可破坏项，不是润色。**

引入资金账户之后，「确认收款」会记一笔真实的资金移动，界面看起来比以前更像「钱到账了」。
但支付依然是**非闭环**的：系统从来不知道顾客有没有真付，摊主点的是「我看到到账提示了」。
`docs/guide/workflow.md` 那句「『订单已完成』仅代表记录了一笔账，不代表钱真的到了你的账户」
在复式记账引入后**依然成立且更需要强调**。

- 渠道选择弹窗的标题用「**记录收款方式**」，不要用「收款」「已收款」。
- 弹窗里带一行小字：「这只是记账。请先确认手机上真的收到了到账提示，再点确认。」
- `PaymentModal.vue` 顾客端那句「确认已付款 · 关闭」不要改成任何暗示系统已验证的措辞。
- 同步更新 `docs/guide/workflow.md` 第 2 节第三步，把「确认收款」改成「记录收款方式」并保留原有的诚实披露句。

- [ ] **Step 4: 社团管理最小页面**

`frontend/src/stores/societyStore.js`：`fetchSocieties` / `createSociety` / `updateSociety` / `deleteSociety`，打 Task 4 那四个端点。
`frontend/src/views/AdminSocieties.vue`：一张表（名字 / 是否本社团 / 操作），加一个「设为本社团」按钮和新建输入框。挂到 `AdminLayout` 的导航里。
`views/AdminEventProducts.vue` 的选品预览里显示归属社团名（后端 `EventProductResponse.owner_society_name` 已经给了）。

- [ ] **Step 5: 跑前端门禁**

```bash
cd /data/sunyunbo/www/ABL-BoothApp
npm run lint --prefix frontend
npm run format:check --prefix frontend
npm run test:unit --prefix frontend
npm run build --prefix frontend
```
Expected: lint 0 warning；money.spec.js 5 个新用例通过（原有 20 个仍通过）。

- [ ] **Step 6: 在 VNC 上真跑一遍完整链路**

```bash
tauri-env vnc npx tauri dev
```
走一遍：建展会（状态「筹备」）→ 建社团「黄昏堂」→ 选品（一个归本社团、一个归黄昏堂）录进货 → 改状态「进行中」→ 顾客端下单 → 摊主端选「微信」确认收款 → 再下一单并取消 → 检查库存数字在三处（顾客端卡片、摊主端 LiveStats、管理端列表）一致。

**这一步是本 plan 唯一能发现「前后端字段名对不上」的地方**，HTTP 测试和前端单测都覆盖不到。

- [ ] **Step 7: Commit**

```bash
git add frontend/src
git commit -m "$(cat <<'EOF'
feat: :lipstick: 前端接线到新领域模型——金额用分、库存字段改名、收款录渠道

金额一律传分，新增 frontend/src/utils/money.js 作为唯一换算与显示入口；
非法值显示 '--' 而不是 NaN。顺手修掉三处既有缺陷：ShoppingCart 单价漏了
toFixed(2)（19.9 显示成 ¥19.9）、ProductGrid 库存条没有除零保护、
AdminEventProducts 选品回填用 truthy 判断导致 default_price === 0 不回填。

current_stock → onsite_qty，initial_stock → stocked_qty。编辑商品的「初始库存」
输入框删掉，改成补货按钮——新模型下库存不能直接改数字。

确认收款要选渠道（spec 6.2：否则钱那条腿没有对手账户），上次选择记 localStorage
做默认值，现场一场展会里渠道基本不变。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: 首启迁移提示与历史数据导出

老用户升级后打开会发现展会和订单都没了。Task 2 已经把老库存成 `sale_system.db.v1-backup`，
这一步给它一个出口——**否则那份备份等于不存在**（路线图附录第 2 条对快照的批评，同样适用于这里）。

**Files:**
- Create: `src-tauri/src/api/legacy.rs`、`frontend/src/views/MigrationNotice.vue`
- Modify: `src-tauri/src/api/mod.rs`、前端路由

- [ ] **Step 1: 后端两个端点**

| 方法 | 路径 | 行为 |
|---|---|---|
| GET | `/api/legacy/status` | `{ has_backup: bool, path: String, event_count: i64, order_count: i64 }`；打开备份库只读查三个数 |
| GET | `/api/legacy/export.xlsx` | 用 `rust_xlsxwriter` 把备份库里的展会/订单/明细导成一个 xlsx |

实现要点：
- 用 `SqlitePool::connect(&format!("sqlite://{}?mode=ro", path))` **只读**打开备份库。
  加注释：`mode=ro` 不是洁癖——这份文件是用户数据的最后一份副本，任何写入都不可接受。
- 备份不存在时 `has_backup: false`，前端不显示提示页。
- 导出的三张 sheet：`展会` / `订单` / `订单明细`，列名用中文，金额按老库的元原样写。
  **不要试图换算成分**——老库那一侧就是元，换算只会引入误差。

- [ ] **Step 2: 前端提示页**

`MigrationNotice.vue`：`App.vue` 挂载时调 `/api/legacy/status`，`has_backup && event_count > 0` 就弹一次
（弹过之后写 `localStorage.setItem('migration_notice_seen','1')`）。内容：

> **v1.2 更新了账本模型**
> 旧版的展会和订单记录没有迁移到新模型（新模型要记录每一件货的来源和去向，旧数据补不出这些信息）。
> 你的旧数据**完整保留**在 `sale_system.db.v1-backup`，随时可以导出。
> 商品库（含图片和识别数据）已经自动带过来了。
> 　[导出旧数据为 Excel]　[知道了]

- [ ] **Step 3: 测试**

后端：备份不存在时 `has_backup: false` 且不 500；备份存在时三个计数正确。
前端：`localStorage` 已标记时不弹。

- [ ] **Step 4: 跑全量门禁 + VNC 验证 + Commit**

---

## 完成标准

全部打勾才算 ②-1 完成：

- [ ] `cd src-tauri && tauri-env linux cargo fmt --all --check` 无输出
- [ ] `tauri-env linux cargo clippy --all-targets --all-features -- -D warnings` 零警告
- [ ] `tauri-env linux cargo test --all-features` 全绿，且**测试数从 20 增长到 50 左右**
- [ ] `npm run lint --prefix frontend` 零警告；`format:check` / `test:unit` / `build` 全绿
- [ ] `tauri-env vnc npx tauri dev` 起得来，且能完整走通：
      建展会 → 建代卖社团 → 选品录进货 → 顾客下单 → 摊主选渠道收款 → 取消一单 →
      三处库存数字一致
- [ ] `docs/guide/workflow.md` 的「确认收款」文案已按 spec 第 11 节改成「记录收款方式」，诚实披露句保留
- [ ] `grep -rn "current_stock\|initial_stock\|order_items\|total_amount" src-tauri/src frontend/src`
      只剩注释和 `legacy.rs`（读老库）里的命中
- [ ] `grep -rn "f64" src-tauri/src/api src-tauri/src/domain` 没有任何一处表示金额
      （`master_products.default_price` 是唯一例外，且只在 `product.rs` 的换算处出现）
- [ ] 老库升级路径验过：拿一份 v1.1.1 的 `sale_system.db` 放进 app data 目录启动，
      确认 `sale_system.db.v1-backup` 生成、商品库还在、提示页弹出、导出 xlsx 能打开

### 交给 ②-2 / ②-3 的接口契约与已知不变量

②-2 只需要改这三处，其余不该动：

1. `api/order.rs` 的 `create_order`：把「`allocated = paid = unit_price × qty`」换成求解器 + 分摊的结果，并填 `order_lots`。
2. `api/order.rs` 的收款 journal：在按货主分组的腿之外，追加一条 `实收-<渠道> → 社团往来:<本社团>` 的手工折让腿（金额 = `solved − final`，为 0 时跳过，`post_journal` 已经会过滤）。
3. 新增 `domain/solver.rs`（纯函数）和 `api/lot.rs`（Lot 配置 CRUD）。

**不要改**：`domain/ledger.rs` 的任何签名、`orders` / `order_lines` 的表结构、`EventProductResponse` 的字段。

### ②-3 必须知道的两条不变量（执行中发现，别让它们从注释里蒸发）

1. **「订单是 completed」不蕴含「存在收款 journal」。**
   spec 4.6 的赠品是 0 元行，一张全赠品订单的各货主合计都是 0，所有资金腿都被滤掉，
   于是收款 journal 整个被跳过——订单仍然是 `completed` 且 `channel` 非空。
   这是唯一可行解（记 0 元腿撞 `CHECK(amount > 0)`；记空 journal 会造出「造得出却冲不掉」
   的幽灵 journal；不跳过则完成订单直接 400）。
   **②-3 做结算对账时，若从 `journals WHERE kind='收款'` 反推已收款订单，会漏掉全赠品单。**
   要按 `orders.status` 而不是按 journal 存在性来判定。

2. **「有 `order_lines` 就必然有 `stock_movements`」结构上成立。**
   销售 journal 的 `money` 参数恒为 `&[]`，所以 `stock_legs` 必须非空、否则 `post_journal`
   直接报错。`api/product.rs` 的「有流水才禁止删除」守卫因此是完备的，不需要额外查 `order_lines`。
   **②-2/②-3 若引入任何「只写 order_lines 不写 stock_movements」的路径（比如纯服务类商品），
   这个不变量就破了，那个删除守卫必须同步补上 `order_lines`。**

3. **`events.status` 只有 `进行中` 能下单**（Task 7 加的守卫）。完整的冻结语义
   （不能退货、不能改盘点数）仍归 ②-3。
