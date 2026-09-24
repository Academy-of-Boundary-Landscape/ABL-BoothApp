# ②-3 展会闭环与结算 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把展会从「货进得来、出不去」推到闭环——收摊向导（清 pending / 盘点 / 带回 / 冻结）、赠送与报废、退货、垫付与结算调整、自定义渠道、结算单与四 sheet 导出。

**Architecture:** 12 张表在 ②-1 就建齐了，本轮只往里写数据 + 一条 `ALTER TABLE events ADD COLUMN stocktaken_at`。新代码按业务块开四个 `api/` 模块（`inventory` / `closing` / `refund` / `settlement`）加一个纯函数领域模块 `domain/settlement.rs`；结算单页面与 xlsx 导出**渲染同一个 `SettlementReport`**。冻结语义收口成 `guard::require_event_open()` 一个函数，外加一条 CI 门禁挡住新写入口漏守。

**Tech Stack:** Rust / axum 0.7 / sqlx 0.9（运行时 `migrate!()`，无 `DATABASE_URL`）/ rust_xlsxwriter 0.84 / Vue 3 `<script setup>` / Pinia / vitest。

**Spec:** `docs/superpowers/specs/2026-09-24-event-closing-and-settlement-design.md`

**前一份 plan:** `docs/superpowers/plans/2026-09-23-lot-and-discount.md`（②-2，末尾「交给 ②-3 的接口契约与已知不变量」**动手前必读**）

---

## Global Constraints

本机与本项目的硬约束，**每个 task 都适用**，不再逐条重复：

- **构建命令必须带前缀。** 裸 `cargo` 缺 webkit2gtk 必失败。Rust 侧一律：
  `cd src-tauri && tauri-env linux cargo test --workspace`、`tauri-env linux cargo clippy --all-targets -- -D warnings`、`tauri-env linux cargo fmt`。
- **前端在 `frontend/`，不在仓库根。** `npm --prefix frontend run test:unit`、`npm --prefix frontend run build`。
- **不要起 dev server。** `npx tauri dev` / `vite` 永不退出，会卡死整批任务。要看界面是人的活，不是 worker 的活。
- **金额一律整数分（`Money`，`src-tauri/src/domain/money.rs`）。** `f64` 不得用于任何金额。前端后端一律传分，只在显示的最后一刻用 `@/utils/money` 的 `formatYuan` 换算。
- **货和钱唯一的写入口是 `domain::ledger::post_journal()`。** 不得直接 INSERT `stock_movements` / `money_movements`（夹具也不行）。
- **`post_journal` 拒绝空 journal**（`stock` 和过滤掉零金额后的 `money` 同时为空 → `BadRequest("空的 journal：既没有货也没有钱")`）。凡是「可能一条腿都没有」的路径，必须在调用前判空并跳过。
- **资金腿金额恒为正，方向由 `from`/`to` 表达。** 传负数 → `BadRequest("资金移动的金额必须为正——方向由 from/to 表达，不靠负数")`。
- **`post_journal` 的 `order_id` 与 `reverses` 相邻且同为 `Option<i64>`，传反编译器不报错。** 记普通业务操作时 `reverses` 恒为 `None`。
- **错误一律用 `crate::error::ApiError`**，响应体形状必须是 `{"error": "..."}`——前端 `services/api.js` 的 adapter 和全仓的 `err.response?.data?.error` 都依赖它。
- **不得改动任何已被应用过的迁移文件。** 本轮只新增 `202609240002_add_stocktaken_at.sql`。改已发布的迁移会让老用户 App 启动 panic（`docs/BUILD.md`「发布前必查」）。
- **不得改动 `domain/solver.rs`、`domain/allocation.rs` 的 `allocate_lot` / `apply_manual_adjustment` 签名**，不得改 `order_lines` / `order_lots` 表结构。（Task 4 把私有的 `apportion` 改成 `pub`，这是**新增导出**，不是改上述三个签名。）
- **测试夹具在 `src-tauri/src/test_support.rs`**（`#[cfg(test)]`）：`test_router_with()` 返回 `(Router, TempDir, SqlitePool)`，`seed_event_and_product(&pool)` 种一场「进行中」的展会 + 商品 A（本社团 1，¥30，10 件）/ B（代卖「黄昏堂」2，¥20，5 件）。`admin_token()` / `json_request()` / `read_json()` 直接用。
- **`test_pool()` 是 `max_connections(1)` 的内存库**，所有查询天然串行。
- **每个 task 结束前必须跑**：`tauri-env linux cargo fmt`、`cargo clippy --all-targets -- -D warnings`、`cargo test --workspace`（前端 task 则是 `npm --prefix frontend run lint` + `test:unit` + `build`），全绿才提交。
- **提交信息用 gitmoji 中文格式**，末尾带 `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>`。

---

## Review Focus

spec 是愿景文档，它说软件必须做到什么，不说软件会遇到什么。下面五类输入 spec 没有点名，但最可能咬到真实用户。**每一条在下面都被指派给了拥有那段代码的 task，以该 task 自己的步骤形式写进去了**，这里只做索引：

1. **同一商品被并发扣两次**（两台手机同时报废/退货同一件货）→ 现场仓余额被扣成负数，之后 `settle` 永远过不去。期望：第二个请求 409，余额不为负。→ **Task 3 Step 1** 的 `successive_scraps_cannot_overdraw_either`（`BEGIN IMMEDIATE` + 事务内余额复核）
2. **退款金额为 0**（顾客只退货不要钱，或摊主把 R 抹成 0）→ 三条腿里有两条金额为 0 被 `post_journal` 滤掉，只剩一条，journal 仍须成立且货腿照走。期望：200，货回现场仓，实收账户净出 0。→ **Task 4 Step 3** 的 `refunding_zero_still_moves_the_goods`
3. **0 元商品参与退货**（预售取货用的 0 元 SKU，母 spec 4.6）→ 按 `paid` 权重摊回退款时权重全零，`apportion` 会返回 `BadRequest("无法分摊：权重之和为零")`。期望：不报错，按件数退，退款额 0。→ **Task 4 Step 3** 的 `a_zero_price_line_refunds_without_dividing_by_zero`
4. **渠道名是纯空白、超长、或带换行**→ 它会成为账户名 `实收-<渠道>` 的一部分写进 `money_movements`，污染之后没有任何办法清理。期望：规范化后为空 → 400；超过 20 字 → 400；内部连续空白折叠成一个。→ **Task 2 Step 1** 的 `rejects_blank` / `length_limit_counts_chars_not_bytes` / `trims_and_collapses_inner_whitespace`
5. **`societies` 里没有 `is_home = 1` 的行**（用户把本社团删了，或库是手工改过的）→ 手工折让腿和退货第二条腿找不到对手账户，`unwrap` 会 panic 掉整个 handler。期望：500 之前先给出可读的 `Conflict("没有设置本社团…")`。→ **Task 4 Step 9** 的 `refunding_without_a_home_society_fails_readably`

---

## 文件结构

**新建：**

| 文件 | 职责 |
|---|---|
| `src-tauri/migrations/202609240002_add_closing_timestamps.sql` | `events` 加 `stocktaken_at` / `reconciled_at` 两列 |
| `src-tauri/src/domain/channel.rs` | 收款渠道名的规范化与上限，纯函数 |
| `src-tauri/src/domain/settlement.rs` | 结算单计算，**纯函数，不 import sqlx** |
| `src-tauri/src/api/inventory.rs` | 赠送 / 报废登记与撤销 |
| `src-tauri/src/api/refund.rs` | 退货 |
| `src-tauri/src/api/closing.rs` | 收摊向导四步 |
| `src-tauri/src/api/settlement.rs` | 垫付 / 结算调整 / 渠道列表 / 结算单 / 收摊清点 / xlsx |
| `scripts/check-event-guards.py` | CI 门禁：非 GET handler 必须守展会状态 |
| `frontend/src/stores/closingStore.js` | 收摊向导状态 |
| `frontend/src/stores/settlementStore.js` | 垫付 / 调整 / 结算单 |
| `frontend/src/utils/refund.js` | 退款金额摊回，纯函数 |
| `frontend/src/utils/refund.spec.js` | 上者的 vitest |
| `frontend/src/components/vendor/RefundModal.vue` | 退货弹窗 |
| `frontend/src/components/vendor/InventoryLogModal.vue` | 赠送 / 报废登记 |
| `frontend/src/components/vendor/ClosingWizard.vue` | 收摊向导 |
| `frontend/src/components/shared/ChannelSelect.vue` | 可输入的渠道下拉，三处共用 |
| `frontend/src/views/AdminEventSettlement.vue` | 管理端结算单页 |

**修改：**

| 文件 | 改什么 |
|---|---|
| `src-tauri/src/api/guard.rs` | 加 `require_event_open()` |
| `src-tauri/src/api/lot.rs` | 私有 `ensure_event_open` 换成 `guard::require_event_open` |
| `src-tauri/src/api/order.rs` | `update_order_status` 加守卫；渠道走规范化 |
| `src-tauri/src/api/product.rs` | 三个写入口加守卫 |
| `src-tauri/src/api/mod.rs` | merge 四个新 router |
| `src-tauri/src/domain/mod.rs` | 声明两个新模块 |
| `src-tauri/src/domain/allocation.rs` | `fn apportion` → `pub fn apportion` |
| `.github/workflows/ci.yml` | 接入守卫门禁 |
| `frontend/src/views/VendorView.vue` | 挂三个新入口 |
| `frontend/src/router/index.js` | 加结算单路由 |
| `frontend/src/views/AdminLayout.vue` | 侧栏加结算单入口 |
| `frontend/src/components/vendor/ReceiptModal.vue` | 硬编码的三个 radio 换成共用的渠道选择组件 |

---

## 任务总览

| # | Task | 交付物 |
|---|---|---|
| 1 | 冻结守卫收口 + 4 个既有敞口 + CI 门禁 | `require_event_open` 全仓唯一，漏守会被 CI 拦 |
| 2 | 渠道规范化 + `/api/channels` | 账户名不再可能被污染 |
| 3 | 赠送与报废 | `api/inventory.rs`，含撤销 |
| 4 | 退货 | `api/refund.rs`，三条腿 + 两个金额 |
| 5 | 收摊向导 | 迁移 + `api/closing.rs` 四步 |
| 6 | 垫付与结算调整 | `api/settlement.rs` 第一部分 |
| 7 | `domain/settlement.rs` 纯函数 | 结算单的全部算术，零 I/O |
| 8 | 结算单 JSON + 收摊清点 | `GET /settlement`、`POST /settlement/reconcile` |
| 9 | `settlement.xlsx` 四 sheet | 与页面同源 |
| 10 | 前端：赠送/报废登记 | 摊主端 |
| 11 | 前端：退货弹窗 | 摊主端 |
| 12 | 前端：收摊向导 | 摊主端 |
| 13 | 前端：管理端垫付/调整 + 渠道选择器 | |
| 14 | 前端：管理端结算单页 + 导出 | |
| 15 | 文档、②-2 遗留三条测试、交接段 | |

---

## Task 1: 冻结守卫收口 + 补齐 4 个既有敞口 + CI 门禁

**Files:**
- Modify: `src-tauri/src/api/guard.rs`
- Modify: `src-tauri/src/api/lot.rs`（删私有 `ensure_event_open`，约 :84-95）
- Modify: `src-tauri/src/api/order.rs`（`update_order_status`，约 :517 `let mut tx` 之后）
- Modify: `src-tauri/src/api/product.rs`（`add_product_to_event` / `restock_product` / `update_product`）
- Create: `scripts/check-event-guards.py`
- Modify: `.github/workflows/ci.yml`
- Test: `src-tauri/src/api/order.rs` 与 `src-tauri/src/api/product.rs` 各自的 `#[cfg(test)] mod tests`

**Interfaces:**
- Produces: `crate::api::guard::require_event_open(conn: &mut SqliteConnection, event_id: i64) -> ApiResult<()>` —— Task 3/4/5 全都调用它。参数是 `&mut SqliteConnection` 而不是 `&SqlitePool`，这样在事务里（`&mut *tx`）和事务外（`&mut *state.db.acquire().await?`）都能用，**且在事务内检查才是真的守住**。
- Consumes: 无。

- [ ] **Step 1: 写失败的测试——四个既有敞口在已结算展会上必须 409**

加到 `src-tauri/src/api/order.rs` 的 `mod tests` 末尾：

```rust
    /// ②-1 交接段列了 4 个「当前完全不查 events.status」的既有敞口，②-2 一个没补。
    /// 冻结语义靠守卫而不是「流程上到不了」，所以每个口子都要有自己的测试。
    #[tokio::test]
    async fn a_settled_event_refuses_order_status_change() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let order_id = place(&router, event_id, json!([{"product_id": ep_a, "quantity": 1}])).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id).execute(&pool).await.unwrap();
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
```

加到 `src-tauri/src/api/product.rs` 的 `mod tests` 末尾（若该模块尚无 `mod tests`，按 `api/lot.rs` 的头部照搬 `use` 列表新建一个）：

```rust
    /// 三个商品写入口在冻结后都要挡住。分开三条而不是循环，
    /// 是因为它们各自的 URL 形状和 body 不同，合起来写会把断言糊掉。
    #[tokio::test]
    async fn a_settled_event_refuses_adding_products() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id).execute(&pool).await.unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/products"),
                Some(&token),
                json!({"master_product_id": 1, "unit_price": 3000, "initial_stock": 5}),
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
            .bind(event_id).execute(&pool).await.unwrap();
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
            .bind(event_id).execute(&pool).await.unwrap();
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
```

> ⚠️ 上面三个请求体的字段名要和 `api/product.rs` 里现有的 `Deserialize` 结构体对齐——**先读那三个 handler 的请求结构体再写**，字段对不上会得到 422 而不是 409，测试变成假绿。

- [ ] **Step 2: 跑测试确认失败**

```bash
cd src-tauri && tauri-env linux cargo test --workspace a_settled_event_refuses
```
Expected: 四条全 FAIL（拿到 200/201 而不是 409）。

- [ ] **Step 3: 在 `guard.rs` 里实现 `require_event_open`**

在 `src-tauri/src/api/guard.rs` 末尾追加（`use` 处补 `sqlx::SqliteConnection`）：

```rust
/// 已结算的展会账已冻结，不能再改账。
///
/// **判据是「不是已结算」，不是「必须进行中」**——筹备阶段本来就要能选品、
/// 改价、录带货数。要求「必须进行中」的只有下单一处（`api/order.rs` 的
/// `create_order`，②-1 Task 7 加的），那条更严的检查保持独立，不要合并进来。
///
/// 参数是 `&mut SqliteConnection` 而不是 `&SqlitePool`：在事务里检查才是真的守住，
/// 事务外查一遍再进事务写，中间隔着一个可以被 `settle` 插进来的窗口。
///
/// 冻结之后**仍然允许**三件事，它们不调用本函数（spec 偏离 3）：
/// 垫付、结算调整、收摊清点。改动那三处前先读 spec 3.2。
pub async fn require_event_open(
    conn: &mut sqlx::SqliteConnection,
    event_id: i64,
) -> ApiResult<()> {
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM events WHERE id = ?")
        .bind(event_id)
        .fetch_optional(&mut *conn)
        .await?;
    match status.as_deref() {
        None => Err(ApiError::NotFound("展会不存在".into())),
        Some("已结算") => Err(ApiError::Conflict("展会已结算，不能再改账".into())),
        Some(_) => Ok(()),
    }
}
```

- [ ] **Step 4: 四个敞口逐个接上**

`api/order.rs` 的 `update_order_status`，在 `let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;` 的**下一行**插入：

```rust
    // ②-1 交接段列的第 4 个敞口。放在事务内、读订单状态之前。
    crate::api::guard::require_event_open(&mut tx, event_id).await?;
```

`api/product.rs` 的三个 handler：`add_product_to_event` 与 `restock_product` 已经有事务，在 `begin` 之后立刻插同样一行；`update_product` 若没有事务，在 handler 开头加：

```rust
    let mut conn = state.db.acquire().await?;
    crate::api::guard::require_event_open(&mut conn, event_id).await?;
    drop(conn);
```

> `update_product` 的路径是 `/products/:id`，**没有 `event_id`**。先反查：
> ```rust
> let event_id: i64 = sqlx::query_scalar("SELECT event_id FROM event_products WHERE id = ?")
>     .bind(product_id)
>     .fetch_optional(&mut *conn)
>     .await?
>     .ok_or_else(|| ApiError::NotFound("商品不存在".into()))?;
> ```
> 这一步同时替掉了原来「商品不存在」的判断，不要重复查两遍。

- [ ] **Step 5: 把 `lot.rs` 的私有版本换成公共版本**

删掉 `api/lot.rs` 约 :80-95 的 `ensure_event_open`，把四处调用改成
`crate::api::guard::require_event_open(&mut *conn, event_id).await?`。

错误文案会从「展会已结算，不能再改套装配置」变成「展会已结算，不能再改账」。**`api/lot.rs` 现有测试只断言状态码，不断言文案**，不会红；若发现有断言文案的，改测试而不是留两份函数。

- [ ] **Step 6: 跑测试确认通过**

```bash
cd src-tauri && tauri-env linux cargo test --workspace
```
Expected: 全绿，包含 Step 1 的四条。

- [ ] **Step 7: 写 CI 门禁脚本**

`scripts/check-event-guards.py`：

```python
#!/usr/bin/env python3
"""每个非 GET 的 API handler 要么守展会状态，要么写明为什么不需要。

②-1 列出 4 个「不查 events.status」的敞口，②-2 一个没补，②-3 又新增了
十来个写入口——靠记是记不住的，所以做成门禁。

判据：凡是被 post()/put()/patch()/delete() 包起来的 handler 函数，
函数体里必须出现 require_event_open，或者出现豁免标记注释
    // 不需要展会守卫：<理由>
理由必须非空。
"""
import re
import sys
from pathlib import Path

API_DIR = Path(__file__).resolve().parent.parent / "src-tauri" / "src" / "api"
ROUTE_RE = re.compile(r"\b(?:post|put|patch|delete)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)")
EXEMPT_RE = re.compile(r"//\s*不需要展会守卫：\s*\S+")

def handler_body(src: str, name: str) -> str | None:
    m = re.search(rf"^async fn {re.escape(name)}\s*\(", src, re.M)
    if not m:
        return None
    # handler 一律是顶层函数，结束于第一个位于第 0 列的 '}'
    end = src.find("\n}\n", m.start())
    return src[m.start() : end if end != -1 else len(src)]

def main() -> int:
    problems = []
    for path in sorted(API_DIR.glob("*.rs")):
        src = path.read_text(encoding="utf-8")
        # 只看 router() 里的注册，避免把 `post(` 的其它用法算进来
        for name in sorted(set(ROUTE_RE.findall(src))):
            body = handler_body(src, name)
            if body is None:
                problems.append(f"{path.name}: 路由注册了 {name}，但找不到 `async fn {name}(`")
                continue
            if "require_event_open" in body or EXEMPT_RE.search(body):
                continue
            problems.append(
                f"{path.name}:{name} 既没调用 require_event_open，"
                f"也没写 `// 不需要展会守卫：<理由>`"
            )
    if problems:
        print("展会守卫门禁未通过：", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        print(
            "\n写入口必须守住 events.status（spec 3.1）。"
            "确实不需要的（登录、全局商品库、展会本身的 CRUD 等），"
            "在函数体里写一行 `// 不需要展会守卫：<理由>`。",
            file=sys.stderr,
        )
        return 1
    print("展会守卫门禁通过")
    return 0

if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 8: 跑门禁，给现有的豁免 handler 补注释**

```bash
python3 scripts/check-event-guards.py
```
Expected: **第一次必然失败**，列出一批本来就不需要守卫的 handler（`auth.rs` 的登录、`event.rs` 的建/改/删展会、`master_product.rs` 全部、`society.rs` 全部、`admin.rs`、`sync.rs`、`vision.rs` 的模型管理等）。

逐个在函数体第一行加豁免注释，理由要具体。例如：

```rust
async fn login(/* ... */) {
    // 不需要展会守卫：登录不属于任何展会
```

```rust
async fn update_society(/* ... */) {
    // 不需要展会守卫：社团是全局实体，改名不动任何展会的账
```

```rust
async fn update_status(/* ... */) {
    // 不需要展会守卫：这个 handler 本身就是改展会状态的那一个
```

反复跑到通过为止。**不要为了让门禁过而给真正需要守卫的 handler 写豁免**——判断标准是「它写不写 journals / stock_movements / money_movements / order_lines / event_products」。

- [ ] **Step 9: 接进 CI**

`.github/workflows/ci.yml` 的 `rust` job，在 `cargo test` 之前加一步：

```yaml
      - name: 展会守卫门禁
        run: python3 scripts/check-event-guards.py
```

- [ ] **Step 10: 全量验证**

```bash
cd src-tauri && tauri-env linux cargo fmt
cd src-tauri && tauri-env linux cargo clippy --all-targets -- -D warnings
cd src-tauri && tauri-env linux cargo test --workspace
python3 scripts/check-event-guards.py
```
Expected: 四条全绿。

- [ ] **Step 11: 提交**

```bash
git add src-tauri/src/api/guard.rs src-tauri/src/api/lot.rs src-tauri/src/api/order.rs \
        src-tauri/src/api/product.rs scripts/check-event-guards.py .github/workflows/ci.yml
git commit -m "$(cat <<'EOF'
feat: :lock: 冻结守卫收口成一个函数，并补上 ②-1 列的四个敞口

判据是「不是已结算」而不是「必须进行中」——筹备阶段本来就要能选品改价。
require_event_open 收在 guard.rs，参数取 &mut SqliteConnection，好让检查
发生在事务内；事务外查一遍再进事务写，中间隔着能被 settle 插进来的窗口。

顺手加了 CI 门禁：非 GET handler 要么调用它，要么写明为什么不需要。
②-1 列出四个敞口、②-2 一个没补，靠记是记不住的。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: 收款渠道的规范化与历史列表

**Files:**
- Create: `src-tauri/src/domain/channel.rs`
- Modify: `src-tauri/src/domain/mod.rs`
- Create: `src-tauri/src/api/settlement.rs`（本 task 只放 `/channels` 一个路由，Task 6/8/9 往里加）
- Modify: `src-tauri/src/api/mod.rs`
- Modify: `src-tauri/src/api/order.rs`（完成订单时的渠道处理）
- Test: `src-tauri/src/domain/channel.rs` 的 `mod tests` + `api/settlement.rs` 的 `mod tests`

**Interfaces:**
- Produces:
  - `crate::domain::channel::normalize(raw: &str) -> ApiResult<String>` —— Task 4（退货）和 `api/order.rs` 都调用。
  - `GET /api/channels` → `["现金","微信","支付宝", ...历史用过的]`
  - `crate::api::settlement::router() -> Router<AppState>`
- Consumes: Task 1 的 `require_event_open`（本 task 的两个端点都**不**调用它——`/channels` 是 GET，且跨展会）。

- [ ] **Step 1: 写失败的纯函数测试**

新建 `src-tauri/src/domain/channel.rs`，先只写测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_collapses_inner_whitespace() {
        assert_eq!(normalize("  微信  ").unwrap(), "微信");
        assert_eq!(normalize("银行 \t 转账").unwrap(), "银行 转账");
        assert_eq!(normalize("换\n行").unwrap(), "换 行");
    }

    #[test]
    fn rejects_blank() {
        // 空渠道名会造出账户 "实收-"，写进 money_movements 之后没有任何办法清理
        assert!(normalize("").is_err());
        assert!(normalize("   ").is_err());
        assert!(normalize("\t\n").is_err());
    }

    #[test]
    fn length_limit_counts_chars_not_bytes() {
        // 20 个汉字是 60 字节。按字节限长会让中文渠道名只能写 6 个字。
        let twenty = "一二三四五六七八九十一二三四五六七八九十";
        assert_eq!(twenty.chars().count(), 20);
        assert!(normalize(twenty).is_ok());
        assert!(normalize(&format!("{twenty}超")).is_err());
    }

    #[test]
    fn keeps_hyphens_intact() {
        // Account::from_str 用 strip_prefix 取剩余全部，所以带横线的渠道名能原样解回来。
        // 这条钉住的是「不要为了好解析而禁掉横线」。
        assert_eq!(normalize("自定义-渠道").unwrap(), "自定义-渠道");
    }
}
```

- [ ] **Step 2: 跑测试确认失败**

先在 `src-tauri/src/domain/mod.rs` 加 `pub mod channel;`，然后：

```bash
cd src-tauri && tauri-env linux cargo test --workspace channel::tests
```
Expected: 编译失败，`cannot find function normalize`。

- [ ] **Step 3: 实现 `normalize`**

`src-tauri/src/domain/channel.rs` 顶部：

```rust
//! 收款渠道名的规范化。
//!
//! **渠道名会成为账户名的一部分**（`实收-<渠道>`，见 `domain/ledger.rs` 的 `Account`），
//! 一旦写进 `money_movements` 就没有任何办法清理——账户不是一张表，是一堆字符串。
//! 所以规范化必须发生在**写库之前**，而不是显示的时候。
//!
//! ②-1 与 ②-2 的交接段都点名了这件事（「channel 目前是无约束自由文本」）。
//! 三件套：规范化、长度上限、给前端一个已用渠道列表让摊主从已有的里挑。
//! 第三条在 `api/settlement.rs` 的 `GET /channels`，才是真正防「微信」和
//! 「微信支付」分裂成两个账户的那一条。
//!
//! **不加白名单**——母 spec 第 5 节明写「渠道可自定义（有社团用银行转账、有的用闲鱼）」。

use crate::error::{ApiError, ApiResult};

/// 渠道名最长 20 个**字符**（不是字节）。20 个汉字 = 60 字节，按字节限长
/// 会让中文渠道名只能写 6 个字。
pub const MAX_CHANNEL_CHARS: usize = 20;

/// 预置渠道。母 spec 第 5 节：「至少预置现金/微信/支付宝三种」。
pub const PRESET_CHANNELS: [&str; 3] = ["现金", "微信", "支付宝"];

/// 规范化：首尾去空白，内部连续空白（含制表与换行）折叠成一个半角空格。
///
/// 折叠而不是禁止，是因为「银行 转账」这种中间带空格的写法是合理的，
/// 而「银行&nbsp;&nbsp;转账」和「银行 转账」必须是同一个账户。
pub fn normalize(raw: &str) -> ApiResult<String> {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return Err(ApiError::BadRequest("收款渠道不能为空".into()));
    }
    let n = collapsed.chars().count();
    if n > MAX_CHANNEL_CHARS {
        return Err(ApiError::BadRequest(format!(
            "收款渠道名最长 {MAX_CHANNEL_CHARS} 个字，当前 {n} 个"
        )));
    }
    Ok(collapsed)
}
```

- [ ] **Step 4: 跑测试确认通过**

```bash
cd src-tauri && tauri-env linux cargo test --workspace channel::tests
```
Expected: 四条 PASS。

- [ ] **Step 5: 写 `/channels` 的失败测试**

新建 `src-tauri/src/api/settlement.rs`，先写测试：

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
    async fn channels_list_starts_with_the_three_presets() {
        let (router, _dir, pool) = test_router_with().await;
        seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request("GET", "/api/channels", Some(&token), json!(null)))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        let list: Vec<String> = serde_json::from_value(body).unwrap();
        assert_eq!(list, vec!["现金", "微信", "支付宝"]);
    }

    #[tokio::test]
    async fn channels_list_includes_history_across_events() {
        // 跨展会才有意义：上一场用过「银行转账」，这一场当然还想用。
        let (router, _dir, pool) = test_router_with().await;
        seed_event_and_product(&pool).await;
        sqlx::query(
            "INSERT INTO orders (event_id, status, channel, gross_amount, solved_amount, final_amount)
             VALUES (1, 'completed', '银行转账', 100, 100, 100)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request("GET", "/api/channels", Some(&token), json!(null)))
            .await
            .unwrap();
        let list: Vec<String> = serde_json::from_value(read_json(res).await).unwrap();
        assert!(list.contains(&"银行转账".to_string()));
        assert_eq!(list.iter().filter(|c| *c == "现金").count(), 1, "预置的不能重复出现");
    }
}
```

- [ ] **Step 6: 跑测试确认失败**

```bash
cd src-tauri && tauri-env linux cargo test --workspace settlement::tests
```
Expected: 404（路由还不存在）。

- [ ] **Step 7: 实现 `/channels`**

`src-tauri/src/api/settlement.rs` 主体（`mod tests` 之前）：

```rust
//! 结算侧：渠道列表、垫付、结算调整、结算单、收摊清点、xlsx 导出。
//!
//! **本模块里有三个端点故意不调用 `require_event_open`**：垫付、结算调整、
//! 收摊清点。冻结之后它们仍然允许（spec 偏离 3）——「回家翻出一张打印费收据」
//! 和「回家发现少了一本书」是同一类事件。看见别处都守着就顺手补上去，
//! 会把有意的例外当成漏掉的守卫。改之前先读 spec 3.2。

