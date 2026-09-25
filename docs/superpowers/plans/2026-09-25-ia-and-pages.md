# ④-2 信息架构与页面重写 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 管理端改为按生命周期分组的展会工作台 + 设置页，摊主端改为订单 / 库存 / 收摊三 tab，加摊主↔顾客切换、批量选品与从上一场导入，全部页面用 ④-1 原语重写到目标宽度。

**Architecture:** 嵌套路由：`AdminEventWorkbench` 与 `VendorShell` 是外壳（页头、状态条、tab、轮询），子页用 `PageShell embedded`。
后端只加两样：`POST /events/{id}/products/import`（单事务）与订单的 `refunded_*` 聚合字段，无迁移。
**串行骨架（dsh-flash ×3，合审一次）→ 5 个 dsh-flash worker 并行重写页面 → 收口（Claude）→ Opus 终审直接修。**

**Tech Stack:** Vue 3.5 / TS strict / Naive UI 2.44.1（钉死，别升）/ Pinia / vue-router 4 / vitest + @vue/test-utils / Axum + sqlx + utoipa / Playwright（仅 controller 截图）

**Spec:** `docs/superpowers/specs/2026-09-25-ia-and-pages-design.md`

**前一份 plan:** `docs/superpowers/plans/2026-09-25-ui-foundation.md`（worker brief 模板见其附录 A，派发与审查节奏沿用）

---

## 这份计划怎么读

粒度刻意放粗（用户要求：计划不要太琐碎）。每个 task 写清**文件、接口契约、必测用例、验收命令**，实现细节交给执行者。

| Task | 内容 | 执行者 |
|---|---|---|
| 0 | 基线截图 | Claude（controller） |
| 1 | 后端：import 端点 + `refunded_*` 字段 + 契约再生成 | dsh-flash |
| 2 | 管理端骨架：路由、`PageShell embedded`、工作台外壳、侧栏、设置页 | dsh-flash |
| 3 | 摊主端骨架：`VendorShell` 三 tab、收摊页化、`SettlementReportView`、切换按钮 | dsh-flash |
| — | **合审 A**（Task 1–3，一个 Claude 审查者）→ 一次打包修复 | Claude / dsh-flash |
| 4 | 批次 P：5 个 worker 并行重写页面 | dsh-flash × 5 |
| — | **合审 B**（2 个审查者，各看 2–3 个 worker）→ 一次打包修复 | Claude / dsh-flash |
| 5 | 收口：门禁新规则、删除确认色、对照截图、文档、执行后记 | Claude |
| 6 | 终审：Opus 整支分支审查并直接修 | Claude subagent |

Task 1–3 串行（都改 `router/index.ts` 或 `schema.d.ts`，并行必冲突）。Task 4 的 5 个 worker 文件互不相交，用 `./scripts/worktree-new.sh`。
修复轮上限一轮，之后 controller 直接改；Minor 不发回。

---

## Global Constraints

每个 task、每个 worker brief 都隐含包含本节。

- **视觉方向不动**：`theme.ts` 颜色值不改；所有样式值来自 token，stylelint 与 `check-ui-boundary.mjs` 门禁必须全绿。豁免只能行内写并附理由。
- **现有 URL 全部保留**（`/admin/events/:id/{products,lots,orders,stats,settlement}`、`/vendor/:id`、`/events/:id/order`）。
- **不新增迁移**，不改任何已有迁移文件。后端改动只限 Task 1。
- **顾客端：平板美观、手机能用。摊主端：手机为主设计目标。管理端：桌面为主，平板能用，手机无横向页面滚动。**
  「能用」= 无横向页面滚动、可点区域 ≥ 44px、按钮不被遮挡。
- **断点**只用 `@media (--phone)`（≤640）/ `(--tablet)`（≤1024）/ `(--not-phone)` / `(--desktop)`；JS 里用 `useViewport()`（`isPhone` / `isTablet`）。
- **页宽**：`PageShell width="narrow|content|wide|full"`（640 / 960 / 1280 / 不限）。
- 反馈一律 `useFeedback()`；弹窗 `AppModal`；加载/错误/空态 `AsyncState` + `EmptyState`。**页面不手写错误态。**
- **删除确认一律 `danger: true`**；作废、冲正类可逆操作用 `type: 'warning'`。
- 数据列表用 `n-data-table`；原生 `<table>` 只允许在 `components/settlement/**`（结算单报表）。
- 金额是 `Cents`：显示用 `formatYuan` / `<Money>`，元 → 分只用 `toCents()`；**不写 `as Cents`**；全仓 `any` 为零。
- 盘点、清点、导入库存**不预填**（「收全量」原则）。
- **前端在 `frontend/`**：`npm --prefix frontend run lint|format:check|test:unit|typecheck|build`。
- **后端**：`cd src-tauri && tauri-env linux cargo …`（裸 `cargo` 缺 webkit2gtk 必失败）。
  Rust 门禁：`cargo fmt --all --check`、`cargo clippy --all-targets --all-features -- -D warnings`、
  `cargo clippy --no-default-features -- -D warnings`、`python3 scripts/check-event-guards.py`（仓库根）、`cargo test --all-features`。
