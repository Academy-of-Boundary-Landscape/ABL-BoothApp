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
   边界门禁按**行**豁免：从 `naive-ui` 导入 `NModal` 的那一行同样命中 `naive-feedback`，也要在上一行写 `// ui-boundary-ignore: 理由`。
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
    豁免必须**既必要（真的压住了违规）又带理由**：门禁开了 `reportNeedlessDisables` / `reportDescriptionlessDisables`，多余的豁免也会报错。
11. **stylelint 17 recommended 的额外规则**：`word-break: break-word` 会命中 `declaration-property-value-keyword-no-deprecated`（与 token 无关，也不该改行为）。
    保留原写法，在上一行写 `/* stylelint-disable-next-line declaration-property-value-keyword-no-deprecated -- 保留原关键字，不做行为变更 */`。

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
