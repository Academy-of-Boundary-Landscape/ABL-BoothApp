# ④-2 信息架构与页面重写 设计

**日期**: 2026-09-25
**作者**: Renko_6626（与 Claude 协作）
**状态**: Draft
**前置阅读**: `2026-09-25-ui-foundation-design.md` 文末附录（④-2 已达成的共识）；
`docs/superpowers/plans/2026-09-25-ui-foundation.md` 的「执行后记」；路线图附录二～六「留给 ④」各节
**性质**: 路线图 ④（外观）的第二个子项目。**一份 spec、一次做完**（用户明确不再拆分）；
plan 内按依赖排序（骨架 → 页面），但不设中间审批关口。

---

## 1. 目标

让用户**看得出展会的生命周期**，让摊主**在手机上走得完一整场**。

- 管理端：进入一场展会就是一个「工作台」，按「展前 → 现场 → 收摊」分组；侧栏不再随展会跳变；系统类功能收成一个设置页。
- 摊主端：订单 / 库存 / 收摊三个 tab，收摊从弹窗升格为页面；摊主能看自己这场的结算单。
- 摊主 ↔ 顾客一键切换，满足平板代点单（**不做收银台模式**）。
- 展前准备：批量选品 + **从上一场展会导入**（商品、售价、套装），一个事务完成。
- 所有页面用 ④-1 的 token 与原语重写，达到第 7 节的目标宽度。

**不变的**：视觉方向（token 颜色值不动）；三角色互斥的基本模型；④-1 的全部门禁。
**不在范围**：shape-cleanups A 类（错误体统一 + 文案）与 B 类（吞错误），另立小项目。

## 2. 已拍板的决定（brainstorm 2026-09-25）

| # | 问题 | 决定 |
|---|---|---|
| 1 | 拆不拆 | 不拆，一份 spec |
| 2 | 工作台放哪一层 | 独立页面 `/admin/events/:id`，页内分组 tab；侧栏不变 |
| 3 | 系统类归拢 | 侧栏 = 展会 / 商品库 / 社团 ‖ 设置 / 使用教程 ‖ 摊主端、顾客端快捷入口；「控制台」「关于」页取消 |
| 4 | 摊主端 | 订单 / 库存 / 收摊三个 tab，各是子路由；针对单笔订单的短操作留弹窗 |
| 5 | 代点单回程 | 只放常驻「回摊主端」，**不**自动打开收款弹窗 |
| 6 | 展前准备 | 完整做：批量选品 + 从上一场导入，后端新增事务端点 |
| 7 | 路由机制 | 嵌套路由（外壳 + 子路由），现有 URL 保留 |
| 8 | 删除确认颜色 | 统一红色（danger）；只有可逆操作（作废 / 冲正）用 warning |
| 9 | 相机识别结果弹窗 | 手机上改底部抽屉，取景画面保持可见 |
| 10 | `PaymentModal` | 保留自写 overlay，登记为边界豁免并写理由 |
| 11 | ④-1 的视觉变化 | 全部接受（页宽 3 档、Naive 字号 15.2px、页面居中、EmptyState 图标 32px） |
| 12 | 顾客端目标 | **平板美观、手机能用** |

## 3. 路由与导航骨架

### 3.1 管理端路由

子路由继承父路由的 `meta`（`requiresAuth` / `role`），`beforeEach` 不用改。

```
/admin                          AdminLayout
  ''                → redirect { name: 'admin-events' }
  events            admin-events               展会列表（§5.1）
  events/:id        AdminEventWorkbench        工作台外壳（§3.2）
    ''              → 按状态重定向：筹备 → products / 进行中 → orders / 已结算 → settlement
    products        admin-event-products       展前 · 商品（§4）
    lots            admin-event-lots           展前 · 套装（§4）
    orders          admin-event-orders         现场 · 订单（§5.2）
    stats           admin-event-stats          现场 · 统计（§5.3）
    settlement      admin-event-settlement     收摊 · 结算（§5.4）
  master-products   admin-master-products      商品库
  societies         admin-societies            社团
  settings          admin-settings             设置（§3.4）
  help              admin-help                 使用教程
  about             → redirect { name: 'admin-settings', hash: '#about' }
```

- 统计路由现在是绝对路径 `/admin/events/:id/stats` + 大写 name `AdminEventStats`、不传 props，归正为上表写法。
  全仓 grep `AdminEventStats` 改名。