- 改了后端接口：`(cd src-tauri && UPDATE_OPENAPI=1 tauri-env linux cargo test --all-features openapi_snapshot) && npm --prefix frontend run gen:api`。
- **不要起 dev server**：`vite`、`npm run dev|preview`、`tauri dev`、不带 `run` 的 `vitest` 永不退出，会卡死 headless worker。
- 提交信息 gitmoji 中文，末尾 `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`。
- **门禁与提交用 `&&` 串**，不用 `;`。

## 相对 spec 的实现层修正（写 plan 时核实代码得出）

- spec §4.1 写的 422：项目 `ApiError` 只有 `BadRequest` / `NotFound` / `Conflict` / `Db` 等，**没有 422**。映射失败、源套装非法用 **400 `BadRequest`**，重复上架用 **409 `Conflict`**，与现有 `create_lot` / `add_product_to_event` 一致。
- spec §5.2 写「`OrderRow` 加 `refunded_amount`」：`OrderRow` 是 `SELECT *` 的 `FromRow`，加列会炸。改为：
  `OrderItemResponse` 加 `refunded_qty` 与 `refunded_amount`（子查询聚合 `refunds`），`OrderResponse`（`OrderRow` 摊平 + items + lots）
  加 `refunded_amount` = 其 items 之和，在 Rust 里求和。**线上 JSON 形状与 spec 一致**（订单顶层有 `refunded_amount`）。
- 「上一场进货 N」直接用源展会 `GET /events/{id}/products` 已有的 `stocked_qty`，不加端点。

---

## Review Focus

spec 没逐条点名、但最可能咬到真实用户的五类情况，已指派到 task 的测试：

1. **导入时套装的候选商品在目标展会里已经有了**（不在本次请求里）→ 期望：映射到已有的那个 `event_product`，套装照常建出。→ Task 1 `import_maps_lot_to_existing_target_product`。
2. **摊主停在收摊 / 库存 tab 时来了新单** → 期望：照样响提示音、订单 tab 角标 +1；切回订单 tab 不重复响。→ Task 3 `VendorShell` 测试。
3. **工作台打开一个不存在 / 已删除的展会 id** → 期望：`AsyncState` 错误态 + 返回展会列表，不是永远「加载中...」、也不是重定向死循环。→ Task 2 `WorkbenchIndex` 测试。
4. **商品库现价为空（`default_price` 为 null）或与上一场相同** → 期望：不显示价格二选一，直接用上一场售价；不出现 `NaN` / `¥0.00`。→ Task 4 P1 `importPlan.spec.ts`。
5. **导入请求部分失败**（第 3 个套装映射不上）→ 期望：整批回滚，抽屉不关、显示后端错误原文、已勾选状态保留可改后重试。→ Task 1 `import_rolls_back_all_on_lot_failure` + Task 4 P1 抽屉组件测试。

---

## Task 0: 基线截图（controller，不进任何 brief）

- [ ] 照 memory「截图对照」的做法：VNC 上后台 `tauri-env vnc npx tauri dev` 起后端（5140），`frontend` 里 `npm run build && npx vite preview --port 4173 --strictPort`，
  Playwright 脚本沿用 ④-1 的 `$SCRATCH/shots/shoot.mjs`（本会话 scratchpad 里没有就按 ④-1 plan Task 1 Step 1 重写）。
- [ ] 视口 **390×844 / 820×1180 / 1440×900**，亮暗两套；路由：`/`、`/admin/events`、`/admin/master-products`、`/admin/societies`、`/admin`（控制台）、`/admin/about`、
  `/admin/events/:id/{products,lots,orders,stats,settlement}`、`/vendor`、`/vendor/:id`、`/events/:id/order`。存 `$SCRATCH/shots/before-4-2/`。
- [ ] 按端口取 PID 停掉 preview；后端可留给 Task 5。

---

## Task 1: 后端——import 端点 + 订单退货聚合

**Files:**
- Modify: `src-tauri/src/api/product.rs`（抽共享函数 + 新 handler + 测试）
- Modify: `src-tauri/src/api/lot.rs`（把建套装的核心逻辑抽成 `pub(crate)` 函数供 import 复用）
- Modify: `src-tauri/src/api/order.rs`（items 查询加两列、`OrderResponse` 加字段、测试）
- Modify: `src-tauri/openapi.json`（快照）、`frontend/src/api/schema.d.ts`（`gen:api` 生成）
- 可能需要：`scripts/check-event-guards.py` 认得新写操作（跑一下，红了就按它的提示补）

**Interfaces — Produces:**

```rust
// product.rs —— add_product_to_event 与 import 共用，保证「建商品 + 首批进货」只有一份实现
pub(crate) async fn insert_event_product(
    conn: &mut SqliteConnection, event_id: i64, master_product_id: i64,
    unit_price: i64, initial_stock: i64,
) -> ApiResult<i64>;   // 查 master、查重（重复 → Conflict，信息含商品名）、INSERT、initial_stock>0 时 post_journal(Restock, "开场进货")

// lot.rs —— create_lot 与 import 共用
pub(crate) async fn insert_lot(
    conn: &mut SqliteConnection, event_id: i64, name: &str, pick_count: i64,
    total_price: i64, allow_repeat: bool, candidate_ids: &[i64],
) -> ApiResult<LotResponse>;   // validate_numbers + validate_candidates + validate_candidate_count + INSERT + write_candidates
```

