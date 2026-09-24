# ④-1 外观基础层 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把前端「每页各写一遍」的 CSS 收进 token + `components/ui/` 原语 + stylelint 门禁，全部页面机械迁移，门禁全仓强制。

**Architecture:** token 仍以 `config/theme.ts` 为唯一来源，补断点 / 页宽 / 字重 / 行高，Naive 主题同步几何项；
`styles/` 放全局基线、custom media、过渡、极少量工具类。原语是 Naive 外的薄壳，`useFeedback` 基于
`createDiscreteApi`（组件和 store 里都能用）。门禁 = stylelint（值域白名单）+ `check-ui-boundary.mjs`（禁用旧类名与旧反馈 API），
两者都带「故意违规」fixture 自测。**串行地基（Claude）→ 6 个 dsh-flash worker 并行机械迁移 → 收口（Claude）→ 强模型终审。**

**Tech Stack:** Vue 3.5 / TypeScript strict / Naive UI 2.43 / Pinia / vitest 5 + @vue/test-utils + jsdom / stylelint 16 + postcss-html + postcss-custom-media / Playwright（仅 controller 截图用）

**Spec:** `docs/superpowers/specs/2026-09-25-ui-foundation-design.md`

**前一份 plan:** `docs/superpowers/plans/2026-09-24-api-contract-and-ts.md`（并行派发规程与 worktree 脚本沿用）

---

## 这份计划怎么读

粒度刻意放粗（用户要求：计划不要太碎、一口气干完、不要反复审查-修改循环）。

| Task | 内容 | 执行者 |
|---|---|---|
| 1 | token 层 + `styles/` + Naive 几何同步 + `useViewport` + 基线截图 | Claude |
| 2 | 原语层 + `useFeedback`（删 `GlobalAlert` / `alertStore` / `useAlert`） | Claude |
| 3 | 门禁（stylelint + 边界脚本 + fixture 自测）、违规清单、worker brief、worktree 脚本加 `--no-target` | Claude |
| 4 | 批次 M：6 个 worker 并行机械迁移全部页面 | dsh-flash × 6 |
| 5 | 收口：删旧组件、门禁强制、对照截图、量化对账、执行后记 | Claude |
| 6 | 终审：强模型（Opus）整支分支审查并直接修 | Claude subagent |

**审查节奏**：批次 M 由 **2 个 Claude 审查 subagent**（各看 3 个 worker）审一次；Important 以上问题**打包成一次**修复派发；
一轮之后还剩的由 controller 直接改掉，不再派第二轮。Minor 不发回，controller 合并时顺手改或记入执行后记。

---

## Global Constraints

每个 task、每个 worker brief 都隐含包含本节。

- **颜色值一个不改**（`theme.ts` 的 `darkTheme` / `lightTheme`）。用户可配置项（明/暗、主色、商品图比例）行为不变。
- **不改后端、不新增迁移。** 不碰 `src-tauri/`。
- **机械迁移只做**：套原语、token 化、删因此成为死代码的 scoped 类。**不改模板结构、不改交互、不改文案**
  （`alert('x')` → `fb.success('x')` / `fb.error(…)` 这类等价改写除外）。
- **前端在 `frontend/`**：`npm --prefix frontend run lint|test:unit|build|typecheck|format:check`。
- **不要起 dev server。** `vite`、`npm run dev`、`npm run preview`、`tauri dev`、不带 `run` 的 `vitest` 永不退出。
- **断点**：`--phone` = `max-width: 640px`、`--tablet` = `max-width: 1024px`、`--not-phone` = `min-width: 641px`、`--desktop` = `min-width: 1025px`。
- **页宽**：`--page-narrow` 640px / `--page-content` 960px / `--page-wide` 1280px。
- **全仓 `any` 为零**；`@ts-expect-error` 只接受第三方类型缺陷并写原因；金额是 `Cents`，不写 `as Cents`。
- **提交信息 gitmoji 中文**，末尾 `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`。
- 每个 task 结束前全绿才提交：`lint` + `format:check` + `test:unit` + `typecheck` + `build`。

---

## Review Focus

spec 没点名、但最可能咬到真实用户的五类情况，均已指派到 task 的测试或验收步骤：

1. **在 store 里（组件 setup 之外）弹提示**（`customerStore` 现在就这么用 `useAlert`）→ 期望：`useFeedback()` 在 store 里调用不抛
   「inject() can only be used inside setup()」，照常弹出。→ **Task 2** 的 `useFeedback works outside component setup`。
2. **切换明/暗或改主色后弹出的提示**→ 期望：`useFeedback` 的 message/dialog 跟随当前主题，而不是永远亮色。
   → **Task 2** 的 `discrete api follows theme`（断言 `configProviderProps` 是随 themeStore 变化的 computed）。
3. **手机上打开有表单的弹窗**→ 期望：全屏、内容可滚动、底部操作区不被键盘外的区域遮住；`maskClosable=false` 时点遮罩不丢输入。
   → **Task 2** 的 `AppModal` 测试（遮罩不关）+ **Task 5** 的 phone 视口截图。
4. **`AsyncState` 的 error 与 empty 同时成立**（请求失败后列表为空）→ 期望：显示错误而不是「暂无数据」，否则用户以为真的没数据。
   → **Task 2** 的 `error wins over empty`。
5. **门禁配置写错而静默放过**（附录四记过三次门禁空转）→ 期望：每条规则都有一例违规 fixture，自测断言命中。
   → **Task 3** 的 fixture 测试；以及 **Task 5** 最后一步「故意写一行 `color: #fff` 看 CI 是否红」。

---

## Task 1: token 层 + 全局样式 + 基线截图

**Files:**
- Modify: `frontend/src/config/theme.ts`
- Modify: `frontend/src/stores/themeStore.ts`（仅在删 token 字段时需要）
- Create: `frontend/src/styles/{base,media,transitions,utilities}.css`、`frontend/src/styles/index.css`
- Delete: `frontend/src/assets/main.css`
- Modify: `frontend/src/boot.ts`（改 import `@/styles/index.css`）
- Modify: `frontend/vite.config.ts`（postcss custom media）
- Create: `frontend/src/composables/useViewport.ts`
- Test: `frontend/src/config/theme.spec.ts`
- Controller-only（不入库）：`$SCRATCH/shots/shoot.mjs`，`$SCRATCH` = 本会话 scratchpad

**Interfaces — Produces:**
- `tokens.breakpoints = { phone: 640, tablet: 1024 }`、`tokens.pageWidth`、`tokens.fontWeight`、`tokens.lineHeight`
- CSS 变量 `--page-narrow|content|wide`、`--weight-regular|medium|bold`、`--leading-tight|base`
- custom media `(--phone)` `(--tablet)` `(--not-phone)` `(--desktop)`
- `useViewport(): { isPhone: Ref<boolean>; isTablet: Ref<boolean> }`（`isTablet` 为 ≤1024，含手机）
- 全局类 `.table-scroll`；具名过渡 `expand`、`fade`