- 按状态重定向需要展会状态：外壳在 `''` 子路由的组件里（一个只做跳转的 `WorkbenchIndex`）等 `eventStore` 取到这场展会再 `router.replace`；取不到（404）显示 `AsyncState` 的错误态。
- 使用教程（`Help.vue` 与 vitepress 文档）里写到的路径全部仍然有效；「控制台」「关于」的文字描述要改。

### 3.2 工作台外壳 `views/AdminEventWorkbench.vue`

```
┌ ← 展会列表   <展会名>                                   [编辑] ┐
│  2026-10-01 · 上海某会展中心                                   │
│  ● 筹备 ─── ○ 进行中 ─── ○ 已结算            [开始展会]        │
│  展前｜商品  套装    现场｜订单  统计    收摊｜结算             │
└───────────────────────────────────────────────────────────────┘
  <router-view />
```

- 一个 `PageShell`（`width="wide"`），子页不再有自己的页头。`PageShell` 新增 `embedded` 布尔属性：
  只保留页宽与内容区、不渲染标题栏；子页用它包内容。子页原来的副标题说明挪进各自 `SectionCard` 的说明或 `help` 气泡。
- 状态条：三段，当前段高亮。按钮只在「筹备」显示「开始展会」（调现有 `PUT /events/:id/status`）。
  「进行中 → 已结算」**没有按钮**，那只能走收摊流程；状态条上写一行提示「收摊由摊主端完成」。
  ②-3 已知坑：给状态端点加守卫时漏查调用方——这里只调「筹备 → 进行中」这一个允许的迁移。
- tab 行是 `router-link`，三组之间有组标题。窄屏（`--tablet` 以下）tab 行横向滑动，不折行。
- 「编辑」打开 `EventForm` 弹窗（§5.1）。
- 外壳负责加载这场展会（`eventStore`），子页从外壳 `provide` 拿展会对象，不再各自按 `:id` 查。

### 3.3 侧栏 `AdminLayout.vue`

```
管理后台
  展会          /admin/events      （在工作台里也高亮这一项：按路径前缀匹配）
  商品库
  社团
  ───
  设置
  使用教程
  ───
  快捷入口：摊主端 / 顾客端
```

- 删掉：动态插入的展会子菜单、「正在进行的展会」、伪装成菜单项的「主题设置」、底部的「检查更新」「关于」。
- `activeKey` 从 `route.path` 改为「匹配到的最长菜单前缀」。
- 手机上的 FAB + 遮罩抽屉保留（管理端手机用得少，不再投入）。

### 3.4 设置页 `views/AdminSettings.vue`

一列 `SectionCard`，每块一个组件放在 `components/settings/`：

| 区块 | 组件 | 来源 |
|---|---|---|
| 局域网连接 | `LanSettings.vue` | `AdminControlPanel` |
| 安全 | `SecuritySettings.vue` | `AdminControlPanel` |
| 外观 | `AppearanceSettings.vue` | `ThemeSetting.vue`（弹窗 → 内嵌） |
| 数据（v1 历史数据） | `LegacyDataSettings.vue` | `AdminControlPanel` |
| 关于与更新（`id="about"`） | `AboutSettings.vue` | `About.vue` + 侧栏「检查更新」；开发历程时间线默认折叠 |

删除 `AdminControlPanel.vue`、`About.vue`、`ThemeSetting.vue`（内容搬走后）。`UpdateModal` 保留，由「检查更新」按钮打开。
控制台副标题里提到的「AI 视觉识别配置」实际在商品库页（`VisionModelPanel`），不搬。

### 3.5 摊主端路由

```
/vendor                         vendor-select      选展会（不变）
/vendor/:id                     VendorShell        外壳（§3.6）
  ''          → redirect orders
  orders      vendor-orders     待处理 / 已完成（§5.5）
  inventory   vendor-inventory  实时库存 + 登记赠送/报废
  closing     vendor-closing    未结算：收摊页（§5.6）；已结算：只读结算单（§5.4）
```

`meta: { requiresAuth: true, role: 'vendor' }` 放在 `/vendor/:id` 上，子路由继承；守卫读的 `to.params.id` 在子路由上仍然存在。

### 3.6 摊主外壳 `views/VendorShell.vue`

- 页头：展会名；右侧「去点单」（→ `/events/:id/order`）、「切换展会」（→ `/vendor`）。删除「← 管理后台」（对摊主角色必然跳登录页）。
- tab：手机（`--phone`）为**固定底部三格 tab 栏**，含 `env(safe-area-inset-bottom)`；更宽时为页头下的 tab 行。
  订单 tab 带待处理数角标。
- **3 秒轮询、新单提示音、`<audio>` 元素都在外壳里**，切到库存 / 收摊 tab 也继续提醒。
  轮询逻辑从 `VendorView` 原样搬出，不改间隔与去重规则。
