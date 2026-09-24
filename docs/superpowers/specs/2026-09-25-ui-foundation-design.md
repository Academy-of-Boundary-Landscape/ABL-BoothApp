# ④-1 外观基础层 设计

**日期**: 2026-09-25
**作者**: Renko_6626（与 Claude 协作）
**状态**: Draft
**前置阅读**: `2026-09-22-v1.2-roadmap.md` 的 D7 与附录二～四「留给 ④ 的」各节
**性质**: 路线图的 ④（外观）拆成两个子项目，本文是第一个。第二个（④-2）的已达成共识记在文末附录。

---

## 1. 目标

**整洁与统一。** 不是换视觉，是把「每页各写一遍」的 CSS 收进一套 token + 原语 + 门禁，
让它长不回来。

- 视觉方向不动（路线图 D7）：`frontend/src/config/theme.ts` 的**颜色值一个不改**；
  用户可配置项（明/暗、主色、商品图比例，见 `stores/themeStore.ts`）行为不变。
- 结束时：页面的 `<style scoped>` 里只剩布局；颜色、字号、圆角、阴影、间距、断点、页宽
  全部来自 token；反馈、弹窗、加载/错误/空三态各只有一种写法。**stylelint 门禁全仓强制，没有白名单。**
- 不改后端，不新增迁移。

## 2. 分解（④ → ④-1 / ④-2）

| | 内容 | 依赖 |
|---|---|---|
| **④-1 基础层**（本文） | token 补全与收口、Naive 主题几何项同步、`components/ui/` 原语层、全部页面的**机械迁移**、stylelint + 原语边界门禁 | 无 |
| **④-2 IA 与页面重写** | 管理端导航按展会生命周期重组、系统类归一、摊主端重组、摊主↔顾客切换、`AdminEventLots` 重做、订单页显示退货、原生表格改 `n-data-table`、逐页窄屏 | ④-1 |

**顺序理由**：④-2 的每一页都用 ④-1 的原语重写。先建原语，页面在 ④-2 只做一次结构性重写。

**④-1 为什么也迁页面**：只做**不涉及判断的机械替换**——页头 → `PageShell`、
三态 → `AsyncState`、弹窗 → `AppModal`、`alert()` → `useFeedback`、自写字号/间距 → token。
这样门禁在 ④-1 结束时就能全仓强制，不必维护一份豁免名单。代价是 ④-2 会再动一次部分页面，接受。

**不在 ④ 范围**：shape-cleanups 的 A 类（后端错误体统一）与 B 类（吞错误）是后端行为变更，
另立小项目。④-1 只提供前端的错误态组件（`AsyncState` 的 error 态）。

## 3. 现状基线（2026-09-25 实测，`frontend/src`）

执行后记按同样口径对账。

| 指标 | 数值 |
|---|---|
| 前端总行数（`.vue` + `.ts`，不含 `.d.ts`） | 26 905 |
| `<style>` 块总行数 | **9 082**（34%） |
| `.vue` 文件中的 `px` 字面量（style + 模板 + 脚本） | 1 142 |
| `.vue` 中 hex 颜色 / `rgb(a)(` | 38 / 38 |
| `var(--` 引用 | 1 324 |
| 不同 `@media` 断点 | 10 种（480/600/640/767/768/900/992/400 …） |
| 页面 `max-width` 取值 | 19 种 |
| 字号写法 | token、`rem` 字面量、`px` 字面量、`var(--font-sm, 13px)` 四种并存 |
| 反馈方式 | `useMessage` 28 / `useDialog` 24 / `alert(` 11 / `confirm(` 3 / `GlobalAlert` |
| 弹窗写法 | `AppModal`、裸 `n-modal`、自写 overlay 三种 |
| JS 断点判断 | 5 处 `window.innerWidth`（阈值 768 / 992） |

跨文件重复定义的 scoped 类（同名类出现在 ≥ N 个文件）：
`.page-header` 10、`.page` 9、`.error-message` 9、`.section-header` 8、`.table-wrapper` 7、
`.header-title-row` 7、`expand-*` 过渡 5、`.empty-hint` 5、`.loading-message` 5、`.field` 5、
`.modal-header` 4、`.form-grid` 4、`.form-group` 4。

Naive UI 已是事实底座（`n-button` 148、`n-card` 20、`n-modal` 7），`n-data-table` 零使用；
原生 `<table>` 在 6 个文件（结算页 4 个）。

## 4. token 层

