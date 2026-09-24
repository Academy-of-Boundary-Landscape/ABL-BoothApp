# 任务：把以下 `.vue` 文件迁到 `<script setup lang="ts">`，strict 零错误（③b 阶段 3）

你在一个独立的 git worktree 里工作：当前目录就是仓库根，分支 `3b/{{NAME}}`。
**不要 `git commit`**（沙箱写不了 worktree 的 git 元数据）——改动留在工作树里，审查者会代为提交。
完成后把报告写到 `.3b/REPORT.md`。

## 只改这些文件
{{FILES}}

## 规则
- `<script setup>` → `<script setup lang="ts">`；只改 script 段，**不碰 style**；template 只在类型需要时做最小调整（如 `?.`）。
- `defineProps` / `defineEmits` 用类型参数写法：
  `const props = defineProps<{ eventId: number; report: Schemas['SettlementReport'] | null }>()`，
  有默认值用 `withDefaults`；`defineEmits<{ (e: 'update:show', v: boolean): void }>()`。
- API 数据类型用 `import type { Schemas } from '@/api/client'`，不要手写重复 interface。
- 组件里直接调 API 的（`import api from '@/services/api'`），改用新 client：
  `import { api, unwrap, errorMessage } from '@/api/client'`；`x = await unwrap(api.GET('/path/{id}', { params: { path: { id } } }))`
  （路径不带 `/api`；FormData 作 body 时允许 `body: fd as never` 并注释「multipart」；二进制下载加 `parseAs: 'blob'`）。
- 错误信息：`err.response?.data?.error || '文案'` → `errorMessage(err, '文案')`，**文案一字不改**。
- 金额：类型是 `Cents`（`@/utils/money`）；显示走 `formatYuan` / `formatCents`；用户输入的元 → `toCents()`；
  对 Cents 做运算（合计、差额）后用 `cents(n)` 标回。不要自己写 `as Cents`。
- naive-ui 类型从 `naive-ui` 导入（`DataTableColumns<Row>`、`FormInst`、`SelectOption`、`FormRules` 等）；模板 ref：`ref<FormInst | null>(null)`。
- 禁止 `any`；`@ts-expect-error` 只限第三方类型缺陷并写原因。
- **行为零变化**。不要重构、不要拆组件、不要改样式和文案。不要起 dev server，不要 `npm install`。

## 验证（全部通过才算完成）
    npm --prefix frontend run lint
    npm --prefix frontend run format:check        # 不过就 npx --prefix frontend prettier --write <你的文件>
    npm --prefix frontend run test:unit
    npm --prefix frontend run build
    npm --prefix frontend run typecheck 2>&1 | grep "error TS" | grep -E "{{FILE_REGEX}}"    # 期望无输出

## `.3b/REPORT.md` 格式
1. 改了哪些文件；每个组件的 props / emits 类型
2. 类型报错而你判断是契约问题的地方（后端字段与前端用法不符）
3. 不确定的地方、`@ts-expect-error` 与 `as never` 清单、发现的疑似 bug
4. 验证各命令的结果