- 「手动刷新」按钮删除，改为订单 tab 内的刷新图标按钮。
- `VendorView.vue` 拆成 `VendorShell` + `VendorOrders` + `VendorInventory` + `VendorClosing` 后删除。

### 3.7 摊主 ↔ 顾客切换

- 摊主外壳「去点单」→ `/events/:id/order`。
- `CustomerView` 页头：`authStore.canAccessVendorPage(id)` 为真时显示「回摊主端」→ `/vendor/:id/orders`，**不带参数**。
- 下单流程、下单成功提示都不变。约定自助点单的平板不登录摊主，因此不加长按 / PIN 保护。
- 登录态在 `sessionStorage`：同一标签页内切换有效，这正是代点单的场景。

## 4. 展前准备：商品、套装、导入

### 4.1 后端：`POST /events/{event_id}/products/import`

```jsonc
// 请求
{
  "products": [{ "master_product_id": 12, "unit_price": 3000, "initial_stock": 20 }],
  "lots":     [{ "source_lot_id": 7 }]        // 可省略或为空
}
// 响应 200
{ "products": [EventProductResponse...], "lots": [LotResponse...] }
```

- **一个事务**：先逐条建 `event_products`（`initial_stock > 0` 时记首批进货 journal——抽出 `add_product_to_event`
  里「建商品 + 首批进货」的那段为共享函数，两处调用同一实现）；再逐个复制套装。
- 套装映射：源套装的每个候选 `event_product` → 其 `master_product_id` → 目标展会中同 `master_product_id` 的 `event_product`
  （本请求刚建的，或目标展会原本就有的）。任一候选映射不到 → 整批失败。
  新套装沿用源套装的 `name` / `pick_count` / `total_price` / `allow_repeat`，走现有建套装的**同一套校验**（同一货主等）。
- 错误：
  - 商品已在目标展会上架 → 409，错误信息写明商品名；**不**静默跳过。
  - 套装候选映射不到 / 校验失败 → 422，写明套装名与原因。
  - 目标展会已结算 → 走现有 `require_event_open` 拒绝。
  - `source_lot_id` 不属于任何展会 / 属于目标展会自身 → 422。
  - 任何失败整批回滚。
- 鉴权：admin（与现有上架端点相同）。
- 批量选品与从上一场导入**共用此端点**（前者 `lots` 为空）。
- OpenAPI：进 `products` tag，描述里写明事务语义与冲突规则；更新快照，重新生成 `schema.d.ts`。
- 前端候选列表用现有接口拼（源展会的 `GET products`、`GET lots`，商品库 `GET master-products`），不新增查询端点。

### 4.2 商品页（展前 · 商品）`AdminEventProducts.vue`

- 主体：已上架商品 `n-data-table`（图、编号、名称、货主、售价、现场库存、操作：补货 / 改价 / 下架）。
- 页内操作区两个按钮，各开一个抽屉（`n-drawer`，桌面右侧、手机底部）：
  - **从商品库选**：左侧商品库（分类筛选 + 搜索 + 多选卡片），选中项进入下方「待上架」表：
    每行 售价（预填商品库现价）、库存（**不预填**，留空按 0）、移除。底部「上架 N 件」调 import。
    已在本场的商品置灰不可选。
  - **从上一场导入**：先选源展会（默认最近一场非本场），然后两张可勾选表：
    - 商品：名称、上一场售价、商品库现价、**本次售价**（单选：沿用上一场 / 用现价，默认沿用上一场；两价相同时不显示选择）、
      库存（不预填，旁注「上一场进货 N」）。已在本场的置灰并注明。
    - 套装：名称、规则摘要、候选商品。**任一候选商品未勾选且不在本场** → 置灰并注明「需要先勾选 XX」；
      勾选套装时自动勾上它的候选商品（可以再取消，取消后套装随之置灰取消）。
    - 底部汇总「导入 N 件商品、M 个套装」→ 调 import；失败时把后端错误原文显示在抽屉内，不关闭抽屉。
- 候选计算（价格差异、套装可导入性、自动勾选联动）写成纯函数 `utils/importPlan.ts`，单测覆盖。
- 「上一场进货 N」取源展会该商品的首批 + 补货总量；若现有接口拿不到，就只显示上一场售价，不为此加端点。

### 4.3 套装页（展前 · 套装）`AdminEventLots.vue`