```jsonc
// POST /api/events/{event_id}/products/import   （tag: products，admin 或本场写权限：check_write_permission）
{ "products": [{ "master_product_id": 12, "unit_price": 3000, "initial_stock": 20 }],
  "lots":     [{ "source_lot_id": 7 }] }                 // lots 可省略（serde default）
// 200 → { "products": [ProductEventProduct...], "lots": [LotResponse...] }
```

- 一个 `BEGIN IMMEDIATE` 事务：`require_event_open` → 逐个 `insert_event_product` → 逐个套装：
  读源套装（`lots.event_id != event_id`，否则 400「不能从本场导入」；不存在 → 404）→ 候选 `event_product` 的 `master_product_id`
  → 目标展会同 `master_product_id` 的 `event_product.id`（找不到 → 400「套装「X」的候选商品「Y」不在本场」）→ `insert_lot` 沿用源 `name/pick_count/total_price/allow_repeat`。
- `products` 与 `lots` 都为空 → 400。`unit_price < 0` / `initial_stock < 0` → 400。
- 任一失败整批回滚。事务提交后用现有 `load_one` 取响应。
- OpenAPI 描述写明：单事务、重复上架 409 不跳过、已结算 409。

**订单：**
- 两条 items SQL（按单 / 按展会）各加
  `(SELECT COALESCE(SUM(r.qty),0) FROM refunds r WHERE r.order_line_id = ol.id) AS refunded_qty`、
  `(SELECT COALESCE(SUM(r.refund_amount),0) FROM refunds r WHERE r.order_line_id = ol.id) AS refunded_amount`。
- `OrderItemResponse` 加 `refunded_qty: i64`、`refunded_amount: i64`（`Money`）；`OrderResponse` 加 `refunded_amount`（`Money`）= items 之和。
- 所有返回 `OrderResponse` 的路径（创建、列表、状态更新、`load_order_response`）都带上。

**必测（`#[tokio::test]`，用 `test_support::{test_router_with, seed_event_and_product, admin_token, json_request, read_json}`）：**
- `import_products_and_lot_from_previous_event`：源展会 2 商品 + 1 套装 → 目标导入 → 商品各自单价与 `onsite_qty` 正确、套装候选是**目标**的 event_product id、`allow_repeat` 保留。
- `import_maps_lot_to_existing_target_product`（Review Focus 1）：目标已有商品 A，请求只带 B + 套装{A,B} → 成功。
- `import_rolls_back_all_on_lot_failure`（Review Focus 5）：套装候选有一件不在本场也不在请求里 → 400，且**目标展会商品数不变**。
- `import_duplicate_product_conflicts`：请求里的商品已在本场 → 409，信息含商品名，无任何写入。
- `import_rejected_when_event_settled`；`import_rejects_lot_from_same_event`；`import_zero_stock_writes_no_journal`（`journals` 行数不变）。
- `add_product_*` 现有测试不改仍全绿（证明抽函数没改行为）。
- `order_response_carries_refunded_totals`：一单两行，退第一行 1 件 → 该行 `refunded_qty=1`、`refunded_amount` = 退款额，订单 `refunded_amount` 相等，另一行为 0。
- 形状快照（`shape_of`）更新：import 端点新增一条；订单相关快照随字段更新。

- [ ] 实现 + 测试 → Rust 五条门禁 → `UPDATE_OPENAPI` 快照 → `gen:api` → `npm --prefix frontend run typecheck`（订单字段只增不改，前端应无报错）→ 提交。

---

## Task 2: 管理端骨架

**Files:**
- Modify: `frontend/src/router/index.ts`、`frontend/src/views/AdminLayout.vue`、`frontend/src/components/ui/PageShell.vue`（+ `ui.spec.ts`）
- Create: `frontend/src/views/AdminEventWorkbench.vue`、`frontend/src/views/WorkbenchIndex.vue`、`frontend/src/composables/useWorkbenchEvent.ts`
- Create: `frontend/src/views/AdminSettings.vue`、`frontend/src/components/settings/{LanSettings,SecuritySettings,AppearanceSettings,LegacyDataSettings,AboutSettings}.vue`
- Delete: `views/AdminControlPanel.vue`、`views/About.vue`、`views/ThemeSetting.vue`
- Modify（仅改页头为 embedded，不重写内容）：`views/AdminEvent{Products,Lots,Orders,Stat,Settlement}.vue`
- Modify: `views/Help.vue`（「控制台」「关于」的文字改为「设置」）
- Test: `frontend/src/router/router.spec.ts`、`frontend/src/views/workbench.spec.ts`

**Interfaces — Produces:**

```ts
// PageShell 新增
defineProps<{ title?: string; /* 原有 */ embedded?: boolean }>()
// embedded=true：不渲染标题行 / 副标题 / actions 区，只保留页宽容器与默认插槽；title 变为可选

// useWorkbenchEvent.ts —— 外壳 provide、子页 inject
export const WORKBENCH_EVENT: InjectionKey<WorkbenchEventContext>
export interface WorkbenchEventContext {
  event: Ref<Schemas['EventResponse'] | null>
  loading: Ref<boolean>
  error: Ref<string | null>
  reload: () => Promise<void>
}
export function useWorkbenchEvent(): WorkbenchEventContext   // 不在工作台内调用时抛错
```