- [ ] **Step 1: 基线截图（controller 自己做，不进任何 brief）**

  1. 在 VNC :2 上后台起后端：`tauri-env vnc npx tauri dev`（仓库根，`run_in_background`），等 `curl -s 127.0.0.1:5140/api/...` 通。
  2. `npm --prefix frontend run build && npx --prefix frontend vite preview --port 4173 --strictPort`（后台；preview 继承 `server.proxy`，`/api` 转发到 5140）。
  3. `$SCRATCH/shots/shoot.mjs`：Playwright headless chromium，登录拿 admin 与 vendor token（走 `/api` 登录接口，
     token 写进 localStorage 的键名以 `stores/authStore.ts` 为准），对下表每个路由 × {亮, 暗}（写 localStorage `isDark`）×
     {1280×800, 390×844（仅摊主端与顾客端）} 截全页图到 `$SCRATCH/shots/before/`。
     路由清单：`/`、`/login/admin`、`/admin`、`/admin/events`、`/admin/master-products`、`/admin/societies`、
     `/admin/events/:id/{products,lots,orders,stats,settlement}`、`/admin/about`、`/admin/help`、`/vendor`、`/vendor/:id`、
     `/events/:id/order`；`:id` 取 dev 库里有订单的那场展会。弹窗打开态不截（太多路径依赖点击），由 Task 5 的人工点检覆盖。
  4. 截完**用显式 PID** 停掉 preview（`ps -eo pid,args | grep "[v]ite preview"`），后端留着 Task 5 再用或也停掉。

- [ ] **Step 2: 写失败测试 `theme.spec.ts`**

```ts
import { describe, it, expect } from 'vitest'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { tokens, generateCSSVariables, generateNaiveUITheme, lightTheme } from './theme'

const mediaCss = readFileSync(fileURLToPath(new URL('../styles/media.css', import.meta.url)), 'utf8')

describe('tokens', () => {
  it('media.css 与 tokens.breakpoints 同源', () => {
    const { phone, tablet } = tokens.breakpoints
    expect(mediaCss).toContain(`@custom-media --phone (max-width: ${phone}px);`)
    expect(mediaCss).toContain(`@custom-media --tablet (max-width: ${tablet}px);`)
    expect(mediaCss).toContain(`@custom-media --not-phone (min-width: ${phone + 1}px);`)
    expect(mediaCss).toContain(`@custom-media --desktop (min-width: ${tablet + 1}px);`)
  })
  it('输出新增的 CSS 变量', () => {
    const css = generateCSSVariables(lightTheme)
    for (const v of ['--page-narrow: 640px', '--page-content: 960px', '--page-wide: 1280px',
      '--weight-regular: 400', '--weight-medium: 500', '--weight-bold: 600',
      '--leading-tight: 1.3', '--leading-base: 1.6']) expect(css).toContain(v)
  })
  it('Naive 几何项来自 token', () => {
    const c = generateNaiveUITheme(lightTheme).common!
    expect(c.borderRadius).toBe(tokens.radius.md)
    expect(c.borderRadiusSmall).toBe(tokens.radius.sm)
    expect(c.fontSizeMedium).toBe(tokens.font.base)
    expect(c.fontFamily).toContain('PingFang SC')
    expect(generateNaiveUITheme(lightTheme).Card?.borderRadius).toBe(tokens.radius.lg)
  })
  it('不再输出无效的 Naive 键', () => {
    const c = generateNaiveUITheme(lightTheme).common as Record<string, unknown>
    expect(c.borderColorHover).toBeUndefined()
    expect(c.borderColorPressed).toBeUndefined()
  })
})
```

  Run: `npm --prefix frontend run test:unit -- src/config/theme.spec.ts` → FAIL。

- [ ] **Step 3: 实现 theme.ts 增量**

  - `DesignTokens` 加 `breakpoints` / `pageWidth` / `fontWeight` / `lineHeight` 四组，值按 Global Constraints 与 spec §4.1。
  - `export const fontFamily = "system-ui, -apple-system, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', 'Noto Sans CJK SC', sans-serif"`。
  - `generateCSSVariables` 输出 `--page-*`、`--weight-*`、`--leading-*`、`--font-family`。
  - `generateNaiveUITheme`：`common` 加 `borderRadius: radius.md`、`borderRadiusSmall: radius.sm`、
    `fontSize: font.base`、`fontSizeSmall: font.sm`、`fontSizeMedium: font.base`、`fontSizeLarge: font.md`、
    `fontFamily`、`lineHeight: String(lineHeight.base)`；新增 `Card: { borderRadius: radius.lg }`、`Dialog: { borderRadius: radius.lg }`。
    删掉 `borderColorHover` / `borderColorPressed` 两个无效键，`NaiveCommonOverrides` 改回 `NonNullable<GlobalThemeOverrides['common']>`（去掉索引签名）。

- [ ] **Step 4: token 删减（spec §4.2）**

  逐个变量统计引用：
  ```bash
  cd frontend/src && for v in $(grep -oE '^\s*--[a-z0-9-]+' <(node -e "…") ); do …; done
  ```
  实际做法：从 `generateCSSVariables` 的输出里取出全部变量名，对每个 `grep -rho "var($v[,)]" src | wc -l`，
  得到计数表，**贴进本 plan 末尾「执行后记」**。零引用的删；1～2 处且被通用 token 覆盖的，把引用点改到通用 token 后删。
  `ThemeColors` 字段只有在 `generateCSSVariables` / `generateNaiveUITheme` / `ThemeSetting.vue` 都不再用时才删。

- [ ] **Step 5: `styles/` 与 postcss**

  - `npm --prefix frontend i -D postcss-custom-media`。
  - `styles/media.css`：
    ```css
    /* 与 config/theme.ts 的 tokens.breakpoints 同源；theme.spec.ts 校验二者一致 */
    @custom-media --phone (max-width: 640px);
    @custom-media --tablet (max-width: 1024px);
    @custom-media --not-phone (min-width: 641px);
    @custom-media --desktop (min-width: 1025px);
    ```
  - custom media 定义要对**每个** SFC 的 `<style>` 可见：`vite.config.ts` 里
    `css: { postcss: { plugins: [postcssCustomMedia()] } }` 之外，用 `postcss-custom-media` 不支持跨文件导入定义，
    所以改用 `@csstools/postcss-global-data`（`npm i -D @csstools/postcss-global-data`）把 `src/styles/media.css` 注入每个文件：
    `plugins: [postcssGlobalData({ files: ['src/styles/media.css'] }), postcssCustomMedia()]`。
    vitest 走同一份 vite 配置，无需另配。
  - `styles/base.css`：从 `assets/main.css` 搬 `body` / `#app`，`font-family: var(--font-family)`，删 `.btn`。
  - `styles/transitions.css`：`expand`（max-height + opacity，照搬 5 份里最常见的那份实现）与 `fade`。
  - `styles/utilities.css`：`.table-scroll { overflow-x: auto; -webkit-overflow-scrolling: touch; }`。
  - `styles/index.css` 依次 `@import` 以上四个；`boot.ts` 改为 `import '@/styles/index.css'`；删 `assets/main.css`。
  - `EventList.vue` 编辑弹窗里的两个原生 `<button class="btn">` 换成 `n-button`（`.btn` 的唯一消费者）。

- [ ] **Step 6: `useViewport`**

```ts
// composables/useViewport.ts —— JS 侧断点的唯一入口，数值与 styles/media.css 同源（tokens.breakpoints）
import { useBreakpoints } from '@vueuse/core'
import { tokens } from '@/config/theme'

const bp = useBreakpoints({ phone: tokens.breakpoints.phone + 1, tablet: tokens.breakpoints.tablet + 1 })

export function useViewport() {
  return { isPhone: bp.smaller('phone'), isTablet: bp.smaller('tablet') }
}
```
  （`smaller('phone')` = `< 641` = `≤ 640`，与 CSS 的 `max-width: 640px` 一致。）
  测试加一条：mock `window.matchMedia` 返回 `(max-width: 640.9px)` 命中时 `isPhone` 为 true——或直接断言
  `useBreakpoints` 被传入 `{ phone: 641, tablet: 1025 }`（`vi.mock('@vueuse/core')`）。二选一，取实现简单的。
  **5 处 `window.innerWidth` 的替换留给 Task 4 的 worker**（在各自文件里）。