- 列表：`n-data-table`（名称、规则「任选 N 件 · 可同款 / 各 1 件」、总价、货主、候选商品、操作）。
- 新建 / 编辑：抽屉。字段全部带标签（沿用 2026-09-24 的最小修复的原则）；「顾客最多 / 最少怎么拿」预览保留在抽屉内、随表单实时变。
- 删除：红色确认。
- 页面说明（「候选商品必须属于同一个货主」）移入 `help` 气泡与抽屉内的字段说明。

## 5. 其余页面

### 5.1 展会列表 `AdminDashboard.vue` + `EventList.vue`

- 卡片网格，按状态分组：进行中 → 筹备 → 已结算（已结算组默认折叠，显示数量）。卡片点整张进工作台。
- 「新建展会」按钮 → `AppModal` + `EventForm`。
- `CreateEventForm` + `EditEventForm` 合并为 `components/event/EventForm.vue`（`mode: 'create' | 'edit'`）。
  编辑时摊主密码框为空，旁注「留空 = 不修改」；**删掉读取 `vendor_password` 的类型补丁**（`EventResponse` 本就没有该字段）。
- 删除展会：卡片的更多菜单里，红色确认。
- 原生 `<button class="btn">` 清零。

### 5.2 订单（现场 · 订单）`AdminEventOrders.vue`

- `n-data-table`：单号、时间、状态、原价 → 实收（套装折让时两数都显示）、**已退**、渠道、操作。
- 行展开：订单行（商品、数量、所属套装、实付、已退数量）与套装列表。
- 筛选保持现有能力（状态等），放在表格上方一行。
- **后端**：`OrderRow` 加 `refunded_amount: Money`（`SUM(refunds.refund_amount)` 按订单聚合，无退货为 0），
  `OrderItemResponse` 加 `refunded_qty: i64`（`SUM(refunds.qty)` 按 `order_line_id`）。只读聚合，**无迁移**。
  所有返回订单的端点（列表、单个、状态更新后的返回）一致带上；形状快照测试更新。

### 5.3 统计（现场 · 统计）`AdminEventStat.vue`

- 去掉自写错误态（含「后端数据库寄了！」），交给 `AsyncState` 的 error 态。
- 明细表改 `n-data-table`。
- 修正 `var(--overlay-light)`（不存在的变量名）为实际存在的 token；前后截图确认变色合理。

### 5.4 结算：共享只读组件 + 两个入口

- 从 `AdminEventSettlement.vue` 抽出 `components/settlement/SettlementReportView.vue`：结算单全部内容 + xlsx 导出按钮，只读。
- 管理端「收摊 · 结算」= `SettlementReportView` + 垫付 / 结算调整 / 收摊清点三个编辑区块 + 对账警示条。
- 摊主端「收摊」tab 在已结算时 = `SettlementReportView`（后端 `get_settlement` 走 `check_read_permission`，摊主可读，不改后端）。
  摊主端导出同样可用（`download_settlement_xlsx` 的权限在实现时核实；若只许 admin，摊主端隐藏导出按钮，不改后端）。
- 自写错误态交给 `AsyncState`。
- **结算单内的 4 张表保留语义化 `<table>`**：它们是报表，版式要与 xlsx 对应。

### 5.5 摊主 · 订单 `VendorOrders.vue`

- 待处理 / 已完成两个分段；待处理提示条、空态保留。
- 已完成列表：用 `refunded_qty` 判断「已退完」→ 置灰且「退货」按钮禁用（②-3 当时因每单一个请求而放弃的标记）。
- 收款弹窗、退货弹窗不变（仍为 `AppModal`）。`OrderCard` 按新 token 重写外观，事件接口不变。

### 5.6 摊主 · 收摊 `VendorClosing.vue`

`ClosingWizard` 从 `AppModal` 改为页面组件（逻辑不变：**步骤由后端状态推出，不存本地 step**）。按手机优先：

- 步骤指示：宽屏 `n-steps`；手机上缩为一行「第 2 步 / 共 4 步 · 盘点」。
- 盘点：每行商品名 + 账面数（仅参考）+ 大号数字输入框（`inputmode="numeric"`），**不预填**（②-3 定的原则）。
  顶部粘性条「已盘 12 / 40」+「只看未填」开关。
- 各步主按钮在手机上固定于底部 tab 栏之上。
- 结算完成后原地切换为 §5.4 的只读结算单（不再停在「账本已冻结」屏）。

### 5.7 顾客点单 `CustomerView.vue` 与组件

- 按**平板美观、手机能用**调整：平板横屏为分类侧栏 + 商品网格 + 购物车侧栏；平板竖屏购物车改为底部可展开条；手机沿用竖屏形态。
- 修正 `var(--text-color)`（不存在的变量名），截图确认。
- 页头加「回摊主端」（§3.7）。
- `PaymentModal` 保持自写 overlay，在 `check-ui-boundary.mjs` 登记豁免，理由写「全屏收款码展示页，不是对话框」。