路由名（后续 task 与测试都按这个写）：`admin-events`、`admin-event-workbench`、`admin-event-products`、`admin-event-lots`、
`admin-event-orders`、`admin-event-stats`（原 `AdminEventStats` 改名，全仓 grep 替换）、`admin-event-settlement`、
`admin-master-products`、`admin-societies`、`admin-settings`、`admin-help`。`/admin` → `admin-events`，`/admin/about` → `admin-settings` + `hash: '#about'`。

**要点（细节见 spec §3.1–3.4）：**
- `WorkbenchIndex`：拿到展会状态后 `router.replace` 到 `筹备→products / 进行中→orders / 已结算→settlement`；展会不存在 → `AsyncState` 错误态 + 「返回展会列表」按钮，不跳转。
- 工作台状态条：只有「筹备」显示「开始展会」→ `PUT /events/{id}/status` `{status:'进行中'}` → 成功后 `reload()`；失败 `fb.error` 显示后端原文，按钮恢复可点。「进行中」旁提示「收摊由摊主端完成」。
- 工作台「编辑」本 task 先挂现有 `EditEventForm`（Task 4 P2 换成 `EventForm`）。
- 五个子页：`PageShell` 改 `embedded`；原 `subtitle` 挪进页内第一个 `SectionCard` 的说明或 `help` 气泡——**不丢文字**；子页若自己按 `:id` 查展会名，改用 `useWorkbenchEvent()`。
- 侧栏：spec §3.3；`activeKey` = 菜单 key 中是当前路径前缀的最长者。
- 设置页：一列 `SectionCard`，`AboutSettings` 根元素 `id="about"`，路由 hash 为 `#about` 时滚动到它；开发历程时间线默认折叠；「检查更新」开 `UpdateModal`。
  从 `AdminControlPanel` 搬出的每块**逻辑原样搬**（局域网二维码、安全、v1 历史数据），只换容器。

**必测（vitest）：**
- `router.spec.ts`：`/admin` → `admin-events`；`/admin/about` → `admin-settings` 且 hash `#about`；`/admin/events/5/stats` 解析为 `admin-event-stats` 且 `params.id === '5'`；五个子路由的 `meta.role` 继承为 `admin`。
- `workbench.spec.ts`：`WorkbenchIndex` 三种状态各跳到对应子路由；展会 404 渲染错误态且不调用 `router.replace`（Review Focus 3）；
  「开始展会」失败时按钮恢复、调用了 `fb.error`；侧栏 `activeKey` 在 `/admin/events/5/orders` 下为 `/admin/events`。
- `ui.spec.ts`：`PageShell embedded` 不渲染 `h1`。

- [ ] 实现 + 测试 → 前端五条门禁 → 提交。

---

## Task 3: 摊主端骨架 + 切换 + 结算单共享组件

**Files:**
- Create: `frontend/src/views/vendor/{VendorShell,VendorOrders,VendorInventory,VendorClosing}.vue`、`frontend/src/composables/useVendorPolling.ts`
- Create: `frontend/src/components/settlement/SettlementReportView.vue`（从 `AdminEventSettlement.vue` 抽出只读结算单 + 导出按钮）
- Modify: `frontend/src/views/AdminEventSettlement.vue`（改用 `SettlementReportView`）
- Modify: `frontend/src/components/vendor/ClosingWizard.vue`（去掉 `AppModal` 外壳，变成页面组件：删 `show` prop 与 `close` 事件，保留 `settled` 事件）
- Modify: `frontend/src/router/index.ts`、`frontend/src/views/CustomerView.vue`（仅加「回摊主端」）
- Delete: `frontend/src/views/VendorView.vue`
- Test: `frontend/src/views/vendor/vendor.spec.ts`

**Interfaces — Produces:**

```ts
// 路由名
'vendor-select'（/vendor）、'vendor-shell'（/vendor/:id，meta requiresAuth+vendor）、
'vendor-orders'（''→redirect 到此，path 'orders'）、'vendor-inventory'、'vendor-closing'

// useVendorPolling.ts —— 从 VendorView 原样搬出轮询 + 提示音逻辑，间隔与去重规则不变
export function useVendorPolling(eventId: Ref<string>, audio: Ref<HTMLAudioElement | null>): {
  pendingCount: Ref<number>
  refresh: () => Promise<void>
}   // onMounted 启动、onUnmounted 停止；只在外壳里调用一次

// SettlementReportView.vue
defineProps<{ eventId: number }>()   // 自己取 settlementStore 数据，只读；含 xlsx 导出按钮；错误态走 AsyncState
```

**要点（spec §3.5–3.7、§5.4）：**
- `VendorShell`：页头展会名 +「去点单」（`/events/:id/order`）+「切换展会」（`/vendor`）；删「← 管理后台」。
  手机 `useViewport().isPhone` → 底部固定 tab 栏（`padding-bottom: env(safe-area-inset-bottom)`，行内 stylelint 豁免写理由若需要）；否则页头下 tab 行。订单 tab 带 `pendingCount` 角标。`<audio src="/notify.mp3">` 在外壳里。
