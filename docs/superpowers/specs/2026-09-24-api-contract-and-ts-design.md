# ③b API 契约与前端 TypeScript 设计

**日期**: 2026-09-24
**作者**: Renko_6626（与 Claude 协作）
**状态**: Draft — 排期 v1.2.0（子项目 ③b；吸收 ③a 的剩余部分）
**前置阅读**（按顺序，**动手前必读**）:

1. `docs/superpowers/specs/2026-09-22-v1.2-roadmap.md` 的 D5、D6，以及三段附录里「③b 处理的」「留给 ③b 的」
2. `src-tauri/src/error.rs` 的模块注释（`ApiError` 为什么必须是 `{"error": "..."}`）

**前置任务**: ①、②-1、②-2、②-3（均已完成）

---

## 这是什么

一句话：**后端改个字段名，CI 里的 `vue-tsc` 就红，而不是运行时在页面上显示 undefined。**

一条管线加一次迁移：

1. 后端所有 handler 的请求/响应收口成具名类型（③a 剩下的部分）
2. utoipa 从这些类型生成 `openapi.json`，入库并做快照门禁
3. openapi-typescript 生成 `schema.d.ts`，openapi-fetch 在其上提供带类型的 client
4. 前端全部 `.js` → `.ts`、全部 `.vue` → `lang="ts"`，strict，`vue-tsc` 进 CI 硬门禁

## 现状（2026-09-24 实测）

| 项 | 数字 / 状态 |
|---|---|
| 带路由的后端模块 | 17 个（`api/*.rs`，含 `#[cfg(feature = "vision")]` 的 `vision`） |
| `.route(` 调用 | 68 处，路径参数用 axum 0.7 的 `/:id` 语法 41 处 |
| 仍返回 `impl IntoResponse` 的 handler | 39 个（vision 11、master_product 8、event 6、admin 3、stats/society/product/lot 各 2、legacy/order/sync 各 1） |
| 已用 `ApiResult` 的模块 | settlement / closing / lot / inventory / product / society / order / refund / guard（② 写的） |
| 路由组装 | 每个模块已有 `pub fn router() -> Router<AppState>`，`api/mod.rs` 做 `nest`/`merge` |
| axum / tower / tower-http | 0.7 / 0.4 / 0.5 |
| 前端 `.vue` | 52 个，其中 3 个已是 `lang="ts"`（Help / About / UpdateModal） |
| 前端 `.js` | 35 个非测试 + 7 个 `.spec.js`；`.vue` 脚本段合计约 6.4k 行，`.js` 约 3.8k 行 |
| API 调用点 | 74 处 `api.*`，分布在 21 个文件；49 处读 `err.response?.data?.error` |
| `services/socketService.js` | **死代码**：零 import，后端无 socket.io。「实时订单」实为 `orderStore` 3 秒轮询 |

**与路线图的出入**：路线图写「44 个 .vue + 24 个 .js」，是起草时的数；② 新增了一批页面和 store。
路线图 ② 附录第 8 条（`socketService` 的 `FIXME(1.2)`）随死代码删除而消失。

## 已定决策

| # | 决策 | 理由 |
|---|---|---|
| K1 | **一份 spec、一份 plan、三个阶段**；阶段间串行，阶段内按文件并行 | 关键路径只在地基和 store；机械活（后端模块、`.vue`）文件互不相交，可铺开 |
| K2 | **client 用 openapi-fetch，去掉 axios** | 所有调用点反正要动一遍；类型覆盖路径/方法/参数/body/响应；axios 在现架构里只剩壳 |
| K3 | **阶段 2 严格保持 JSON 形状不变**，想改的记清单，阶段 3 前端有类型后统一改 | 阶段 2 并行改后端时前端还是 JS，无类型兜底，改形状会静默坏 |
| K4 | **axum 升 0.8**，上 utoipa 6 + utoipa-axum 0.3 | utoipa-axum 所有版本都要求 axum ^0.8；它让路由注册与文档同源，消除漂移，并去掉并行时唯一的共享冲突点 |
| K5 | `openapi.json` 与 `schema.d.ts` **都入库**，两道 diff 门禁 | 契约变更在 PR diff 里显形 |
| K6 | `Money` → TS branded `Cents` | 路线图附录三：否则「把分当元显示」TS 拦不住 |

