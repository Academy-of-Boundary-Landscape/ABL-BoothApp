# 任务：把以下 store / service 迁到 TypeScript，并切换到新的带类型 API client（③b 阶段 3）

你在一个独立的 git worktree 里工作：当前目录就是仓库根，分支 `3b/{{NAME}}`。
**不要 `git commit`**（沙箱写不了 worktree 的 git 元数据）——改动留在工作树里，审查者会代为提交。
完成后把报告写到 `.3b/REPORT.md`。

## 只改这些文件
{{FILES}}

## 新 client（先通读 `frontend/src/api/client.ts` 顶部注释和 `frontend/src/api/core.ts`）
```ts
import { api, unwrap, errorMessage, type Schemas } from '@/api/client'

const report = await unwrap(
  api.GET('/events/{event_id}/settlement', { params: { path: { event_id: id } } })
)
await unwrap(api.POST('/events/{event_id}/advances', { params: { path: { event_id: id } }, body: payload }))
const list = await unwrap(api.GET('/events', { params: { query: { status: 'ongoing' } } }))
```
- 路径就是 `src-tauri/openapi.json` / `frontend/src/api/schema.d.ts` 里的路径，**不带** `/api` 前缀。
  路径、参数、body、响应都有类型检查。**报类型错误说明调用写错了、或后端契约与前端期望不符——不要用 `as` 压掉**，写进 REPORT。
- 旧写法 `const res = await api.get(url); x = res.data` → 新写法 `x = await unwrap(api.GET(...))`。
- FormData 直接作为 `body` 传（类型上可能需要 `body: fd as never`——**这是唯一允许的 `as`**，并在旁边注释「multipart」）。
  原始字节要显式带 `headers: { 'Content-Type': 'application/octet-stream' }`。二进制下载加 `parseAs: 'blob'`。
- state 类型直接用生成的 schema：`ref<Schemas['EventResponse'][]>([])`，**不要**手写重复的 interface。
- 金额字段在 schema 里已经是 `Cents`（`@/utils/money`）。用户输入的**元** → `toCents(yuan)`；
  对 Cents 做 + − × ÷（合计、差额）之后结果退化成 number，用 `cents(n)` 标回 Cents。不要自己写 `as Cents`。

## 规则
- 用 `git mv x.js x.ts` 改名（被沙箱拒绝就用普通 `mv`）。
- 错误对象现在是 `ApiRequestError`。**store 里**读错误信息一律 `errorMessage(e, '<原来的兜底文案>')`，文案一字不改。
  store 若把错误继续 throw 给组件：原样 throw，不要包装——组件（仍是 JS）靠 `ApiRequestError.response` 兼容 getter 继续读 `err.response.data.error`。
- 禁止 `any`；`@ts-expect-error` 只限第三方类型缺陷并写原因。
- **行为零变化**：不改 state 结构、action 名、getter 名、返回值形状、文案。
- 空 `catch {}` 至少改成 `catch (e) { console.warn('<store 名>.<action 名>', e) }`。
- 不要起 dev server，不要 `npm install`。

## 验证（全部通过才算完成）
    npm --prefix frontend run lint
    npm --prefix frontend run format:check        # 不过就 npx --prefix frontend prettier --write <你的文件>
    npm --prefix frontend run test:unit
    npm --prefix frontend run build
    npm --prefix frontend run typecheck 2>&1 | grep "error TS" | grep -E "{{FILE_REGEX}}"    # 期望无输出
    grep -n "services/api" {{FILE_LIST}}                                                     # 期望无输出

## `.3b/REPORT.md` 格式
1. 改了哪些文件
2. 每个 API 调用：旧写法 → 新路径（一行一个）
3. 类型报错而你判断是契约问题的地方（后端字段与前端用法不符）——这是最有价值的部分
4. 不确定的地方、`@ts-expect-error` 与 `as never` 清单、发现的疑似 bug
5. 验证各命令的结果