- `VendorOrders`：原 `VendorView` 订单栏（待处理 / 已完成、收款弹窗、退货弹窗）原样搬，「手动刷新」改为页内刷新图标按钮调 `refresh`。
- `VendorInventory`：`LiveStats` +「登记赠送/报废」按钮开 `InventoryLogModal`。
- `VendorClosing`：展会未结算 → `ClosingWizard`；已结算（或 `settled` 事件后）→ `SettlementReportView`。
- `CustomerView` 页头：`authStore.canAccessVendorPage(id)` 为真时显示「回摊主端」→ `{ name: 'vendor-orders', params: { id } }`。
- 本 task **只搬不重设计**（窄屏收摊在 Task 4 P4）。

**必测（vitest）：**
- 外壳挂载后 `useVendorPolling` 只启动一次；在 `vendor-closing` 子路由下模拟轮询返回多一条待处理单 → `audio.play` 被调用一次、角标 +1；随后切到 `vendor-orders` 不再次 `play`（Review Focus 2）。
- `/vendor/3` 重定向到 `vendor-orders`；子路由 `meta.role === 'vendor'`；未授权访问 `/vendor/3/closing` 被守卫送去 `login` 且 `query.redirect` 是完整子路径。
- `CustomerView`：`canAccessVendorPage` 为真 / 假时「回摊主端」渲染 / 不渲染。
- `VendorClosing`：结算状态为「已结算」时渲染 `SettlementReportView` 而非 `ClosingWizard`。

- [ ] 实现 + 测试 → 前端五条门禁 → 提交。

### 合审 A（Task 1–3 之后）

- [ ] 一个 Claude 审查 subagent（sonnet）看 Task 1–3 的整体 diff，对照 spec §3、§4.1、§5.2 后端部分、§5.4。重点：抽函数是否改变 `add_product` / `create_lot` 行为；轮询搬迁是否改了间隔 / 去重；子页副标题文字是否丢失；旧 URL 是否都还能到。
- [ ] Important 以上**打包一次**派 dsh-flash 修复；修完不复审，controller 看 diff 合并。

---

## Task 4: 批次 P —— 5 个 worker 并行重写页面

每个 worker 一个 worktree（`./scripts/worktree-new.sh p1` … `p5`），**只改自己名下的文件**（新建文件也在自己的目录内）。
brief 由 controller 按 ④-1 附录 A 的模板写到 `docs/superpowers/plans/4-2/brief-p<N>.md`：本 plan 的 Global Constraints + 该 worker 小节 + 「可用的东西」清单
（`components/ui` 七原语及 ④-1 补的能力、`useFeedback`、`useViewport`、`useWorkbenchEvent`、Task 1 新字段与端点、`SettlementReportView`）。
每个 worker 自己跑前端五条门禁，写 `.4-2/REPORT.md`（做了什么、偏离、没把握的地方）。

启动间隔 ~5 秒（dsh-flash 并行启动会撞配置文件）。

### P1 展前 · 商品与导入

**Files:** `views/AdminEventProducts.vue`；Create `components/event-prep/{LibraryPickDrawer,ImportFromEventDrawer,PendingProductsTable}.vue`、`utils/importPlan.ts`、`utils/importPlan.spec.ts`、`components/event-prep/importDrawer.spec.ts`

- 页面：已上架 `n-data-table`（图、编号、名称、货主、售价、现场库存 `onsite_qty` / 累计 `stocked_qty`、操作：补货 / 改价 / 下架）；操作区「从商品库选」「从上一场导入」各开一个 `n-drawer`（`isPhone` 时 `placement="bottom"`，否则 `right`）。
- 两个抽屉都调 `POST /events/{id}/products/import`；成功 → `fb.success` + 刷新列表 + 关抽屉；失败 → 抽屉内显示后端错误原文，**不关、不清空勾选**。
- `importPlan.ts`（纯函数，UI 只渲染它的输出）：

```ts
export interface SourceProduct { eventProductId: number; masterProductId: number; name: string;
  lastPrice: Cents; stockedQty: number; libraryPrice: Cents | null }   // libraryPrice 由 default_price(元, 可空) 经 toCents
export interface SourceLot { lotId: number; name: string; candidateEventProductIds: number[] }
export interface PlanState { pickedProducts: Set<number>; pickedLots: Set<number>;
  priceChoice: Map<number, 'last' | 'library'> }
export function priceOptions(p: SourceProduct): { showChoice: boolean; defaultPrice: Cents }
  // libraryPrice 为 null 或等于 lastPrice → showChoice=false，defaultPrice=lastPrice
export function lotAvailability(lot: SourceLot, st: PlanState, byEp: Map<number, SourceProduct>,
  targetMasterIds: Set<number>): { ok: true } | { ok: false; missing: string[] }
  // 候选的 master 既不在本场、也没被勾选 → missing 列出商品名
export function toggleLot(lot: SourceLot, st: PlanState, targetMasterIds: Set<number>, byEp: …): PlanState
  // 勾选套装时自动勾上它不在本场的候选商品
export function toggleProduct(epId: number, st: PlanState, …): PlanState
  // 取消商品 → 依赖它且因此不可用的套装自动取消
export function buildImportRequest(st: PlanState, stock: Map<number, number | null>, …): ImportRequest
  // 库存 null → 0
```
- 必测 `importPlan.spec.ts`：`libraryPrice` null / 相等 / 不等三例（Review Focus 4）；套装自动勾候选；取消候选使套装取消；候选已在本场的套装不需勾选即可用；`buildImportRequest` 库存空 → 0、价格按选择。
- 必测 `importDrawer.spec.ts`：import 请求失败时抽屉仍打开、错误文本可见、勾选保留（Review Focus 5）。
- 库存输入不预填，旁注「上一场进货 N」（`stockedQty`）。源展会下拉默认最近一场非本场（按日期降序）。