- [ ] **Step 7: 全绿、提交**

  `lint` / `format:check` / `test:unit` / `typecheck` / `build` 全过。
  `git commit -m "🎨 style: ④-1 token 补断点/页宽/字重/行高，Naive 同步几何项，全局样式拆进 styles/"`。
  **Naive 几何同步单独成一个提交**（spec §9 风险表）：先提交 Step 3 的 Naive 部分，截 3 张图（`/admin/events` 亮暗 + `/vendor/:id`）
  与基线比对，确认只是圆角/字号的预期变化，再提交其余。

---

## Task 2: 原语层 + useFeedback

**Files:**
- Create: `frontend/src/components/ui/{PageShell,SectionCard,AsyncState,EmptyState,AppModal,StatTile,Money}.vue`
- Create: `frontend/src/components/ui/index.ts`（具名再导出）
- Create: `frontend/src/composables/useFeedback.ts`
- Test: `frontend/src/components/ui/ui.spec.ts`、`frontend/src/composables/useFeedback.spec.ts`
- Modify: `frontend/src/App.vue`（去掉 `<GlobalAlert />`）
- Modify: `frontend/src/stores/customerStore.ts`（`useAlert` → `useFeedback`）
- Delete: `frontend/src/components/GlobalAlert.vue`、`frontend/src/stores/alertStore.ts`、`frontend/src/services/useAlert.ts`
  ——`VisionSearch.vue` 与 `CustomerView.vue` 里的 `useAlert` 由本 task 一并改成 `useFeedback`（等价改写，只动这几行），
  否则删文件后 build 断。
- **不删** `shared/AppModal.vue` / `shared/EmptyGuide.vue` / `shared/CollapsibleSection.vue`：Task 4 迁完消费者后由 Task 5 删。

**Interfaces — Produces（worker brief 原样引用）：**

```ts
// components/ui/index.ts
export { default as PageShell } from './PageShell.vue'     // props: title: string; subtitle?: string; help?: string; width?: 'narrow'|'content'|'wide'|'full'（默认 content）
                                                           // slots: default, actions, title
export { default as SectionCard } from './SectionCard.vue' // props: title?: string; collapsible?: boolean; collapsed?: boolean（v-model:collapsed）
                                                           // slots: default, extra, footer
export { default as AsyncState } from './AsyncState.vue'   // props: loading?: boolean; error?: string|null; empty?: boolean; loadingText?: string
                                                           // emits: retry; slots: default, empty, loading
export { default as EmptyState } from './EmptyState.vue'   // props: icon?: string; title: string; desc?: string; hint?: string; compact?: boolean
                                                           // slots: desc, hint, action
export { default as AppModal } from './AppModal.vue'       // props: show: boolean（v-model:show）; title?: string; size?: 'sm'|'md'|'lg'（默认 md）; maskClosable?: boolean（默认 true）
                                                           // emits: update:show; slots: default, header, footer
export { default as StatTile } from './StatTile.vue'       // props: label: string; value?: string|number; hint?: string; tone?: 'default'|'success'|'warning'|'error'
                                                           // slots: value, hint
export { default as Money } from './Money.vue'             // props: value: Cents|null|undefined; signed?: boolean; strike?: boolean; size?: 'sm'|'md'|'lg'

// composables/useFeedback.ts
export interface ConfirmOptions { title: string; content?: string; positiveText?: string; negativeText?: string; danger?: boolean }
export interface Feedback {
  success(msg: string): void
  info(msg: string): void
  warning(msg: string): void
  error(e: unknown, fallback?: string): void      // e 是 string → 原样；Error → e.message；其它 → fallback ?? '操作失败'
  confirm(opts: ConfirmOptions): Promise<boolean> // 确认 true；取消 / 关闭 / 点遮罩 false
  alert(opts: { title?: string; content: string; type?: 'info'|'success'|'warning'|'error' }): Promise<void> // 替代原 alertStore 的模态提示
}
export function useFeedback(): Feedback
export function errorText(e: unknown, fallback?: string): string // 纯函数，error() 内部用，也导出给测试
```

- [ ] **Step 1: 写失败测试**

  `ui.spec.ts`（`mount` + `global.plugins: [createPinia()]`；Naive 组件真渲染，jsdom 下可用）：

```ts
describe('AsyncState', () => {
  const slots = { default: '<p class="ok">ok</p>', empty: '<p class="e">empty</p>' }
  it('loading 优先', () => { const w = mount(AsyncState, { props: { loading: true, error: 'x', empty: true }, slots }); expect(w.find('.ok').exists()).toBe(false); expect(w.find('.e').exists()).toBe(false); expect(w.findComponent(NSpin).exists()).toBe(true) })
  it('error wins over empty', () => { const w = mount(AsyncState, { props: { error: '加载失败', empty: true }, slots }); expect(w.text()).toContain('加载失败'); expect(w.find('.e').exists()).toBe(false) })
  it('empty 渲染 empty 插槽', () => { const w = mount(AsyncState, { props: { empty: true }, slots }); expect(w.find('.e').exists()).toBe(true) })
  it('正常渲染 default', () => { const w = mount(AsyncState, { slots }); expect(w.find('.ok').exists()).toBe(true) })
  it('没有 retry 监听时不显示重试', () => { const w = mount(AsyncState, { props: { error: 'x' } }); expect(w.text()).not.toContain('重试') })
  it('有 retry 监听时显示并触发', async () => { const onRetry = vi.fn(); const w = mount(AsyncState, { props: { error: 'x', onRetry } }); await w.find('button').trigger('click'); expect(onRetry).toHaveBeenCalledOnce() })
  it('缺省 empty 插槽渲染紧凑 EmptyState', () => { const w = mount(AsyncState, { props: { empty: true } }); expect(w.text()).toContain('暂无数据') })
})
describe('AppModal', () => {
  it('关闭按钮 emit update:show false', …)
  it('maskClosable=false 时点遮罩不关', …)   // 通过 NModal 的 onMaskClick / 直接调 NModal 的 onUpdateShow 验证
  it('size 映射宽度 480/640/960', …)
})
describe('Money', () => {
  it.each([[0, '¥0.00'], [-150, '¥-1.50'], [12345, '¥123.45']])('value %i → %s', …) // 与 formatYuan 现有输出一致，不另造格式
  it('signed 给正数加 +', …); it('strike 加删除线类', …); it('null 显示 --', …)
})
describe('PageShell', () => {
  it('help 传入时渲染 HelpBubble', …); it('width=wide 用 --page-wide', …); it('title 渲染为 h1', …)
})
describe('SectionCard', () => { it('collapsible 点标题切换 collapsed 并 emit', …) })
describe('StatTile', () => { it('value 插槽优先于 value prop', …) })
```

  （测试体在实现时写全；每个 `…` 都是一条独立断言，不允许留空 `it`。）

  `useFeedback.spec.ts`：