---

## 1 契约管线

```
Rust handler + ToSchema 类型
   │  utoipa-axum: 各模块 router() -> OpenApiRouter<AppState>
   ▼
api/mod.rs 合并 → split_for_parts() → (axum::Router, utoipa::openapi::OpenApi)
   │  #[test] openapi_snapshot
   ▼
src-tauri/openapi.json（入库）
   │  npm run gen:api --prefix frontend（openapi-typescript + transform）
   ▼
frontend/src/api/schema.d.ts（入库）
   │  openapi-fetch createClient<paths>()
   ▼
api.GET('/events/{id}', { params: { path: { id } } })
```

### 1.1 后端

- 依赖：`utoipa = { version = "6", features = ["axum_extras", "chrono", "uuid"] }`、`utoipa-axum = "0.3"`。
- 每个模块的 `router()` 改为返回 `OpenApiRouter<AppState>`，用 `routes!(handler_a, handler_b)` 注册。
  `api/mod.rs` 只做 `nest`/`merge`，**不列 handler 清单**。
- **过渡期**：未迁移的模块用 `OpenApiRouter::from(module::router())` 接入（utoipa-axum 提供
  `From<Router<S>>`），路由照常工作，只是文档里没有它的路径。
  **阶段 2 完成标准之一是 `api/` 下 `OpenApiRouter::from(` 零命中。**
- `server.rs` 调 `api::router().split_for_parts()`，取 `Router` 挂到 `/api`，`OpenApi` 丢弃（运行时不暴露）。
- 每个 handler 标 `#[utoipa::path(...)]`：`tag` = 模块名，`responses` 列成功类型与各错误码，
  错误一律引用 `ApiErrorBody`（`{ error: string }`，给 `ApiError` 配的 schema 类型）。
- 请求 / 响应 / 查询参数都是具名结构体，`#[derive(Serialize|Deserialize, ToSchema)]`
  （查询参数 `IntoParams`）。`serde_json::Value` 只允许出现在确实是任意 JSON 的地方，并在注释里说明。
- **鉴权**：`OpenApi` 声明 bearer security scheme `bearer`。带 `AdminOnly` / vendor 守卫的 handler
  写 `security(("bearer" = []))`，无守卫的不写。path 的 `description` 注明需要哪个角色。
- **冻结例外**：已结算后仍可写的三个端点（垫付、结算调整、收摊清点），`description` 首句写明
  「展会已结算后仍可调用」。这是契约的一部分，不是实现细节。
- **multipart**：`request_body(content_type = "multipart/form-data", ...)`，字段用一个只用于文档的
  结构体描述。**原始字节**（`import_products_raw`）：`request_body(content_type = "application/octet-stream")`。
  **二进制响应**（xlsx 导出等）：`content_type` 写实际 MIME，body 为 `Vec<u8>`。
- `openapi.json` 按 **all-features** 生成（含 vision），vision 路径 tag 为 `vision`。
  与 CI 的 `cargo test --all-features` 一致。关掉 vision 时路由少了，但文档快照不随之变。
- `info(version)` **不**写入 App 版本号——否则每次 `set-version.sh` 都会让快照变红。固定写 `"1"`。

### 1.2 快照测试

`src-tauri/src/api/openapi_snapshot.rs`（`#[cfg(test)]`）：

- 构造 `api::router()`，`into_openapi()`，`serde_json::to_string_pretty` + 末尾换行。
- 与 `src-tauri/openapi.json` 逐字节比较，不等则失败，并提示 `UPDATE_OPENAPI=1 cargo test openapi_snapshot`。
- 环境变量 `UPDATE_OPENAPI=1` 时改为覆写文件并通过。
- 只在 `feature = "vision"` 开启时断言（否则跳过并打印原因），避免无 vision 的 `cargo test` 永远红。