### P2 展前 · 套装 + 展会列表

**Files:** `views/AdminEventLots.vue`；Create `components/event-prep/LotDrawer.vue`；`views/AdminDashboard.vue`、`components/event/EventList.vue`；Create `components/event/EventForm.vue`；Delete `components/event/{CreateEventForm,EditEventForm}.vue`；Modify `views/AdminEventWorkbench.vue`（仅把「编辑」换成 `EventForm`）；Test `components/event/eventForm.spec.ts`

- 套装页：`n-data-table` 列（名称、规则「任选 N 件 · 可同款 / 各 1 件」、总价、货主、候选商品、操作）；新建 / 编辑在 `LotDrawer`，字段全带标签，「顾客最多 / 最少怎么拿」预览随表单实时变（逻辑从现页原样搬）；删除红色确认。说明文字进 `help` 气泡与字段说明。
- 展会列表：卡片网格按「进行中 → 筹备 → 已结算」分组，已结算组默认折叠并显示数量；整卡可点进 `admin-event-workbench`；「新建展会」开 `AppModal` + `EventForm`；删除在卡片更多菜单里，红色确认。原生 `<button>` 清零。
- `EventForm`：`defineProps<{ mode: 'create' | 'edit'; event?: Schemas['EventResponse'] }>()`，emit `saved`；FormData 字段名与现两表单一致；编辑模式密码框空、旁注「留空 = 不修改」、提交空串；删掉 `vendor_password?` 类型补丁。
- 必测 `eventForm.spec.ts`：编辑模式初始密码为空、提交的 FormData 里 `vendor_password === ''`；创建模式必填校验。

### P3 订单、统计与数据表格

**Files:** `views/AdminEventOrders.vue`、`views/AdminEventStat.vue`、`views/AdminSocieties.vue`、`components/product/MasterProductList.vue`、`views/vendor/VendorOrders.vue`、`components/order/OrderCard.vue`；Test `views/orders.spec.ts`

- 管理端订单：`n-data-table` 列（单号、时间、状态、原价 → 实收——`gross_amount !== final_amount` 时两数都显示、**已退** `refunded_amount`（0 显示「—」）、渠道、操作）；行展开显示 items（商品、数量、`lot_name`、`paid_amount`、`refunded_qty`）与 lots；筛选放表格上方一行。
- 统计：自写错误态（含「后端数据库寄了！」）删掉，交 `AsyncState :error`；明细表 `n-data-table`；`var(--overlay-light)` 改为实际存在的 token（在 `theme.ts` 生成的变量里挑语义最近的，写进 REPORT）。
- 社团、商品库列表：原生 `<table>` → `n-data-table`，列与操作不变。
- 摊主订单：已完成单若每行 `refunded_qty >= quantity` → 卡片置灰、「退货」禁用并提示「已全部退货」；删掉 `VendorView` 搬来的那段「不做已退完置灰」注释。`OrderCard` 按 token 重写外观，props / emits 不变。
- 必测 `orders.spec.ts`：全退的单「退货」禁用、部分退的单可用；管理端表格 `refunded_amount` 为 0 时显示「—」。

### P4 结算与收摊

**Files:** `components/settlement/SettlementReportView.vue`、`views/AdminEventSettlement.vue`、`components/vendor/ClosingWizard.vue`、`views/vendor/{VendorClosing,VendorInventory}.vue`、`components/vendor/{LiveStats,InventoryLogModal}.vue`；Test `components/vendor/closingWizard.spec.ts`

- 结算单：保留语义 `<table>`，版式对齐 xlsx 四 sheet；自写错误态交 `AsyncState`；管理端的垫付 / 调整 / 清点编辑区块与对账警示条排在结算单之上，警示条在页内首位。
- 收摊页（spec §5.6，手机优先）：`isPhone` 时步骤指示为一行「第 N 步 / 共 4 步 · <步骤名>」，否则 `n-steps`；
  盘点每行商品名 + 账面数（弱化）+ `n-input-number`（`input-props="{ inputmode: 'numeric' }"`，不预填、`:show-button="false"`）；
  顶部粘性条「已盘 X / Y」+「只看未填」开关；每步主按钮在手机上固定在底部 tab 栏之上。**步骤仍由后端状态推出，不存本地 step；注释保留。**