```ts
it.each([
  ['字符串', '网络断了', undefined, '网络断了'],
  ['Error', new Error('boom'), undefined, 'boom'],
  ['空 message 的 Error', new Error(''), '保存失败', '保存失败'],
  ['未知值', { x: 1 }, undefined, '操作失败'],
  ['未知值 + fallback', 42, '删除失败', '删除失败'],
])('errorText: %s', (_n, e, fb, want) => expect(errorText(e, fb)).toBe(want))

it('useFeedback works outside component setup', () => {
  setActivePinia(createPinia())
  expect(() => useFeedback().success('ok')).not.toThrow()
})
it('confirm 确认 → true，取消 → false', async () => { /* vi.mock('naive-ui') 的 createDiscreteApi，捕获 dialog.warning 的 opts，调 onPositiveClick / onNegativeClick */ })
it('confirm 被关闭（onClose / onMaskClick）→ false', …)
it('discrete api follows theme', () => { /* 断言传给 createDiscreteApi 的 configProviderProps 是 ref/computed，切 themeStore.isDark 后其 .value.theme 变化 */ })
```

  Run → FAIL。

- [ ] **Step 2: 实现 `useFeedback`**

```ts
// composables/useFeedback.ts —— 反馈（toast / 确认 / 模态提示）的唯一入口。
// 基于 createDiscreteApi：不依赖组件 setup 上下文，store 里也能用；主题跟随 themeStore。
import { computed } from 'vue'
import { createDiscreteApi, darkTheme, type ConfigProviderProps } from 'naive-ui'
import { useThemeStore } from '@/stores/themeStore'

let api: ReturnType<typeof createDiscreteApi<['message', 'dialog']>> | null = null

function discrete() {
  if (!api) {
    const theme = useThemeStore()
    const configProviderProps = computed<ConfigProviderProps>(() => ({
      theme: theme.isDark ? darkTheme : null,
      themeOverrides: theme.naiveThemeOverrides,
    }))
    api = createDiscreteApi(['message', 'dialog'], { configProviderProps })
  }
  return api
}

export function errorText(e: unknown, fallback?: string): string {
  if (typeof e === 'string' && e) return e
  if (e instanceof Error && e.message) return e.message
  return fallback ?? '操作失败'
}
// …Feedback 各方法：confirm 用 dialog[danger ? 'error' : 'warning']({ title, content, positiveText: opts.positiveText ?? '确定',
//   negativeText: opts.negativeText ?? '取消', onPositiveClick: () => resolve(true), onNegativeClick / onClose / onMaskClick: () => resolve(false) })
//   用一个 settled 标志保证只 resolve 一次。alert 用 dialog[type]，只有 positiveText '确认'，任何关闭都 resolve。
```
  实现时核对 `createDiscreteApi` 在当前 naive-ui 版本的签名（`node_modules/naive-ui/es/discrete/src/discrete.d.ts`），
  若不接受 Ref 形式的 `configProviderProps`，改为在 `watch(isDark/overrides)` 时 `unmount()` 后重建。

- [ ] **Step 3: 实现 7 个原语**

  要点（样式全部用 token，本 task 的文件就要能过 Task 3 的门禁）：
  - `PageShell`：根 `div.page-shell`，`max-width: var(--page-<width>)`（`full` 为 `none`），`margin-inline: auto`；
    头部 flex，`h1` 用 `--font-xl` / `--weight-bold` / `--accent-color`（与现有 `.page-header h1` 一致）；
    副标题 `--text-muted` / `--font-base`；`help` 渲染 `<HelpBubble :page="help" />`；`@media (--phone)` 头部纵向排列。
  - `SectionCard`：`n-card` 包装，`title` 进 `#header`；`collapsible` 时标题可点、右侧箭头，内容用 `<Transition name="expand">`。
    视觉与 `shared/CollapsibleSection.vue` 对齐（它是 3 处消费者的现状）。
  - `AsyncState`：`loading` → `n-spin` + 文案；`error` → `n-alert type="error"` + 可选「重试」`n-button`
    （用 `getCurrentInstance()?.vnode.props?.onRetry` 判断是否有监听）；`empty` → `#empty` 插槽，缺省 `<EmptyState compact title="暂无数据" />`。
  - `EmptyState`：照 `shared/EmptyGuide.vue` 的模板与 props 搬过来，加 `compact`（单行、小字号、无图标）与 `#action` 插槽。
  - `AppModal`：`n-modal` + `n-card`，`style` 宽度 `sm 480 / md 640 / lg 960px`（`max-width: 92vw`）；
    `useViewport().isPhone` 为真时卡片 `width: 100vw; height: 100dvh; border-radius: 0` 且正文滚动；
    头部标题 + 关闭按钮；`#footer` 右对齐；`mask-closable` 透传；关闭一律 `emit('update:show', false)`。
  - `StatTile`：`label`（`--text-muted` `--font-sm`）+ 值（`--font-lg` `--weight-bold`，`tone` 映射 `--success-color` 等）+ `hint`。
  - `Money`：`<span class="money" :class="[size, { strike }]">{{ text }}</span>`，`text` = `signed && value > 0 ? '+' + formatYuan(value) : formatYuan(value)`，
    `font-variant-numeric: tabular-nums`。

- [ ] **Step 4: 删 GlobalAlert 链**

  `App.vue` 去掉 `<GlobalAlert />` 与 import；`customerStore.ts` / `VisionSearch.vue` / `CustomerView.vue` 里
  `useAlert().show(msg, { title, type })` → `useFeedback().alert({ title, content: msg, type })`（**保持模态**：原来是模态框，不降级成 toast）；
  删三个文件。`grep -rn "alertStore\|useAlert\|GlobalAlert" frontend/src` 应无输出。

- [ ] **Step 5: 全绿、提交**

  `git commit -m "✨ feat: ④-1 原语层 components/ui 与 useFeedback；删 GlobalAlert/alertStore"`。

---

## Task 3: 门禁 + 违规清单 + worker 基建

**Files:**
- Create: `frontend/stylelint.config.mjs`
- Create: `frontend/scripts/check-ui-boundary.mjs`
- Create: `frontend/scripts/fixtures/stylelint/violations.vue`、`frontend/scripts/fixtures/boundary/violations.vue`
- Test: `frontend/src/gates.spec.ts`
- Modify: `frontend/package.json`（`lint` 串联、devDeps）
- Modify: `frontend/eslint.config.ts`、`.prettierignore`（若 fixture 需要排除）
- Modify: `scripts/worktree-new.sh`（`--no-target`、分支前缀参数）
- Create: `docs/superpowers/plans/4-1/brief-migrate.md`（worker brief 模板，见附录 A）

- [ ] **Step 1: 依赖**

  `npm --prefix frontend i -D stylelint stylelint-config-recommended stylelint-config-recommended-vue postcss-html`

- [ ] **Step 2: `stylelint.config.mjs`**