use axum::{extract::State, routing::get, Json, Router};

use crate::{
    domain::channel::PRESET_CHANNELS,
    error::ApiResult,
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/channels", get(list_channels))
}

/// 已用过的收款渠道，跨展会。
///
/// 挂在 `/api/channels` 而不是 `/api/events/:id/channels`：它按定义就是跨展会的。
/// 这是防「微信」和「微信支付」分裂成两个账户的那一条（②-1/②-2 交接段第 3 条）。
async fn list_channels(
    State(state): State<AppState>,
    _claims: Claims,
) -> ApiResult<Json<Vec<String>>> {
    // 不需要展会守卫：只读，且按定义跨展会
    let used: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT channel FROM orders  WHERE channel IS NOT NULL AND channel <> ''
         UNION
         SELECT DISTINCT channel FROM refunds WHERE channel <> ''
         ORDER BY 1",
    )
    .fetch_all(&state.db)
    .await?;

    let mut out: Vec<String> = PRESET_CHANNELS.iter().map(|s| s.to_string()).collect();
    for c in used {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    Ok(Json(out))
}
```

在 `src-tauri/src/api/mod.rs` 的 `mod` 列表加 `mod settlement;`，并在 `router()` 里 `.merge(settlement::router())`。

- [ ] **Step 8: 跑测试确认通过**

```bash
cd src-tauri && tauri-env linux cargo test --workspace settlement::tests
```
Expected: 两条 PASS。

- [ ] **Step 9: 让完成订单走规范化**

`api/order.rs` 的 `update_order_status` 里，`("pending", "completed")` 分支现在是：

```rust
            let channel = payload
                .channel
                .as_deref()
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(str::to_string)
                .ok_or_else(|| ApiError::BadRequest("完成订单必须提供收款渠道".into()))?;
```

换成：

```rust
            let raw = payload
                .channel
                .as_deref()
                .ok_or_else(|| ApiError::BadRequest("完成订单必须提供收款渠道".into()))?;
            // 渠道名会成为账户名的一部分，规范化必须在写库之前（domain/channel.rs 的模块注释）
            let channel = crate::domain::channel::normalize(raw)?;