- 库存 tab：`LiveStats` 手机单列、平板两列。
- 必测 `closingWizard.spec.ts`：盘点输入框初始全空；填 2/5 后粘性条显示「已盘 2 / 5」；开「只看未填」只剩 3 行；模拟重新挂载（退出重进）时步骤取自 store 状态而非组件内变量。

### P5 顾客端

**Files:** `views/CustomerView.vue`（「回摊主端」按钮 Task 3 已加，保留）、`components/customer/{ProductGrid,ShoppingCart,PaymentModal}.vue`、`components/shared/VisionSearch.vue`、`scripts/check-ui-boundary.mjs`（仅加 `PaymentModal` 豁免——若规则要求）；Test `components/customer/customer.spec.ts`

- 布局（spec §5.7）：平板横屏（`--desktop` 或横向 ≥1025）分类侧栏 + 商品网格 + 购物车侧栏；平板竖屏（`--tablet` 且非 phone）购物车改为底部可展开条（收起时显示件数与合计）；手机沿用竖屏形态。
- `var(--text-color)` 改为实际存在的 token。
- `PaymentModal` 保持自写 overlay，在其 overlay 行上写 `<!-- ui-boundary-ignore: 全屏收款码展示页，不是对话框 -->`（若边界脚本会报）。
- `VisionSearch` 识别结果：`isPhone` 时用 `n-drawer placement="bottom"`（高 60%），取景画面仍可见；其余仍 `AppModal`。
- 必测 `customer.spec.ts`：`isPhone` 为真时识别结果渲染为 drawer；平板竖屏下购物车条收起态显示件数与合计。

### 合审 B

- [ ] 5 个 worker 完成后，controller 逐个合并进 `1.2-dev`（冲突只可能在 `router/index.ts` 等共享文件——worker 不应碰；碰了就退回）。
- [ ] 2 个 Claude 审查 subagent（sonnet）：A 看 P1+P2，B 看 P3+P4+P5。对照 spec §4.2–§5.8 与本 plan 各小节。
- [ ] 所有 Important 以上 + controller 截图发现，**打包一次**派 dsh-flash 修复；修完不复审。

---

## Task 5: 收口（Claude）

- [ ] **门禁新规则**（`check-ui-boundary.mjs`，每条加 fixture 并让 `--self-test` 通过、再故意违规一次看真红）：
  1. `views/vendor/**` 与工作台子页（`views/AdminEvent{Products,Lots,Orders,Stat,Settlement}.vue`）里的 `<PageShell` 必须带 `embedded`；
  2. `views/**` 里 `n-alert` 带 `type="error"` 且同一文件出现 `store.error` / `.error }}` 这类加载错误渲染 → 报（误报用行内豁免加理由）；
  3. 原生 `<table` 只允许在 `components/settlement/**`。
- [ ] **删除确认色**：全仓 grep `fb.confirm(` / `confirm({`，删除类统一 `danger: true`，作废 / 冲正类 `type: 'warning'`。
- [ ] **对照截图**：同 Task 0 的视口 / 主题 / 路由（新路由替换旧：`/admin/settings`、`/vendor/:id/{orders,inventory,closing}`、各工作台子页），存 `after-4-2/`，pixelmatch 排序后逐张看差异最大的；
  另外对照 spec §7 目标宽度逐条自查（无横向滚动、按钮不被遮）。发现问题直接修。
- [ ] **全门禁**：前端五条 + Rust 五条 + `gen:api` 无 diff + `npm run docs:build`（仓库根）。
- [ ] **文档**：`docs/` 下用户文档与 `Help.vue` 里的导航描述（控制台 / 关于 / 摊主端布局 / 收摊）改为新结构；路线图追加「附录七：④-2 执行后的状态」；本 plan 末尾写「执行后记」（偏离、量化对账：`<style>` 行数 / 原生 `<table>` / 页面自写错误态数、真机待验清单）。
- [ ] 提交（门禁 `&&` 提交）。

## Task 6: 终审

- [ ] Opus subagent 看 `1.2-dev` 上 ④-2 全部提交，对照 spec 全文与本 plan Review Focus；**直接修并提交**，附一段发现清单给 controller 写进执行后记。

---

## 真机待验（本轮新增，并入 beta 前的总清单）

1. 平板：摊主端「去点单」→ 顾客下单 →「回摊主端」→ 在待处理里收款，来回三次。
2. 手机：完整收摊（清点 → 盘点逐行输入，数字键盘 → 带回 → 结算 → 原地看到结算单并导出 xlsx）。
3. 手机：停在收摊 tab 时来新单，提示音与角标。
4. 管理端：从上一场导入 30+ 商品 + 若干套装的耗时与结果；故意制造一个套装失败看回滚与提示。
5. 手机：相机识别结果底部抽屉不挡取景。

---

## 执行后记（2026-09-25）

`1.2-dev` 上 `a089bac..` 本轮共 20 个提交（15 个非合并）。执行者：Task 1–3、两个修复包、批次 P 的 5 个 worker 全部由 **dsh-flash（deepseek-flash）** 实现；
审查是 Claude：Task 1/2/3 各一次（sonnet，Task 2 与 3 边做边审，合审 A 实际拆成三次），批次 P 两份（sonnet，2 + 3 个 worker），终审见下节（opus）。
每一轮审查的发现都**打包成一次**修复派发，没有复审来回。