文件：`frontend/src/config/theme.ts`。**颜色值不改。**

### 4.1 新增

```ts
breakpoints: { phone: 640, tablet: 1024 }        // ≤640 手机；≤1024 平板
pageWidth:   { narrow: '640px', content: '960px', wide: '1280px' }
fontWeight:  { regular: 400, medium: 500, bold: 600 }
lineHeight:  { tight: 1.3, base: 1.6 }
```

两档断点的依据（设备矩阵，2026-09-25 与用户确认）：

| 角色 | 设备 |
|---|---|
| 管理端 | Windows 桌面 / 平板；**基本不在手机上用** |
| 摊主端 | 手机 / 平板 |
| 顾客端 | **平板**（立在摊位上自助点单，或摊主拿着代点单）；顾客自己扫码用手机的情况很少 |

- CSS：`postcss-custom-media` 提供 `@media (--phone)` / `@media (--tablet)`，
  定义放在 `styles/media.css`，由 `vite.config.ts` 的 postcss 配置注入。
  数值与 `tokens.breakpoints` 是同一个来源——`media.css` 由一个小脚本从 `theme.ts` 生成并入库，
  CI 校验二者一致（生成后 `git diff --exit-code`）。
- JS：新增 `composables/useViewport.ts`，内部是 `useBreakpoints(tokens.breakpoints)`，
  导出 `isPhone` / `isTablet`。5 处 `window.innerWidth` 全部改用它。
- CSS 变量：`--page-narrow/content/wide`、`--weight-*`、`--leading-*`。

### 4.2 删除

逐个统计 `generateCSSVariables` 输出的每个变量在 `src/` 的引用数：

- 零引用 → 从 `generateCSSVariables` 删掉；`ThemeColors` 中对应字段若也无其他消费者则一并删。
- 1～2 处引用且语义被更通用的 token 覆盖（候选：`--alert-*`、`--btn-secondary*`、`--order-completed`、
  `--warning-color-alt`、`--error-color-alt`）→ 引用点改用通用 token 后删除。
- 统计结果与处置清单写进 plan，不在本文预判。

### 4.3 Naive 主题几何项同步

`generateNaiveUITheme` 现在只同步颜色，Naive 组件的圆角、字号是另一套。补上 `common` 中的：
`borderRadius`（← `radius.md`）、`borderRadiusSmall`（← `radius.sm`）、
`fontSize` / `fontSizeSmall` / `fontSizeMedium` / `fontSizeLarge`（← `font.*`）、
`fontFamily`（见 4.4）、`lineHeight`。`Card` / `Modal` 的圆角取 `radius.lg`。

同时删掉 `NaiveCommonOverrides` 那两个已知无效的键（`borderColorHover` / `borderColorPressed`，
文件内注释已承认无效），去掉放宽类型用的索引签名。

### 4.4 全局样式

`assets/main.css` 拆为 `styles/`：

| 文件 | 内容 |
|---|---|
| `styles/base.css` | `body`、`#app`、字体栈、`font-variant-numeric` 等全局基线 |
| `styles/media.css` | custom media 定义（生成文件） |
| `styles/transitions.css` | `expand` / `fade` 过渡，替掉 5 份各自的 `expand-*` |
| `styles/utilities.css` | 仅 `.table-scroll`（横向滚动容器）等**极少量**有重复证据的工具类 |

- 删掉全局 `.btn`（唯一消费者是 `EventList.vue` 编辑弹窗，改 `n-button`）。
- 字体栈补中文：`system-ui, -apple-system, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', 'Noto Sans CJK SC', sans-serif`。
  不打包字体文件（离线、体积）。

## 5. 原语层 `frontend/src/components/ui/`

**准入标准：只收有重复证据的。** 都是 Naive 组件外的薄壳，不重新实现表单、按钮、输入。

### 5.1 `PageShell`

```vue
<PageShell title="展会管理" subtitle="创建和管理展会活动。" help="events" width="content">
  <template #actions>…</template>
  …正文…
</PageShell>
```

| prop | 类型 | 说明 |
|---|---|---|
| `title` | `string` | 必填，渲染为 `h1` |
| `subtitle` | `string?` | |
| `help` | `string?` | 传入即渲染 `HelpBubble :page="help"` |
| `width` | `'narrow' \| 'content' \| 'wide' \| 'full'` | 默认 `content` |