```js
// 门禁：设计刻度只能来自 token（config/theme.ts → CSS 变量）。豁免只能行内写，且必须写理由：
//   /* stylelint-disable-next-line <rule> -- 理由 */
// 规则写错会静默放过——src/gates.spec.ts 用 scripts/fixtures/stylelint/violations.vue 断言每条都能报错。
const SPACE = String.raw`(?:var\(--space-[a-z0-9]+\)|0|auto|calc\((?:\s|var\(--space-[a-z0-9]+\)|[-+*/]|\d+(?:\.\d+)?)+\))`
const spaceList = new RegExp(`^${SPACE}(?:\\s+${SPACE}){0,3}$`)
const RADIUS = String.raw`(?:var\(--radius-[a-z0-9]+\)|0|50%)`
const radiusList = new RegExp(`^${RADIUS}(?:\\s+${RADIUS}){0,3}$`)

export default {
  extends: ['stylelint-config-recommended', 'stylelint-config-recommended-vue'],
  reportDescriptionlessDisables: true,
  reportNeedlessDisables: true,
  reportInvalidScopeDisables: true,
  rules: {
    'no-descending-specificity': null, // 存量代码大量触发，与本次目标无关
    'color-no-hex': true,
    'color-named': 'never',
    'function-disallowed-list': ['rgb', 'rgba', 'hsl', 'hsla'],
    'declaration-property-value-allowed-list': {
      'font-size': [/^var\(--font-[a-z0-9]+\)$/, 'inherit'],
      'font-weight': [/^var\(--weight-[a-z]+\)$/, 'inherit'],
      '/^border(-(top|bottom)-(left|right))?-radius$/': [radiusList],
      'box-shadow': [/^var\(--shadow-[a-z0-9]+\)$/, 'none'],
      '/^(row-|column-)?gap$/': [spaceList],
      '/^(padding|margin)(-(top|right|bottom|left|inline|block)(-start|-end)?)?$/': [spaceList],
      'max-width': [/^var\(--page-[a-z]+\)$/, /%$/, 'none', /^\d+(\.\d+)?(ch|em|rem|vw)$/, /^[1-3]?\d{1,2}px$/, /^min\(/],
    },
    'media-feature-name-disallowed-list': ['/width$/', '/height$/'], // 只允许 custom media 与 prefers-* / orientation / hover / pointer
    'comment-no-empty': true,
  },
  overrides: [{ files: ['**/*.vue'], customSyntax: 'postcss-html' }],
}
```
  实现时逐条对照 stylelint 16 文档确认规则名与 regex 写法（`declaration-property-value-allowed-list` 的 key 用 `/…/` 字符串表示 regex）。
  `(--phone)` 这类 custom media 不含 width 字样，不会被 `media-feature-name-disallowed-list` 误伤——fixture 里放一条合法用例证明。

- [ ] **Step 3: fixture 与 `gates.spec.ts`**

  `scripts/fixtures/stylelint/violations.vue`：`<style scoped>` 里**每条规则至少一例违规**，每例前一行注释标明期望的 rule 名，例如：
  ```css
  /* expect: color-no-hex */            .a { color: #fff; }
  /* expect: color-named */             .b { color: red; }
  /* expect: function-disallowed-list */ .c { background: rgba(0, 0, 0, 0.1); }
  /* expect: declaration-property-value-allowed-list */ .d { font-size: 14px; }
  … font-weight: 700 / border-radius: 6px / box-shadow: 0 1px 2px var(--shadow-color) / gap: 10px / padding: 5px 8px / margin-top: 3px / max-width: 900px
  /* expect: media-feature-name-disallowed-list */ @media (max-width: 768px) { .e { display: none; } }
  /* expect: (none) */ @media (--phone) { .f { padding: var(--space-sm) 0; gap: calc(var(--space-sm) * 2); } }
  ```
  另一份 `scripts/fixtures/stylelint/disables.vue` 覆盖：无理由的 `stylelint-disable-next-line`（期望报 descriptionless）。

  `gates.spec.ts`：
```ts
import stylelint from 'stylelint'
it('stylelint fixture：每条期望规则都命中，合法用例零告警', async () => {
  const code = readFileSync(fixture, 'utf8')
  const { results } = await stylelint.lint({ code, codeFilename: fixture, configFile: resolve('stylelint.config.mjs') })
  const hits = results[0].warnings.map((w) => ({ line: w.line, rule: w.rule }))
  const expected = [...code.matchAll(/expect: ([a-z-]+|\(none\))/g)] // 按行号配对：期望行的下一条声明必须有对应 rule 的告警；(none) 所在块必须零告警
  …
})
it('无理由的豁免会报错', …)
it('边界脚本：fixture 每条规则都命中', () => { const r = spawnSync('node', ['scripts/check-ui-boundary.mjs', '--self-test']); expect(r.status).toBe(0) })
it('边界脚本：src 下对干净文件零命中', …) // 用 --files 传一个 ui/ 组件
```

- [ ] **Step 4: `check-ui-boundary.mjs`**

  规则（spec §6.2）：扫描 `src/**/*.{vue,ts}`，排除 `src/components/ui/**`、`src/composables/useFeedback.ts`、`*.spec.ts`：

  | id | 匹配 |
  |---|---|
  | `native-dialog` | `/\b(window\.)?(alert|confirm)\(/`（排除 `.alert(`、`fb.confirm(` 这类成员调用：前一个字符不是 `.`） |
  | `naive-feedback` | 从 `naive-ui` 导入 `useMessage` / `useDialog` / `NModal`，或模板里 `<n-modal` |
  | `legacy-class` | 模板 `class="…"` / `:class` 字符串里出现 `page-header` `section-header` `error-message` `loading-message` `empty-hint` `empty-line` `modal-header` `table-wrapper`（按词边界） |
  | `legacy-component` | 导入 `shared/AppModal` / `shared/EmptyGuide` / `shared/CollapsibleSection` |
  | `inner-width` | `window.innerWidth` |

  豁免：命中行的**上一行**是 `// ui-boundary-ignore: <理由>` 或 `<!-- ui-boundary-ignore: <理由> -->`，理由非空。
  参数：`--files a b c`（只查这些）、`--self-test`（对 `scripts/fixtures/boundary/` 跑，每个规则 id 至少命中一次，且带理由豁免的那行不命中，否则 exit 1）、
  `--report`（只打印按文件汇总的计数，exit 0）。默认模式：有命中 exit 1，逐条打印 `file:line rule`。

- [ ] **Step 5: package.json**

```json
"lint": "eslint . --max-warnings 0 && npm run lint:style && npm run lint:boundary",
"lint:style": "stylelint \"src/**/*.{vue,css}\"",
"lint:boundary": "node scripts/check-ui-boundary.mjs"
```
  **本 task 先不串进 `lint`**（存量违规会让 CI 红）：先加 `lint:style` / `lint:boundary` 两个独立脚本，Task 5 才把它们串进 `lint`。
  eslint 忽略 `scripts/fixtures/**`；prettier 若改动 fixture 的违规写法就在 `.prettierignore` 排除。

- [ ] **Step 6: 违规清单与分批**

  ```bash
  cd frontend && npx stylelint "src/**/*.{vue,css}" -f json > $SCRATCH/stylelint-before.json || true
  node scripts/check-ui-boundary.mjs --report > $SCRATCH/boundary-before.txt
  ```
  按文件汇总计数，贴进执行后记。核对附录 B 的分批表（每个 worker 的违规总数大致均衡；Task 1/2 已由 Claude 处理掉的文件不在表内）。

- [ ] **Step 7: worktree 脚本**

  `scripts/worktree-new.sh` 加两个可选开关（保持旧用法兼容）：`--no-target`（不复制 `src-tauri/target/debug` 与 ONNX DLL——纯前端 worker 用不到，省 8G）
  和 `--prefix <p>`（分支前缀，默认 `3b`）。`.3b/` 工作目录名不变（已写进 exclude 与模板）。

- [ ] **Step 8: 写 brief 模板**（附录 A 原文落到 `docs/superpowers/plans/4-1/brief-migrate.md`）

- [ ] **Step 9: 全绿、提交**

  `git commit -m "🔧 chore: ④-1 stylelint 与原语边界门禁（带 fixture 自测），worker brief 与 worktree --no-target"`。

---

## Task 4: 批次 M —— 6 个 worker 并行机械迁移

