# 任务：把以下前端文件从 JS 迁到 strict TypeScript（③b 阶段 2）

你在一个独立的 git worktree 里工作：当前目录就是仓库根，分支 `3b/{{NAME}}`。
**不要 `git commit`**（沙箱写不了 worktree 的 git 元数据）——改动留在工作树里，审查者会代为提交。
完成后把报告写到 `.3b/REPORT.md`。

## 只改这些文件（及其同名 `.spec.js`）
{{FILES}}

## 规则
- 用 `git mv x.js x.ts` 改名（spec 同理 `.spec.js → .spec.ts`）。如果 `git mv` 也被沙箱拒绝，就用普通 `mv`，审查者会处理。
- spec 里带扩展名的 import（`from './xxx.js'`）改成不带扩展名（`from './xxx'`）。
- tsconfig 是 strict（`frontend/tsconfig.app.json`）。**禁止 `any`**（eslint 报错）；确实拿不到类型的外部值用 `unknown` 再收窄。
  `@ts-expect-error` 只允许用于第三方库类型缺陷，且必须写原因（≥3 个字）。
- 金额用 `import type { Cents } from '@/utils/money'`；把 number 变成 Cents 只能调 `toCents()`。`money.ts` 已经迁完，不要改它。
- 导出函数写全参数与返回类型；导出的对象/配置写 `interface` 或 `as const`；函数内部局部变量靠推断即可。
- **行为零变化**：不改逻辑、不改文案、不改导出名、不改默认值。调用这些模块的 `.vue` / store 仍是 JS，它们照常 import（Vite 会解析 `.ts`），你不用改它们。
- 调用 `@/services/api`（旧 axios client）的文件（如 `composables/useConnectionCheck.js`、`utils/legacyExport.js`）：
  **这一批不切换 client**——保留 `import api from '@/services/api'`（它是 JS，TS 里会被推断成 axios 实例），只给其余部分加类型。
- 需要 naive-ui / vue / @tauri-apps 的类型时从对应包导入（`import type { GlobalThemeOverrides } from 'naive-ui'` 等）。
- 不要起 dev server，不要跑 `vitest`（不带 `run`）或 `vite`，不要 `npm install`。

## 验证（全部通过才算完成）
    npm --prefix frontend run lint
    npm --prefix frontend run format:check        # 不过就 npx --prefix frontend prettier --write <你的文件>
    npm --prefix frontend run test:unit
    npm --prefix frontend run build
    npm --prefix frontend run typecheck 2>&1 | grep "error TS" | grep -E "{{FILE_REGEX}}"    # 期望无输出

typecheck 整体仍会报 `src/views/Help.vue` 的 2 个错误，那是已知的、不归你管；只看你自己文件的错误。

## `.3b/REPORT.md` 格式
1. 改了哪些文件（旧名 → 新名）
2. 每个文件导出的类型/签名一览
3. 不确定的地方、`@ts-expect-error` 清单（没有就写「无」）、发现的疑似 bug
4. 验证各命令的结果