### 1.3 前端生成

- `frontend/package.json` 加 `"gen:api": "node scripts/gen-api.mjs"`。脚本调 openapi-typescript 的
  Node API，读 `../src-tauri/openapi.json`，写 `src/api/schema.d.ts`，带 `transform` 钩子（见 1.4）。
- `schema.d.ts` 入库，**不许手改**（文件头有生成注释）；eslint / prettier 忽略它。
- CI `frontend` job：`npm run gen:api` 后 `git diff --exit-code src/api/schema.d.ts`。

### 1.4 Money

- Rust：`Money` 手写 `ToSchema`，输出 `{ "type": "integer", "format": "cents" }`。
  所有金额字段都已是 `Money`（② 的约定），不是则在类型化时改成 `Money`（JSON 不变，仍是整数）。
- TS：`frontend/src/utils/money.ts` 导出 `type Cents = number & { readonly __brand: 'Cents' }`。
  `gen-api.mjs` 的 `transform` 把 `format === 'cents'` 的 schema 映射为 `import('@/utils/money').Cents`。
- 元与分之间的换算、显示格式化**只**经 `money.ts` 的函数（现有 `money.js` 的函数签名加上 `Cents`）。
  需要从用户输入构造金额的地方用 `money.ts` 导出的 `toCents()`（唯一的 `as Cents` 所在）。

---

## 2 前端 client 层

新文件 `frontend/src/api/client.ts`，取代 `services/api.js`（阶段 1 两者并存，3.4 删旧的）。

```ts
const transportFetch: typeof fetch = …   // 原 tauriAdapter 的 localhost→原生 fetch / 其它→plugin-http 切换，逻辑原样搬
export const raw = createClient<paths>({ baseUrl, fetch: transportFetch })
raw.use(authMiddleware)          // sessionStorage access_token → Authorization: Bearer
raw.use(unauthorizedMiddleware)  // 401/403 且不在 /login → router.push('/login')
raw.use(uploadErrorMiddleware)   // FormData 请求失败 → 按 URL 前缀弹对应上传失败对话框（三条规则原样搬）

export class ApiRequestError extends Error { status: number; body: unknown }
export const api = { GET, POST, PUT, PATCH, DELETE }  // 成功返回 data；失败抛 ApiRequestError（message = body.error ?? 状态文本）
export function errorMessage(e: unknown, fallback = '网络错误'): string
```

- `baseUrl` 逻辑与现在一致：Tauri 内 `http://127.0.0.1:5140/api`，浏览器内 `/api`。
- 超时：每个请求带 `AbortSignal.timeout(30_000)`；超时抛出的错误也包成 `ApiRequestError`（status 0）。
- 请求日志（`[Req #id] … (ms) [native|plugin-http]`）保留，生产构建照旧由 esbuild 去掉。
- multipart 调用传 `FormData` + `bodySerializer: (b) => b`；原始字节传 `Uint8Array`，显式设 `Content-Type`。
- 二进制响应用 `parseAs: 'blob' | 'arrayBuffer'`。
- **不走这个 client 的**：updater 查 GitHub 等外部请求，保持现状。

### 2.1 错误处理约定

- 全仓 `err.response?.data?.error` → `errorMessage(e)`。**用户可见的提示文字一字不变**。
- 现存唯一一处空 `catch {}` 至少改为 `console.warn`。
- `client.ts` 有单测（vitest，mock `fetch`）：错误包装、`errorMessage`、401/403 跳转与 `/login` 例外、
  上传弹窗的三条 URL 路由、超时。

---

## 3 TypeScript 与 lint 规则

- `frontend/tsconfig.json`（+ `tsconfig.app.json` / `tsconfig.node.json` 按 create-vue 惯例拆）：
  继承 `@vue/tsconfig/tsconfig.dom.json`，`strict: true`，路径别名 `@/*`。
  `noUncheckedIndexedAccess` **不开**（对 naive-ui 表格/列数据噪音过大）。