按「并行派发规程」（沿用 ③b plan 同名一节，差异见下）执行。

| worker | 文件（`frontend/src/` 下） | style 行 |
|---|---|---|
| `m-customer` | `views/CustomerView.vue` `views/EventPortalView.vue` `components/customer/ProductGrid.vue` `components/customer/ShoppingCart.vue` `components/customer/PaymentModal.vue` | ~1710 |
| `m-event-ops` | `views/AdminEventProducts.vue` `views/AdminEventLots.vue` `views/AdminEventOrders.vue` `components/order/OrderCard.vue` | ~1370 |
| `m-event-report` | `views/AdminEventStat.vue` `components/stats/SalesLineChart.vue` `components/stats/StatFilters.vue` `views/AdminEventSettlement.vue` `views/MigrationNotice.vue` `views/AdminControlPanel.vue` | ~1440 |
| `m-product` | `views/AdminMasterProducts.vue` `components/product/*.vue`（5 个） `components/shared/ImageUploader.vue` `components/shared/ImageCropper.vue` | ~1270 |
| `m-vendor` | `views/VendorView.vue` `views/VendorEventSelection.vue` `components/vendor/*.vue`（5 个） `components/event/*.vue`（3 个） `views/AdminDashboard.vue` `views/AdminSocieties.vue` `views/LoginView.vue` | ~1280 |
| `m-shell` | `App.vue` `views/AdminLayout.vue` `components/shared/MainHeader.vue` `components/shared/VisionSearch.vue` `views/About.vue` `views/Help.vue` `views/ThemeSetting.vue` `components/shared/UpdateModal.vue` `components/shared/HelpBubble.vue` `views/NotFound.vue` `views/ServerError.vue` | ~1750 |

**与 ③b 规程的差异：**
- 开 worktree：`./scripts/worktree-new.sh --no-target --prefix 4-1 <name>`。
- 启动：6 个后台命令，**每个之间 `sleep 5`**（`~/.dsh/profiles/headless/cordis.yml` 争用；exit 1 且 stderr 是 cordis 报错就原样重派）。
  ```bash
  WT=/data/sunyunbo/www/ABL-wt/<name>
  dsh-flash --cwd "$WT" --timeout 3600 --log "$WT/.3b/worker.log" "$(cat "$WT/.3b/BRIEF.md")" > "$WT/.3b/final.md"
  ```
  不传 `--model`。
- 审查：**2 个 Claude 审查 subagent**，各负责 3 个 worktree，读 `REPORT.md` + `git -C <wt> diff`（worker 不提交，看工作树 diff），
  对照 brief 的「验收清单」出 `REVIEW.md`。只报 Important 以上 + 便宜的 Minor。
- 修复：最多一轮，打包派发；之后 controller 直接改。
- 合并：controller 在每个 worktree 里代为提交（`git -C <wt> add -A && git -C <wt> commit`），回主仓 `git merge --no-ff 4-1/<name> -m "🔀 merge: ④-1 <name>"`。
  文件不相交，冲突即说明切分出错。
- **批次验收**（全部合并后，主仓）：`npm --prefix frontend run lint:style` 与 `lint:boundary` 零告警；`lint` / `format:check` / `test:unit` / `typecheck` / `build` 全绿。

---

## Task 5: 收口

- [ ] **Step 1**：删 `components/shared/{AppModal,EmptyGuide,CollapsibleSection}.vue`；`grep -rn "shared/AppModal\|EmptyGuide\|CollapsibleSection" frontend/src` 无输出。
- [ ] **Step 2**：`package.json` 的 `lint` 串上 `lint:style` 与 `lint:boundary`（Task 3 Step 5 的最终形态）。CI 无需改（`frontend` job 已跑 `npm run lint`）。
- [ ] **Step 3**：**门禁真红测试**——临时在任一页面 `<style>` 加 `color: #fff;`，`npm --prefix frontend run lint` 必须 exit ≠ 0；
  临时加 `alert('x')`，同样必须失败。还原。
- [ ] **Step 4**：对照截图——复用 Task 1 Step 1 的脚本截 `after/`，逐对比较（同尺寸 + 像素差异百分比，`pixelmatch` 临时装在 scratchpad 里）。
  差异 > 2% 的逐张肉眼看：属于「token 取整」「Naive 几何同步」的预期变化记入执行后记；布局错位的直接修。
  另外**人工点检**（controller 用 Playwright 点开）：收款弹窗、退货弹窗、收摊向导、编辑母商品弹窗、图片裁剪器，
  1280 与 390 两个视口各一张，确认 `AppModal` 手机全屏与底部操作区。挑 8～12 张关键对照图拼成一张总览给用户。
- [ ] **Step 5**：量化对账——按 spec §3 同一口径重测（style 行数、px 字面量、hex/rgba、断点种类、max-width 种类、反馈方式计数、豁免注释条数），写进执行后记。
- [ ] **Step 6**：文档——本 plan 末尾写「执行后记」（做了什么、偏离、遗留给 ④-2 的判断点清单、真机待验）；
  路线图追加「附录六：④-1 执行后的状态」；`CLAUDE.md` 的日常命令里补 `npm --prefix frontend run lint:style`。
- [ ] **Step 7**：全绿，提交。

---

## Task 6: 强模型终审

派一个 **Opus** Claude subagent（`general-purpose`，`model: opus`），范围 `git diff 7b67ebf..HEAD -- frontend/ scripts/`，要求：
- 对照 spec 与本 plan 的 Global Constraints 找**正确性**问题：行为变化（文案、交互、弹窗模态性、`maskClosable`）、
  类型逃逸（`any`、`as never`、无理由 `@ts-expect-error`）、门禁漏洞（能绕过的写法）、原语 API 被误用、豁免注释滥用（无真实理由）。
- **直接修**，每类一个提交，跑全套门禁；无法判定的列清单回报，不改。
- 回报：修了什么、没修什么及理由。

controller 读回报，复核它的改动 diff，全绿后收尾。

---

## 附录 A：worker brief 模板（落到 `docs/superpowers/plans/4-1/brief-migrate.md`）

````markdown
# 任务：把以下前端文件机械迁移到 ④-1 的 token + 原语层，门禁零违规

你在一个独立的 git worktree 里工作：当前目录就是仓库根，分支 `4-1/{{NAME}}`。
**不要 `git commit`**（沙箱写不了 worktree 的 git 元数据）——改动留在工作树里，审查者代为提交。
完成后把报告写到 `.3b/REPORT.md`。

## 只改这些文件
{{FILES}}

不在清单里的文件一律不改（包括 `components/ui/`、`config/theme.ts`、`styles/`）。发现原语不够用，写进报告，不要自己扩原语。

## 绝对不要做
- 不要跑 `vite`、`npm run dev`、`npm run preview`、`tauri dev`、不带 `run` 的 `vitest`——它们永不退出。
- 凡是「在浏览器/VNC 上看一眼」「手动确认」的步骤一律跳过。不要截图。
- 不要 `npm install`。不要改文案、不要改交互、不要重排模板结构、不要拆组件。