插槽：`default`、`actions`（页头右侧，`(--phone)` 下换行到标题下方）、`title`（需要自定义标题时，如 `AdminEventStat` 的动态标题）。
替掉：`.page` / `.page-header` / `.header-title-row` / `.header-content`，以及 19 种页宽。

### 5.2 `SectionCard`

基于 `n-card`。props：`title?`、`collapsible?`、`v-model:collapsed`。插槽：`default`、`extra`（标题右侧）、`footer`。
吸收 `shared/CollapsibleSection.vue`（3 个消费者迁完后删除）。替掉 `.section-header` 及各页的 `xxx-card` 外壳。

### 5.3 `AsyncState`

```vue
<AsyncState :loading="store.isLoading" :error="store.error" :empty="!list.length" @retry="load">
  <template #empty><EmptyState title="还没有展会" /></template>
  …正常内容…
</AsyncState>
```

优先级 `loading > error > empty > default`。`error` 为 `string | null`，非空即 error 态，
渲染 `n-alert type="error"` + 「重试」按钮（仅在监听了 `retry` 时出现）。
`loading` 可选 `loading-text`。`#empty` 插槽缺省时渲染 `<EmptyState compact title="暂无数据" />`。

### 5.4 `EmptyState`

`shared/EmptyGuide.vue` 改名迁入并规整。保留现有 props（`icon` / `title` / `desc` / `hint` 及同名插槽），
新增 `compact`（行内/表格内用，替掉 `.empty-hint` / `.empty-line`）。5 个消费者一并改名。

### 5.5 `AppModal`

现有 `shared/AppModal.vue` 升级后迁入，成为**唯一**弹窗写法。

| prop | 说明 |
|---|---|
| `show` | `v-model:show` |
| `title` | 或用 `#header` 插槽 |
| `size` | `'sm' \| 'md' \| 'lg'` → 480 / 640 / 960 px；`(--phone)` 下一律全屏 |
| `maskClosable` | 默认 `true`；有未保存表单的弹窗传 `false` |

插槽：`default`、`header`、`footer`（右对齐操作区）。关闭时 `emit('update:show', false)`。
**API 从 `@close` 改为 `v-model:show`**，现有消费者随迁移改写。
`ImageCropper` 若因全屏裁剪的交互需要保留裸 `n-modal`，在其文件内以带理由的豁免注释说明（见 §6.2）。

### 5.6 `StatTile`

props：`label`、`value`（`string | number`）、`hint?`、`tone?: 'default' | 'success' | 'warning' | 'error'`。
插槽 `value` 用于放 `<Money>`。替掉 `stat-card` / `summary-card` 外壳。

### 5.7 `Money`

props：`value: Cents`、`signed?`（正数显示 `+`）、`strike?`（划线原价）、`size?: 'sm' | 'md' | 'lg'`。
内部调用现有 `formatYuan`，`font-variant-numeric: tabular-nums`。**不接受裸 `number`**——沿用 ③b 的 branded `Cents`。

### 5.8 `useFeedback()`（`composables/useFeedback.ts`）

```ts
const fb = useFeedback()
fb.success('导出成功'); fb.error(e)            // e: unknown → 取 message，缺省「操作失败」
if (await fb.confirm({ title: '删除展会？', content: '…', danger: true })) { … }
```

- 底层是 Naive 的 `useMessage` / `useDialog`，**成为唯一入口**。
- `error(e: unknown, fallback?: string)` 统一「从异常里取文案」这件现在各处手写的事。
- 删除 `components/GlobalAlert.vue` 与 `stores/alertStore.ts`，11 处 `alert(`、3 处 `confirm(` 全部改写。
- 直接使用 `useMessage` / `useDialog` 的地方（grep 计 52 次，含 import 行）**也迁到 `useFeedback`**，门禁禁止在 `ui/` 之外直接引入这两个。

### 5.9 约定（不做成组件）

- **表单**：`n-form` + `n-form-item` + `n-grid`；删掉自写 `.field` / `.form-grid` / `.form-group`。
- **表格**：④-1 只把 7 处 `.table-wrapper` 换成全局 `.table-scroll`，**不改写表格本身**（改 `n-data-table` 属 ④-2）。
- **过渡**：只用 `styles/transitions.css` 里具名的过渡。

## 6. 门禁

### 6.1 stylelint