```

> 行为变化：原来空渠道报「完成订单必须提供收款渠道」，现在报「收款渠道不能为空」。**现有测试若断言了前一句文案，改测试**——两句都对，但让 `normalize` 独占这条判断，比在两处各写一遍可靠。

- [ ] **Step 10: 补一条「完成订单时渠道被规范化」的测试**

加到 `api/order.rs` 的 `mod tests`：

```rust
    #[tokio::test]
    async fn completing_an_order_normalizes_the_channel() {
        // 账户名是 `实收-<渠道>`，"  微信  " 和 "微信" 必须是同一个账户。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let order_id = place(&router, event_id, json!([{"product_id": ep_a, "quantity": 1}])).await;
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
            .bind(order_id).fetch_one(&pool).await.unwrap();
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
        let order_id = place(&router, event_id, json!([{"product_id": ep_a, "quantity": 1}])).await;
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
```

- [ ] **Step 11: 全量验证并提交**

```bash
cd src-tauri && tauri-env linux cargo fmt
cd src-tauri && tauri-env linux cargo clippy --all-targets -- -D warnings
cd src-tauri && tauri-env linux cargo test --workspace
python3 scripts/check-event-guards.py
```
Expected: 全绿。

```bash
git add src-tauri/src/domain/channel.rs src-tauri/src/domain/mod.rs \
        src-tauri/src/api/settlement.rs src-tauri/src/api/mod.rs src-tauri/src/api/order.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 收款渠道规范化与历史列表，账户名不再可能被污染

渠道名会成为账户名的一部分（实收-<渠道>），写进 money_movements 之后
没有任何办法清理——账户不是一张表，是一堆字符串。所以三件套一次做齐：
写库前规范化（trim + 内部连续空白折叠）、20 个字符的上限（按字符不按
字节，否则中文只能写 6 个字）、跨展会的已用渠道列表。

列表那条才是真正防「微信」和「微信支付」分裂成两个账户的。不加白名单：
有社团用银行转账、有的用闲鱼。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: 赠送与报废

**Files:**
- Create: `src-tauri/src/api/inventory.rs`
- Modify: `src-tauri/src/api/mod.rs`
- Test: `src-tauri/src/api/inventory.rs` 的 `mod tests`

**Interfaces:**
- Produces:
  - `POST /api/events/:event_id/gifts` `{ event_product_id, qty, note?, vendor_pays? }` → 201 `{ journal_id }`
  - `POST /api/events/:event_id/scraps` `{ event_product_id, qty, note? }` → 201 `{ journal_id }`
  - `GET /api/events/:event_id/gifts` / `GET /api/events/:event_id/scraps` → `[{ journal_id, occurred_at, event_product_id, product_code, name, owner_society_id, owner_name, qty, note, vendor_paid }]`
  - `POST /api/events/:event_id/journals/:journal_id/reverse` → 200 `{ journal_id }`（新冲正 journal 的 id）
  - `crate::api::inventory::router() -> Router<AppState>`
- Consumes: Task 1 的 `guard::require_event_open`；`domain::ledger::{post_journal, reverse_journal, onsite_balance, StockLeg, MoneyLeg, Location, Account, JournalKind}`。

- [ ] **Step 1: 写失败的测试——报废走一条货腿，零资金腿**

新建 `src-tauri/src/api/inventory.rs`，先只写 `mod tests`：

```rust
#[cfg(test)]
mod tests {
    use crate::domain::ledger::{account_balance, onsite_balance, Account};
    use crate::domain::money::Money;
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn scrapping_moves_goods_and_touches_no_money() {
        // 报废在记账口径上就是货从现场仓挪进虚拟的损耗仓，钱那一侧一个字不动。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/scraps"),
                Some(&token),
                json!({"event_product_id": ep_a, "qty": 2, "note": "被雨淋了"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 8, "10 − 2");
        let money: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM money_movements mm JOIN journals j ON j.id = mm.journal_id
             WHERE j.event_id = ?",
        )
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(money, 0, "报废不该记任何资金腿");
    }

    #[tokio::test]
    async fn gifting_defaults_to_no_money_leg() {
        // 默认口径：货主自己承担。绝大多数赠品是送自家货，记一笔
        // 「摊主个人买下自家社团的货」只会让单人摊主看不懂结算单。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/gifts"),
                Some(&token),
                json!({"event_product_id": ep_b, "qty": 1}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 4);
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::ZERO,
            "默认不补偿货主"
        );
    }

    #[tokio::test]
    async fn gifting_with_vendor_pays_owes_the_owner() {
        // 方向和一次销售同形，只是把「实收-<渠道>」换成「摊主自有」：
        // 摊主把自己当顾客，用自己的钱买下这件货再送出去。
        // 反过来写就是垫付的方向，账面照样平，只有对着「我应转给」才看得出错。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/gifts"),
                Some(&token),
                json!({"event_product_id": ep_b, "qty": 2, "vendor_pays": true}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::from_cents(-4000),
            "往来 −4000 ⇒ 我应转给黄昏堂 +4000。写成垫付方向的话这里会是 +4000"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::VendorOwn).await.unwrap(),
            Money::from_cents(4000)
        );
    }

    #[tokio::test]
    async fn cannot_give_away_more_than_is_on_site() {
        // 现场仓余额是从流水聚合出来的，没有可以漂移的第二个数字；
        // 但没有这条检查，余额会被扣成负数，之后 settle 永远过不去。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/scraps"),
                Some(&token),
                json!({"event_product_id": ep_b, "qty": 6}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 5, "一件都不该动");
    }

    #[tokio::test]
    async fn successive_scraps_cannot_overdraw_either() {
        // Review Focus #1：两台手机同时报废同一件货。
        //
        // `test_pool()` 是 max_connections(1)，真并发在这个夹具里模拟不出来，
        // 所以这里钉的是「余额检查读的是事务内的最新值」——连着报两次，
        // 第二次必须看见第一次的结果。真正的并发安全靠 BEGIN IMMEDIATE，
        // 它在同一条代码路径上，这条测试一旦被改成事务外检查就会红。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let body = json!({"event_product_id": ep_b, "qty": 3});

        let first = router
            .clone()
            .oneshot(json_request(
                "POST", &format!("/api/events/{event_id}/scraps"), Some(&token), body.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::CREATED);

        let second = router
            .clone()
            .oneshot(json_request(
                "POST", &format!("/api/events/{event_id}/scraps"), Some(&token), body,
            ))
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::CONFLICT, "只剩 2 件了");
        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn reversing_a_gift_puts_everything_back() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/gifts"),
                Some(&token),
                json!({"event_product_id": ep_b, "qty": 2, "vendor_pays": true}),
            ))
            .await
            .unwrap();
        let journal_id = read_json(res).await["journal_id"].as_i64().unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/journals/{journal_id}/reverse"),
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 5);
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::ZERO
        );

        // 列表里不该再出现它（被冲正的和冲正条目都要滤掉）
        let res = router
            .clone()
            .oneshot(json_request(
                "GET", &format!("/api/events/{event_id}/gifts"), Some(&token), json!(null),
            ))
            .await
            .unwrap();
        let list = read_json(res).await;
        assert_eq!(list.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn a_journal_cannot_be_reversed_twice() {
        // DB 层有偏唯一索引兜底，但直接撞上去会是 500。这里要的是可读的 409。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/scraps"), Some(&token),
            json!({"event_product_id": ep_a, "qty": 1}),
        )).await.unwrap();
        let journal_id = read_json(res).await["journal_id"].as_i64().unwrap();

        let uri = format!("/api/events/{event_id}/journals/{journal_id}/reverse");
        let first = router.clone()
            .oneshot(json_request("POST", &uri, Some(&token), json!(null))).await.unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        let second = router.clone()
            .oneshot(json_request("POST", &uri, Some(&token), json!(null))).await.unwrap();
        assert_eq!(second.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn only_gift_and_scrap_journals_can_be_reversed_here() {
        // 销售和收款各有自己的冲正通道（取消订单），不能拿这个端点去冲它们。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        // 夹具的开场带货是一条「进货」journal
        let restock_id: i64 =
            sqlx::query_scalar("SELECT id FROM journals WHERE event_id = ? AND kind = '进货'")
                .bind(event_id).fetch_one(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "POST",
            &format!("/api/events/{event_id}/journals/{restock_id}/reverse"),
            Some(&token),
            json!(null),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn a_settled_event_refuses_gifts_and_scraps() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id).execute(&pool).await.unwrap();
        let token = admin_token();

        for path in ["gifts", "scraps"] {
            let res = router.clone().oneshot(json_request(
                "POST", &format!("/api/events/{event_id}/{path}"), Some(&token),
                json!({"event_product_id": ep_a, "qty": 1}),
            )).await.unwrap();
            assert_eq!(res.status(), StatusCode::CONFLICT, "{path} 应该被冻结挡住");
        }
    }
}
```

- [ ] **Step 2: 跑测试确认失败**

先在 `src-tauri/src/api/mod.rs` 加 `mod inventory;` 和 `.merge(inventory::router())`，然后：

```bash
cd src-tauri && tauri-env linux cargo test --workspace inventory::tests
```
Expected: 编译失败（`router` 未定义）。

- [ ] **Step 3: 实现模块骨架与共用的登记逻辑**

`src-tauri/src/api/inventory.rs` 主体：

```rust
//! 赠送与报废登记。
//!
//! **记账口径上这两件事只是货从现场仓挪进一个虚拟仓**（`赠品` / `损耗`），
//! 钱那一侧默认一个字不动。所以它们不走订单、不走求解器、不走分摊——
//! 母 spec 4.6 写的「订单上一条 0 元行」被 ②-3 的 spec 偏离 1 推翻了，
//! 理由是做成订单行要让求解器和两个分摊函数全部容忍 0 元行，而收益是零：
//! 结算单上「赠送 4 / 报废 1」本来就是从 stock_movements 按位置汇总的。
//!
//! 唯一的例外是「摊主自掏」开关（spec 偏离 2）。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;

use crate::{
    api::guard::{check_read_permission, check_write_permission, require_event_open},
    domain::{
        ledger::{
            onsite_balance, post_journal, reverse_journal, Account, JournalKind, Location,
            MoneyLeg, StockLeg,
        },
        money::Money,
    },
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events/:event_id/gifts", get(list_gifts).post(create_gift))
        .route("/events/:event_id/scraps", get(list_scraps).post(create_scrap))
        .route(
            "/events/:event_id/journals/:journal_id/reverse",
            post(reverse_entry),
        )
}

#[derive(Deserialize)]
pub struct LogRequest {
    event_product_id: i64,
    qty: i64,
    #[serde(default)]
    note: Option<String>,
    /// 只对赠送有意义。报废时忽略——报废的货没有「谁买单」这回事，
    /// 真要赔给货主是结算调整的事（协商结果，系统推不出来）。
    #[serde(default)]
    vendor_pays: bool,
}

#[derive(Serialize)]
pub struct LogResponse {
    journal_id: i64,
}

/// 一条登记记录。`vendor_paid` 由「这条 journal 有没有资金腿」推出来，
/// 不另存一列——存两处就会有一处先腐烂。
#[derive(Serialize, sqlx::FromRow)]
pub struct LogEntry {
    journal_id: i64,
    occurred_at: String,
    event_product_id: i64,
    product_code: String,
    name: String,
    owner_society_id: i64,
    owner_name: String,
    qty: i64,
    note: Option<String>,
    vendor_paid: bool,
}

async fn create_gift(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<LogRequest>,
) -> ApiResult<(StatusCode, Json<LogResponse>)> {
    log_movement(state, claims, event_id, payload, JournalKind::Gift, Location::Gift).await
}

async fn create_scrap(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    mut payload: Json<LogRequest>,
) -> ApiResult<(StatusCode, Json<LogResponse>)> {
    payload.vendor_pays = false; // 报废没有「谁买单」，静默忽略比报错友好
    log_movement(state, claims, event_id, payload.0, JournalKind::Scrap, Location::Loss).await
}

async fn log_movement(
    state: AppState,
    claims: Claims,
    event_id: i64,
    payload: LogRequest,
    kind: JournalKind,
    dest: Location,
) -> ApiResult<(StatusCode, Json<LogResponse>)> {
    check_write_permission(&claims, event_id)?;
    if payload.qty <= 0 {
        return Err(ApiError::BadRequest("数量必须为正".into()));
    }

    // BEGIN IMMEDIATE：余额检查和写入必须在同一个写事务里，否则两台设备
    // 同时登记会各自读到「还有 5 件」然后各扣 3 件，把余额扣成 −1。
    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    require_event_open(&mut tx, event_id).await?;

    let row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT owner_society_id, unit_price FROM event_products WHERE id = ? AND event_id = ?",
    )
    .bind(payload.event_product_id)
    .bind(event_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (owner_society_id, unit_price) =
        row.ok_or_else(|| ApiError::NotFound("商品不在这场展会里".into()))?;

    let available = onsite_balance(&mut *tx, payload.event_product_id).await?;
    if payload.qty > available {
        return Err(ApiError::Conflict(format!(
            "现场仓只剩 {available} 件，登记不了 {} 件",
            payload.qty
        )));
    }

    let stock = vec![StockLeg {
        event_product_id: payload.event_product_id,
        from: Location::OnSite,
        to: dest,
        qty: payload.qty,
    }];

    // 摊主自掏（spec 偏离 2）：方向和一次销售同形，只是把「实收-<渠道>」
    // 换成「摊主自有」——摊主把自己当顾客，用自己的钱买下这件货再送出去。
    //
    // ⚠️ 写成 `摊主自有 → 社团往来` 是**垫付**的方向，意思变成「货主欠我」，
    // 正好错一个符号，而且账面照样平、测不出来，只有对着结算单
    // 「我应转给」那个数才看得出。
    let money = if payload.vendor_pays {
        let amount = Money::from_cents(unit_price)
            .checked_mul_qty(payload.qty)
            .ok_or_else(|| ApiError::BadRequest("金额溢出".into()))?;
        vec![MoneyLeg {
            from: Account::SocietyDue(owner_society_id),
            to: Account::VendorOwn,
            amount,
        }]
    } else {
        Vec::new()
    };

    let journal_id = post_journal(
        &mut tx,
        event_id,
        kind,
        None,
        None,
        payload.note.as_deref(),
        &stock,
        &money,
    )
    .await?;

    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(LogResponse { journal_id })))
}
```

- [ ] **Step 4: 实现两个列表**

```rust
/// 列表 SQL。`赠送` / `报废` 只有这一个字不同，所以共用常量，
/// kind 走 bind 而不是拼串——拼 SQL 是 sqlx 0.9 下还要包 AssertSqlSafe 的麻烦事。
///
/// 两层过滤缺一不可：
///   j.reverses_journal_id IS NULL   滤掉冲正条目本身
///   NOT EXISTS(...)                 滤掉已被冲正的原条目
const LOG_ROWS: &str = r#"
SELECT j.id                AS journal_id,
       j.occurred_at       AS occurred_at,
       sm.event_product_id AS event_product_id,
       ep.product_code     AS product_code,
       ep.name             AS name,
       ep.owner_society_id AS owner_society_id,
       s.name              AS owner_name,
       sm.qty              AS qty,
       j.note              AS note,
       EXISTS(SELECT 1 FROM money_movements mm WHERE mm.journal_id = j.id) AS vendor_paid
FROM journals j
JOIN stock_movements sm ON sm.journal_id = j.id
JOIN event_products ep  ON ep.id = sm.event_product_id
JOIN societies s        ON s.id = ep.owner_society_id
WHERE j.event_id = ?
  AND j.kind = ?
  AND j.reverses_journal_id IS NULL
  AND NOT EXISTS (SELECT 1 FROM journals r WHERE r.reverses_journal_id = j.id)
ORDER BY j.id DESC
"#;

async fn list_gifts(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<Vec<LogEntry>>> {
    list_entries(state, claims, event_id, JournalKind::Gift).await
}

async fn list_scraps(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<Vec<LogEntry>>> {
    list_entries(state, claims, event_id, JournalKind::Scrap).await
}

async fn list_entries(
    state: AppState,
    claims: Claims,
    event_id: i64,
    kind: JournalKind,
) -> ApiResult<Json<Vec<LogEntry>>> {
    check_read_permission(&claims, event_id)?;
    let rows: Vec<LogEntry> = sqlx::query_as(LOG_ROWS)
        .bind(event_id)
        .bind(kind.as_str())
        .fetch_all(&state.db)
        .await?;
    Ok(Json(rows))
}
```

- [ ] **Step 5: 实现撤销**

```rust
/// 撤销一条赠送 / 报废登记。
///
/// **只接受这两种 kind。** 销售和收款有自己的冲正通道（取消订单，
/// `reverse_order_journals`），进货和盘点没有撤销语义（记错了就再记一条反向的）。
/// 不设这道闸，这个端点就成了一个能把任何 journal 冲掉的万能口子。
async fn reverse_entry(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, journal_id)): Path<(i64, i64)>,
) -> ApiResult<Json<LogResponse>> {
    check_write_permission(&claims, event_id)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    require_event_open(&mut tx, event_id).await?;

    let kind: Option<String> =
        sqlx::query_scalar("SELECT kind FROM journals WHERE id = ? AND event_id = ?")
            .bind(journal_id)
            .bind(event_id)
            .fetch_optional(&mut *tx)
            .await?;
    let kind = kind.ok_or_else(|| ApiError::NotFound("记录不存在".into()))?;
    let kind = match kind.as_str() {
        "赠送" => JournalKind::Gift,
        "报废" => JournalKind::Scrap,
        other => {
            return Err(ApiError::BadRequest(format!(
                "只有赠送和报废能在这里撤销，这是一条「{other}」"
            )))
        }
    };

    // DB 层的偏唯一索引 idx_journals_reverses 会拦住重复冲正，但直接撞上去是 500。
    let already: Option<i64> =
        sqlx::query_scalar("SELECT id FROM journals WHERE reverses_journal_id = ?")
            .bind(journal_id)
            .fetch_optional(&mut *tx)
            .await?;
    if already.is_some() {
        return Err(ApiError::Conflict("这条记录已经撤销过了".into()));
    }

    let new_id = reverse_journal(&mut tx, journal_id, kind, Some("撤销登记")).await?;
    tx.commit().await?;
    Ok(Json(LogResponse { journal_id: new_id }))
}
```

- [ ] **Step 6: 跑测试确认通过**

```bash
cd src-tauri && tauri-env linux cargo test --workspace inventory::tests
```
Expected: 九条全 PASS。

- [ ] **Step 7: 门禁与全量验证**

```bash
python3 scripts/check-event-guards.py
cd src-tauri && tauri-env linux cargo fmt
cd src-tauri && tauri-env linux cargo clippy --all-targets -- -D warnings
cd src-tauri && tauri-env linux cargo test --workspace
```
Expected: 全绿。`list_gifts` / `list_scraps` 是 GET，门禁不管；`create_*` 与 `reverse_entry` 都调了 `require_event_open`。

- [ ] **Step 8: 提交**

```bash
git add src-tauri/src/api/inventory.rs src-tauri/src/api/mod.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 赠送与报废登记：货挪进虚拟仓，钱默认不动

记账口径上这两件事只是货从现场仓挪进赠品/损耗仓，所以不走订单、不走
求解器、不走分摊。母 spec 说「订单上一条 0 元行」，做成那样要让求解器
和两个分摊函数全部容忍 0 元行，而收益是零——结算单上「赠送 4 / 报废 1」
本来就是从 stock_movements 按位置汇总的。

唯一的例外是「摊主自掏」开关，方向与一次销售同形（社团往来 → 摊主自有）：
摊主把自己当顾客，用自己的钱买下这件货再送出去。反过来写是垫付的方向，
账面照样平，只有对着结算单「我应转给」才看得出错，所以测试直接钉了符号。

撤销走 reverse_journal，且只认赠送/报废两种 kind——不设闸它就是个能把
任何 journal 冲掉的万能口子。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: 退货

**这是本轮最容易算错钱的一个 task。** 动手前把 ②-2 交接契约的第 2、3、4 条读完。

**Files:**
- Create: `src-tauri/src/api/refund.rs`
- Modify: `src-tauri/src/domain/allocation.rs`（`fn apportion` → `pub fn apportion`）
- Modify: `src-tauri/src/domain/ledger.rs`（新增 `home_society_id`）
- Modify: `src-tauri/src/api/order.rs`（若其中有找本社团的 `unwrap`/内联查询，改调 `home_society_id`）
- Modify: `src-tauri/src/api/mod.rs`
- Test: `src-tauri/src/api/refund.rs` 的 `mod tests`

**Interfaces:**
- Produces:
  - `POST /api/events/:event_id/orders/:order_id/refunds`
    `{ channel, refund_amount?, lines: [{ order_line_id, qty, destination }] }` → 201
    `{ journal_id, refund_amount, allocated_total, paid_total }`
  - `GET /api/events/:event_id/orders/:order_id/refunds` →
    `{ history: [...], lines: [{ order_line_id, event_product_id, product_code, name, lot_name, qty, refunded_qty, remaining_qty, remaining_allocated, remaining_paid }] }`
  - `crate::domain::allocation::apportion(total: i64, weights: &[i64], caps: Option<&[i64]>) -> ApiResult<Vec<i64>>`（由私有改公开）
  - `crate::domain::ledger::home_society_id(conn: &mut SqliteConnection) -> ApiResult<i64>`
- Consumes: Task 1 的 `require_event_open`、Task 2 的 `domain::channel::normalize`。

### 算法（先读这一段，再看代码）

设某一行原始 `qty` / `allocated_amount` / `paid_amount`，已退累计 `done_*`：

```
剩余         left_qty  = qty - done_qty
             left_a    = allocated_amount - done_allocated
             left_p    = paid_amount      - done_paid

这次退的     A = apportion(left_a, [退的件数, left_qty - 退的件数])[0]
             P = apportion(left_p, [退的件数, left_qty - 退的件数])[0]
```

**切「剩余」而不是切「原值」**：分两次各退一半，切原值会让两次各自向下取整，
差额永远回不到账上；切剩余则第二次拿到的必然是「原值减去第一次实际给出去的」。

实退总额 `R_total` 默认 `Σ P`，摊主可改，`0 ≤ R_total ≤ Σ P`。按各行 `P` 为权重、
以各自 `P` 为上限摊回：`apportion(R_total, &P列表, Some(&P列表))`。

三条钱腿（每行各一组），`A` / `P` / `R` 均为该行的值：

```
① 实收-<退款渠道> → 社团往来:<货主>       A            冲销货主的销售
② A ≠ P 时：
   A > P：社团往来:本社团 → 实收-<退款渠道>   A − P      冲销本社团那份手工折让
   A < P：实收-<退款渠道> → 社团往来:本社团   P − A      原单是加价，反向
③ P > R 时：社团往来:<货主> → 实收-<退款渠道>  P − R     顾客没拿回的部分归货主
```

实收净流出 `= A − (A−P) − (P−R) = R`。**覆盖差额（③）归货主不归本社团**——
它本质上是「剩下的货重新按原价算」，那是货主的货。母 spec 4.5 点名这是
「早先版本的一个真漏洞」。

货腿：每行 `顾客仓 → <现场仓|损耗>`，数量 = 退的件数。

- [ ] **Step 1: 把 `apportion` 改成公开**

`src-tauri/src/domain/allocation.rs` 第 35 行：

```rust
fn apportion(total: i64, weights: &[i64], caps: Option<&[i64]>) -> ApiResult<Vec<i64>> {
```
改成

```rust
/// **退货也用它**（②-3）：把一行的剩余金额按件数切成「这次退的」和「还留着的」，
/// 以及把实退总额按各行实付摊回去。取整规则和 Lot 分摊是同一套，
/// 所以「退一半再退一半」和「一次退完」的总额必然一致。
pub fn apportion(total: i64, weights: &[i64], caps: Option<&[i64]>) -> ApiResult<Vec<i64>> {
```

> 这是**新增导出**，不动 `allocate_lot` / `apply_manual_adjustment` / solver 的签名，
> 不违反交接契约「不要改的边界」。

- [ ] **Step 2: 在 `ledger.rs` 加 `home_society_id`**

`src-tauri/src/domain/ledger.rs` 末尾（`#[cfg(test)] mod tests` 之前）：

```rust
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
```

然后 `grep -n "is_home" src-tauri/src/api/*.rs`，把 `api/order.rs` 里内联查本社团的地方改成调用它（如果它用了 `unwrap()` / `expect()`，这一步同时修掉一个 panic）。

- [ ] **Step 3: 写失败的测试——退货的三条腿**

新建 `src-tauri/src/api/refund.rs`，先写 `mod tests`。

```rust
#[cfg(test)]
mod tests {
    use crate::domain::ledger::{account_balance, onsite_balance, Account};
    use crate::domain::money::Money;
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, seed_lot, test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::{json, Value};
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    /// 下单并完成，返回 (order_id, 各 order_line 的 id)。
    async fn place_and_complete(
        router: &axum::Router,
        pool: &SqlitePool,
        event_id: i64,
        items: Value,
        channel: &str,
        final_amount: Option<i64>,
        unapply: Option<Vec<i64>>,
    ) -> (i64, Vec<i64>) {
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
        assert_eq!(res.status(), StatusCode::CREATED, "下单失败");
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        let mut body = json!({ "status": "completed", "channel": channel });
        if let Some(f) = final_amount {
            body["final_amount"] = json!(f);
        }
        if let Some(u) = unapply {
            body["unapply_lot_ids"] = json!(u);
        }
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&admin_token()),
                body,
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "完成订单失败");

        let line_ids: Vec<i64> =
            sqlx::query_scalar("SELECT id FROM order_lines WHERE order_id = ? ORDER BY id")
                .bind(order_id)
                .fetch_all(pool)
                .await
                .unwrap();
        (order_id, line_ids)
    }

    #[tokio::test]
    async fn a_plain_full_refund_undoes_the_sale_exactly() {
        // 没有 Lot、没有手工折让时 A == P == R，②③ 两条腿都不该出现。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router, &pool, event_id,
            json!([{"product_id": ep_b, "quantity": 2}]), "微信", None, None,
        ).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()),
            json!({"channel": "微信",
                   "lines": [{"order_line_id": lines[0], "qty": 2, "destination": "现场仓"}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 5, "货全回来了");
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into())).await.unwrap(),
            Money::ZERO
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::ZERO
        );

        let legs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM money_movements mm JOIN journals j ON j.id = mm.journal_id
             WHERE j.kind = '退货'",
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(legs, 1, "A == P == R 时只该有第 ① 条腿");
    }

    #[tokio::test]
    async fn refunding_less_than_paid_leaves_the_difference_with_the_owner() {
        // 母 spec 4.5 点名的那个「早先版本的真漏洞」：
        // 覆盖差额（顾客没拿回的部分）归**货主**，不归本社团。
        //
        // 场景：黄昏堂的货 2 件 ¥40，微信收；退 1 件但只退 ¥5。
        // A = P = 2000，R = 500 ⇒ ③ 腿 1500 归黄昏堂。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router, &pool, event_id,
            json!([{"product_id": ep_b, "quantity": 2}]), "微信", None, None,
        ).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()),
            json!({"channel": "微信", "refund_amount": 500,
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into())).await.unwrap(),
            Money::from_cents(3500),
            "收 4000 退 500"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::from_cents(-3500),
            "黄昏堂拿 2000（没退的那件）+ 1500（顾客没拿回的）"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1)).await.unwrap(),
            Money::ZERO,
            "本社团完全不该被牵连——覆盖差额归货主"
        );
    }

    #[tokio::test]
    async fn refunding_a_discounted_mixed_owner_order_splits_all_three_legs() {
        // 四个条件同时成立：混货主 + Lot 折让 + 手工折让 + 部分退 + 改 R。
        // 这是整个退货通道存在的理由，也是最容易算错的一处。
        //
        // 购物车：A(本社团 ¥30) ×1 + B(黄昏堂 ¥20) ×2，原价 7000。
        // Lot「任选 2 件 ¥40」会套用一次；再把实收改成 5000（手工折让）。
        // 退 B 的一件，退款额改成 1000。
        //
        // 断言只锁三件事，不锁中间量：
        //   1. 实收账户净额 = 收到的 − 退出去的
        //   2. 两个社团往来之和 + 实收 + 摊主自有 = 0（复式账总闭合）
        //   3. 本社团只承担手工折让那一份，不承担覆盖差额
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        seed_lot(&pool, event_id, "任选2件40", 2, 4000, &[ep_b]).await;

        let (order_id, _lines) = place_and_complete(
            &router, &pool, event_id,
            json!([{"product_id": ep_a, "quantity": 1}, {"product_id": ep_b, "quantity": 2}]),
            "现金", Some(5000), None,
        ).await;

        // 找一条属于 B 的订单行
        let line_b: i64 = sqlx::query_scalar(
            "SELECT id FROM order_lines WHERE order_id = ? AND event_product_id = ? ORDER BY id LIMIT 1",
        ).bind(order_id).bind(ep_b).fetch_one(&pool).await.unwrap();
        let qty_b: i64 = sqlx::query_scalar("SELECT qty FROM order_lines WHERE id = ?")
            .bind(line_b).fetch_one(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()),
            json!({"channel": "现金", "refund_amount": 1000,
                   "lines": [{"order_line_id": line_b, "qty": 1, "destination": "现场仓"}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED, "{:?}", res.status());

        let cash = account_balance(&pool, event_id, &Account::Received("现金".into())).await.unwrap();
        assert_eq!(cash, Money::from_cents(4000), "收 5000 退 1000");

        let home = account_balance(&pool, event_id, &Account::SocietyDue(1)).await.unwrap();
        let other = account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap();
        let vendor = account_balance(&pool, event_id, &Account::VendorOwn).await.unwrap();
        assert_eq!(
            cash + home + other + vendor,
            Money::ZERO,
            "复式账总闭合：所有资金账户余额之和恒为 0"
        );

        // 货：B 回来一件
        let _ = qty_b;
        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 4);
    }

    #[tokio::test]
    async fn refunding_zero_still_moves_the_goods() {
        // Review Focus #2：顾客只退货不要钱。②③ 两条腿一条 0 一条满额，
        // post_journal 会把零金额腿滤掉——货腿必须仍然在，journal 必须成立。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router, &pool, event_id,
            json!([{"product_id": ep_b, "quantity": 1}]), "现金", None, None,
        ).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()),
            json!({"channel": "现金", "refund_amount": 0,
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "损耗"}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 4, "退到损耗，不回现场仓");
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into())).await.unwrap(),
            Money::from_cents(2000),
            "一分没退"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::from_cents(-2000),
            "货主照拿全款——顾客把货送回来了但没要钱"
        );
    }

    #[tokio::test]
    async fn a_zero_price_line_refunds_without_dividing_by_zero() {
        // Review Focus #3：0 元 SKU（母 spec 4.6 的预售取货）。
        // 按 paid 权重摊回时权重全零，apportion 会报「权重之和为零」。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        sqlx::query(
            "INSERT INTO event_products
               (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
             VALUES (3, 1, 1, 1, 'Z', '预售取货券', 0)",
        ).execute(&pool).await.unwrap();
        let mut tx = pool.begin().await.unwrap();
        crate::domain::ledger::post_journal(
            &mut tx, event_id, crate::domain::ledger::JournalKind::Restock, None, None,
            Some("0 元商品带货"),
            &[crate::domain::ledger::StockLeg {
                event_product_id: 3,
                from: crate::domain::ledger::Location::External,
                to: crate::domain::ledger::Location::OnSite,
                qty: 3,
            }],
            &[],
        ).await.unwrap();
        tx.commit().await.unwrap();

        let (order_id, lines) = place_and_complete(
            &router, &pool, event_id,
            json!([{"product_id": 3, "quantity": 1}]), "现金", None, None,
        ).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()),
            json!({"channel": "现金",
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED, "0 元行不该把分摊打爆");
        assert_eq!(onsite_balance(&pool, 3).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn partial_refunds_never_lose_a_cent_to_rounding() {
        // 切「剩余」而不是切「原值」的理由。
        // 3 件、实付 1000（Lot 分摊后是个除不尽的数），分三次各退 1 件，
        // 三次退款之和必须精确等于 1000。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router, &pool, event_id,
            json!([{"product_id": ep_b, "quantity": 3}]), "现金", Some(1000), None,
        ).await;

        let mut total = 0i64;
        for _ in 0..3 {
            let res = router.clone().oneshot(json_request(
                "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
                Some(&admin_token()),
                json!({"channel": "现金",
                       "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
            )).await.unwrap();
            assert_eq!(res.status(), StatusCode::CREATED);
            total += read_json(res).await["refund_amount"].as_i64().unwrap();
        }
        assert_eq!(total, 1000, "三次退款之和必须精确等于顾客实付");
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into())).await.unwrap(),
            Money::ZERO
        );
        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 5);
    }

    #[tokio::test]
    async fn cannot_refund_more_units_than_were_bought() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router, &pool, event_id,
            json!([{"product_id": ep_b, "quantity": 1}]), "现金", None, None,
        ).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()),
            json!({"channel": "现金",
                   "lines": [{"order_line_id": lines[0], "qty": 2, "destination": "现场仓"}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn cannot_refund_more_money_than_the_customer_paid() {
        // 退多于实付是白送钱，母 spec 6.3 明说要走结算调整。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router, &pool, event_id,
            json!([{"product_id": ep_b, "quantity": 1}]), "现金", None, None,
        ).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()),
            json!({"channel": "现金", "refund_amount": 2001,
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn only_completed_orders_can_be_refunded() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders"), None,
            json!({"items": [{"product_id": ep_b, "quantity": 1}]}),
        )).await.unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();
        let line: i64 = sqlx::query_scalar("SELECT id FROM order_lines WHERE order_id = ?")
            .bind(order_id).fetch_one(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()),
            json!({"channel": "现金",
                   "lines": [{"order_line_id": line, "qty": 1, "destination": "现场仓"}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST, "待处理的单请直接取消");
    }

    #[tokio::test]
    async fn refunding_to_a_different_channel_than_the_sale() {
        // 微信收、现金退很常见。实收那条腿必须是**实际退出去的**渠道，
        // 否则收摊清点对不上。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router, &pool, event_id,
            json!([{"product_id": ep_b, "quantity": 1}]), "微信", None, None,
        ).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()),
            json!({"channel": "现金",
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("微信".into())).await.unwrap(),
            Money::from_cents(2000),
            "微信还是收了 2000"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into())).await.unwrap(),
            Money::from_cents(-2000),
            "现金盒少了 2000"
        );
    }

    #[tokio::test]
    async fn the_refundable_list_shows_what_is_left() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router, &pool, event_id,
            json!([{"product_id": ep_b, "quantity": 3}]), "现金", None, None,
        ).await;

        router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()),
            json!({"channel": "现金",
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
        )).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "GET", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()), json!(null),
        )).await.unwrap();
        let body = read_json(res).await;
        assert_eq!(body["history"].as_array().unwrap().len(), 1);
        let line = &body["lines"][0];
        assert_eq!(line["refunded_qty"], 1);
        assert_eq!(line["remaining_qty"], 2);
        assert_eq!(line["remaining_paid"], 4000);
    }

    #[tokio::test]
    async fn a_settled_event_refuses_refunds() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router, &pool, event_id,
            json!([{"product_id": ep_b, "quantity": 1}]), "现金", None, None,
        ).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id).execute(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()),
            json!({"channel": "现金",
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }
}
```

> **下单路径的请求体形状要现场核对。** `place_and_complete` 里的
> `json!({ "items": ... })` 和 `read_json(res).await["id"]` 是按 ②-2 的
> `api/order.rs` 写的——动手前先 `grep -n "struct CreateOrderRequest" -A 10 src-tauri/src/api/order.rs`
> 确认字段名，对不上就改 helper，不要改被测代码去迁就 helper。

- [ ] **Step 4: 跑测试确认失败**

先在 `api/mod.rs` 加 `mod refund;` 与 `.merge(refund::router())`，然后：

```bash
cd src-tauri && tauri-env linux cargo test --workspace refund::tests
```
Expected: 编译失败。

- [ ] **Step 5: 实现模块头与请求/响应类型**

```rust
//! 退货。
//!
//! **两个金额都要用**（②-2 交接契约第 2 条）：退给顾客的默认按 `paid_amount`，
//! 冲销货主按 `allocated_amount`。只用一个数，要么多退给顾客、要么让货主吃了
//! 摊主做的人情——母 spec 4.5 明写这是「早先版本的一个真漏洞」。
//!
//! **按 order_lines 逐行退，不合并显示**（交接契约第 3 条）：同一个商品因为
//! Lot 归属会拆成多行，退哪一行是摊主的决定，不是系统猜的。
//!
//! **不做「退货的同时拆套装」**（spec 5.5）：手工改 R 这条通道已经能表达任何
//! 结果，而拆套装要在退货里重跑一遍「先拆后摊」的顺序，复杂度不成比例。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::guard::{check_read_permission, check_write_permission, require_event_open},
    domain::{
        allocation::apportion,
        channel::normalize,
        ledger::{
            home_society_id, post_journal, Account, JournalKind, Location, MoneyLeg, StockLeg,
        },
        money::Money,
    },
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/events/:event_id/orders/:order_id/refunds",
        post(create_refund).get(list_refunds),
    )
}

#[derive(Deserialize)]
pub struct RefundRequest {
    channel: String,
    /// 实际退给顾客的总额（分）。省略 = 按各行实付全额退。
    #[serde(default)]
    refund_amount: Option<i64>,
    lines: Vec<RefundLineRequest>,
}

#[derive(Deserialize)]
pub struct RefundLineRequest {
    order_line_id: i64,
    qty: i64,
    /// `现场仓`（还能卖）或 `损耗`（已损坏）。
    destination: String,
}

#[derive(Serialize)]
pub struct RefundResponse {
    journal_id: i64,
    refund_amount: i64,
    allocated_total: i64,
    paid_total: i64,
}

/// 一行的中间量。字段全是**这一次**要退的部分，不是原行的值。
struct Part {
    order_line_id: i64,
    event_product_id: i64,
    owner_society_id: i64,
    qty: i64,
    allocated: i64,
    paid: i64,
    destination: Location,
}
```

- [ ] **Step 6: 实现 `create_refund`**

```rust
async fn create_refund(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, order_id)): Path<(i64, i64)>,
    Json(payload): Json<RefundRequest>,
) -> ApiResult<(StatusCode, Json<RefundResponse>)> {
    check_write_permission(&claims, event_id)?;
    let channel = normalize(&payload.channel)?;
    if payload.lines.is_empty() {
        return Err(ApiError::BadRequest("至少要退一行".into()));
    }

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    require_event_open(&mut tx, event_id).await?;

    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM orders WHERE id = ? AND event_id = ?")
            .bind(order_id)
            .bind(event_id)
            .fetch_optional(&mut *tx)
            .await?;
    match status.as_deref() {
        None => return Err(ApiError::NotFound("订单不存在".into())),
        Some("completed") => {}
        Some("pending") => {
            return Err(ApiError::BadRequest(
                "待处理的订单请直接取消，不要走退货——货还没交出去，钱也没收".into(),
            ))
        }
        Some(_) => return Err(ApiError::BadRequest("已取消的订单没有可退的东西".into())),
    }

    let home = home_society_id(&mut tx).await?;

    // ---- 逐行算出这次退的 A 和 P ----
    let mut parts: Vec<Part> = Vec::with_capacity(payload.lines.len());
    for l in &payload.lines {
        if l.qty <= 0 {
            return Err(ApiError::BadRequest("退货数量必须为正".into()));
        }
        let destination = match l.destination.as_str() {
            "现场仓" => Location::OnSite,
            "损耗" => Location::Loss,
            other => {
                return Err(ApiError::BadRequest(format!(
                    "退货去向只能是「现场仓」或「损耗」，收到「{other}」"
                )))
            }
        };

        let row: Option<(i64, i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT ol.event_product_id, ep.owner_society_id, ol.qty,
                    ol.allocated_amount, ol.paid_amount
             FROM order_lines ol
             JOIN event_products ep ON ep.id = ol.event_product_id
             WHERE ol.id = ? AND ol.order_id = ?",
        )
        .bind(l.order_line_id)
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (event_product_id, owner_society_id, line_qty, line_alloc, line_paid) =
            row.ok_or_else(|| ApiError::BadRequest("订单行不属于这张订单".into()))?;

        let (done_qty, done_alloc, done_paid): (i64, i64, i64) = sqlx::query_as(
            "SELECT COALESCE(SUM(qty), 0), COALESCE(SUM(allocated_amount), 0),
                    COALESCE(SUM(paid_amount), 0)
             FROM refunds WHERE order_line_id = ?",
        )
        .bind(l.order_line_id)
        .fetch_one(&mut *tx)
        .await?;

        let left_qty = line_qty - done_qty;
        if l.qty > left_qty {
            return Err(ApiError::Conflict(format!("这一行只剩 {left_qty} 件可退")));
        }

        // 切「剩余」而不是切「原值」：分两次各退一半，切原值会让两次各自向下
        // 取整，差额永远回不到账上；切剩余则第二次必然拿到「原值减去第一次
        // 实际给出去的」。weights 的第二项可能是 0（退光），apportion 允许。
        let weights = [l.qty, left_qty - l.qty];
        let allocated = apportion(line_alloc - done_alloc, &weights, None)?[0];
        let paid = apportion(line_paid - done_paid, &weights, None)?[0];

        parts.push(Part {
            order_line_id: l.order_line_id,
            event_product_id,
            owner_society_id,
            qty: l.qty,
            allocated,
            paid,
            destination,
        });
    }

    // ---- 实退总额，以及摊回每行 ----
    let paid_total: i64 = parts.iter().map(|p| p.paid).sum();
    let allocated_total: i64 = parts.iter().map(|p| p.allocated).sum();
    let refund_total = payload.refund_amount.unwrap_or(paid_total);
    if refund_total < 0 {
        return Err(ApiError::BadRequest("退款金额不能为负".into()));
    }
    if refund_total > paid_total {
        return Err(ApiError::BadRequest(format!(
            "退款不能多于顾客为这些货实付的 {}——白送钱请走结算调整",
            Money::from_cents(paid_total)
        )));
    }

    let weights: Vec<i64> = parts.iter().map(|p| p.paid).collect();
    // Review Focus #3：0 元行（预售取货 SKU）权重全零，apportion 会报
    // 「无法分摊：权重之和为零」。这种单的 refund_total 必然是 0
    //（上面的上限检查已经保证），直接给一组 0，不为退化情形改纯函数。
    let refunds: Vec<i64> = if paid_total == 0 {
        vec![0; parts.len()]
    } else {
        apportion(refund_total, &weights, Some(&weights))?
    };

    // ---- 腿 ----
    let mut stock = Vec::with_capacity(parts.len());
    let mut money = Vec::new();
    for (p, &r) in parts.iter().zip(refunds.iter()) {
        stock.push(StockLeg {
            event_product_id: p.event_product_id,
            from: Location::Customer,
            to: p.destination,
            qty: p.qty,
        });

        // ① 冲销货主的销售
        money.push(MoneyLeg {
            from: Account::Received(channel.clone()),
            to: Account::SocietyDue(p.owner_society_id),
            amount: Money::from_cents(p.allocated),
        });

        // ② 冲销本社团那份手工折让。加价时方向反过来（母 spec 4.3 允许加价），
        //    金额恒取绝对值——post_journal 对非正金额直接 400。
        if p.allocated != p.paid {
            let (from, to) = if p.allocated > p.paid {
                (Account::SocietyDue(home), Account::Received(channel.clone()))
            } else {
                (Account::Received(channel.clone()), Account::SocietyDue(home))
            };
            money.push(MoneyLeg {
                from,
                to,
                amount: Money::from_cents((p.allocated - p.paid).abs()),
            });
        }

        // ③ 顾客没拿回的部分归**货主**（不是本社团）——它本质上是
        //    「剩下的货重新按原价算」，那是货主的货。母 spec 4.5 点名的漏洞。
        if p.paid > r {
            money.push(MoneyLeg {
                from: Account::SocietyDue(p.owner_society_id),
                to: Account::Received(channel.clone()),
                amount: Money::from_cents(p.paid - r),
            });
        }
    }

    // stock 恒非空（lines 非空且 qty > 0），所以不会撞 post_journal 的空 journal 检查，
    // 哪怕三条钱腿全是 0（顾客只退货不要钱）。
    let journal_id = post_journal(
        &mut tx,
        event_id,
        JournalKind::Refund,
        Some(order_id),
        None,
        Some("退货"),
        &stock,
        &money,
    )
    .await?;

    for (p, &r) in parts.iter().zip(refunds.iter()) {
        sqlx::query(
            "INSERT INTO refunds
               (order_line_id, journal_id, qty, allocated_amount, paid_amount,
                refund_amount, channel, destination)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(p.order_line_id)
        .bind(journal_id)
        .bind(p.qty)
        .bind(p.allocated)
        .bind(p.paid)
        .bind(r)
        .bind(&channel)
        .bind(p.destination.as_str())
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(RefundResponse {
            journal_id,
            refund_amount: refund_total,
            allocated_total,
            paid_total,
        }),
    ))
}
```

- [ ] **Step 7: 实现 `list_refunds`**

```rust
#[derive(Serialize, sqlx::FromRow)]
pub struct RefundHistoryRow {
    id: i64,
    order_line_id: i64,
    product_code: String,
    name: String,
    qty: i64,
    refund_amount: i64,
    channel: String,
    destination: String,
    occurred_at: String,
}

/// 可退的行。**同一个商品可能出现多行**（按 Lot 归属拆的），
/// 所以前端不要按 `event_product_id` 去重（②-2 交接契约第 3 条）。
#[derive(Serialize, sqlx::FromRow)]
pub struct RefundableLine {
    order_line_id: i64,
    event_product_id: i64,
    product_code: String,
    name: String,
    lot_name: Option<String>,
    qty: i64,
    refunded_qty: i64,
    remaining_qty: i64,
    remaining_allocated: i64,
    remaining_paid: i64,
}

#[derive(Serialize)]
pub struct RefundListResponse {
    history: Vec<RefundHistoryRow>,
    lines: Vec<RefundableLine>,
}

async fn list_refunds(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, order_id)): Path<(i64, i64)>,
) -> ApiResult<Json<RefundListResponse>> {
    check_read_permission(&claims, event_id)?;

    let history: Vec<RefundHistoryRow> = sqlx::query_as(
        "SELECT r.id, r.order_line_id, ep.product_code, ep.name, r.qty,
                r.refund_amount, r.channel, r.destination, j.occurred_at
         FROM refunds r
         JOIN order_lines ol    ON ol.id = r.order_line_id
         JOIN event_products ep ON ep.id = ol.event_product_id
         JOIN journals j        ON j.id = r.journal_id
         WHERE ol.order_id = ?
         ORDER BY r.id DESC",
    )
    .bind(order_id)
    .fetch_all(&state.db)
    .await?;

    let lines: Vec<RefundableLine> = sqlx::query_as(
        "SELECT ol.id                              AS order_line_id,
                ol.event_product_id                AS event_product_id,
                ep.product_code                    AS product_code,
                ep.name                            AS name,
                olot.name                          AS lot_name,
                ol.qty                             AS qty,
                COALESCE(r.done_qty, 0)            AS refunded_qty,
                ol.qty - COALESCE(r.done_qty, 0)   AS remaining_qty,
                ol.allocated_amount - COALESCE(r.done_alloc, 0) AS remaining_allocated,
                ol.paid_amount      - COALESCE(r.done_paid, 0)  AS remaining_paid
         FROM order_lines ol
         JOIN event_products ep ON ep.id = ol.event_product_id
         LEFT JOIN order_lots olot ON olot.id = ol.order_lot_id
         LEFT JOIN (
            SELECT order_line_id,
                   SUM(qty) AS done_qty,
                   SUM(allocated_amount) AS done_alloc,
                   SUM(paid_amount) AS done_paid
            FROM refunds GROUP BY order_line_id
         ) r ON r.order_line_id = ol.id
         WHERE ol.order_id = ?
         ORDER BY ol.id",
    )
    .bind(order_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(RefundListResponse { history, lines }))
}
```

- [ ] **Step 8: 跑测试确认通过**

```bash
cd src-tauri && tauri-env linux cargo test --workspace refund::tests
```
Expected: 十三条全 PASS。红了先看是不是 `place_and_complete` 的请求体形状和 `api/order.rs` 对不上（Step 3 的提示）。

- [ ] **Step 9: 补 Review Focus #5 的测试（没有本社团）**

加到 `mod tests`：

```rust
    #[tokio::test]
    async fn refunding_without_a_home_society_fails_readably() {
        // Review Focus #5：用户把本社团删了，或者库是手工改过的。
        // 没有这条检查，home_society_id 那里会 panic 成 500「服务器错误」，
        // 摊主看到之后无从下手。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let (order_id, lines) = place_and_complete(
            &router, &pool, event_id,
            json!([{"product_id": ep_b, "quantity": 1}]), "现金", None, None,
        ).await;
        sqlx::query("UPDATE societies SET is_home = 0 WHERE is_home = 1")
            .execute(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"),
            Some(&admin_token()),
            json!({"channel": "现金",
                   "lines": [{"order_line_id": lines[0], "qty": 1, "destination": "现场仓"}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        let body = read_json(res).await;
        assert!(
            body["error"].as_str().unwrap().contains("本社团"),
            "错误信息要说清楚缺的是什么：{body}"
        );
    }
```

- [ ] **Step 10: 全量验证并提交**

```bash
python3 scripts/check-event-guards.py
cd src-tauri && tauri-env linux cargo fmt
cd src-tauri && tauri-env linux cargo clippy --all-targets -- -D warnings
cd src-tauri && tauri-env linux cargo test --workspace
```
Expected: 全绿。

```bash
git add src-tauri/src/api/refund.rs src-tauri/src/api/mod.rs \
        src-tauri/src/domain/allocation.rs src-tauri/src/domain/ledger.rs src-tauri/src/api/order.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 退货：两个金额、三条腿，覆盖差额归货主

退给顾客按 paid、冲销货主按 allocated。只用一个数，要么多退给顾客、
要么让货主吃了摊主做的人情——母 spec 点名这是早先版本的一个真漏洞。

第三条腿（顾客没拿回的那部分）归**货主**不归本社团：它本质上是「剩下的
货重新按原价算」，那是货主的货。混货主 + Lot 折让 + 手工折让 + 部分退 +
改退款额，五个条件同时成立的那条测试就是为它写的。

分多次部分退时切的是「剩余」而不是「原值」——切原值会让每次各自向下取整，
差额永远回不到账上。三次各退一件、总额必须精确等于实付，有测试钉住。

apportion 由私有改公开（新增导出，不动 allocate_lot / apply_manual_adjustment
/ solver 的签名）；顺手把找本社团那段收成 ledger::home_society_id，
缺本社团时给一句能照着做的话而不是 500。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: 收摊向导

**Files:**
- Create: `src-tauri/migrations/202609240002_add_closing_timestamps.sql`
- Create: `src-tauri/src/api/closing.rs`
- Modify: `src-tauri/src/api/mod.rs`
- Test: `src-tauri/src/api/closing.rs` 的 `mod tests`

**Interfaces:**
- Produces:
  - `GET /api/events/:event_id/closing` → `{ status, pending_orders, onsite_remaining, stocktaken_at, blockers }`
  - `POST /api/events/:event_id/closing/stocktake` `{ counts: [{event_product_id, counted_qty}] }` → 200 `{ journal_id: Option<i64>, diffs: [...] }`
  - `POST /api/events/:event_id/closing/takeback` → 200 `{ journal_id: Option<i64>, moved: i64 }`
  - `POST /api/events/:event_id/closing/settle` → 200 `{ status: "已结算" }`
  - `crate::api::closing::router() -> Router<AppState>`
- Consumes: Task 1 的 `require_event_open`；`domain::ledger::onsite_balances`。

- [ ] **Step 1: 写迁移**

`src-tauri/migrations/202609240002_add_closing_timestamps.sql`：

```sql
-- 收摊闭环：记下「盘过了」和「钱清点过了」这两个事实。
--
-- 为什么不能靠反查 journal 的存在性：**零差异的清点写不出 journal**。
-- post_journal 拒绝空腿（②-1 的原话：记空 journal 会造出「造得出却冲不掉」
-- 的幽灵 journal），而零金额的腿本来就会被静默滤掉。于是
--   「盘了一遍，全对」 vs 「根本没盘」
--   「数了现金盒，分文不差」 vs 「还没数」
-- 两组都会长得一模一样——而结算单上「未盘点，剩余数为账面推算」和
-- 「微信 1,540 ✓」正好要分开它们。
--
-- 两列都顺带供结算单打印时间（「盘点于 10-01 18:23」）。
ALTER TABLE events ADD COLUMN stocktaken_at DATETIME;
ALTER TABLE events ADD COLUMN reconciled_at DATETIME;
```

> `reconciled_at` 本轮由 Task 8 的收摊清点写入，Task 5 只负责把列建出来。
> **一条迁移加两列**，不要拆成两个文件——迁移文件一旦发布就永久冻结，
> 能合并的时候合并。

- [ ] **Step 2: 跑一次测试确认迁移能应用**

```bash
cd src-tauri && tauri-env linux cargo test --workspace domain::ledger
```
Expected: 全绿（`test_pool()` 会跑迁移，列加错了这里就编译/运行失败）。

> ⚠️ **本机的 dev 库可能会撞 `VersionMismatch`**。那是因为 dev 库里已经有一条同名
> 不同内容的迁移记录，和本轮无关。处理办法在项目 CLAUDE.md：
> `mv ~/.local/share/com.abl.BoothKernel-dev/sale_system.db{,.stale-$(date +%Y%m%d%H%M%S)}`
> ——**改名留存，不要删**。测试用的是内存库，不受影响。

- [ ] **Step 3: 写失败的测试**

新建 `src-tauri/src/api/closing.rs`，先写 `mod tests`：

```rust
#[cfg(test)]
mod tests {
    use crate::domain::ledger::onsite_balance;
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::json;
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    async fn closing(router: &axum::Router, event_id: i64) -> serde_json::Value {
        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/closing"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        read_json(res).await
    }

    async fn place_pending(router: &axum::Router, event_id: i64, ep: i64) -> i64 {
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep, "quantity": 1}]}),
            ))
            .await
            .unwrap();
        read_json(res).await["id"].as_i64().unwrap()
    }

    async fn settle(router: &axum::Router, event_id: i64) -> axum::http::StatusCode {
        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/settle"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap()
            .status()
    }

    async fn takeback(router: &axum::Router, event_id: i64) -> axum::http::StatusCode {
        router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/closing/takeback"),
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap()
            .status()
    }

    #[tokio::test]
    async fn pending_orders_block_everything_downstream() {
        // 硬性阻断（母 spec 6.4）：pending 订单的货已经在顾客仓、钱一分没收。
        // 留着它们，「顾客仓余额 = 卖出」这条不变量就不成立。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        place_pending(&router, event_id, ep_a).await;

        let body = closing(&router, event_id).await;
        assert_eq!(body["pending_orders"].as_array().unwrap().len(), 1);
        assert!(!body["blockers"].as_array().unwrap().is_empty());

        assert_eq!(takeback(&router, event_id).await, StatusCode::CONFLICT);
        assert_eq!(settle(&router, event_id).await, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn a_stocktake_with_no_difference_writes_no_journal_but_still_counts() {
        // 这条是那条新迁移存在的全部理由。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/closing/stocktake"), Some(&admin_token()),
            json!({"counts": [{"event_product_id": ep_a, "counted_qty": 10},
                              {"event_product_id": ep_b, "counted_qty": 5}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(read_json(res).await["journal_id"].is_null(), "没差异就没有 journal");

        let journals: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM journals WHERE event_id = ? AND kind = '盘点'",
        ).bind(event_id).fetch_one(&pool).await.unwrap();
        assert_eq!(journals, 0);

        let body = closing(&router, event_id).await;
        assert!(!body["stocktaken_at"].is_null(), "「盘了，全对」必须留得下痕迹");
    }

    #[tokio::test]
    async fn a_stocktake_writes_both_directions() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/closing/stocktake"), Some(&admin_token()),
            json!({"counts": [{"event_product_id": ep_a, "counted_qty": 8},   // 盘亏 2
                              {"event_product_id": ep_b, "counted_qty": 7}]}), // 盘盈 2
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 8);
        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 7);

        let (to_var, from_var): (i64, i64) = sqlx::query_as(
            "SELECT COALESCE(SUM(CASE WHEN to_location = '差异' THEN qty ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN from_location = '差异' THEN qty ELSE 0 END), 0)
             FROM stock_movements sm JOIN journals j ON j.id = sm.journal_id
             WHERE j.kind = '盘点'",
        ).fetch_one(&pool).await.unwrap();
        assert_eq!((to_var, from_var), (2, 2), "盘亏进差异、盘盈出差异");

        // 钱一分没动——赔付是协商结果，走结算调整，系统推不出来
        let money: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM money_movements mm JOIN journals j ON j.id = mm.journal_id
             WHERE j.kind = '盘点'",
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(money, 0);
    }

    #[tokio::test]
    async fn a_stocktake_must_cover_every_product_still_on_site() {
        // 「我数了，一致」和「我没数这个」是两件事，请求体不该把它们混成同一个缺省。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/closing/stocktake"), Some(&admin_token()),
            json!({"counts": [{"event_product_id": ep_a, "counted_qty": 10}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = read_json(res).await;
        assert!(body["error"].as_str().unwrap().contains("本子B"), "要说清楚漏了哪个：{body}");
    }

    #[tokio::test]
    async fn takeback_empties_the_on_site_location() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;

        assert_eq!(takeback(&router, event_id).await, StatusCode::OK);
        assert_eq!(onsite_balance(&pool, ep_a).await.unwrap(), 0);
        assert_eq!(onsite_balance(&pool, ep_b).await.unwrap(), 0);
        assert_eq!(settle(&router, event_id).await, StatusCode::OK);

        let status: String = sqlx::query_scalar("SELECT status FROM events WHERE id = ?")
            .bind(event_id).fetch_one(&pool).await.unwrap();
        assert_eq!(status, "已结算");
    }

    #[tokio::test]
    async fn takeback_on_an_empty_booth_is_a_no_op_not_an_error() {
        // 全卖光了也要能收摊。post_journal 拒绝空 journal，所以这里必须
        // 判空跳过，不能硬调。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        assert_eq!(takeback(&router, event_id).await, StatusCode::OK);

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/closing/takeback"),
            Some(&admin_token()), json!(null),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        assert_eq!(body["moved"], 0);
        assert!(body["journal_id"].is_null());
    }

    #[tokio::test]
    async fn settling_is_refused_while_goods_are_still_on_site() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        assert_eq!(settle(&router, event_id).await, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn settling_is_allowed_without_a_stocktake() {
        // 可跳过，不硬性阻断——总有意外（展会清场赶人）。
        // 但结算单要能看出来没盘过，所以 stocktaken_at 保持 null。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        assert_eq!(takeback(&router, event_id).await, StatusCode::OK);
        assert_eq!(settle(&router, event_id).await, StatusCode::OK);

        let at: Option<String> = sqlx::query_scalar("SELECT stocktaken_at FROM events WHERE id = ?")
            .bind(event_id).fetch_one(&pool).await.unwrap();
        assert!(at.is_none());
    }

    #[tokio::test]
    async fn a_settled_event_refuses_the_whole_wizard() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id).execute(&pool).await.unwrap();

        assert_eq!(takeback(&router, event_id).await, StatusCode::CONFLICT);
        assert_eq!(settle(&router, event_id).await, StatusCode::CONFLICT);
        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/closing/stocktake"), Some(&admin_token()),
            json!({"counts": [{"event_product_id": ep_a, "counted_qty": 0}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn the_wizard_survives_being_interrupted() {
        // 四步之间没有会话状态，每一步的成果都是已落库的 journal。
        // 盘完退出、重新进来，看到的是真实状态而不是从头开始。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;

        router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/closing/stocktake"), Some(&admin_token()),
            json!({"counts": [{"event_product_id": ep_a, "counted_qty": 9},
                              {"event_product_id": ep_b, "counted_qty": 5}]}),
        )).await.unwrap();

        let body = closing(&router, event_id).await;
        assert!(!body["stocktaken_at"].is_null());
        let remaining: i64 = body["onsite_remaining"].as_array().unwrap()
            .iter().map(|r| r["qty"].as_i64().unwrap()).sum();
        assert_eq!(remaining, 14, "9 + 5，盘点的结果留下来了");
        let _ = pool;
    }
}
```

- [ ] **Step 4: 跑测试确认失败**

先在 `api/mod.rs` 加 `mod closing;` 与 `.merge(closing::router())`：

```bash
cd src-tauri && tauri-env linux cargo test --workspace closing::tests
```
Expected: 编译失败。

- [ ] **Step 5: 实现只读的 `GET /closing`**

```rust
//! 收摊向导：清 pending → 盘点 → 带回 → 转已结算（母 spec 6.4）。
//!
//! **四步之间没有会话状态。** 每一步的成果都是已落库的 journal，向导只是按
//! `GET /closing` 的返回决定给你看哪一屏。退出、换设备、重进，都从当前真实
//! 状态继续。
//!
//! **能不能进下一步由后端说了算**（`blockers`），前端不自己判断。理由和 ②-2
//! 把试算放后端一样：判据写在两处就会有一处先腐烂。

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::guard::{check_read_permission, check_write_permission, require_event_open},
    domain::ledger::{onsite_balances, post_journal, JournalKind, Location, StockLeg},
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events/:event_id/closing", get(get_closing))
        .route("/events/:event_id/closing/stocktake", post(stocktake))
        .route("/events/:event_id/closing/takeback", post(takeback))
        .route("/events/:event_id/closing/settle", post(settle))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct PendingOrderRow {
    id: i64,
    created_at: String,
    final_amount: i64,
    item_count: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct OnSiteRow {
    event_product_id: i64,
    product_code: String,
    name: String,
    owner_society_id: i64,
    owner_name: String,
    qty: i64,
}

#[derive(Serialize)]
pub struct ClosingState {
    status: String,
    pending_orders: Vec<PendingOrderRow>,
    onsite_remaining: Vec<OnSiteRow>,
    stocktaken_at: Option<String>,
    blockers: Vec<String>,
}

async fn get_closing(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<ClosingState>> {
    check_read_permission(&claims, event_id)?;

    let row: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT status, stocktaken_at FROM events WHERE id = ?")
            .bind(event_id)
            .fetch_optional(&state.db)
            .await?;
    let (status, stocktaken_at) = row.ok_or_else(|| ApiError::NotFound("展会不存在".into()))?;

    let pending_orders: Vec<PendingOrderRow> = sqlx::query_as(
        "SELECT o.id, o.created_at, o.final_amount,
                (SELECT COALESCE(SUM(qty), 0) FROM order_lines WHERE order_id = o.id) AS item_count
         FROM orders o
         WHERE o.event_id = ? AND o.status = 'pending'
         ORDER BY o.id",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    let onsite_remaining = onsite_rows(&state, event_id).await?;

    let mut blockers = Vec::new();
    if status == "已结算" {
        blockers.push("展会已结算，账本已冻结".into());
    }
    if !pending_orders.is_empty() {
        blockers.push(format!(
            "还有 {} 单待处理，逐单完成或取消之后才能盘点",
            pending_orders.len()
        ));
    }
    if !onsite_remaining.is_empty() {
        blockers.push(format!(
            "还有 {} 种商品在现场仓，带回之后才能结算",
            onsite_remaining.len()
        ));
    }

    Ok(Json(ClosingState {
        status,
        pending_orders,
        onsite_remaining,
        stocktaken_at,
        blockers,
    }))
}

/// 现场仓余额非 0 的商品。**从流水聚合，没有第二个可以漂移的数字。**
async fn onsite_rows(state: &AppState, event_id: i64) -> ApiResult<Vec<OnSiteRow>> {
    let rows: Vec<OnSiteRow> = sqlx::query_as(
        "SELECT ep.id AS event_product_id, ep.product_code, ep.name,
                ep.owner_society_id, s.name AS owner_name,
                COALESCE(SUM(CASE WHEN sm.to_location   = '现场仓' THEN sm.qty ELSE 0 END), 0)
              - COALESCE(SUM(CASE WHEN sm.from_location = '现场仓' THEN sm.qty ELSE 0 END), 0)
                AS qty
         FROM event_products ep
         JOIN societies s ON s.id = ep.owner_society_id
         LEFT JOIN stock_movements sm ON sm.event_product_id = ep.id
         WHERE ep.event_id = ?
         GROUP BY ep.id
         HAVING qty <> 0
         ORDER BY ep.id",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;
    Ok(rows)
}
```

- [ ] **Step 6: 实现盘点**

```rust
#[derive(Deserialize)]
pub struct StocktakeRequest {
    counts: Vec<CountRow>,
}

#[derive(Deserialize)]
pub struct CountRow {
    event_product_id: i64,
    counted_qty: i64,
}

#[derive(Serialize)]
pub struct StocktakeResponse {
    /// 零差异时为 `None`——`post_journal` 拒绝空 journal，而
    /// 「盘了全对」这个事实靠 `events.stocktaken_at` 记，不靠 journal 存在性。
    journal_id: Option<i64>,
    diffs: Vec<DiffRow>,
}

#[derive(Serialize)]
pub struct DiffRow {
    event_product_id: i64,
    name: String,
    book_qty: i64,
    counted_qty: i64,
}

async fn stocktake(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<StocktakeRequest>,
) -> ApiResult<Json<StocktakeResponse>> {
    check_write_permission(&claims, event_id)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    require_event_open(&mut tx, event_id).await?;

    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE event_id = ? AND status = 'pending'")
            .bind(event_id)
            .fetch_one(&mut *tx)
            .await?;
    if pending > 0 {
        return Err(ApiError::Conflict(format!(
            "还有 {pending} 单待处理，清完才能盘点"
        )));
    }

    let book = onsite_balances(&state.db, event_id).await?;

    // 收全量：现场仓余额非 0 的商品必须全部出现在请求里。
    // 「我数了，一致」和「我没数这个」是两件事。
    let mut missing: Vec<String> = Vec::new();
    for (ep_id, qty) in book.iter() {
        if *qty != 0 && !payload.counts.iter().any(|c| c.event_product_id == *ep_id) {
            let name: String = sqlx::query_scalar("SELECT name FROM event_products WHERE id = ?")
                .bind(ep_id)
                .fetch_one(&mut *tx)
                .await?;
            missing.push(name);
        }
    }
    if !missing.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "这些商品还在现场仓但没报数：{}。数过一致的也要报，否则分不出「数了一致」和「没数」",
            missing.join("、")
        )));
    }

    let mut stock = Vec::new();
    let mut diffs = Vec::new();
    for c in &payload.counts {
        if c.counted_qty < 0 {
            return Err(ApiError::BadRequest("实数不能为负".into()));
        }
        let name: Option<String> =
            sqlx::query_scalar("SELECT name FROM event_products WHERE id = ? AND event_id = ?")
                .bind(c.event_product_id)
                .bind(event_id)
                .fetch_optional(&mut *tx)
                .await?;
        let name = name.ok_or_else(|| ApiError::BadRequest("商品不在这场展会里".into()))?;

        let book_qty = *book.get(&c.event_product_id).unwrap_or(&0);
        if book_qty == c.counted_qty {
            continue;
        }
        // 盘亏：现场仓 → 差异；盘盈：差异 → 现场仓。只动货，不动钱。
        let (from, to, qty) = if c.counted_qty < book_qty {
            (Location::OnSite, Location::Variance, book_qty - c.counted_qty)
        } else {
            (Location::Variance, Location::OnSite, c.counted_qty - book_qty)
        };
        stock.push(StockLeg {
            event_product_id: c.event_product_id,
            from,
            to,
            qty,
        });
        diffs.push(DiffRow {
            event_product_id: c.event_product_id,
            name,
            book_qty,
            counted_qty: c.counted_qty,
        });
    }

    let journal_id = if stock.is_empty() {
        None
    } else {
        Some(
            post_journal(
                &mut tx,
                event_id,
                JournalKind::Stocktake,
                None,
                None,
                Some("收摊盘点"),
                &stock,
                &[],
            )
            .await?,
        )
    };

    // 有没有差异都要落时间戳——这正是那条迁移存在的理由。
    sqlx::query("UPDATE events SET stocktaken_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(event_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(StocktakeResponse { journal_id, diffs }))
}
```

> ⚠️ 上面用的 `book` **必须在事务内读**。`ledger::onsite_balances` 现有签名收
> `&SqlitePool`，事务外读一遍再进事务写，中间隔着一个能被别的请求插进来的窗口。
> 不动 `ledger.rs` 的既有签名，在本模块加一个走事务的版本：
>
> ```rust
> /// 本展会每个商品的现场仓账面数。**走事务**——盘点和带回都要「读到的数就是
> /// 待会儿要写的那个数」，事务外读会留下一个可以被下单插进来的窗口。
> async fn book_balances(
>     conn: &mut sqlx::SqliteConnection,
>     event_id: i64,
> ) -> ApiResult<std::collections::HashMap<i64, i64>> {
>     let rows: Vec<(i64, i64)> = sqlx::query_as(
>         "SELECT ep.id,
>                 COALESCE(SUM(CASE WHEN sm.to_location   = '现场仓' THEN sm.qty ELSE 0 END), 0)
>               - COALESCE(SUM(CASE WHEN sm.from_location = '现场仓' THEN sm.qty ELSE 0 END), 0)
>          FROM event_products ep
>          LEFT JOIN stock_movements sm ON sm.event_product_id = ep.id
>          WHERE ep.event_id = ?
>          GROUP BY ep.id",
>     )
>     .bind(event_id)
>     .fetch_all(&mut *conn)
>     .await?;
>     Ok(rows.into_iter().collect())
> }
> ```
>
> Step 5 的 `let book = onsite_balances(&state.db, event_id).await?;` 相应改成
> `let book = book_balances(&mut tx, event_id).await?;`，import 里去掉 `onsite_balances`。

- [ ] **Step 7: 实现带回与转已结算**

```rust
#[derive(Serialize)]
pub struct TakebackResponse {
    journal_id: Option<i64>,
    moved: i64,
}

async fn takeback(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<TakebackResponse>> {
    check_write_permission(&claims, event_id)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    require_event_open(&mut tx, event_id).await?;

    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE event_id = ? AND status = 'pending'")
            .bind(event_id)
            .fetch_one(&mut *tx)
            .await?;
    if pending > 0 {
        return Err(ApiError::Conflict(format!(
            "还有 {pending} 单待处理，清完才能带回"
        )));
    }

    let book = book_balances(&mut tx, event_id).await?;
    let mut stock = Vec::new();
    let mut moved = 0i64;
    for (ep_id, qty) in book {
        if qty > 0 {
            stock.push(StockLeg {
                event_product_id: ep_id,
                from: Location::OnSite,
                to: Location::External,
                qty,
            });
            moved += qty;
        } else if qty < 0 {
            // 结构上不该出现（下单是 CAS，报废/退货都在事务内复核余额）。
            // 出现了就是有路径绕过了检查，硬报出来比默默带回一个负数好。
            return Err(ApiError::Conflict(format!(
                "商品 {ep_id} 的现场仓余额是 {qty}，账本不自洽，不能带回"
            )));
        }
    }

    // 全卖光了也要能收摊——post_journal 拒绝空 journal，所以判空跳过。
    let journal_id = if stock.is_empty() {
        None
    } else {
        Some(
            post_journal(
                &mut tx,
                event_id,
                JournalKind::TakeBack,
                None,
                None,
                Some("收摊带回"),
                &stock,
                &[],
            )
            .await?,
        )
    };

    tx.commit().await?;
    Ok(Json(TakebackResponse { journal_id, moved }))
}

#[derive(Serialize)]
pub struct SettleResponse {
    status: String,
}

async fn settle(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<SettleResponse>> {
    check_write_permission(&claims, event_id)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    require_event_open(&mut tx, event_id).await?;

    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE event_id = ? AND status = 'pending'")
            .bind(event_id)
            .fetch_one(&mut *tx)
            .await?;
    if pending > 0 {
        return Err(ApiError::Conflict(format!("还有 {pending} 单待处理")));
    }

    let leftovers: Vec<(String, i64)> = book_balances(&mut tx, event_id)
        .await?
        .into_iter()
        .filter(|(_, q)| *q != 0)
        .map(|(id, q)| (id.to_string(), q))
        .collect();
    if !leftovers.is_empty() {
        return Err(ApiError::Conflict(format!(
            "还有 {} 种商品在现场仓，带回之后才能结算",
            leftovers.len()
        )));
    }

    // 盘点没做也放行——总有意外（展会清场赶人）。结算单上会写
    //「未盘点，剩余数为账面推算」，靠 events.stocktaken_at 是不是 null。
    sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
        .bind(event_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(SettleResponse {
        status: "已结算".into(),
    }))
}
```

- [ ] **Step 8: 跑测试确认通过**

```bash
cd src-tauri && tauri-env linux cargo test --workspace closing::tests
```
Expected: 十条全 PASS。

- [ ] **Step 9: 全量验证并提交**

```bash
python3 scripts/check-event-guards.py
cd src-tauri && tauri-env linux cargo fmt
cd src-tauri && tauri-env linux cargo clippy --all-targets -- -D warnings
cd src-tauri && tauri-env linux cargo test --workspace
```
Expected: 全绿。

```bash
git add src-tauri/migrations/202609240002_add_closing_timestamps.sql \
        src-tauri/src/api/closing.rs src-tauri/src/api/mod.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 收摊向导：清 pending → 盘点 → 带回 → 冻结

四步之间没有会话状态，每一步的成果都是已落库的 journal，向导只按
GET /closing 的返回决定显示哪一屏——退出、换设备、重进都从真实状态继续。
能不能进下一步由后端的 blockers 说了算，前端不自己判断。

清 pending 硬阻断：那些订单的货已经在顾客仓、钱一分没收，留着它们
「顾客仓余额 = 卖出」就不成立。盘点可跳过但要留痕，带回是一键（账面数
就是答案），转已结算检查现场仓全部归零。

新增 events.stocktaken_at / reconciled_at 两列。零差异的盘点和分文不差的
现金清点都写不出 journal（post_journal 拒绝空腿，零金额腿本来就被滤掉），
没有这两列，「盘了一遍全对」和「根本没盘」长得一模一样——而结算单上
「未盘点，剩余数为账面推算」和「微信 1,540 ✓」正好要分开它们。
reconciled_at 由 ②-3 的收摊清点写入，本 task 只建列。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: 垫付与结算调整

**Files:**
- Modify: `src-tauri/src/api/settlement.rs`（Task 2 建的，往里加）
- Test: 同文件的 `mod tests`

**Interfaces:**
- Produces:
  - `POST /api/events/:event_id/advances` `{ society_id, label, amount }` → 201 `{ id, journal_id }`
  - `GET /api/events/:event_id/advances` → `[{ id, society_id, society_name, label, amount }]`
  - `DELETE /api/events/:event_id/advances/:id` → 204
  - `POST /api/events/:event_id/adjustments` `{ society_id, label, direction, amount }` → 201；`direction` 是 `"to_them"` 或 `"to_me"`
  - `GET` / `DELETE` 同形
- Consumes: Task 4 的 `ledger::home_society_id`（不直接用，但同一个模块里会看到）。

**⚠️ 这三类端点故意不调用 `require_event_open`**（spec 偏离 3 + 3.2）。CI 门禁要求写豁免注释，**理由必须写清是有意的**，否则下一个人会顺手补上守卫。

- [ ] **Step 1: 写失败的测试**

加到 `src-tauri/src/api/settlement.rs` 的 `mod tests`：

```rust
    use crate::domain::ledger::{account_balance, Account};
    use crate::domain::money::Money;

    async fn post_advance(
        router: &axum::Router, event_id: i64, society_id: i64, label: &str, amount: i64,
    ) -> serde_json::Value {
        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/advances"), Some(&admin_token()),
            json!({"society_id": society_id, "label": label, "amount": amount}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        read_json(res).await
    }

    #[tokio::test]
    async fn an_advance_is_a_journal_not_a_note() {
        // 「结算单 = 往来账户的余额」这条不变量成立的前提，就是垫付在账本里。
        // 早先的 schema 草图漏了这个外键，那样结算单就得从两处拼数字。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        let body = post_advance(&router, event_id, 2, "摊位费", 40000).await;
        assert!(body["journal_id"].as_i64().unwrap() > 0);

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::from_cents(40000),
            "往来 +400 ⇒ 他们欠我 400"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::VendorOwn).await.unwrap(),
            Money::from_cents(-40000),
            "摊主自己的口袋少了 400"
        );
    }

    #[tokio::test]
    async fn deleting_an_advance_reverses_it_and_keeps_the_trail() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let id = post_advance(&router, event_id, 2, "打印费", 8000).await["id"]
            .as_i64().unwrap();

        let res = router.clone().oneshot(json_request(
            "DELETE", &format!("/api/events/{event_id}/advances/{id}"),
            Some(&admin_token()), json!(null),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::ZERO
        );
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM advances")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(rows, 0, "列表保持干净");
        let journals: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM journals WHERE kind = '垫付'",
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(journals, 2, "账本留下「一条垫付 + 一条冲正」");
    }

    #[tokio::test]
    async fn adjustment_direction_decides_the_sign_so_the_user_never_has_to() {
        // 界面上不给摊主填正负号。让人在收摊后的疲惫状态下判断
        // 「赔付该填正还是负」是设计失误。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/adjustments"), Some(&admin_token()),
            json!({"society_id": 2, "label": "清点少一本按成本赔", "direction": "to_them",
                   "amount": 2000}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::from_cents(-2000),
            "「我要多给他们 20」⇒ 往来 −20 ⇒ 我应转给 +20"
        );

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/adjustments"), Some(&admin_token()),
            json!({"society_id": 2, "label": "上次多结的尾数", "direction": "to_me",
                   "amount": 500}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::from_cents(-1500)
        );
    }

    #[tokio::test]
    async fn advances_and_adjustments_still_work_after_the_event_is_settled() {
        // spec 偏离 3：「回家翻出一张打印费收据」和「回家发现少了一本书」
        // 是同一类事件，允许后者却禁止前者说不通。
        //
        // 这条测试同时是给未来的人看的：看见别处都守着 require_event_open
        // 就顺手补上去的话，它会红。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id).execute(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/advances"), Some(&admin_token()),
            json!({"society_id": 2, "label": "回家翻出的打印费", "amount": 8000}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED, "垫付在冻结后仍然能补");

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/adjustments"), Some(&admin_token()),
            json!({"society_id": 2, "label": "协商赔付", "direction": "to_them", "amount": 2000}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED, "结算调整是母 spec 原有的例外");
    }

    #[tokio::test]
    async fn rejects_nonpositive_amounts_and_blank_labels() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        for body in [
            json!({"society_id": 2, "label": "x", "amount": 0}),
            json!({"society_id": 2, "label": "x", "amount": -100}),
            json!({"society_id": 2, "label": "   ", "amount": 100}),
        ] {
            let res = router.clone().oneshot(json_request(
                "POST", &format!("/api/events/{event_id}/advances"), Some(&admin_token()), body,
            )).await.unwrap();
            assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        }
    }

    #[tokio::test]
    async fn rejects_an_unknown_society() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/advances"), Some(&admin_token()),
            json!({"society_id": 999, "label": "摊位费", "amount": 100}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
```

- [ ] **Step 2: 跑测试确认失败**

```bash
cd src-tauri && tauri-env linux cargo test --workspace settlement::tests
```
Expected: 404 / 编译失败。

- [ ] **Step 3: 实现共用的写入逻辑**

加到 `src-tauri/src/api/settlement.rs`（路由表同时补上六条）：

```rust
/// 名目的长度上限。和渠道名一样，长度不设限的自由文本迟早会有人贴一整段进来。
const MAX_LABEL_CHARS: usize = 50;

fn normalize_label(raw: &str) -> ApiResult<String> {
    let s = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.is_empty() {
        return Err(ApiError::BadRequest("名目不能为空".into()));
    }
    if s.chars().count() > MAX_LABEL_CHARS {
        return Err(ApiError::BadRequest(format!(
            "名目最长 {MAX_LABEL_CHARS} 个字"
        )));
    }
    Ok(s)
}

async fn ensure_society(conn: &mut sqlx::SqliteConnection, society_id: i64) -> ApiResult<()> {
    let ok: Option<i64> = sqlx::query_scalar("SELECT id FROM societies WHERE id = ?")
        .bind(society_id)
        .fetch_optional(&mut *conn)
        .await?;
    ok.map(|_| ())
        .ok_or_else(|| ApiError::BadRequest("社团不存在".into()))
}

#[derive(Deserialize)]
pub struct AdvanceRequest {
    society_id: i64,
    label: String,
    /// 分，必须为正。方向是固定的（摊主掏钱给社团），不需要符号。
    amount: i64,
}

#[derive(Serialize)]
pub struct CreatedEntry {
    id: i64,
    journal_id: i64,
}

async fn create_advance(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<AdvanceRequest>,
) -> ApiResult<(StatusCode, Json<CreatedEntry>)> {
    // 不需要展会守卫：spec 偏离 3——冻结后仍然允许追加垫付。
    // 「回家翻出一张打印费收据」和「回家发现少了一本书」是同一类事件，
    // 母 spec 允许后者却禁止前者说不通。**不要顺手补上 require_event_open**，
    // settlement.rs 的模块注释和 tests 里那条测试都在守这件事。
    check_write_permission(&claims, event_id)?;
    let label = normalize_label(&payload.label)?;
    if payload.amount <= 0 {
        return Err(ApiError::BadRequest("垫付金额必须为正".into()));
    }

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    ensure_society(&mut tx, payload.society_id).await?;

    // 摊主掏钱、社团欠他：`摊主自有 → 社团往来:<社团>`。
    // 这和「摊主自掏赠品」（api/inventory.rs）恰好相反，别抄混。
    let journal_id = post_journal(
        &mut tx,
        event_id,
        JournalKind::Advance,
        None,
        None,
        Some(&label),
        &[],
        &[MoneyLeg {
            from: Account::VendorOwn,
            to: Account::SocietyDue(payload.society_id),
            amount: Money::from_cents(payload.amount),
        }],
    )
    .await?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO advances (event_id, owner_society_id, journal_id, label, amount)
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(event_id)
    .bind(payload.society_id)
    .bind(journal_id)
    .bind(&label)
    .bind(payload.amount)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(CreatedEntry { id, journal_id })))
}
```

- [ ] **Step 4: 实现结算调整**

```rust
#[derive(Deserialize)]
pub struct AdjustmentRequest {
    society_id: i64,
    label: String,
    /// `to_them` = 我要多给他们；`to_me` = 他们要多给我。
    ///
    /// **界面不给摊主填正负号。** 母 spec 5.2 那个例子自己都要算一遍才对得上
    /// 方向，让人在收摊后的疲惫状态下判断「赔付该填正还是负」是设计失误。
    direction: String,
    /// 分，必须为正。符号由 `direction` 决定。
    amount: i64,
}

async fn create_adjustment(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<AdjustmentRequest>,
) -> ApiResult<(StatusCode, Json<CreatedEntry>)> {
    // 不需要展会守卫：母 spec 6.4 明写这是冻结后唯一允许的操作
    //（spec 偏离 3 又加了垫付和收摊清点）。
    check_write_permission(&claims, event_id)?;
    let label = normalize_label(&payload.label)?;
    if payload.amount <= 0 {
        return Err(ApiError::BadRequest("金额必须为正，方向用 direction 表达".into()));
    }
    let to_them = match payload.direction.as_str() {
        "to_them" => true,
        "to_me" => false,
        other => {
            return Err(ApiError::BadRequest(format!(
                "方向只能是 to_them 或 to_me，收到「{other}」"
            )))
        }
    };

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    ensure_society(&mut tx, payload.society_id).await?;

    // to_them：往来 −amount ⇒ 我应转给 +amount
    // to_me  ：往来 +amount ⇒ 我应转给 −amount
    let (from, to) = if to_them {
        (Account::SocietyDue(payload.society_id), Account::SettlementAdj)
    } else {
        (Account::SettlementAdj, Account::SocietyDue(payload.society_id))
    };
    let journal_id = post_journal(
        &mut tx,
        event_id,
        JournalKind::Adjust,
        None,
        None,
        Some(&label),
        &[],
        &[MoneyLeg {
            from,
            to,
            amount: Money::from_cents(payload.amount),
        }],
    )
    .await?;

    // 存进表里的是**对往来余额的影响**（带符号），和 journal 的方向一致。
    // DB 上有 CHECK (amount <> 0)。
    let signed = if to_them { -payload.amount } else { payload.amount };
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO settlement_adjustments (event_id, owner_society_id, journal_id, label, amount)
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(event_id)
    .bind(payload.society_id)
    .bind(journal_id)
    .bind(&label)
    .bind(signed)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(CreatedEntry { id, journal_id })))
}
```

- [ ] **Step 5: 实现两组列表与删除**

两张表形状相同，所以列表和删除各写一个泛化的私有函数，按表名分派。**表名是代码里的字面量，不来自请求**，所以拼进 SQL 是安全的；但 sqlx 0.9 下拼出来的 `&str` 不能直接传 `query`，要包 `sqlx::AssertSqlSafe`（②-2 执行时踩过这个坑）。

```rust
#[derive(Serialize, sqlx::FromRow)]
pub struct LedgerEntryRow {
    id: i64,
    owner_society_id: i64,
    society_name: String,
    label: String,
    /// 垫付恒为正；结算调整带符号（负 = 我要多给他们）。
    amount: i64,
}

async fn list_entries_of(
    state: &AppState,
    event_id: i64,
    table: &'static str,
) -> ApiResult<Vec<LedgerEntryRow>> {
    let sql = format!(
        "SELECT t.id, t.owner_society_id, s.name AS society_name, t.label, t.amount
         FROM {table} t JOIN societies s ON s.id = t.owner_society_id
         WHERE t.event_id = ? ORDER BY t.id"
    );
    let rows = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(event_id)
        .fetch_all(&state.db)
        .await?;
    Ok(rows)
}

/// 删除 = 冲正 journal + 删业务表那一行。
///
/// 列表保持干净，账本留下「一条原始 + 一条冲正」的痕迹，而
/// 「结算单 = 往来账户余额」这条不变量自动成立——余额本来就是聚合出来的。
async fn delete_entry_of(
    state: &AppState,
    event_id: i64,
    id: i64,
    table: &'static str,
    kind: JournalKind,
) -> ApiResult<StatusCode> {
    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;

    let sql = format!("SELECT journal_id FROM {table} WHERE id = ? AND event_id = ?");
    let journal_id: Option<i64> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .bind(event_id)
        .fetch_optional(&mut *tx)
        .await?;
    let journal_id = journal_id.ok_or_else(|| ApiError::NotFound("记录不存在".into()))?;

    reverse_journal(&mut tx, journal_id, kind, Some("删除")).await?;

    let sql = format!("DELETE FROM {table} WHERE id = ?");
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
```

四个薄 handler（`list_advances` / `delete_advance` / `list_adjustments` / `delete_adjustment`）各自转调上面两个，`delete_*` 里同样写
`// 不需要展会守卫：与新增同理（spec 偏离 3）`。

路由表补成：

```rust
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/channels", get(list_channels))
        .route(
            "/events/:event_id/advances",
            get(list_advances).post(create_advance),
        )
        .route("/events/:event_id/advances/:id", delete(delete_advance))
        .route(
            "/events/:event_id/adjustments",
            get(list_adjustments).post(create_adjustment),
        )
        .route("/events/:event_id/adjustments/:id", delete(delete_adjustment))
}
```

- [ ] **Step 6: 跑测试、门禁、全量验证并提交**

```bash
cd src-tauri && tauri-env linux cargo test --workspace settlement::tests
python3 scripts/check-event-guards.py
cd src-tauri && tauri-env linux cargo fmt
cd src-tauri && tauri-env linux cargo clippy --all-targets -- -D warnings
cd src-tauri && tauri-env linux cargo test --workspace
```
Expected: 全绿。门禁会看到四个写 handler 带着豁免注释通过。

```bash
git add src-tauri/src/api/settlement.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 垫付与结算调整，各自入账

两者都带 journal_id，因为「结算单 = 往来账户的余额」这条不变量成立的前提
就是所有影响往来的东西都在账本里。删除走冲正而不是硬删：列表保持干净，
账本留下「一条原始 + 一条冲正」的痕迹。

结算调整不给摊主填正负号，给两个方向：「我要多给他们」/「他们要多给我」。
母 spec 5.2 那个例子自己都要算一遍才对得上方向。

这四个写入口**故意不调用 require_event_open**（spec 偏离 3），并各自带了
豁免注释和一条「已结算展会上仍然成功」的测试——看见别处都守着就顺手补
上去的话，那条测试会红。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: `domain/settlement.rs` —— 结算单的全部算术，零 I/O

**这是本轮最高价值的测试靶子**，地位和 ②-2 的求解器相同。结算单页面和 xlsx 导出
渲染的是**同一个 `SettlementReport`**——「页面显示 1,170、导出写 1,150」在结构上
不可能发生的唯一办法。

**Files:**
- Create: `src-tauri/src/domain/settlement.rs`
- Modify: `src-tauri/src/domain/mod.rs`
- Test: 同文件的 `mod tests`

**Interfaces:**
- Produces: `crate::domain::settlement::{build_report, SettlementInput, SettlementReport, SocietyInput, SocietyBlock, GoodsLine, GoodsTotals, Entry, ChannelCount, ChannelLine}`，以及 `pub fn build_report(input: &SettlementInput) -> SettlementReport`。
- Consumes: 只有 `crate::domain::money::Money`。**不 import sqlx，不 import axum。**

### 两条恒等式（这个模块存在的理由）

```
货： 带去 − 带回 = 卖出 + 赠送 + 报废 + 差异 + 现场仓余额
钱： 我应转给X = 净额 + 退货保留 + 自掏赠品 − 垫付合计 + 调整合计
      而「我应转给X」独立地等于 −（账本里 社团往来:X 的余额）
```

第二条是关键：**左边从业务表加出来，右边从账本聚合出来**，两条路必须撞上。
撞不上就是某条腿方向写错了或者漏记了，`build_report` 把差额写进 `warnings`，
页面和导出都显眼地标出来——**不静默出一个错数**。

各项定义（`Money`，均已扣除退货部分）：

| 项 | 定义 |
|---|---|
| `gross` | `Σ unit_price × (qty − 已退 qty)` |
| `allocated_net` | `Σ allocated_amount − Σ refunds.allocated_amount` |
| `lot_discount` | `gross − allocated_net`（显示项，按定义恒成立） |
| `manual_discount_net` | 手工折让净额，**只有本社团非零**；折让为正、加价为负 |
| `net` | `allocated_net − manual_discount_net` |
| `refund_kept` | `Σ (refunds.paid_amount − refunds.refund_amount)`（退货第 ③ 条腿） |
| `gift_self_paid` | 摊主自掏赠品合计 |
| `advances` / `adjustments` | 逐条，金额都按**对「我应转给」的影响**存：垫付为正（要减）、调整正数 = 我要多给他们 |

- [ ] **Step 1: 写失败的测试**

新建 `src-tauri/src/domain/settlement.rs`，先只写 `mod tests`：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn goods(code: &str, brought: i64, sold: i64, gift: i64, scrap: i64, var: i64, back: i64, on: i64)
        -> GoodsLine
    {
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
            Entry { label: "摊位费".into(), amount: Money::from_cents(40000) },
            Entry { label: "打印费".into(), amount: Money::from_cents(8000) },
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
                ChannelCount { channel: "微信".into(), book: Money::from_cents(154000),
                               actual: Some(Money::from_cents(154000)) },
                ChannelCount { channel: "支付宝".into(), book: Money::from_cents(32000),
                               actual: Some(Money::from_cents(32000)) },
                ChannelCount { channel: "现金".into(), book: Money::from_cents(15000),
                               actual: Some(Money::from_cents(14500)) },
            ],
        ));

        assert!(report.warnings.is_empty(), "样例应当自洽：{:?}", report.warnings);
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
        assert!(report.warnings[0].contains("A"), "要说清是哪个商品：{:?}", report.warnings);
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
```

- [ ] **Step 2: 跑测试确认失败**

先在 `domain/mod.rs` 加 `pub mod settlement;`：

```bash
cd src-tauri && tauri-env linux cargo test --workspace domain::settlement
```
Expected: 编译失败。

- [ ] **Step 3: 定义输入类型**

```rust
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
```

- [ ] **Step 4: 定义输出类型**

```rust
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
```

- [ ] **Step 5: 实现 `build_report`**

```rust
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
```

> `Money` 需要的 trait **全都已经有了**，`domain/money.rs` 一行不用改：
> `Serialize`（带 `#[serde(transparent)]`，所以 JSON 里是裸整数分而不是
> `{"0": 1990}`）、`Sum`、`Neg`、`AddAssign`、`Sub`、`Ord`。
> 提交时也就不要把 `money.rs` 加进 `git add`。

- [ ] **Step 6: 跑测试确认通过**

```bash
cd src-tauri && tauri-env linux cargo test --workspace domain::settlement
```
Expected: 七条全 PASS。

`the_spec_sample_adds_up` 是最容易红的一条——它把母 spec 第 7 节的样例原样搬了过来。
红了先算一遍手算值，**不要改测试去迁就实现**：那个样例的自洽性母 spec 里自己验过
（「应有 2,010 = 1,650 + 360；摊主留存 = 2,005 − 1,170 − 380 = 455」）。

- [ ] **Step 7: 全量验证并提交**

```bash
cd src-tauri && tauri-env linux cargo fmt
cd src-tauri && tauri-env linux cargo clippy --all-targets -- -D warnings
cd src-tauri && tauri-env linux cargo test --workspace
```

```bash
git add src-tauri/src/domain/settlement.rs src-tauri/src/domain/mod.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 结算单的算术收成一个纯函数

不 import sqlx、不 import axum、不碰时间。api 层查数据喂进来，页面 JSON
和 settlement.xlsx 渲染同一个 SettlementReport——这是「页面显示 1,170、
导出写 1,150」在结构上不可能发生的唯一办法。

两条恒等式在这里断言：货那侧「带去 − 带回 = 卖出 + 赠送 + 报废 + 差异 +
现场仓」，钱那侧「明细加出来 == −(社团往来余额)」。后者的左边从业务表来、
右边从账本来，两条独立的路必须撞上；撞不上就写进 warnings 让页面显眼地
标出来，不静默出一个错数。

母 spec 第 7 节那个样例原样进了测试。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: 结算单 JSON 与收摊清点

**Files:**
- Modify: `src-tauri/src/api/settlement.rs`
- Test: 同文件的 `mod tests`

**Interfaces:**
- Produces:
  - `GET /api/events/:event_id/settlement` → `SettlementReport` 的 JSON
  - `POST /api/events/:event_id/settlement/reconcile` `{ counts: [{ channel, actual }] }` → 200 `{ journal_id: Option<i64> }`
  - `crate::api::settlement::load_input(state, event_id) -> ApiResult<SettlementInput>`（Task 9 的 xlsx 复用它）
- Consumes: Task 7 的 `domain::settlement::*`；`ledger::account_balance`。

### 每个字段从哪来（写代码前先对着这张表）

| `SettlementInput` 字段 | 来源 |
|---|---|
| `goods` | `stock_movements` 按「方向 / 位置」聚合，按 `event_products.owner_society_id` 分组 |
| `gross` | `Σ ol.unit_price × (ol.qty − 已退 qty)`，只算 `orders.status = 'completed'` |
| `allocated_net` | `Σ (ol.allocated_amount − 已退 allocated)` |
| `manual_discount_net` | **全局** `Σ(allocated_net) − Σ(paid_net)`，整个数落在本社团头上（母 spec 4.4） |
| `refund_kept` | `Σ (refunds.paid_amount − refunds.refund_amount)` |
| `gift_self_paid` | kind = `赠送` 的 journal 里 `社团往来:X ↔ 摊主自有` 的净额（冲正条目自动相抵） |
| `advances` / `adjustments` | 两张业务表逐条。**符号要翻**：DB 里存的是对往来余额的影响，`Entry.amount` 要的是对「我应转给」的影响 |
| `due_balance` | `ledger::account_balance(pool, event_id, &Account::SocietyDue(id))` |
| `channels[].book` | 当前 `实收-<渠道>` 余额 **减去** 对账差异那部分 |
| `channels[].actual` | `reconciled_at` 非空时 = 当前余额；否则 `None` |

- [ ] **Step 1: 写失败的测试**

加到 `src-tauri/src/api/settlement.rs` 的 `mod tests`：

```rust
    /// 端到端：下单 → 完成（带手工折让）→ 退一部分 → 垫付 → 调整 → 带回 → 结算单。
    /// 断言只有一条：**warnings 为空**。
    ///
    /// 这比逐个字段对数字有力得多——warnings 里那两条恒等式分别从业务表和账本
    /// 两条独立的路算出来，只要有一条腿方向写错或漏记，它们就撞不上。
    #[tokio::test]
    async fn a_full_event_produces_a_self_consistent_settlement() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        // 卖一单：A×1（本社团）+ B×2（黄昏堂），原价 7000，实收改成 6500（手工折让 500）
        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders"), None,
            json!({"items": [{"product_id": ep_a, "quantity": 1},
                             {"product_id": ep_b, "quantity": 2}]}),
        )).await.unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();
        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "微信", "final_amount": 6500}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // 退 B 的一件，只退 800（顾客没拿回的部分归黄昏堂）
        let line_b: i64 = sqlx::query_scalar(
            "SELECT id FROM order_lines WHERE order_id = ? AND event_product_id = ? LIMIT 1",
        ).bind(order_id).bind(ep_b).fetch_one(&pool).await.unwrap();
        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders/{order_id}/refunds"), Some(&token),
            json!({"channel": "现金", "refund_amount": 800,
                   "lines": [{"order_line_id": line_b, "qty": 1, "destination": "现场仓"}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        // 送一件 A（默认口径）、摊主自掏送一件 B、报废一件 A
        for body in [
            json!({"event_product_id": ep_a, "qty": 1}),
            json!({"event_product_id": ep_b, "qty": 1, "vendor_pays": true}),
        ] {
            let res = router.clone().oneshot(json_request(
                "POST", &format!("/api/events/{event_id}/gifts"), Some(&token), body,
            )).await.unwrap();
            assert_eq!(res.status(), StatusCode::CREATED);
        }
        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/scraps"), Some(&token),
            json!({"event_product_id": ep_a, "qty": 1, "note": "压坏了"}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        // 垫付 + 调整
        router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/advances"), Some(&token),
            json!({"society_id": 2, "label": "摊位费", "amount": 40000}),
        )).await.unwrap();
        router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/adjustments"), Some(&token),
            json!({"society_id": 2, "label": "赔一本", "direction": "to_them", "amount": 2000}),
        )).await.unwrap();

        // 盘点 + 带回 + 结算
        let remaining = router.clone().oneshot(json_request(
            "GET", &format!("/api/events/{event_id}/closing"), Some(&token), json!(null),
        )).await.unwrap();
        let counts: Vec<serde_json::Value> = read_json(remaining).await["onsite_remaining"]
            .as_array().unwrap().iter()
            .map(|r| json!({"event_product_id": r["event_product_id"], "counted_qty": r["qty"]}))
            .collect();
        router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/closing/stocktake"), Some(&token),
            json!({"counts": counts}),
        )).await.unwrap();
        router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/closing/takeback"), Some(&token), json!(null),
        )).await.unwrap();
        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/closing/settle"), Some(&token), json!(null),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // ---- 结算单 ----
        let res = router.clone().oneshot(json_request(
            "GET", &format!("/api/events/{event_id}/settlement"), Some(&token), json!(null),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let report = read_json(res).await;
        assert_eq!(
            report["warnings"].as_array().unwrap().len(), 0,
            "结算单自相矛盾：{}", report["warnings"]
        );
        assert!(report["stocktaken"].as_bool().unwrap());

        // 摊主留存 + Σ我应转给 == 实际到手
        let retained = report["vendor_retained"].as_i64().unwrap();
        let transfer_total = report["transfer_total"].as_i64().unwrap();
        let actual_total = report["actual_total"].as_i64().unwrap();
        assert_eq!(retained + transfer_total, actual_total);
    }

    #[tokio::test]
    async fn reconciling_records_the_shortfall_against_the_vendor_not_the_owners() {
        // 母 spec 第 5 节：对账差异默认由摊主自吞，不进任何货主的结算。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders"), None,
            json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
        )).await.unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();
        router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "现金"}),
        )).await.unwrap();

        let before = read_json(router.clone().oneshot(json_request(
            "GET", &format!("/api/events/{event_id}/settlement"), Some(&token), json!(null),
        )).await.unwrap()).await;
        let transfer_before = before["societies"][0]["transfer"].as_i64().unwrap();

        // 现金盒里少了 5 块
        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/settlement/reconcile"), Some(&token),
            json!({"counts": [{"channel": "现金", "actual": 2500}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let after = read_json(router.clone().oneshot(json_request(
            "GET", &format!("/api/events/{event_id}/settlement"), Some(&token), json!(null),
        )).await.unwrap()).await;
        assert!(after["warnings"].as_array().unwrap().is_empty(), "{}", after["warnings"]);
        assert_eq!(
            after["societies"][0]["transfer"].as_i64().unwrap(), transfer_before,
            "货主该拿多少和摊主数出来多少钱无关"
        );
        assert_eq!(after["channels"][0]["book"], 3000);
        assert_eq!(after["channels"][0]["actual"], 2500);
        assert_eq!(after["channels"][0]["diff"], -500);
        assert_eq!(after["channels"][0]["counted"], true);
        assert_eq!(
            after["vendor_retained"].as_i64().unwrap(),
            2500 - transfer_before,
            "短的 5 块由摊主自吞"
        );
    }

    #[tokio::test]
    async fn counting_an_exact_match_still_marks_it_counted() {
        // 分文不差时写不出 journal（零金额腿被 post_journal 滤掉，
        // 只剩空 journal 会被拒），所以「数过了」靠 events.reconciled_at 记。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders"), None,
            json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
        )).await.unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();
        router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "现金"}),
        )).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/settlement/reconcile"), Some(&token),
            json!({"counts": [{"channel": "现金", "actual": 3000}]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(read_json(res).await["journal_id"].is_null());

        let report = read_json(router.clone().oneshot(json_request(
            "GET", &format!("/api/events/{event_id}/settlement"), Some(&token), json!(null),
        )).await.unwrap()).await;
        assert_eq!(report["channels"][0]["counted"], true);
        assert_eq!(report["channels"][0]["diff"], 0);
    }

    #[tokio::test]
    async fn reconciling_twice_writes_only_the_new_delta() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/orders"), None,
            json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
        )).await.unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();
        router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "现金"}),
        )).await.unwrap();

        for actual in [2500, 2800] {
            let res = router.clone().oneshot(json_request(
                "POST", &format!("/api/events/{event_id}/settlement/reconcile"), Some(&token),
                json!({"counts": [{"channel": "现金", "actual": actual}]}),
            )).await.unwrap();
            assert_eq!(res.status(), StatusCode::OK);
        }

        let report = read_json(router.clone().oneshot(json_request(
            "GET", &format!("/api/events/{event_id}/settlement"), Some(&token), json!(null),
        )).await.unwrap()).await;
        assert_eq!(report["channels"][0]["actual"], 2800, "第二次数出来是 28");
        assert_eq!(report["channels"][0]["book"], 3000, "账面应有始终是 30");
    }

    #[tokio::test]
    async fn reconciling_works_after_the_event_is_settled() {
        // spec 偏离 3：回家才有空数现金盒、对微信账单是常态。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id).execute(&pool).await.unwrap();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/settlement/reconcile"), Some(&admin_token()),
            json!({"counts": []}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
```

- [ ] **Step 2: 跑测试确认失败**

```bash
cd src-tauri && tauri-env linux cargo test --workspace settlement::tests
```
Expected: 404 / 编译失败。

- [ ] **Step 3: 实现 `load_input`——货那一侧**

```rust
#[derive(sqlx::FromRow)]
struct GoodsRow {
    owner_society_id: i64,
    event_product_id: i64,
    product_code: String,
    name: String,
    brought_in: i64,
    taken_back: i64,
    sold: i64,
    gifted: i64,
    scrapped: i64,
    variance: i64,
    on_site: i64,
}

/// 货的全部去向。每一项都从 `stock_movements` 按方向取，**没有任何缓存字段**。
///
/// 「带去」和「带回」是**方向和**而不是净额——结算单上这两个数是分开显示的。
/// 其余四项是**位置净额**，所以退货到损耗会自动让「卖出」减一、「报废」加一，
/// 不需要在这里为退货开分支。
const GOODS_SQL: &str = r#"
SELECT ep.owner_society_id, ep.id AS event_product_id, ep.product_code, ep.name,
  COALESCE(SUM(CASE WHEN sm.from_location='外部'   AND sm.to_location='现场仓' THEN sm.qty ELSE 0 END),0) AS brought_in,
  COALESCE(SUM(CASE WHEN sm.from_location='现场仓' AND sm.to_location='外部'   THEN sm.qty ELSE 0 END),0) AS taken_back,
  COALESCE(SUM(CASE WHEN sm.to_location='顾客仓' THEN sm.qty ELSE 0 END)
         - SUM(CASE WHEN sm.from_location='顾客仓' THEN sm.qty ELSE 0 END),0) AS sold,
  COALESCE(SUM(CASE WHEN sm.to_location='赠品'   THEN sm.qty ELSE 0 END)
         - SUM(CASE WHEN sm.from_location='赠品'   THEN sm.qty ELSE 0 END),0) AS gifted,
  COALESCE(SUM(CASE WHEN sm.to_location='损耗'   THEN sm.qty ELSE 0 END)
         - SUM(CASE WHEN sm.from_location='损耗'   THEN sm.qty ELSE 0 END),0) AS scrapped,
  COALESCE(SUM(CASE WHEN sm.to_location='差异'   THEN sm.qty ELSE 0 END)
         - SUM(CASE WHEN sm.from_location='差异'   THEN sm.qty ELSE 0 END),0) AS variance,
  COALESCE(SUM(CASE WHEN sm.to_location='现场仓' THEN sm.qty ELSE 0 END)
         - SUM(CASE WHEN sm.from_location='现场仓' THEN sm.qty ELSE 0 END),0) AS on_site
FROM event_products ep
LEFT JOIN stock_movements sm ON sm.event_product_id = ep.id
LEFT JOIN journals j ON j.id = sm.journal_id AND j.event_id = ep.event_id
WHERE ep.event_id = ?
GROUP BY ep.id
ORDER BY ep.owner_society_id, ep.id
"#;
```

> ⚠️ **`variance` 的符号**：`domain/settlement.rs` 的恒等式是
> `带去 − 带回 = 卖出 + 赠送 + 报废 + 差异 + 现场仓`。盘亏时货 `现场仓 → 差异`，
> 所以 `差异` 位置净额为正、`现场仓` 减少——两边同时变，恒等式仍然成立。
> **不要在这里给 `variance` 取反**。

- [ ] **Step 4: 实现 `load_input`——钱那一侧**

```rust
#[derive(sqlx::FromRow)]
struct MoneyRow {
    owner_society_id: i64,
    gross: i64,
    allocated_net: i64,
    paid_net: i64,
    refund_kept: i64,
}

/// 只算 `orders.status = 'completed'`。
///
/// **不要按「存在收款 journal」来判已收款订单**（②-1 交接段第 1 条）：
/// 全赠品单和被抹成 0 元的单都会让 journal 缺席或为空。按 orders.status 判。
const MONEY_SQL: &str = r#"
SELECT ep.owner_society_id,
  COALESCE(SUM(ol.unit_price * (ol.qty - COALESCE(r.done_qty, 0))), 0)        AS gross,
  COALESCE(SUM(ol.allocated_amount - COALESCE(r.done_alloc, 0)), 0)           AS allocated_net,
  COALESCE(SUM(ol.paid_amount      - COALESCE(r.done_paid, 0)), 0)            AS paid_net,
  COALESCE(SUM(COALESCE(r.done_paid, 0) - COALESCE(r.done_refund, 0)), 0)     AS refund_kept
FROM order_lines ol
JOIN orders o          ON o.id = ol.order_id AND o.status = 'completed'
JOIN event_products ep ON ep.id = ol.event_product_id
LEFT JOIN (
  SELECT order_line_id,
         SUM(qty)               AS done_qty,
         SUM(allocated_amount)  AS done_alloc,
         SUM(paid_amount)       AS done_paid,
         SUM(refund_amount)     AS done_refund
  FROM refunds GROUP BY order_line_id
) r ON r.order_line_id = ol.id
WHERE o.event_id = ?
GROUP BY ep.owner_society_id
"#;

/// 摊主自掏赠品：kind = `赠送` 的 journal 里 `社团往来:X ↔ 摊主自有` 的净额。
/// 冲正条目的 kind 也是 `赠送`（`api/inventory.rs` 原样传回去），方向相反，
/// 所以净额天然把撤销掉的那些抵消了。
const GIFT_SELF_PAID_SQL: &str = r#"
SELECT COALESCE(SUM(CASE WHEN mm.to_account   = '摊主自有' THEN mm.amount ELSE 0 END)
              - SUM(CASE WHEN mm.from_account = '摊主自有' THEN mm.amount ELSE 0 END), 0)
FROM money_movements mm
JOIN journals j ON j.id = mm.journal_id
WHERE j.event_id = ? AND j.kind = '赠送'
  AND (mm.from_account = ? OR mm.to_account = ?)
"#;
```

调用时**同一个账户名 bind 两次**（SQLite 驱动按位置取参，没有重复编号参数）：

```rust
let account = Account::SocietyDue(society_id).to_string();
let gift_self_paid: i64 = sqlx::query_scalar(GIFT_SELF_PAID_SQL)
    .bind(event_id)
    .bind(&account)
    .bind(&account)
    .fetch_one(&state.db)
    .await?;
```

账户名一律用 `Account::SocietyDue(id).to_string()` 生成，**不要手工拼 `"社团往来:"`**——
前缀常量在 `domain/ledger.rs` 里，手抄一份就多一个会漂移的地方。

**`manual_discount_net` 是全局量**：

```rust
// 手工折让 = solved − final，摊进各行之后就是 Σallocated − Σpaid。
// 母 spec 4.4 定了它由本社团**全额**承担，所以整个数落在本社团头上，
// 不按货主分。代卖货主身上出现非零手工折让，domain/settlement.rs 会报警。
let manual_discount_net: i64 = money_rows
    .iter()
    .map(|m| m.allocated_net - m.paid_net)
    .sum();
```

- [ ] **Step 5: 实现 `load_input`——渠道与组装**

```rust
/// 对账差异对某个渠道的影响。清点写的是 `实收-<渠道> ↔ 对账差异`，
/// 所以「账面应有」= 当前余额 − 这个差额。
const RECON_DIFF_SQL: &str = r#"
SELECT COALESCE(SUM(CASE WHEN mm.to_account   = ? THEN mm.amount ELSE 0 END)
              - SUM(CASE WHEN mm.from_account = ? THEN mm.amount ELSE 0 END), 0)
FROM money_movements mm
JOIN journals j ON j.id = mm.journal_id
WHERE j.event_id = ?
  AND (mm.from_account = '对账差异' OR mm.to_account = '对账差异')
"#;
```

组装逻辑要点：

1. 渠道清单 = 本展会 `orders.channel` ∪ `refunds.channel`（**按展会过滤**，不是全局——
   `GET /channels` 那个是给下拉用的，这里是这一场实际用过的）。
2. `actual = if reconciled_at.is_some() { Some(current_balance) } else { None }`；
   `book = current_balance − recon_diff`。
3. `Entry.amount` 的符号要翻：
   ```rust
   // advances.amount 存的是正数（对往来余额的影响 +amount）。
   // 对「我应转给」的影响是 −amount，而 SocietyBlock 里 advances_total 是被减的，
   // 所以 Entry 存正数即可。
   Entry { label, amount: Money::from_cents(a.amount) }

   // settlement_adjustments.amount 存的是对往来余额的影响（负 = 我要多给他们）。
   // 对「我应转给」的影响正好相反，所以取负。
   Entry { label, amount: Money::from_cents(-adj.amount) }
   ```
4. `generated_at` 用 `chrono::Local::now()` 格式化成 `%Y-%m-%d %H:%M:%S`；
   `last_changed_at` 取 `SELECT MAX(occurred_at) FROM journals WHERE event_id = ?`。
5. 没有任何交易的社团**也要出现**（有垫付但一件货没卖是合法的），所以社团清单取
   `event_products` 的货主 ∪ `advances` 的社团 ∪ `settlement_adjustments` 的社团。

- [ ] **Step 6: 实现收摊清点**

```rust
#[derive(Deserialize)]
pub struct ReconcileRequest {
    counts: Vec<ChannelActual>,
}

#[derive(Deserialize)]
pub struct ChannelActual {
    channel: String,
    /// 摊主数出来的实际到手（分）。
    actual: i64,
}

async fn reconcile(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<ReconcileRequest>,
) -> ApiResult<Json<ReconcileResponse>> {
    // 不需要展会守卫：spec 偏离 3——回家才有空数现金盒、对微信账单是常态。
    // 对账差异默认由摊主自吞，不进任何货主的结算，只影响「摊主留存」那一个数。
    check_write_permission(&claims, event_id)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;

    let mut money = Vec::new();
    for c in &payload.counts {
        let channel = normalize(&c.channel)?;
        if c.actual < 0 {
            return Err(ApiError::BadRequest("实际到手不能为负".into()));
        }
        let account = Account::Received(channel);
        let current = account_balance_tx(&mut tx, event_id, &account).await?;
        let diff = Money::from_cents(c.actual) - current;
        if diff.is_zero() {
            continue; // 分文不差——没有腿可记，靠 reconciled_at 记「数过了」
        }
        // 账面多于实际 ⇒ 钱少了 ⇒ 实收流向对账差异；反之亦然。
        let (from, to) = if diff.is_negative() {
            (account.clone(), Account::ReconDiff)
        } else {
            (Account::ReconDiff, account.clone())
        };
        money.push(MoneyLeg { from, to, amount: diff.abs() });
    }

    // 全都对得上就没有腿，post_journal 会拒绝空 journal，所以判空跳过。
    let journal_id = if money.is_empty() {
        None
    } else {
        Some(
            post_journal(
                &mut tx, event_id, JournalKind::Adjust, None, None,
                Some("收摊清点"), &[], &money,
            )
            .await?,
        )
    };

    sqlx::query("UPDATE events SET reconciled_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(event_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(ReconcileResponse { journal_id }))
}
```

> `ledger::account_balance` 现有签名收 `&SqlitePool`，这里要在事务内读。
> 照 Task 5 的 `book_balances` 同一个办法，在本模块加：
>
> ```rust
> /// 事务内版的账户余额。签名和 `ledger::account_balance` 一致，只是换了 executor。
> /// 清点必须「读到的余额就是待会儿要写差额的那个余额」。
> async fn account_balance_tx(
>     conn: &mut sqlx::SqliteConnection,
>     event_id: i64,
>     account: &Account,
> ) -> ApiResult<Money> {
>     let name = account.to_string();
>     let cents: i64 = sqlx::query_scalar(
>         "SELECT COALESCE(
>                   SUM(CASE WHEN mm.to_account   = ?1 THEN mm.amount ELSE 0 END)
>                 - SUM(CASE WHEN mm.from_account = ?1 THEN mm.amount ELSE 0 END), 0)
>          FROM money_movements mm
>          JOIN journals j ON j.id = mm.journal_id
>          WHERE j.event_id = ?2",
>     )
>     .bind(&name)
>     .bind(event_id)
>     .fetch_one(&mut *conn)
>     .await?;
>     Ok(Money::from_cents(cents))
> }
> ```
>
> （`ledger::account_balance` 那条 SQL 用的是 `?1`/`?2` 编号参数，照抄即可——
> 它已经在生产里跑了两轮，不要顺手改写。）

- [ ] **Step 7: 挂上两条路由并跑测试**

```rust
        .route("/events/:event_id/settlement", get(get_settlement))
        .route("/events/:event_id/settlement/reconcile", post(reconcile))
```

```bash
cd src-tauri && tauri-env linux cargo test --workspace settlement::tests
```
Expected: 全 PASS。

`a_full_event_produces_a_self_consistent_settlement` 是最容易红的一条。**它红了就是有一条腿方向错了或者归集查询漏了**，`warnings` 里会直接写出差多少和差在谁头上——照着查，不要改断言。

- [ ] **Step 8: 全量验证并提交**

```bash
python3 scripts/check-event-guards.py
cd src-tauri && tauri-env linux cargo fmt
cd src-tauri && tauri-env linux cargo clippy --all-targets -- -D warnings
cd src-tauri && tauri-env linux cargo test --workspace
```

```bash
git add src-tauri/src/api/settlement.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 结算单 JSON 与收摊清点

结算单用 allocated_amount（货主该得多少不受摊主当天做了什么人情的影响），
仪表盘那套 paid_amount 是另一个口径，两者合法地对不上、差额恰好是手工折让。

端到端那条测试只断言一件事：warnings 为空。这比逐个字段对数字有力得多——
那两条恒等式分别从业务表和账本两条独立的路算出来，只要有一条腿方向写错或
漏记，它们就撞不上，而且报文里直接写出差多少、差在谁头上。

收摊清点允许在冻结之后做：对账差异默认摊主自吞，不进任何货主的结算，只影响
「摊主留存」那一个数，而回家才有空数现金盒。分文不差时写不出 journal，
所以「数过了」靠 events.reconciled_at 记。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: `settlement.xlsx` 四 sheet 导出

**Files:**
- Modify: `src-tauri/src/api/settlement.rs`
- Test: 同文件的 `mod tests`

**Interfaces:**
- Produces: `GET /api/events/:event_id/settlement.xlsx` → `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`
- Consumes: Task 8 的 `load_input` + Task 7 的 `build_report`。**不得另写一套算术。**

- [ ] **Step 1: 写失败的测试**

```rust
    #[tokio::test]
    async fn the_xlsx_is_a_real_workbook_with_four_sheets() {
        // 不解析 xlsx 内容（要引一个读库，不值）。断言三件事：
        // 状态码、Content-Type、以及 body 是个 zip（xlsx 就是 zip，魔数 PK\x03\x04）。
        // 数字对不对由 Task 7 的纯函数测试和 Task 8 的端到端测试保证——
        // 导出和页面渲染的是同一个 SettlementReport，这正是那样设计的理由。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        let res = router.clone().oneshot(json_request(
            "GET", &format!("/api/events/{event_id}/settlement.xlsx"),
            Some(&admin_token()), json!(null),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let ct = res.headers().get("content-type").unwrap().to_str().unwrap().to_string();
        assert!(ct.contains("spreadsheetml"), "content-type 是 {ct}");

        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        assert!(bytes.len() > 1000, "空文件");
        assert_eq!(&bytes[..4], b"PK\x03\x04", "xlsx 应当是个 zip");
    }

    #[tokio::test]
    async fn the_xlsx_filename_is_ascii_safe() {
        // 中文文件名在部分浏览器/CDN 上会变成 ???，下载直接坏掉——
        // 安装包那边已经踩过一次（见项目 CLAUDE.md 的发版流程）。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        let res = router.clone().oneshot(json_request(
            "GET", &format!("/api/events/{event_id}/settlement.xlsx"),
            Some(&admin_token()), json!(null),
        )).await.unwrap();
        let cd = res.headers().get("content-disposition").unwrap().to_str().unwrap();
        assert!(cd.is_ascii(), "Content-Disposition 必须是纯 ASCII：{cd}");
        assert!(cd.contains("settlement"));
    }
```

- [ ] **Step 2: 跑测试确认失败**

```bash
cd src-tauri && tauri-env linux cargo test --workspace settlement::tests::the_xlsx
```
Expected: 404。

- [ ] **Step 3: 实现导出**

沿用 `api/stats.rs` 的 `rust_xlsxwriter` 用法（`Workbook` / `Format` / `write_string_with_format`），
**但不要抄它那套合并标题和总计行的装饰**——那是给「销售汇总」设计的。

四个 sheet：

| sheet 名 | 列 |
|---|---|
| `结算汇总` | 货主 / 商品原价 / Lot折让 / 手工折让 / 净额 / 退货保留 / 自掏赠品 / 垫付 / 调整 / **我应转给**；下方空一行后是清点栏：渠道 / 账面应有 / 实际到手 / 差额 / 是否清点；最后是「实际到手合计」「Σ我应转给」「摊主留存」 |
| `货主明细` | 货主 / 商品编号 / 名称 / 带去 / 卖出 / 赠送 / 报废 / 差异 / 带回 / 现场仓 |
| `账本流水` | journal id / 时间 / 种类 / 订单号 / 摘要 / 货腿（商品·从→到·数量）/ 钱腿（从→到·金额） |
| `订单明细` | 订单号 / 状态 / 渠道 / 行号 / 商品 / 套装 / 件数 / 单价 / **货主应得** / **顾客实付** / 已退件数 |

要点：

```rust
// 金额一律写成**元的浮点**并套两位小数的 number format——
// xlsx 是给人看和给社团财务再加工的，写分会让每个数都要除 100。
// 这是唯一允许把 Money 变成浮点的地方，因为它离开系统了。
let money_fmt = Format::new().set_num_format("0.00");
worksheet.write_number_with_format(row, col, m.cents() as f64 / 100.0, &money_fmt)?;
```

```rust
// warnings 非空时，在「结算汇总」第一行写一条醒目的红色提示。
// 结算单自相矛盾却安安静静地导出去，比不导出更坏。
if !report.warnings.is_empty() {
    let warn = Format::new().set_bold().set_font_color(Color::Red);
    worksheet.write_string_with_format(0, 0, "⚠ 这张表和账本对不上，见下方说明", &warn)?;
    for (i, w) in report.warnings.iter().enumerate() {
        worksheet.write_string(1 + i as u32, 0, w)?;
    }
}
```

```rust
// 文件名纯 ASCII。展会名可能是中文，不要放进 filename。
(
    StatusCode::OK,
    [
        (header::CONTENT_TYPE,
         "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        (header::CONTENT_DISPOSITION,
         &format!("attachment; filename=\"settlement-event-{event_id}.xlsx\"")),
    ],
    buf,
)
```

`rust_xlsxwriter` 的错误不是 `sqlx::Error`，所以 `?` 不通。用
`.map_err(|e| { eprintln!("[settlement] xlsx: {e}"); ApiError::Db(sqlx::Error::Protocol("导出失败".into())) })`
会把 xlsx 的问题伪装成数据库错误——**不要那样**。给 `ApiError` 加一个变体不在本轮范围，
所以用 `ApiError::Conflict(format!("生成表格失败：{e}"))`：它是 409，语义不完美但
**诚实**，而且文案能到用户眼里。③a 收口错误类型时再归位。

- [ ] **Step 4: 跑测试、全量验证并提交**

```bash
cd src-tauri && tauri-env linux cargo test --workspace
python3 scripts/check-event-guards.py
cd src-tauri && tauri-env linux cargo fmt
cd src-tauri && tauri-env linux cargo clippy --all-targets -- -D warnings
```

```bash
git add src-tauri/src/api/settlement.rs
git commit -m "$(cat <<'EOF'
feat: :sparkles: 结算单导出成四 sheet 的 xlsx

结算汇总 / 货主明细 / 账本流水 / 订单明细。和页面渲染同一个
SettlementReport，不另写一套算术——「页面显示 1,170、导出写 1,150」
在结构上就不可能发生。

warnings 非空时在第一行写红字提示：结算单自相矛盾却安安静静地导出去，
比不导出更坏。

文件名纯 ASCII：中文文件名在部分浏览器/CDN 上会变成 ???，下载直接坏掉，
安装包那边已经踩过一次。

老的 sales_summary/download 保留不动——它回答「卖了什么」，这个回答
「账怎么算」，受众不同。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: 前端 —— 渠道选择组件 + 赠送/报废登记

**前端约定**（后面四个 task 都适用）：

- **金额一律传分。** 输入框收「元」，提交前 `toCents()`；显示一律 `formatYuan()`。
  见 `frontend/src/utils/money.js` 的模块注释。
- **不重复后端的业务规则。** 后端说不行就把 `err.response?.data?.error` 原样显示出来。
  两边各判一遍迟早不一致（`lotStore.js` 的注释写了这条）。
- **不要起 dev server 看效果。** 那是人的活。验证靠 `npm --prefix frontend run lint`、
  `test:unit`、`build` 三条。

**Files:**
- Create: `frontend/src/components/shared/ChannelSelect.vue`
- Create: `frontend/src/components/vendor/InventoryLogModal.vue`
- Create: `frontend/src/stores/inventoryLogStore.js`
- Modify: `frontend/src/components/vendor/ReceiptModal.vue`
- Modify: `frontend/src/views/VendorView.vue`

**Interfaces:**
- Produces: `<ChannelSelect v-model="channel" />`（自己加载 `/channels`，允许现填新的）；
  `useInventoryLogStore()` 暴露 `gifts` / `scraps` / `fetchGifts` / `fetchScraps` / `logGift` / `logScrap` / `reverse`。
- Consumes: Task 2 的 `GET /api/channels`、Task 3 的六个端点。

- [ ] **Step 1: 写 `ChannelSelect.vue`**

替掉 `ReceiptModal.vue` 里那组硬编码 radio（`const CHANNELS = ['现金','微信','支付宝']`）。

```vue
<!--
  可输入的渠道下拉。**这才是真正防「微信」和「微信支付」分裂成两个账户的那一条**
  （②-1/②-2 交接段第 3 条）——列表来自后端 `/channels`（预置三个 + 历史用过的），
  摊主优先从已有的里挑，确实要新渠道才现填。

  长度上限和规范化在后端（domain/channel.rs），这里不重复判，
  报错原样显示后端那句话。
-->
<template>
  <n-select
    :value="modelValue"
    :options="options"
    filterable
    tag
    placeholder="选择或输入收款渠道"
    :loading="loading"
    @update:value="(v) => emit('update:modelValue', v)"
  />
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import api from '@/services/api'

const props = defineProps({ modelValue: { type: String, default: '' } })
const emit = defineEmits(['update:modelValue'])

const known = ref([])
const loading = ref(false)

// tag 模式下用户新填的值不在 options 里，补进去才不会显示成空白
const options = computed(() => {
  const all = [...known.value]
  if (props.modelValue && !all.includes(props.modelValue)) all.push(props.modelValue)
  return all.map((c) => ({ label: c, value: c }))
})

onMounted(async () => {
  loading.value = true
  try {
    const { data } = await api.get('/channels')
    known.value = Array.isArray(data) ? data : []
  } catch {
    // 拿不到历史列表不该让摊主收不了款——退回三个预置的
    known.value = ['现金', '微信', '支付宝']
  } finally {
    loading.value = false
  }
})
</script>
```

> `ReceiptModal.vue` 里 `CHANNEL_STORAGE_KEY` 那段「记住上次用的渠道」的逻辑保留，
> 只是校验从 `CHANNELS.includes(saved)` 改成「非空就用」——自定义渠道也该被记住。

- [ ] **Step 2: 写 store**

`frontend/src/stores/inventoryLogStore.js`，照 `lotStore.js` 的形状：`ref` + 若干 async action，
错误抛 `new Error(err.response?.data?.error || '默认文案')`。六个 action 对应 Task 3 的六个端点。

- [ ] **Step 3: 写 `InventoryLogModal.vue`**

一个弹窗，两个 tab（赠送 / 报废）。每个 tab：

- 商品下拉（只列现场仓余额 > 0 的，数据来自 `GET /events/:id/closing` 的 `onsite_remaining`——
  **复用它而不是另开端点**，那个接口本来就在算这个数）
- 数量输入（上限 = 该商品的现场仓余额，超了后端也会挡，前端只是少一次往返）
- 备注
- **仅赠送**：一个开关「这笔我自掏（按原价补给货主）」，默认关，下面一行小字说明
  「不开 = 货主自己承担，结算单上只会显示送了几件」
- 下方是已登记列表，每条一个「撤销」

- [ ] **Step 4: 挂进 `VendorView.vue`**

头部加一个「登记赠送/报废」按钮。**不要改现有的两个订单列表**——那是 ④ 要重做的东西。

- [ ] **Step 5: 验证并提交**

```bash
npm --prefix frontend run lint
npm --prefix frontend run test:unit
npm --prefix frontend run build
```
Expected: 三条全绿。

```bash
git add frontend/src/components/shared/ChannelSelect.vue \
        frontend/src/components/vendor/InventoryLogModal.vue \
        frontend/src/stores/inventoryLogStore.js \
        frontend/src/components/vendor/ReceiptModal.vue frontend/src/views/VendorView.vue
git commit -m "$(cat <<'EOF'
feat: :lipstick: 摊主端赠送/报废登记，渠道改成可输入的下拉

渠道下拉的列表来自后端（预置三个 + 历史用过的），摊主优先从已有的里挑、
确实要新渠道才现填——这才是真正防「微信」和「微信支付」分裂成两个账户的
那一条，光有规范化和长度上限挡不住。

赠送的「这笔我自掏」默认关，下面写清楚不开就是货主自己承担、结算单上只会
显示送了几件。不发明分摊规则，但也不让代卖社团的货被静默送掉。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: 前端 —— 退货弹窗

**Files:**
- Create: `frontend/src/utils/refund.js`
- Create: `frontend/src/utils/refund.spec.js`
- Create: `frontend/src/components/vendor/RefundModal.vue`
- Modify: `frontend/src/views/VendorView.vue`

**Interfaces:**
- Produces: `splitRefund(total, lines)` —— 纯函数，把实退总额按各行 `remaining_paid` 摊回，
  用于**界面上实时显示每行退多少**。权威计算仍在后端。
- Consumes: Task 4 的两个端点。

- [ ] **Step 1: 写纯函数的失败测试**

`frontend/src/utils/refund.spec.js`：

```js
import { describe, it, expect } from 'vitest'
import { splitRefund, defaultRefundTotal } from './refund'

describe('splitRefund', () => {
  it('和精确等于总额', () => {
    // 后端用的是「向下取整 + 余数逐分派发」（domain/allocation.rs 的 apportion）。
    // 前端这份只用于**显示**，但显示的数加起来对不上总额一样会让摊主不信任。
    const out = splitRefund(1000, [334, 333, 333])
    expect(out.reduce((a, b) => a + b, 0)).toBe(1000)
  })

  it('每一份都不超过该行实付', () => {
    const out = splitRefund(1000, [200, 900])
    expect(out[0]).toBeLessThanOrEqual(200)
    expect(out[1]).toBeLessThanOrEqual(900)
    expect(out.reduce((a, b) => a + b, 0)).toBe(1000)
  })

  it('权重全零时给全零而不是 NaN', () => {
    // 0 元 SKU（预售取货）。除以零会让界面显示一排 NaN。
    expect(splitRefund(0, [0, 0])).toEqual([0, 0])
  })

  it('总额为零时给全零', () => {
    expect(splitRefund(0, [500, 500])).toEqual([0, 0])
  })

  it('空输入不炸', () => {
    expect(splitRefund(0, [])).toEqual([])
  })
})

describe('defaultRefundTotal', () => {
  it('默认退款等于所选各行实付之和', () => {
    const lines = [
      { order_line_id: 1, remaining_qty: 2, remaining_paid: 1000 },
      { order_line_id: 2, remaining_qty: 1, remaining_paid: 600 },
    ]
    // 第一行退 1 件（一半），第二行不退
    expect(defaultRefundTotal(lines, { 1: 1, 2: 0 })).toBe(500)
  })

  it('一件不退时是 0', () => {
    const lines = [{ order_line_id: 1, remaining_qty: 2, remaining_paid: 1000 }]
    expect(defaultRefundTotal(lines, { 1: 0 })).toBe(0)
  })
})
```

- [ ] **Step 2: 跑测试确认失败**

```bash
npm --prefix frontend run test:unit -- refund
```
Expected: FAIL，模块不存在。

- [ ] **Step 3: 实现纯函数**

```js
/**
 * 退款金额的摊分。**这一份只用于界面实时显示，权威计算在后端**
 * （`src-tauri/src/domain/allocation.rs` 的 `apportion`）。
 *
 * 两边必须是同一套取整规则，否则摊主会看到「界面上写退 33，提交完变成 34」。
 * 规则：各自向下取整，余数按「权重降序、下标升序」逐分派发，跳过已顶到上限的那一份。
 */
export function splitRefund(total, weights) {
  const n = weights.length
  const out = new Array(n).fill(0)
  if (n === 0 || total <= 0) return out
  const sum = weights.reduce((a, b) => a + b, 0)
  if (sum <= 0) return out // 0 元行：除以零会让界面显示一排 NaN

  let assigned = 0
  for (let i = 0; i < n; i++) {
    out[i] = Math.floor((total * weights[i]) / sum)
    assigned += out[i]
  }

  const order = weights
    .map((w, i) => [w, i])
    .sort((a, b) => b[0] - a[0] || a[1] - b[1])
    .map(([, i]) => i)

  let rest = total - assigned
  while (rest > 0) {
    let moved = false
    for (const i of order) {
      if (rest === 0) break
      if (out[i] < weights[i]) {
        out[i] += 1
        rest -= 1
        moved = true
      }
    }
    if (!moved) break // 全部顶到上限，理论上不可达（total ≤ Σweights）
  }
  return out
}

/** 所选各行按件数等比切出的实付之和——「实际退款」输入框的默认值。 */
export function defaultRefundTotal(lines, qtyByLine) {
  return lines.reduce((sum, l) => {
    const q = Number(qtyByLine[l.order_line_id] || 0)
    if (q <= 0 || l.remaining_qty <= 0) return sum
    return sum + Math.floor((l.remaining_paid * q) / l.remaining_qty)
  }, 0)
}
```

- [ ] **Step 4: 跑测试确认通过**

```bash
npm --prefix frontend run test:unit -- refund
```
Expected: 七条全 PASS。

- [ ] **Step 5: 写 `RefundModal.vue`**

数据来自 `GET /events/:id/orders/:oid/refunds`（一次请求同时给历史和可退行）。

界面要点：

- **逐行列出，不合并。** 同一个商品可能出现多行，行标题带上 `lot_name`
  （有套装时显示「本子A（任选3本100）」，没有就只显示商品名）。**不要按
  `event_product_id` 去重**——②-2 交接契约第 3 条。
- 每行一个数量步进器，上限 `remaining_qty`；右侧小字显示该行 `remaining_paid` 的换算值。
- 去向：每行一组两个 radio「回现场仓（还能卖）」/「进损耗（已损坏）」，默认前者。
- 「退款渠道」用 `<ChannelSelect>`，**默认值取订单的 `channel`**，并在下方写一行小字
  「默认与收款渠道相同；现金退就选现金，否则收摊清点会对不上」。
- 「实际退款」输入框默认 `defaultRefundTotal(...)`，摊主改动后用 `splitRefund` 实时
  显示每行分到多少。**改高于默认值要挡住并提示**「不能多于顾客实付，白送钱请走结算调整」——
  后端也会挡，前端这一层是为了不让摊主白填一遍。
- 历史区：已退过的逐条列出（时间 / 商品 / 件数 / 退款额 / 渠道 / 去向），**不提供撤销**
  （spec 第 11 节：退货没有撤销，退错了再开一张反向的单，或走结算调整）。

- [ ] **Step 6: 挂进 `VendorView.vue`**

「已完成订单」每张卡片加一个「退货」按钮。已经全额退完的单把按钮置灰
（`lines` 里 `remaining_qty` 全为 0）。

- [ ] **Step 7: 验证并提交**

```bash
npm --prefix frontend run lint
npm --prefix frontend run test:unit
npm --prefix frontend run build
```

```bash
git add frontend/src/utils/refund.js frontend/src/utils/refund.spec.js \
        frontend/src/components/vendor/RefundModal.vue frontend/src/views/VendorView.vue
git commit -m "$(cat <<'EOF'
feat: :lipstick: 摊主端退货弹窗

按订单行逐行退，不按商品合并——同一个商品因为套装归属会拆成多行，退哪一行
是摊主的决定，不是系统猜的。行标题带套装名，否则两行「本子A」看着像重复计数。

退款金额的摊分前端也算一份，只为实时显示，规则和后端的 apportion 完全一致
（向下取整 + 余数按权重降序逐分派发）。两边不一致的话摊主会看到「界面上写
退 33，提交完变成 34」。

退款渠道默认等于收款渠道但可改，下面写明现金退就选现金，否则收摊清点对不上。
不提供撤销：退错了再开一张反向的单，或走结算调整。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 12: 前端 —— 收摊向导

**Files:**
- Create: `frontend/src/stores/closingStore.js`
- Create: `frontend/src/components/vendor/ClosingWizard.vue`
- Modify: `frontend/src/views/VendorView.vue`

**Interfaces:**
- Consumes: Task 5 的四个端点。

- [ ] **Step 1: 写 store**

```js
/**
 * 收摊向导。**能不能进下一步由后端说了算**——`blockers` 非空就挡住。
 * 前端不自己判断，判据写在两处就会有一处先腐烂（②-2 把试算放后端是同一个理由）。
 */
export const useClosingStore = defineStore('closing', () => {
  const state = ref(null) // { status, pending_orders, onsite_remaining, stocktaken_at, blockers }
  const isLoading = ref(false)

  async function fetchState(eventId) { /* GET /events/:id/closing */ }
  async function stocktake(eventId, counts) { /* POST .../stocktake，成功后 fetchState */ }
  async function takeback(eventId) { /* POST .../takeback，成功后 fetchState */ }
  async function settle(eventId) { /* POST .../settle，成功后 fetchState */ }
  return { state, isLoading, fetchState, stocktake, takeback, settle }
})
```

每个 action 成功后**重新拉一次 `fetchState`**，不要本地改 state——四步之间没有会话状态，
真实状态在后端（spec 1.6）。

- [ ] **Step 2: 写 `ClosingWizard.vue`**

一个全屏弹窗，四屏。当前在第几屏由 `state` 推出来，**不存本地 step 变量**：

```js
// 进度完全由后端状态推出来。存一个本地 step 变量，就会出现
// 「退出重进回到第一步、但货已经带回了」这种错位。
const step = computed(() => {
  const s = store.state
  if (!s) return 1
  if (s.status === '已结算') return 4
  if (s.pending_orders.length > 0) return 1
  if (!s.stocktaken_at) return 2
  if (s.onsite_remaining.length > 0) return 3
  return 4
})
```

**第 ① 屏 清 pending**：列出每张待处理单（时间 / 件数 / 金额）。每张两个动作：

- 「完成」→ 弹 `ReceiptModal`（复用现成的）补录渠道
- 「取消」→ `PUT .../status` 到 `cancelled`

底部「全部取消」按钮：**前端循环调用现有端点，不新开批量端点**。每单独立事务——
一单因为并发失败，不该把已经取消成功的那几单一起回滚。界面按单显示结果：

```js
async function cancelAll() {
  const failed = []
  for (const o of store.state.pending_orders) {
    try {
      await api.put(`/events/${eventId}/orders/${o.id}/status`, { status: 'cancelled' })
    } catch (err) {
      failed.push({ id: o.id, msg: err.response?.data?.error || '取消失败' })
    }
  }
  await store.fetchState(eventId)
  if (failed.length) {
    // 失败的留在列表里，逐条把后端那句话显示出来
    alertStore.error(failed.map((f) => `#${f.id}：${f.msg}`).join('\n'))
  }
}
```

**第 ② 屏 盘点**：列出 `onsite_remaining`，每行「账面 N」+ 一个数量输入框（**预填账面数**）。
下方两个按钮：「提交盘点」（提交全量，包括没改的）和「跳过盘点」（直接进第 ③ 屏）。
跳过时显示一行小字：「跳过之后结算单上会写『未盘点，剩余数为账面推算』」。

**第 ③ 屏 带回**：显示将要带回的清单与总件数，一个「确认带回」按钮。

**第 ④ 屏 完成结算**：显示 `blockers`（应为空），一个「结束展会」按钮，
按下去之后提示「账本已冻结。之后仍然可以补垫付、结算调整和收摊清点，
其余都改不了了」——**把 spec 偏离 3 那三个例外说给用户听**，否则摊主会以为什么都不能改了。

- [ ] **Step 3: 挂进 `VendorView.vue`**

头部一个「收摊」按钮，展会状态是「已结算」时改成「查看结算」并跳到管理端结算页。

- [ ] **Step 4: 验证并提交**

```bash
npm --prefix frontend run lint
npm --prefix frontend run test:unit
npm --prefix frontend run build
```

```bash
git add frontend/src/stores/closingStore.js \
        frontend/src/components/vendor/ClosingWizard.vue frontend/src/views/VendorView.vue
git commit -m "$(cat <<'EOF'
feat: :lipstick: 摊主端收摊向导

在第几步完全由后端状态推出来，不存本地 step 变量——存了就会出现「退出重进
回到第一步、但货已经带回了」这种错位。能不能进下一步也由后端的 blockers
说了算，前端不自己判断。

「全部取消」走前端循环调现有端点而不是新开批量端点：每单取消本来就该是独立
事务，一单因为并发失败不该把已经取消成功的那几单一起回滚。失败的留在列表里，
逐条把后端那句话显示出来。

最后一步的提示要把三个例外说给用户听（冻结后仍可补垫付、结算调整、收摊清点），
否则摊主会以为什么都不能改了。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 13: 前端 —— 管理端结算页（垫付 / 调整 / 清点）

**Files:**
- Create: `frontend/src/stores/settlementStore.js`
- Create: `frontend/src/views/AdminEventSettlement.vue`
- Modify: `frontend/src/router/index.js`
- Modify: `frontend/src/views/AdminLayout.vue`

- [ ] **Step 1: 路由与入口**

`router/index.js` 在 admin 子路由里加：

```js
      {
        path: 'events/:id/settlement',
        name: 'admin-event-settlement',
        component: () => import('@/views/AdminEventSettlement.vue'),
        props: true,
      },
```

`AdminLayout.vue` 的展会子菜单里加一个「结算」入口，位置排在「统计」之后。

- [ ] **Step 2: 写 store**

`settlementStore.js`：`report` / `advances` / `adjustments` 三个 `ref`，
action 覆盖 Task 6、8 的全部端点。

- [ ] **Step 3: 页面的垫付与调整两块**

**垫付**：表格（社团 / 名目 / 金额 / 删除）+ 一个新增表单（社团下拉、名目、金额）。

**结算调整**：同形，但金额那一列要显示方向。新增表单**不给正负号输入**，给两个按钮：

```
[ 我要多给他们 ]  [ 他们要多给我 ]     金额 [____] 元    名目 [__________]
```

选中哪个决定提交时的 `direction`。表格里正数显示成「我多给 ¥20」、负数显示成
「他们多给 ¥20」，**不显示原始符号**。

> 两块都要在页面上写一行小字：**「展会结算之后，这两项仍然可以增删」**。
> 这是 spec 偏离 3 的用户可见部分，不说的话摊主根本不会想到回来补。

- [ ] **Step 4: 页面的收摊清点一块**

表格：渠道 / 账面应有 / 实际到手（输入框，预填当前 `actual`）/ 差额（实时算）/ 是否清点。
一个「提交清点」按钮提交全部行。

差额非零时显示一行说明：**「差额由摊主自己承担，不进任何货主的结算。要推给某个货主，
请到上面加一条结算调整。」**——母 spec 第 5 节的原话，摊主看到短款第一反应就是这个问题。

- [ ] **Step 5: 验证并提交**

```bash
npm --prefix frontend run lint
npm --prefix frontend run test:unit
npm --prefix frontend run build
```

```bash
git add frontend/src/stores/settlementStore.js frontend/src/views/AdminEventSettlement.vue \
        frontend/src/router/index.js frontend/src/views/AdminLayout.vue
git commit -m "$(cat <<'EOF'
feat: :lipstick: 管理端结算页：垫付、结算调整、收摊清点

结算调整不给摊主填正负号，给「我要多给他们 / 他们要多给我」两个按钮，
表格里也按方向显示而不是显示原始符号。母 spec 那个例子自己都要算一遍才
对得上方向，何况是收摊后累了一天的人。

两块都写明「展会结算之后仍然可以增删」——这是冻结例外的用户可见部分，
不说的话摊主根本不会想到回来补。

清点出现差额时直接把母 spec 第 5 节那句话摆出来：差额摊主自吞，要推给某个
货主请加一条结算调整。摊主看到短款的第一反应就是这个问题。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 14: 前端 —— 结算单与导出

**Files:**
- Modify: `frontend/src/views/AdminEventSettlement.vue`

- [ ] **Step 1: 结算单主体**

页面顶部渲染 `GET /events/:id/settlement` 的结果，版式照母 spec 第 7 节：

```
━━ 展会结算 ━━  ABC漫展 · 2026-10-01

┌─ 货主：星见社（本社团）────────────────
│ 【货】带去 90 → 卖出 61 / 赠送 4 / 报废 1 / 带回 24   盘点差异 0
│ 【钱】商品原价 1,830  Lot折让 -120  手工折让 -60 → 净额 1,650
│ 【我垫付】摊位费 400 + 打印费 80 = 480
│ 【调整】（无）
│ ───────────────────────────────
│ 我应转给星见社：1,170
└─────────────────────────────────
```

要点：

- 【货】那一行**按商品可展开**，收起时显示该货主的合计。
- `report.stocktaken` 为 false 时，【货】那一行末尾加一个显眼的
  「**未盘点，剩余数为账面推算**」标记。
- 「我应转给」是这一块最大的数字，视觉上要压过其它行。
- 底部显示 `generated_at` 与 `last_changed_at`，并在两者不同时提示
  「账本在 xx 之后还有变动，导出前请刷新」。

- [ ] **Step 2: warnings 必须显眼**

```
report.warnings 非空时，在整张结算单**最上方**放一个红色警告块，逐条列出。
```

**不要把它折叠、不要放到页面底部。** 这些是「业务表加出来的数和账本对不上」，
意味着某笔账记错了；安安静静地显示一个错数，比显示不出来更坏。

- [ ] **Step 3: 导出按钮**

一个「导出 Excel」按钮，直接打开 `GET /events/:id/settlement.xlsx`。

Tauri 环境下 `window.open` 不一定能触发下载——**照 `AdminEventStat.vue` 现有导出
按钮的做法来**（`grep -n "download" frontend/src/views/AdminEventStat.vue`），
那条路径是验过的。不要另发明一套。

- [ ] **Step 4: 验证并提交**

```bash
npm --prefix frontend run lint
npm --prefix frontend run test:unit
npm --prefix frontend run build
```

```bash
git add frontend/src/views/AdminEventSettlement.vue
git commit -m "$(cat <<'EOF'
feat: :lipstick: 结算单页面与 Excel 导出

版式照母 spec 第 7 节：每个货主一块，【货】【钱】【垫付】【调整】，
最后是「我应转给」——那是这一块唯一要被记住的数字，视觉上压过其它行。

warnings 非空时在整张单最上方放红色警告块，不折叠、不挪到底部。那些是
「业务表加出来的数和账本对不上」，安安静静地显示一个错数比显示不出来更坏。

未盘点时在【货】那一行末尾标出来，导出的 xlsx 里也有同样的标记。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 15: 文档、②-2 的三条遗留测试、交接段

**Files:**
- Modify: `src-tauri/src/api/order.rs`、`src-tauri/src/api/stats.rs`、`src-tauri/src/api/lot.rs`（各补一条测试）
- Modify: `docs/guide/workflow.md`、`docs/guide/export.md`、`frontend/src/config/helpContent.js`
- Modify: `docs/superpowers/specs/2026-09-22-v1.2-roadmap.md`（追加附录三）
- Modify: 本 plan（追加「执行后记」）

- [ ] **Step 1: 补 ②-2 留下的第 1 条——全代卖订单加价的镜像场景**

②-2 只测了折让方向。加价方向的腿是反着记的（`社团往来:本社团 → 实收-<渠道>`），
写反了照样平账。加到 `api/order.rs` 的 `mod tests`：

```rust
    #[tokio::test]
    async fn marking_up_an_all_consignment_order_charges_the_home_society() {
        // ②-2 deferred #1。折让方向有测试，加价方向没有。
        // 加价时那条腿是反的（社团往来:本社团 → 实收），写反了照样平账，
        // 只有盯着本社团余额的符号才看得出。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, ep_b) = seed_event_and_product(&pool).await;
        let order_id = place(&router, event_id, json!([{"product_id": ep_b, "quantity": 1}])).await;
        let token = admin_token();

        // 原价 2000，顾客给了 2500（凑整、打赏）
        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{order_id}/status"), Some(&token),
            json!({"status": "completed", "channel": "现金", "final_amount": 2500}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(2)).await.unwrap(),
            Money::from_cents(-2000),
            "代卖社团只拿原价——多的 5 块不是他们的货挣的"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::SocietyDue(1)).await.unwrap(),
            Money::from_cents(-500),
            "多出来的 5 块归本社团。写反方向的话这里会是 +500"
        );
        assert_eq!(
            account_balance(&pool, event_id, &Account::Received("现金".into())).await.unwrap(),
            Money::from_cents(2500)
        );
    }
```

- [ ] **Step 2: 补第 2 条——事件级不变量**

加到 `api/stats.rs` 的 `mod tests`（没有就照 `api/lot.rs` 的头部新建）：

```rust
    #[tokio::test]
    async fn paid_amounts_sum_to_final_amounts_across_mixed_statuses() {
        // ②-2 deferred #2。pending / completed / cancelled 三种状态混在一起时，
        // 「Σ order_lines.paid_amount == Σ orders.final_amount」必须仍然成立——
        // 仪表盘的「总额」和「按商品汇总之和」结构上相等就靠这一条。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let completed = place(&router, event_id, json!([{"product_id": ep_a, "quantity": 1}])).await;
        router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{completed}/status"), Some(&token),
            json!({"status": "completed", "channel": "现金", "final_amount": 2800}),
        )).await.unwrap();

        let cancelled = place(&router, event_id, json!([{"product_id": ep_b, "quantity": 1}])).await;
        router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/orders/{cancelled}/status"), Some(&token),
            json!({"status": "cancelled"}),
        )).await.unwrap();

        place(&router, event_id, json!([{"product_id": ep_b, "quantity": 2}])).await; // pending

        let (sum_paid, sum_final): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT COALESCE(SUM(ol.paid_amount), 0)
                     FROM order_lines ol JOIN orders o ON o.id = ol.order_id
                     WHERE o.event_id = ?1),
                    (SELECT COALESCE(SUM(final_amount), 0) FROM orders WHERE event_id = ?1)",
        ).bind(event_id).fetch_one(&pool).await.unwrap();
        assert_eq!(sum_paid, sum_final);
    }
```

- [ ] **Step 3: 补第 3 条——`/quote` 的另外两条规模上限**

②-2 只有 `MAX_UNITS` 的单元测试，`MAX_STATE_SPACE` / `MAX_STEPS` 没有 HTTP 层测试。
加到 `api/lot.rs` 的 `mod tests`：

```rust
    #[tokio::test]
    async fn quote_refuses_a_cart_that_blows_the_search_budget() {
        // ②-2 deferred #3。超限必须是可读的 400 而不是让公开未鉴权端点
        // 把一个核转满。**不要改成「预算内尽力搜」**——母 spec 第 10 节把那条路砍掉了，
        // 「优惠算得不一样」比「算不出来」更难向顾客解释。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;

        // 造一个能撑爆状态空间的购物车：多个候选集重叠的 Lot + 每种若干件。
        // 具体规模照 domain/solver.rs 顶部三个常量的当前值反推，
        // **动手前先读那三个常量**——它们是硬编码的，改过就要同步改这条测试。
        let mut items = Vec::new();
        for i in 0..40 {
            let ep = 100 + i;
            sqlx::query(
                "INSERT INTO event_products
                   (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
                 VALUES (?, 1, 1, 1, ?, ?, 1000)",
            )
            .bind(ep).bind(format!("P{i}")).bind(format!("商品{i}"))
            .execute(&pool).await.unwrap();
            items.push(json!({"product_id": ep, "quantity": 8}));
        }
        seed_lot(&pool, event_id, "任选5件", 5, 4000,
                 &(100..140).collect::<Vec<i64>>()).await;

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/quote"), None,
            json!({"items": items}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = read_json(res).await;
        assert!(
            body["error"].as_str().unwrap().contains("分单"),
            "要告诉摊主该怎么办：{body}"
        );
    }
```

> 这条测试的规模参数可能要调。**先读 `domain/solver.rs` 顶部的
> `MAX_UNITS` / `MAX_STATE_SPACE` / `MAX_STEPS`**，构造一个刚好越过
> `MAX_STATE_SPACE` 但不越过 `MAX_UNITS` 的购物车（40 × 8 = 320 > MAX_UNITS = 300，
> **所以上面这组数会先撞 MAX_UNITS，测不到想测的那条**——把件数调到总量 300 以内、
> 靠候选集重叠把状态空间撑起来）。

- [ ] **Step 4: 跑全部测试**

```bash
cd src-tauri && tauri-env linux cargo test --workspace
```
Expected: 全绿。

- [ ] **Step 5: 改文档站**

**`docs/guide/workflow.md`**：加「收摊」一节（三步 + 冻结后还能改什么），
加「退货」一节。顶部那句黑体的诚实披露**保留并加强**：

> App 里的「订单已完成」仅代表记录了一笔账，不代表钱真的到了你的账户。
> **结算单上的「我应转给 XX」是账本算出来的应转数，不是已经转过；
> 「实际到手」是你自己数出来录进去的，系统没有能力核对任何一笔到账。**

**`docs/guide/export.md`**：这个文件现在全文只有「这个页面还在建设中，敬请期待！」
（路线图的零风险清扫点名过），而侧边栏挂着入口。本轮正好把它写了：
两个导出的区别（销售汇总「卖了什么」vs 结算单「账怎么算」）、四个 sheet 各是什么、
导进社团财务系统时该看哪一列。

**`frontend/src/config/helpContent.js` 与 `Help.vue`**：加赠送/报废、退货、收摊、结算单四条。

- [ ] **Step 6: 查 FAQ 有没有和新行为矛盾的**

②-2 的教训：`docs/faq/advanced.md` 里「支持套装吗」一节和同一文件上一节自相矛盾，
五轮审查全漏，因为逐个 task 审查结构上看不到跨文件的矛盾。

```bash
grep -rn "退货\|盘点\|收摊\|结算\|赠品\|损耗" docs/faq docs/guide docs/en docs/ja | grep -v node_modules
```

逐条读，把和新行为矛盾的改掉。**中英日三份都要改**——`docs/translate_faq.py` 是一次性
批量翻译，没有持续同步机制，漏了就会长期不一致。

- [ ] **Step 7: 追加路线图附录**

在 `docs/superpowers/specs/2026-09-22-v1.2-roadmap.md` 末尾追加「附录三：②-3 执行后的状态」，
照附录二的结构写：现在能做什么 / 测试数 / 真机验过的与没验过的 / 值得记住的 / 下一步（③b）。

- [ ] **Step 8: 追加本 plan 的「执行后记」**

记下：计划本身的缺陷（执行中被撞出来的）、审查抓到的、知情留下的 deferred minor。
②-1 和 ②-2 的后记都证明这一节是下一个子项目最有用的输入。

- [ ] **Step 9: 提交**

```bash
git add -A
git commit -m "$(cat <<'EOF'
docs: :memo: ②-3 文档收口，并补上 ②-2 留下的三条测试

三条 deferred：全代卖订单**加价**的镜像场景（折让方向有测试、加价方向没有，
而加价那条腿是反着记的，写反了照样平账）、事件级「Σpaid == Σfinal」在三种
订单状态混合时仍然成立、/quote 的 MAX_STATE_SPACE 与 MAX_STEPS 的 HTTP 层测试。

docs/guide/export.md 原来全文只有「建设中，敬请期待」而侧边栏挂着入口，
本轮正好把两个导出的区别写清楚。workflow.md 的诚实披露句加强了：结算单上
「我应转给」是账本算出来的应转数不是已经转过，「实际到手」是摊主自己数出来的，
系统没有能力核对任何一笔到账。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## 完成标准

全部打勾才算 ②-3 做完。

**门禁**

- [ ] `cd src-tauri && tauri-env linux cargo fmt --check` 无输出
- [ ] `cd src-tauri && tauri-env linux cargo clippy --all-targets -- -D warnings` 通过
- [ ] `cd src-tauri && tauri-env linux cargo test --workspace` 全绿
- [ ] `python3 scripts/check-event-guards.py` 通过
- [ ] `npm --prefix frontend run lint` / `test:unit` / `build` 三条全绿
- [ ] `./scripts/set-version.sh` 报告四处版本号一致

**不变量**

- [ ] 走完一整场展会（下单 / 折让 / 退货 / 赠送 / 自掏赠品 / 报废 / 垫付 / 调整 / 盘点 /
      带回 / 结算 / 清点）之后，结算单的 `warnings` 为空
- [ ] 每个货主：`带去 − 带回 == 卖出 + 赠送 + 报废 + 差异 + 现场仓`
- [ ] 每个货主：`净额 + 退货保留 + 自掏赠品 − 垫付 + 调整 == −(社团往来余额)`
- [ ] `摊主留存 + Σ我应转给 == 实际到手合计`
- [ ] 转「已结算」之后每个商品的现场仓余额为 0
- [ ] 全部资金账户余额之和恒为 0

**冻结语义**（一条一条验，不要只验下单）

- [ ] 已结算的展会上：下单 / 改订单状态 / 加商品 / 补货 / 改商品 / 改套装 / 赠送 /
      报废 / 撤销 / 退货 / 盘点 / 带回 / 再次结算 —— **全部 409**
- [ ] 已结算的展会上：加垫付 / 删垫付 / 加结算调整 / 删结算调整 / 收摊清点 ——
      **全部成功**（spec 偏离 3）
- [ ] `grep -rn "require_event_open" src-tauri/src/api | wc -l` 的数量和路由表对得上，
      且每个豁免注释的理由都经得起问

**口径**

- [ ] `grep -rn "paid_amount" src-tauri/src/api/settlement.rs` 只出现在
      「退货保留」和「手工折让」两处推导里，**结算单的销售额一律 `allocated_amount`**
- [ ] 结算单页面显示的数字和导出 xlsx 里的数字逐项一致（同一个 `SettlementReport`）
- [ ] `grep -rn "f64" src-tauri/src/api/settlement.rs src-tauri/src/domain/settlement.rs`
      只在 xlsx 写入处命中（金额离开系统的那一刻）

**迁移**

- [ ] `git diff main --stat -- src-tauri/migrations/` 只显示**新增**一个文件，
      既有文件一行未改
- [ ] 拿一份 ②-2 时期的 dev 库启动，迁移能应用、数据还在

**发 beta 前的真机走查**（②-2 遗留 + 本轮新增，**这些 CI 覆盖不到**）

- [ ] ②-2 遗留：连点加号的防抖观感、清空「实收」点确认要提示、收款弹窗点套装名
      整行能取消勾选、改成比应收更高的数、取消一单后资金账户回到 0
- [ ] 本轮：退货弹窗里同一商品的两行能分别退、退款渠道改成和收款不同的、
      实退改到 0、收摊向导退出重进不丢进度、跳过盘点后结算单的标记
- [ ] 真 Windows：COM / WebView2 / 摄像头 / NSIS 安装卸载 / updater 签名
- [ ] 真手机：NNAPI / 局域网多设备 / 收摊向导在窄屏上的可用性

---

## 交给后续（③b / ④）的接口契约与已知不变量

**执行完之后按实际情况修订这一节**，它是 ③b 起草 spec 的必读材料。起草时的预期：

### 1. `SettlementReport` 是 ③b 生成 TS client 时最值得先动的类型

它是本轮唯一一个**结构复杂且前后端共享**的类型（嵌套三层、十几个 `Money` 字段）。
现在前端是手写的解构，字段名错了不报错、只在页面上显示 undefined。
utoipa 注解从它开始加，收益最大。

### 2. `Money` 在 JSON 里是裸整数，没有单位标记

`domain/money.rs` 的 `Serialize` 输出 `i64`（分）。TS 侧生成出来会是 `number`，
和件数、id 没有任何类型上的区别。③b 应当给它一个 branded type，
否则「把分当成元显示」这类错误 TS 也拦不住。

### 3. 冻结例外的三个端点是 API 契约的一部分，不是实现细节

垫付、结算调整、收摊清点在已结算的展会上仍然可写（spec 偏离 3）。
`api/settlement.rs` 的模块注释、四条豁免注释、`check-event-guards.py` 的白名单
机制、以及三条「已结算展会上仍然成功」的测试，**四处共同承载这一条**。
③b 加 utoipa 注解时要把它写进 OpenAPI 的描述里，否则从契约上看不出来。

### 4. `events.stocktaken_at` / `reconciled_at` 是「做过没做过」而不是「做得对不对」

两列都只记时间点。「盘点于 18:23」不声称 18:23 之后没再动过货，
「已清点」也不声称账面和实际一致（差额在 `channels[].diff`）。
④ 做 IA 时不要把它们渲染成一个绿色对勾。

### 5. 退货没有撤销，这是有意的

spec 第 11 节。退错了的补救是再开一张反向的单或走结算调整。
④ 重做订单管理页时不要顺手加一个「撤销退货」按钮——三条钱腿加顾客手里的现金，
凭空少一笔比多一笔更难查。

### 6. 留给 ④ 的界面债

- `AdminEventLots.vue` 仍然要彻底重做（②-2 附录二已列）。
- `AdminEventOrders.vue` 仍然只显示 `final_amount`，看不到原价、套装和**退货**。
  本轮又给它加了一层没显示的信息。
- 本轮新建的四个前端页面都按现有模式做、没有视觉投入，等 ④ 统一收口。
- **收摊向导在窄屏上的可用性没有专门设计**。摊主收摊时手上多半是手机，
  而盘点那一屏是个逐行输入的表格——这是 ④ 的移动端那一批里优先级最高的一处。

### 7. 求解器的三条上限仍然是硬编码常量

`MAX_UNITS = 300` / `MAX_STATE_SPACE = 200_000` / `MAX_STEPS = 2_000_000`。
本轮给后两条补了 HTTP 层测试（Task 15），**测试里的购物车规模和这三个常量耦合**，
改常量要同步改测试。仍然不要改成「预算内尽力搜」。

---

## 执行后记（2026-09-24 完成）

②-3 于 2026-09-24 执行完毕，`1.2-dev` 上 **35 个提交**。
执行方式：**dsh-flash（DeepSeek）做实现者、Claude 做审查者**，2–3 个 task 一批，七批。
Rust 测试 **133 → 226**，前端 **31 → 47**。七轮 task 审查 + 七轮定向复审 + 一轮整支分支审查（Opus），
**全程 0 Critical**，最终 0 未决。

### 计划本身的缺陷（执行中被撞出来的，全是我的，不是实现者的）

按被发现的顺序：

1. **门禁能被一句注释骗过。** `check-event-guards.py` 做的是原始文本子串匹配，
   于是 `create_gift` / `create_scrap` 靠注释里出现 `require_event_open` 就通过了——
   真正的守卫在它们共用的 `log_movement` 里。今天行为是对的，但门禁已经分不清
   「调用了守卫」和「声称别人调了」，**而它存在的全部理由就是「靠记是记不住的」**。
2. **`plan` 的示例代码有两个硬伤，我只标注了一个**：`40×8 = 320 > MAX_UNITS`（标了）、
   全部复用 `master_product_id = 1` 撞 `UNIQUE(event_id, master_product_id)`（没标）——
   **而后一个坑 ②-2 的执行后记里刚记过**。
3. **`step` 推导自相矛盾。** 写的是 `if (!s.stocktaken_at) return 2`，
   而同一份 plan 又要求「盘点可跳过」——跳过后 `stocktaken_at` 恒为 null，
   向导会永远退回盘点屏。两条需求并排写着，我没看出它们打架。
4. **`defaultRefundTotal` 比后端少一分。** 我写的是 `Math.floor`，
   而后端 `apportion` 会把余数派给权重大的那份。审查把 Rust 的 `apportion` 移植成 JS
   对拍四万组输入，找到 4834 处不匹配。**正是那个文件头注释里写着要防止的
   「界面上写退 33，提交完变成 34」**，而且 `overLimit` 会把正确值判成超限，摊主连手改都改不对。
5. **结算单正文漏了两项。** 我照母 spec §7 的版式写显示格式，而那个样例里
   `refund_kept` 和 `gift_self_paid` 恰好都是 0。**样例自洽 ≠ 版面完备。**
6. **清点输入框预填账面值**——而「收全量」这个设计的全部意义就是分开
   「我数了，一致」和「我没数这个」。**我亲手写了那条规则，又亲手在 brief 里把它抹平了。**
7. **两套相反的符号约定，我自己在 brief 里混了一次**。从 Batch 3 起我就在每份派发稿里
   警告别人别混 `settlement_adjustments.amount`（对往来余额的影响）和 `Entry.amount`
   （对「我应转给」的影响），写 task-13 的 brief 时自己混了。
8. **「三步」的 defect 清单我列了 5 处，实际 6 处**——而上一轮的扫描改的正是同一个文件的相邻行。
9. **加守卫时没查调用方。** F1 给 `PUT /events/:id/status` 加了迁移守卫之后，
   管理端「结束」「重新开始」两个按钮永远失败，而同文件的帮助文案还在教用户按它们。
10. **`reconcile` 渠道名规范化那条指令照字面实现会引入更隐蔽的 bug。** 我写「把 `used` 也过一遍
    `normalize` 再比对」，若连读余额和记腿也用规范化名，钱会记到一个新账户上、
    老账户余额被孤立——**为了修 bug 的改动制造出一个更难发现的 bug**。实现者看出来了。

### 跨模型执行的实际收益

和 ②-2 一致：**实现者撞出计划缺陷，审查者撞出实现缺陷**，两者抓的东西不重叠。

dsh-flash 作为实现者抓到的是上面第 2、3、7、10 条——都是「照你写的做会出问题」，
而且它**每次都是报上来而不是默默改掉或硬抄**。第 10 条尤其：它比指令多做一步，
并在报告里说清为什么必须多做那一步。

Claude 作为审查者抓到的则是另一类：
- 把 Rust 的 `apportion` 移植成 JS **fuzz 四万组**去对拍前端实现（第 4 条）
- 手算恒等式的每个分支、手算母 spec §7 样例的每个数字，而不是信测试通过
- 手工推演变异体（「把 `delete_adjustment` 的表名换成 `advances`，测试照样全绿」）
- 反向验证修复本身会不会引入新问题（放宽清点白名单后有没有路径能造出新渠道账户）

### 审查抓到的东西有个清晰的模式

**账务算术一次都没错过。** 三条退货腿、两条恒等式、两套符号约定、两个相反的腿方向——
审查每次都手算重验，每次都对。

**错的全是「执行机制」**：门禁能被注释骗过、列表端点从没在有数据时跑过、
重复 id 绕过按行检查、四个 handler 靠一个手打字符串区分却只测了一个、
一个测试名承诺四个 sheet 却只检查了 zip 魔数。

**plan 写得越细，实现越不容易在逻辑上出错；但 plan 写不出「这个测试其实什么都没验」。**

### 逐批审查结构上看不见的东西

整支分支审查抓到的三条 Important，**没有一条是单批审查有可能发现的**：

- 管理端两个按钮在 F1 之后永远失败（守卫在 Batch 2、按钮在更早的既有代码里）
- 「查看结算」对唯一看得到它的角色不可用（跳转在 Batch 5、路由角色守卫是既有的）
- **同一条「收全量」原则在两个组件间被不一致地应用**（清点在 Batch 4、盘点在 Batch 5）

最后一条是最值得记的：**逐批审查无法发现一条原则被不一致地应用**，因为每一批各自都是对的。

### 流程上的两件事

- **有一次 worker 空跑**：退出码 0、工作树干净、零提交、输出 92 字节停在半句话。
  只看退出码会以为成功了，而那两组修复会静默消失。**每批回来核一遍 `git log` 和工作树**
  不是形式主义。
- **修复轮上限从 5 降到 2**（用户明确要求避开「小错-修复-审阅」的循环），实际七批里
  **每批都是一轮修复就过**，没有一批用到第二轮。把审查发现一次性打包成一个派发、
  并把 Minor 里够便宜的一并塞进去，比逐条来回省得多。

### 知情留下的（均不阻断合并）

| # | 事项 | 位置 |
|---|---|---|
| 1 | `reconciled_at` 是全局标志，清点之后才出现的渠道会被报成已清点 | 要新表/新列，spec 8.3 已订正说明 |
| 2 | `check-event-guards.py` 是棘轮不是证明：只认 token 出现（写在 `if` 分支里也算过）、只扫 `api/**`、不认 `on(MethodFilter…)` / `any()` / `route_service`、**不检查守卫是否与写入同事务** | `scripts/check-event-guards.py` |
| 3 | `societies.is_home` 可在结算后翻转（**是响的**——新旧两个本社团都会触发断言） | `api/society.rs` |
| 4 | `GET /api/channels` 跨展会可见（设计如此，对事件级 vendor token 是个小泄露） | `api/settlement.rs` |
| 5 | `normalize_label` 与 `domain::channel::normalize` 重复，第三个自由文本字段出现时再抽 | `api/settlement.rs` |
| 6 | `GET /settlement` 在没有本社团时返回 409（对 GET 不自然，但文案可操作） | `api/settlement.rs` |
| 7 | store 的读路径把后端错误文案吞成通用串（沿袭自 `lotStore.js` 的全仓模式；写路径都已透传） | `frontend/src/stores/*` |
| 8 | 0 元商品勾「摊主自掏」时标签与开关不符（经济上无操作） | `api/inventory.rs` |
| 9 | `docs/en\|ja` 的 `workflow.md` / `export.md` 是 0 字节空壳而侧边栏有入口（先于本轮存在，本轮让中文单语的差距变大） | `docs/` |