## 可用的东西（已存在，直接 import）
```ts
import { PageShell, SectionCard, AsyncState, EmptyState, AppModal, StatTile, Money } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import { useViewport } from '@/composables/useViewport'
```
- `PageShell` props：`title: string`、`subtitle?`、`help?`（HelpBubble 的 page key）、`width?: 'narrow'|'content'|'wide'|'full'`；插槽 `actions`、`title`、默认。
- `SectionCard` props：`title?`、`collapsible?`、`v-model:collapsed`；插槽 `extra`、`footer`、默认。
- `AsyncState` props：`loading?`、`error?: string|null`、`empty?`、`loadingText?`；`@retry`；插槽 `empty`、`loading`、默认。
- `EmptyState` props：`icon?`、`title`、`desc?`、`hint?`、`compact?`；插槽 `desc`、`hint`、`action`。（旧名 `EmptyGuide`，props 相同）
- `AppModal` props：`v-model:show`、`title?`、`size?: 'sm'|'md'|'lg'`（480/640/960）、`maskClosable?`；插槽 `header`、`footer`、默认。
  **旧 `shared/AppModal` 的 `@close` 改成 `v-model:show`。**
- `StatTile` props：`label`、`value?`、`hint?`、`tone?`；插槽 `value`、`hint`。
- `Money` props：`value: Cents|null|undefined`、`signed?`、`strike?`、`size?`。
- `useFeedback()`：`success(msg)` `info(msg)` `warning(msg)` `error(e: unknown, fallback?)` `confirm({ title, content?, danger?, positiveText?, negativeText? }): Promise<boolean>`
  `alert({ title?, content, type? }): Promise<void>`。
- `useViewport()`：`{ isPhone, isTablet }`（Ref<boolean>；`isTablet` 含手机）。
- 全局类 `.table-scroll`（横向滚动容器）；全局过渡 `<Transition name="expand">`、`name="fade"`。

## 迁移规则（逐条照做）

1. **页面外壳**：`views/*.vue` 根部的 `.page` / `.page-header` / `.header-title-row` / `.header-content` 结构 → `<PageShell>`。
   原 `max-width` ≤ 700px → `width="narrow"`；701–1000 → `content`；> 1000 → `wide`；没有 → `full`。
   页头按钮放 `#actions`；`HelpBubble page="x"` → `help="x"`。
2. **三态**：`v-if="loading" … v-else-if="error" … v-else-if="!list.length" … v-else` 这种链 → `<AsyncState>`；
   只有空态的 → `<EmptyState>`（行内/表格内用 `compact`）。`EmptyGuide` → `EmptyState`（props 不变）。
   **文案原样搬**（`loading-text`、`EmptyState` 的 `title`）。
3. **弹窗**：`shared/AppModal`、裸 `<n-modal>`、自写 overlay（`position: fixed` + 遮罩）→ `<AppModal>`；
   原宽度 ≤ 520 → `sm`，≤ 760 → `md`，否则 `lg`。原来 `:mask-closable="false"` 的保持 `false`。
   `ImageCropper` 这类全屏交互如确实无法套 `AppModal`，保留 `n-modal` 并在上一行写 `<!-- ui-boundary-ignore: 理由 -->`。
4. **反馈**：`alert(x)` → 成功类 `fb.success(x)`、失败类 `fb.error(e, '原兜底文案')`；`confirm(x)` → `await fb.confirm({ title: x })`；
   `useMessage()` / `useDialog()` → `useFeedback()`：`message.success/error/warning/info` 一一对应；
   `dialog.warning({ title, content, positiveText, negativeText, onPositiveClick })` → `if (await fb.confirm({ title, content, positiveText, negativeText, danger: <原来是 error/warning 类型且是删除类操作> })) { 原 onPositiveClick 的内容 }`。
   注意 `onPositiveClick` 返回 Promise 时（等待期间按钮 loading）——改写后在 `if` 块里 `await`，行为等价。
5. **CSS token 化**（`<style>` 里，逐个值替换，取**最近**的档位）：
   - 间距（padding/margin/gap）：4→`--space-xs` 8→`--space-sm` 12→`--space-md` 16→`--space-lg` 24→`--space-xl` 32→`--space-2xl`；
     1–5px → `xs`，6–10 → `sm`，11–14 → `md`，15–20 → `lg`，21–28 → `xl`，≥29 → `2xl`；`rem` 先 ×16 换算。负 margin 用 `calc(-1 * var(--space-x))`。
   - 字号：≤12.5px（≤0.78rem）→ `--font-xs`；≤14.4（≤0.9rem）→ `--font-sm`；≤16（≤1rem）→ `--font-base`；≤18（≤1.12rem）→ `--font-md`；
     ≤21（≤1.3rem）→ `--font-lg`；≤26（≤1.6rem）→ `--font-xl`；更大 → `--font-2xl`。
     `var(--font-sm, 13px)` → `var(--font-sm)`。装饰性超大字号（emoji 图标 3rem 之类）→ `--font-2xl`，若明显变小影响观感则保留原值并写豁免注释。
   - 字重：400/normal → `--weight-regular`；500 → `--weight-medium`；600/700/bold → `--weight-bold`。
   - 圆角：≤5px → `--radius-sm`，6–9 → `--radius-md`，10–13 → `--radius-lg`，14–20 → `--radius-xl`，胶囊 → `--radius-pill`；`50%` 保留。
   - 阴影：按模糊半径就近取 `--shadow-sm/md/lg/xl`；用于焦点环的 `0 0 0 2px var(--x)` 保留并写豁免注释（理由：焦点环）。
   - 颜色：hex / 具名色 → 语义最接近的 `var(--*)`（见 `src/config/theme.ts` 的 `generateCSSVariables`）；
     `rgba(…)` 半透明 → `color-mix(in srgb, var(--x) N%, transparent)`；纯黑/白遮罩 → `var(--overlay-color)`；`white` 文字 → `var(--text-white)`。
   - 页宽：`max-width: 960px` 之类页面级宽度已由 `PageShell` 接管 → 删；内容级小宽度（≤399px，如截断文字）保留。
   - `@media (max-width: 480/600/640/767/768/800px)` → `@media (--phone)`；`(max-width: 900/992/1024px)` → `(--tablet)`；
     `(min-width: 768px)` → `(--not-phone)`；`(min-width: 992/1025px)` → `(--desktop)`；`(max-width: 600px) and (orientation: portrait)` → `(--phone) and (orientation: portrait)`。
6. **JS 断点**：`window.innerWidth <= 768` 之类 → `const { isPhone } = useViewport()`，768 → `isPhone`，992 → `isTablet`；删掉对应的 resize 监听。
7. **表格容器**：`.table-wrapper` → 模板 class 改 `table-scroll`，删 scoped 定义。表格本身不改。
8. **过渡**：本地 `expand-*` / `fade-*` 定义删掉，`<Transition name="…">` 名字对齐全局的 `expand` / `fade`。
9. **死类清理**：以上替换导致模板里不再引用的 scoped 类，删掉。**仍被引用的布局类（grid/flex/定位）保留**，只把其中的值 token 化。
10. 豁免只在确属内容几何（摄像头取景框、裁剪框、图表画布尺寸）或像素级对齐时使用，格式
    `/* stylelint-disable-next-line <rule> -- 理由 */`，每条都会被审查逐条看。

## 验证（全部通过才算完成）
    cd frontend
    npx stylelint {{FILES_REL}}                                   # 期望 0 告警
    node scripts/check-ui-boundary.mjs --files {{FILES_REL}}      # 期望 0 命中
    npm run lint
    npm run format:check        # 不过就 npx prettier --write <你的文件>
    npm run test:unit
    npm run typecheck
    npm run build