- `vite.config.js` → `vite.config.ts`；vitest `include: ['src/**/*.spec.ts']`。
- `eslint.config.js` → `eslint.config.ts` 重做（路线图 ① 附录要求）：`typescript-eslint` recommended
  （**非** type-checked）+ `pluginVue` flat/essential + skipFormatting，外加：
  - `@typescript-eslint/no-explicit-any: error`
  - `@typescript-eslint/ban-ts-comment`：只允许带说明的 `@ts-expect-error`
- **逃生口**：全仓 `any` 为零；`@ts-expect-error` 每处必须写原因，只接受第三方类型缺陷一类理由，
  3.4 收口时逐条列入 plan 执行后记。
- `vue-tsc --noEmit` 作为 `npm run typecheck`；阶段 1 起进 CI 但 `continue-on-error: true`，**3.4 转硬门禁**。
- prettier 范围扩大到 frontend 根配置文件、`scripts/`、`docs/`、`.github/`（路线图 ① 附录），
  全量重排**单独一个提交**，不和逻辑改动混。

---

## 4 阶段与任务

### 阶段 1 · 地基（串行）

| # | 任务 | 完成标准 |
|---|---|---|
| 1.1 | axum 0.8 + tower 0.5 + tower-http 0.6：41 处 `/:x` → `/{x}`，`guard.rs` 两个、`auth.rs` 一个 extractor 去 `#[async_trait]` | `cargo test --all-features` 226 条全绿；clippy 干净；`tauri-env win cargo xwin build` 过 |
| 1.2 | 接 utoipa：依赖、`ApiErrorBody`、`Money` schema、bearer scheme、`mod.rs` 改 `OpenApiRouter`（全部模块先 `from` 接入）、`split_for_parts`、快照测试 | 生成的 `openapi.json` 有 components 无 paths；快照测试可红可更新 |
| 1.3 | **样板模块 `settlement.rs`**：先补形状快照测试钉住现有 JSON → 类型化 → 标注解（`SettlementReport` 优先；冻结例外的描述） | 快照测试不变；`openapi.json` 出现 settlement 全部路径；产出 `docs/superpowers/plans/…` 附带的「模块迁移说明」（阶段 2 brief 的模板） |
| 1.4 | 前端地基：tsconfig、eslint 重做、vite/vitest 转 TS、`gen:api` + Money transform、`api/client.ts` + 单测、CI 两道 diff 门禁、`typecheck` 非阻塞 | `npm run lint / test:unit / build / gen:api` 全过；`client.ts` 单测覆盖 2.1 所列行为 |

### 阶段 2 · 后端铺开 + 前端底层（并行，每批 3～4 个，文件不相交）

- **后端**：其余 16 个模块，一模块一 task，按 1.3 的模板：
  1. 用 `test_support::test_router()` 为该模块**每个路由**补一条形状快照测试（成功与主要错误路径），先绿
  2. 类型化（`impl IntoResponse` → `ApiResult<Json<T>>` 等），标 `#[utoipa::path]`，`router()` 改 `routes!`
  3. 第 1 步的测试不改一行照样绿
  4. 发现想改的形状，记入 `docs/superpowers/specs/2026-09-24-shape-cleanups.md`（字段、现状、建议、理由），**不动手**
- 顺序：先已用 `ApiResult` 的 closing / inventory / lot / order / refund / society / product，
  再 master_product / event / admin / stats / sync / legacy / auth / info / vision。
- `vision.rs` 只类型化和标注解，**不拆文件**。
- **worker 不提交 `openapi.json`**。每批结束由审查者统一 `UPDATE_OPENAPI=1` 重新生成并审 diff，一个提交。
- **前端**：`utils/`（7 个纯函数 + spec）、`config/`、`composables/` 逐个 `.js` → `.ts`（spec 同步 `.spec.ts`）。
  这批文件只被 `.vue` 和 store 引用，JS 侧通过 Vite 照常 import `.ts`，不断链。