新增依赖：`stylelint`、`stylelint-config-recommended`、`stylelint-config-recommended-vue`、`postcss-html`、
`postcss-custom-media`（后者也是 vite 构建依赖）。`npm run lint` 串上 `stylelint "src/**/*.{vue,css}"`，
CI 的 `frontend` job 已跑 `npm run lint`，无需改 workflow。
不用 `stylelint-config-standard`：它带进大量与本次无关的风格规则，会制造一次无意义的全量改写。

规则（`ui/` 与 `styles/` 内同样适用，token 定义本身在 `theme.ts` 里，不在 CSS 中）：

| 属性 | 允许 |
|---|---|
| 所有颜色（`color` / `background*` / `border*-color` / `fill` / `stroke` / `outline-color` / `box-shadow` 内的颜色） | 仅 `var(--*)`、`transparent`、`currentColor`、`inherit` |
| `font-size` | `var(--font-*)`、`inherit` |
| `font-weight` | `var(--weight-*)`、`inherit` |
| `border-radius` | `var(--radius-*)`、`0`、`50%` |
| `box-shadow` | `var(--shadow-*)`、`none` |
| `gap` / `row-gap` / `column-gap` / `padding*` / `margin*` | `var(--space-*)`、`0`、`auto`（后者仅 margin）、`calc()` 内只含 token |
| `max-width` / `width` 取页宽档位时 | `var(--page-*)`、百分比、`none` |
| `@media` | 仅 `(--phone)` / `(--tablet)`，以及 `prefers-*` / `print` |

实现：`declaration-property-value-allowed-list` + `color-no-hex` + `color-named: never` +
`function-disallowed-list: [rgb, rgba, hsl, hsla]` + `media-query-no-invalid` 配合 custom media。
`width` / `height` 的一般像素值（图片、图标、摄像头取景框）**不设限**——那是内容尺寸不是设计刻度。

豁免：只允许行内 `/* stylelint-disable-next-line <rule> -- 理由 */`，
开 `reportDescriptionlessDisables`、`reportNeedlessDisables`、`reportInvalidScopeDisables`。
禁止文件级 `stylelint-disable`。

门禁自测：`frontend/scripts/stylelint-fixture/violations.vue` 覆盖上表每一行各一例违规，
vitest 里用 stylelint 的 Node API 跑它，**断言每条规则都报了错**。
这防的是「配置写错导致规则静默失效」——附录四记过三次门禁空转。

### 6.2 原语边界守卫

`frontend/scripts/check-ui-boundary.ts`，接进 `npm run lint`：

- 在 `src/`（`components/ui/` 与 `composables/useFeedback.ts` 除外）中禁止：
  - 裸 `alert(` / `confirm(` / `window.alert` / `window.confirm`
  - 从 `naive-ui` 导入 `useMessage` / `useDialog` / `NModal`
  - 模板中 `class` 含已被原语取代的类名：`page-header`、`section-header`、`error-message`、
    `loading-message`、`empty-hint`、`empty-line`、`modal-header`、`table-wrapper`
- 豁免写法：命中行上一行 `// ui-boundary-ignore: 理由`（模板里用 `<!-- ui-boundary-ignore: 理由 -->`），理由必填。
- **交叉校验**：脚本对自带的 fixture 目录跑一遍，每条规则必须命中至少一次，否则脚本自身失败。

## 7. 迁移范围与顺序

1. token 层 + `styles/` + Naive 几何同步 + `useViewport`
2. 原语 + 各自的组件测试
3. `useFeedback` 落地、删 `GlobalAlert`/`alertStore`
4. 门禁（先以 `--report-only` 跑出全仓违规清单，清单写进 plan 执行后记）
5. **逐文件机械迁移**：按违规数从少到多，每批 3～5 个文件；每批迁完该批文件对门禁零违规
6. 门禁切到强制，CI 全绿

每个文件的迁移只做：套原语、token 化、删掉因此成为死代码的 scoped 类。
**不改模板结构、不改交互、不改文案**（`alert('导出成功')` → `fb.success('导出成功')` 这种等价改写除外）。
遇到需要判断的（例如某个自写卡片不知该算 `SectionCard` 还是 `StatTile`），记入执行后记留给 ④-2，
本轮用最接近的原语或保留布局 CSS。

## 8. 测试与验收