### 5.8 其他

- **相机识别结果**（`VisionSearch`）：手机上改 `n-drawer` 底部抽屉（约 60% 高），取景画面保持可见；桌面 / 平板仍为 `AppModal`。
- **删除确认**：全仓统一 `danger`；作废、冲正类可逆操作用 `warning`。
- **数据列表表格改 `n-data-table`**：社团、商品库、商品页、套装页、统计明细、订单页。例外仅 §5.4 的结算单。
- **错误态**：页面不再手写错误态，一律经 `AsyncState`。

## 6. 门禁增补

在 `check-ui-boundary.mjs` 增加（每条带 fixture，照 ④-1 的做法验证真红）：

- 挂在工作台或摊主外壳下的子页（路由表可枚举）不得使用非 `embedded` 的 `PageShell`。
- `views/**` 中不得出现 `n-alert type="error"` 作为加载错误态（应走 `AsyncState`）——具体匹配规则在 plan 里定，误报用行内豁免加理由。
- 原生 `<table>` 只允许出现在 `components/settlement/**`。

## 7. 目标宽度

「能用」= 无横向页面滚动、可点区域 ≥ 44px、按钮不被遮挡或键盘顶走。

| 角色 | 设计目标 | 要求 |
|---|---|---|
| 顾客 | **平板美观**（820 竖屏、1180 横屏） | 手机（390）能用 |
| 摊主 | **手机**（390）为主设计目标；平板用两栏（订单 + 库存摘要） | 桌面能用 |
| 管理 | 桌面（1440） | 平板能用；手机无横向页面滚动，表格在自身区域内横滑 |

断点沿用 ④-1 的 4 个 custom media，不新增。

## 8. 测试与验收

- **Rust**：import 端点——正常导入、套装按 `master_product_id` 映射、映射失败整批回滚（断言商品也没建）、
  重复商品 409、已结算拒绝、源套装属于目标展会自身拒绝、`initial_stock = 0` 不记 journal；
  `refunded_amount` / `refunded_qty` 的形状快照与一个部分退货的数值用例；OpenAPI 快照更新。
- **前端 vitest**：
  工作台按状态重定向（三态各一）；侧栏 `activeKey` 前缀匹配；
  `VendorShell` 切 tab 时轮询不中断、提示音不重复；
  「回摊主端」仅在 `canAccessVendorPage(id)` 为真时渲染；
  `importPlan.ts`：价格差异、套装可导入性、勾选联动；
  `EventForm` 编辑模式密码留空不提交该字段（或提交空串——以现后端「留空不改」的实际语义为准，实现时核实）。
- **截图对照**：Playwright 在 390 / 820 / 1440 三宽度，按角色的主要页面（清单进 plan），亮暗两套主题，重写前后各一组。
  controller 在 VNC 上做，**不进 worker brief**（dev server 不退出）。门禁与提交用 `&&` 串。
- **门禁**：④-1 全部门禁 + §6 新增规则；`UPDATE_OPENAPI` 流程后 `gen:api` 无 diff；Rust 六条门禁全绿。
- **真机**：本轮结束与 ③b、④-1 积压的真机清单一起走，然后打 beta。本轮新增的真机项：
  平板上代点单来回切换；手机上完整收摊（盘点逐行输入、数字键盘）；从上一场导入 30+ 商品的实际耗时。

## 9. 风险

| 风险 | 缓解 |
|---|---|
| 摊主轮询从页面搬到外壳时改变了提醒行为（漏响 / 重复响） | 搬运不改逻辑；vitest 覆盖切 tab；真机项 |
| 子页去掉 `PageShell` 后说明文字丢失 | plan 为每个子页列出原副标题的去处 |
| import 端点与 `add_product` 的首批进货逻辑分叉 | 抽共享函数，两处同一实现；Rust 测试断言 journal 相同 |
| 删除 `AdminControlPanel` / `About` / `ThemeSetting` 时漏搬功能 | plan 逐块列出源 → 目标对照；截图对照 |
| 旧链接（教程、书签）失效 | 保留全部旧 URL；`about` 与 `''` 做重定向 |
| 页面重写范围大，worker 并行冲突 | 骨架先行且由单人完成；页面按文件分批，每个 worker 只碰自己的文件（`worktree-new.sh`） |
| 收摊页改页面时破坏「步骤由后端推出」 | 保留原注释与逻辑；测试「退出重进不丢进度」 |