## 验收清单（审查者照此逐条看）
- [ ] 只改了清单内文件
- [ ] 文案零改动（`git diff` 里没有中文字符串的增删改，反馈 API 等价改写除外）
- [ ] 模板结构只多了原语包裹层，没有重排
- [ ] 原 `maskClosable=false` 的弹窗仍为 `false`；原模态提示仍为模态（`fb.alert`），没有降级成 toast
- [ ] `confirm` 改写后，取消分支什么都不做（与原来一致）
- [ ] 豁免注释每条都有真实理由
- [ ] 上述验证命令全过

## `.3b/REPORT.md` 格式
1. 每个文件：用了哪些原语、删了哪些 scoped 类、style 行数前→后
2. 取整导致可能有**可见变化**的地方（字号/间距跨了一档以上的），逐条列出
3. 所有豁免注释的位置与理由
4. 需要判断、本轮按最接近原语处理或保留的地方（留给 ④-2）
5. 验证各命令的结果
````

## 附录 B：分批依据

按 `<style>` 行数（2026-09-25 实测）与领域切分，文件互不相交。Task 1/2 已处理 `App.vue` 的 `GlobalAlert` 行、
`VisionSearch.vue` / `CustomerView.vue` 的 `useAlert` 行、`EventList.vue` 的 `.btn`——worker 在这些文件上看到的是已改过的版本，
只需做剩余迁移。`ChannelSelect.vue` / `SocietySelect.vue` 无 style，不分配；`components/ui/` 由 Task 2 写成门禁合规，不分配。
Task 3 Step 6 出违规清单后若某个 worker 的违规数明显偏多（> 均值 1.5 倍），在派发前把它的一个文件挪给最少的 worker。

---

## 执行后记（2026-09-25）

`1.2-dev` 上 `957ee63..` 本轮共 21 个提交（含 6 个 worker 合并）。执行者：Task 1–3、批次 M（6 个 worker 并行）、修复轮全部由 **dsh-flash（deepseek-flash）** 实现；
审查是 Claude：地基合审 1 次（sonnet）、批次合审 2 份（sonnet，各看 3 个 worker）、终审 1 次（opus，**直接修**）。修复只派了一轮，没有「审查-修改」来回。

### 偏离 plan 的裁定

- Task 1–3 原写「Claude 执行」，改派 dsh-flash（用户要求 worker 活交给 flash），审查合并成一次。
- 保留零引用的 `--space-*`（门禁与迁移依赖它们）；`#app` 的 `1600px` 改 `min(1600px, 100%)`（终审后改为带理由的行内豁免）。
- stylelint 实装为 **17.15**（plan 写 16），recommended 18 多出的 `declaration-property-value-keyword-no-deprecated` 一并清零。
- 修复轮后不派 scoped re-review，由 opus 终审覆盖。

### 迁移暴露的原语缺口（已补，只加不改）

批次审查发现，机械迁移在三个地方**必然**改变行为，原因是原语表达力不够，而不是 worker 做错了。于是在原语上补了能力，并逐处还原：

| 缺口 | 补的能力 | 还原了什么 |
|---|---|---|
| `AsyncState` 加载时整块替换内容 | `overlay` | 覆盖式 spinner（加载时旧表格仍可见） |
| `confirm()` 只能返回布尔 | `onConfirm` | `dialog.onPositiveClick` 的「操作结束才关、return false 不关」 |
| 没有常驻 loading 提示 | `loading(msg) → destroy` | 重置数据库期间的「正在重置…」 |
| `AppModal` 靠隐式 attrs 透传 | `closable` / `closeOnEsc` / `after-enter|leave`，`#header` 整行可排版 | 自写 overlay 没有 ×、下载中禁止 Esc、相机结果弹窗头部两端对齐 |
| toast 无法定制 | `success/info/warning/error` 可选 duration/closable/keepAliveOnHover | 各处原有的时长与关闭按钮 |
| 副标题只能纯文本 | `PageShell #subtitle` | 加粗的展会名、结算页页头里的日期 |

**教训**：「机械迁移、不改行为」这个约束，只有在原语能表达旧行为的全部变体时才做得到。原语是从「重复的样式」归纳出来的，但旧代码里藏着很多交互上的细微差别，比如确认框是否等待操作完成、toast 挂几秒。这些在样式统计里看不见，只有逐文件对照原代码才会暴露。

### 量化对账（口径同 spec §3）

| 指标 | 前 | 后 |
|---|---|---|
| 前端总行数（`.vue` + `.ts`） | 26 905 | 26 549 |
| `<style>` 块总行数 | 9 082 | 7 746（−15%） |
| `.vue` 中 `px` 字面量 | 1 142 | 443（余下多为内容尺寸 width/height 与豁免） |
| `.vue` 中 hex / `rgb(a)(` | 38 / 38 | 22 / 0（hex 全在模板/脚本数据：色板、SVG） |
| `@media` 写法 | 10 种断点 | 4 个 custom media |
| 页面 `max-width` | 19 种 | 页宽由 `PageShell` 3 档接管 |
| 反馈 API | `useMessage` 28 / `useDialog` 24 / `alert(` 11 / `confirm(` 3 / `GlobalAlert` | 全部收进 `useFeedback()` |
| `window.innerWidth` | 5 | 0（`useViewport`） |
| 行内豁免 | — | stylelint 29 条、边界 2 条，每条带理由 |
| 前端测试 | 218 | 245 |

style 行数只降了 15%，因为**机械迁移保留了布局类**（grid/flex/定位），只把其中的值换成 token。真正的收缩要等 ④-2 按页面重写。本轮的收益在「值域统一且锁死」：任何新写的颜色、字号、间距、断点、页宽都只能来自 token，这一点由门禁强制。

### 门禁

`npm run lint` = eslint + stylelint + `check-ui-boundary.mjs`，CI 的 `frontend` job 原样生效。
真红测试：页面里写 `color: #fff` → exit 2；写 `alert('x')` → 边界脚本 exit 1。
终审又堵了一批绕过写法（文件级 disable、`oklch()` / 系统色、同名覆盖 token 变量、`-webkit-` 变体、rem 页宽、`@container`、`globalThis.alert`、`import * as naive` 等），每条都有 fixture。

**仍然拦不住的**（spec 有意不管，或者代价太高）：`width` 用 px 设页宽；`var(--写错的名字)`（存量两处：`CustomerView` 的 `--text-color`、`AdminEventStat` 的 `--overlay-light`，修了会变色，留给 ④-2）；模板属性里的颜色（`<n-icon color="#…">`）与 inline `style`。

### 需要用户拍板 / 留给 ④-2

1. **删除类确认框的颜色**：7 处原来是 warning（黄），按 brief 迁成了 `danger: true`（红）。想还原的话，每处传 `type: 'warning'` 就行，一行的事。
2. **手机上 `AppModal` 全屏**（spec 规定的）：相机识别的结果弹窗在手机上会盖住整个取景画面，原来是浮在上面的。
3. `VendorView` 的「← 管理后台」从标题旁挪进了操作区；`AdminEventSettlement` 的警示条从页头上方挪到了页头下方。`PageShell` 没有对应的位置，留给 ④-2 的 IA 重排。
4. `PaymentModal`（全屏收款码页）仍是自写 overlay，没有换成 `AppModal`。
5. 页宽档位取整：720→960、1080/1100→1280、680/720/760→640、500→480。
6. 视觉上的预期变化：Naive 组件的字号从 14px 变成 15.2px（`--font-base`），圆角统一；页面从左对齐改为居中；`EmptyState` 的图标从 48px 缩到 32px。

### 真机

④-1 没有改交互，真机验证并入 ④-2 结束时的 beta。**③b 留下的真机清单（路线图附录四）仍然没走。**