- 阶段 2 完成标准：`api/` 下 `OpenApiRouter::from(` 零命中；`impl IntoResponse` 在 handler 签名中零命中
  （`ApiError` 自身的 impl 除外）；`openapi.json` 含全部 68 个路由。

### 阶段 3 · 前端上层

| # | 任务 | 方式 |
|---|---|---|
| 3.1 | `services/` 其余文件转 TS；删 `socketService.js` 与 `socket.io-client`；14 个 store 按依赖顺序转 `.ts` 并切到新 client | 串行，2～3 个一批 |
| 3.2 | 按 `shape-cleanups.md` 统一改后端形状，重新生成契约，跟着 `vue-tsc` 修消费方 | 串行，一个 task |
| 3.3 | 49 个 `.vue` → `lang="ts"`，按目录分批（每批 6～8 个，同批不共享子组件） | 并行 |
| 3.4 | 收口：删 `services/api.js` 与 axios；`typecheck` 转硬门禁；列出全部 `@ts-expect-error`；prettier 扩范围（单独提交） | 串行 |

### 执行方式

- 实现者 dsh-flash，审查者 Claude，2～3 个 task 一批，修复意见一次打包发回。
- 阶段 2、3 的批次用 Workflow 编排，单批不超过 10 个 agent。
- brief 必须自包含，**不含任何需要人的步骤**（不起 `tauri dev`、不开 VNC）。
- 本机命令前缀写进 brief：`cd src-tauri && tauri-env linux cargo …`。

---

## 5 测试与门禁

| 层 | 手段 | 何时生效 |
|---|---|---|
| 后端形状 | 每模块每路由一条 HTTP 形状快照测试 | 该模块迁移前 |
| 契约 | `openapi_snapshot` 测试 + `schema.d.ts` 重生成 diff | 1.2 / 1.4 起 |
| client | `client.ts` 单测 | 1.4 起 |
| 前端类型 | `vue-tsc --noEmit` | 1.4 起非阻塞，3.4 起硬门禁 |
| 回归 | Rust 226 条、前端 47 条全程保持绿（只增不减） | 全程 |
| 真机（发 beta 前） | 见下 | 本轮结束后 |

**真机清单（新增，本轮无法在开发机上验）**：上传付款二维码、上传商品预览图、boothpack 导入（FormData 与原始字节两条路）、
token 过期后的 401 跳转、xlsx 导出与保存、LAN 顾客端下单与摊主端 3 秒轮询。Windows、Android 各走一遍。

## 6 不在范围

- `vision.rs` 拆文件
- ④ 的任何 UI / 信息架构改动（包括 `AdminEventLots.vue` 重做）
- 需要类型信息的 lint 规则（`recommendedTypeChecked`、`no-floating-promises` 等）
- 运行时暴露 `openapi.json` 或 Swagger UI
- E2E / Playwright
- 用户可见行为的任何变化（提示文字、流程、页面）

## 7 风险

| 风险 | 缓解 |
|---|---|
| axum 0.8 遇旧路径语法会 panic，编译期不报 | panic 发生在构造 `Router` 时而非请求时；任何调用 `test_router()` 的测试都会构造完整路由树，1.1 即暴露 |
| 并行 worker 改出不一致的类型风格 | 1.3 的样板 + 「模块迁移说明」进 brief；每批审查者统一审 |
| 形状快照测试漏掉某个字段/路径 | 快照断言整段 JSON 而非挑字段；错误路径至少一条 |
| 换掉 axios 后传输层行为在真机上有差异 | `transportFetch` 逐行搬原逻辑；真机清单覆盖上传、导入、导出 |
| store 迁移牵动大量组件 | 3.1 期间 `.vue` 仍是 JS，不受类型约束；3.3 再统一修 |
| `schema.d.ts` 生成结果随 openapi-typescript 版本漂移 | 版本锁在 `package-lock.json`；升级时 diff 门禁会显形 |