- **组件测试**（vitest + `@vue/test-utils` + jsdom，已有）：每个原语至少覆盖——
  `AsyncState` 四态各渲染哪个插槽、`retry` 按钮仅在有监听时出现；
  `AppModal` 遮罩点击与关闭按钮都 `emit('update:show', false)`、`maskClosable=false` 时遮罩不关；
  `Money` 对 0、负数、`signed`、`strike` 的输出；
  `useFeedback().error` 对 `Error` / 字符串 / 未知值的文案提取、`confirm` 的 `Promise<boolean>`；
  `PageShell` 的 `help` 渲染 `HelpBubble`、`width` 映射到对应 CSS 变量。
- **门禁自测**：§6.1 与 §6.2 的 fixture 测试。
- **现有门禁**：lint / format:check / test:unit / typecheck / build 全绿；Rust 侧不受影响。
- **视觉回归（人工）**：controller 在 VNC 上按页面清单**亮、暗两套主题**各截迁移前后对照图，交用户过目。
  页面清单（含弹窗打开态）写进 plan。**此步不进 worker brief**（dev server 不退出，会卡死 headless worker）。
- **量化对账**：执行后记按 §3 口径重测，列出前后对比。不预设目标数，以门禁全绿为硬标准。
- **真机**：④-1 不改交互，真机验证并入 ④-2 结束时的 beta。
  但 ③b 遗留的真机清单（路线图附录四）仍建议在 ④-1 之前或期间走完。

## 9. 风险

| 风险 | 缓解 |
|---|---|
| 机械迁移悄悄改了布局 | 前后对照截图；每批文件小；不改模板结构 |
| Naive 几何项同步让所有 Naive 组件一次性变样 | 单独一个提交，截图对照后再往下走 |
| stylelint 规则写错而静默放过 | §6.1 fixture 断言每条规则都能报错 |
| `AppModal` 从 `@close` 改 `v-model:show`，漏改消费者 | `vue-tsc` strict 会拦住未声明的事件监听；另 grep `@close` |
| 删 token 误删有消费者的变量 | 删除前逐个 grep 计数（§4.2），清单进 plan |

---

## 附录：④-2 已达成的共识（2026-09-25）

④-2 另起 spec，下列结论起草时直接继承，不再重新讨论。

**范围：中度 IA 重构（方案 B）。** 不动三角色互斥的基本模型（那是方案 C，未采纳）。

**用户确认的 7 个问题，全部进 ④-2：**

1. 展会子菜单（商品 / 套装 / 订单 / 统计 / 结算）只在 URL 带 `:id` 时出现，离开即消失；
   侧栏「正在进行的展会」只列前 3 个。
2. 「主题设置」伪装成菜单项、实际弹 modal，高亮不跟随。
3. 系统类功能散在三处：控制台（局域网 / 安全 / v1 历史数据）、侧栏底部（检查更新 / 关于）、主菜单（使用教程）。
4. 展会 5 个子页平铺，看不出生命周期（展前：商品、套装 → 现场：订单、统计 → 收摊：结算）。
   方向：「展会工作台」，按生命周期分组。
5. 展会管理页上是创建表单、下是列表，编辑走弹窗且用了原生 `<button class="btn">`。
6. 摊主端标题是「待处理订单」，但同页还有订单 tab、实时统计、4 个弹窗；**收摊向导这个大流程塞在弹窗里**，应独立成页。
7. 摊主看不到结算单（角色互斥导致跳登录页，路线图附录三遗留）。

**摊主 ↔ 顾客切换（替代「收银台模式」）：**
用户的场景是平板既做自助点单、也由摊主拿着代点单。结论是**不做收银台模式**，只加切换按钮：

- 摊主端 → 「去点单」，跳本场展会的顾客点单页。
- 顾客点单页 → 「回摊主端」，**仅当本设备已登录该场展会的摊主时显示**。
- 顾客点单流程不变，下单照常进待处理。
- 约定：自助点单的平板不登录摊主，因此不加额外保护（长按 / PIN）。
- 可选：从顾客页切回时，URL 带上刚下的订单号，自动打开该单的收款弹窗（④-2 起草时再定）。

**从路线图附录继承的 ④ 待办**（附录二～四）：
`AdminEventLots` 彻底重做；从上一场展会复制套装配置（与选品、价格变更提示一起设计）；
订单页显示退货；收摊向导窄屏可用性（盘点逐行输入）；`components/vendor/` 三种弹窗写法（④-1 已收口）；
导出用 `alert()`（④-1 已收口）；原生表格改 `n-data-table`；`EditEventForm` 读不存在的 `vendor_password`。

**不在 ④：** shape-cleanups A 类（错误体统一 + 文案）与 B 类（吞错误），另立小项目。