### 偏离 plan 的裁定（全部记在 SDD ledger，这里是摘要）

| # | 裁定 | 为什么 |
|---|---|---|
| 1 | 摊主外壳在根元素上给 `--vendor-tabbar-height` | P4 要把收摊主按钮放在 tab 栏之上，否则只能写死数值 |
| 2 | 门禁「子页 PageShell 必须 embedded」豁免外壳 `VendorShell` | 外壳就是提供页头的那一个 |
| 3 | import 端点 tag 用单数 `product` | plan/spec 写的 `products` 与既有分组不一致 |
| 4 | 设置页加「AI 视觉识别」区块（`VisionModelPanel` + v1.1 推荐卡） | spec §3.4 说它在商品库页是**我查错了**，删控制台后它成了孤儿 |
| 5 | 「快速开始」引导卡搬到展会列表页顶部；`control-panel` 帮助词条改名 `settings` | 同上，Task 2 按字面删掉了 |
| 6 | 修旧 bug：待处理为 0 时开摊，第一张新单不响 | watch 在 0→0 时不触发，`isInitialized` 永不置位；摊主每天都是从 0 单开始 |
| 7 | `AsyncState` 加 `keepContent`（只加不改） | 结算单刷新失败会吞掉已加载的旧单，旧版是「错误条 + 旧单」 |
| 8 | 管理端收摊清点恢复「已清点渠道预填上次人工值」 | 「收全量」只禁账面值预填；P4 过度泛化成永不预填 |
| 9 | 顾客端横屏断点维持纯宽度（≥1025 三栏） | 全仓无方向断点，目标机 1180；1024 宽横屏旧平板走底部购物车条 |
| 10 | 只读结算单只渲染汇总 + 货主明细 | 账本流水 / 订单明细只读 API 没有，本轮不改后端；两者仍在 xlsx 里 |
| 11 | `theme.ts` 的 Radio 按钮组配色改映射（不改色值） | 未选中也填实心主色，全应用所有单选按钮组都看不出选的是哪个；worker 按约束只在结算页局部覆盖，收口时根治 |

spec 的两处实现层修正已写在 plan 开头（无 422；`refunded_amount` 放 `OrderResponse` 而非 `OrderRow`）。

### 截图对照抓到的（审查全没提）

演示数据（6 商品、2 套装、5 单含 1 笔部分退货）下 390 / 820 / 1180 / 1440 四宽截图，发现 6 处，全部在修复包 B 里修掉并重截确认：
工作台子页宽度不一致；套装表货主列折行、操作按钮竖堆；结算调整两个方向按钮都像选中（→ 裁定 11）；
**摊主营业额前 5 秒显示 ¥0**（子组件 `onMounted` 早于外壳设置当前展会）且没扣退款；收摊第 1 步按钮 < 44px；
**顾客商品价格在手机和横屏平板上被截成「¥11…」**。

### 量化对账

| 指标 | 前（a089bac） | 后 |
|---|---|---|
| 前端 `.vue` + `.ts` 行数（不含生成的 `schema.d.ts`） | 26 617 | 30 415 |
| `.vue` 文件 | 56 | 70 |
| `<style>` 块总行数 | 7 711 | 7 411 |
| 原生 `<table>`（`.vue`） | 9 | 4（全在 `components/settlement/`） |
| 页面 `n-alert type="error"` | 3 | 1（登录表单的提交错误，非加载错误态） |
| 前端测试 | 261 | 318 |
| Rust 测试 | 311 | 330 |
| 边界门禁规则 | 6 | 9 |

行数涨了约 3 800：新增的是导入抽屉与 `importPlan`、设置页五个区块、摊主外壳与三个 tab、`EventForm`、`LotDrawer` 和对应测试。
`<style>` 只降了 4%——重写把布局写全了（窄屏、两栏、底部 tab 栏），不是只删。

### 门禁

新增三条（`check-ui-boundary.mjs`，fixture 用首行 `boundary-fixture-path:` 模拟路径）：外壳子页 `PageShell` 必须 `embedded`；
页面不手写 `store.error` 类加载错误态；原生 `<table>` 只留给 `components/settlement/**`。首次全仓扫描命中的两处都是规则写宽了（HTML 注释里的 `<table>`、登录表单的提交错误），改规则不改代码。

**又踩了一次门禁空转**：`npm run test:unit 2>&1 | grep Tests && git commit`——管道的退出码是 grep 的，带着一条失败的 self-test 提交了（`fc56a33`，下一个提交修复）。

### 没做 / 留给后续

- 统计页的 `StatFilters` 仍是原生 date / select（本轮没动这个组件）。
- 展会卡片从 `RouterLink` 改成 `div role="link"`，中键新标签打开没了。
- 英文 / 日文用户文档没有跟着改导航描述（只改了中文）。
- `helpContent` 没有「套装」词条，套装页说明留在页内提示里。
- shape-cleanups A / B 类仍在另立的小项目里。

### 真机

本轮新增的真机项见上文「真机待验」5 条；加上 ③b（路线图附录四）与 ④-1 积压的清单，**全部没走**。发 beta 前必须走。
