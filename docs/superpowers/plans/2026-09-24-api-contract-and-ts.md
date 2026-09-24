# ③b API 契约与前端 TypeScript 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 后端 68 个路由全部收口成具名类型并生成 `openapi.json`，前端换成由它生成的带类型 client，全部 `.js`/`.vue` 迁到 strict TS，`vue-tsc` 进 CI 硬门禁。

**Architecture:** axum 升 0.8 后接 utoipa 6 + utoipa-axum 0.3，每个 `api/*.rs` 的 `router()` 返回 `OpenApiRouter`，路由与文档同源；一个 `#[test]` 把文档快照成 `src-tauri/openapi.json`。前端 `gen:api` 用 openapi-typescript 生成 `schema.d.ts`（`format: cents` → branded `Cents`），openapi-fetch 在其上建 client，取代 axios。**串行地基 → 按文件铺开 → 收口**，铺开阶段每个 worker 一个独立 git worktree。

**Tech Stack:** Rust / axum 0.8 / tower 0.5 / tower-http 0.6 / utoipa 6 / utoipa-axum 0.3 / sqlx 0.9 · Vue 3 / Pinia / TypeScript 5.8 / vue-tsc 3 / openapi-typescript 7 / openapi-fetch 0.17 / vitest 5 / typescript-eslint

**Spec:** `docs/superpowers/specs/2026-09-24-api-contract-and-ts-design.md`

**前一份 plan:** `docs/superpowers/plans/2026-09-24-event-closing-and-settlement.md`（末尾「交给后续」与「执行后记」）

---

## 这份计划怎么读

**粒度是刻意放粗的。** 13 个 task，其中 4 个是「批次」：一个批次里并行派出多个 dsh-flash worker，
每个 worker 负责一个模块或一组文件。批次内的每个 worker 都拿同一份模板 brief（见「附录 A / B / C」），
只换变量。审查以**批次**为单位做，修复意见**一次打包**发回，最多一轮；一轮之后还剩的小问题由审查者自己改掉，
不再来回（见「并行派发规程」第 5 步）。

| 阶段 | Task | 执行者 | 形态 |
|---|---|---|---|
| 1 地基 | 1 axum 0.8 升级 | Claude | 串行 |
| | 2 utoipa 骨架 + 契约快照 | Claude | 串行 |
| | 3 样板模块 settlement + 并行基建 | Claude | 串行 |
| | 4 前端 TS 地基 + client | Claude | 串行 |
| 2 铺开 | 5 批次 B1：后端 closing / inventory / lot / order / refund ＋ 前端底层 | dsh-flash × 6 | 并行 |
| | 6 批次 B2：后端 society / product / info / auth / stats | dsh-flash × 5 | 并行 |
| | 7 批次 B3：后端 master_product / event / admin / sync / legacy / vision | dsh-flash × 6 | 并行 |
| | 8 阶段 2 收口 | Claude | 串行 |
| 3 上层 | 9 批次 F1：services + 14 个 store 切新 client | dsh-flash × 4 | 并行 |
| | 10 形状清理 | Claude | 串行 |
| | 11 批次 F2：49 个 `.vue` → `lang="ts"` | dsh-flash × 5 | 并行 |
| | 12 收口：删 axios、硬门禁、prettier 扩范围 | Claude | 串行 |
| | 13 文档与交接 | Claude | 串行 |

---

## Global Constraints

每个 task、每个 worker brief 都隐含包含本节。brief 模板里已经原样抄了与 worker 相关的几条。

- **构建命令必须带前缀。** 裸 `cargo` 缺 webkit2gtk 必失败。Rust 侧一律在 `src-tauri/` 下跑
  `tauri-env linux cargo fmt` / `tauri-env linux cargo clippy --all-targets --all-features -- -D warnings` /
  `tauri-env linux cargo test --all-features`。
- **前端在 `frontend/`，不在仓库根。** `npm --prefix frontend run lint|test:unit|build|typecheck|gen:api`。
- **不要起 dev server。** `npx tauri dev` / `vite` / `vitest`（不带 `run`）永不退出，会卡死整批任务。
- **用户可见行为零变化**（spec §6）：提示文字、流程、页面、HTTP 状态码。
- **阶段 2 严格保持 JSON 形状不变**（spec K3）。想改的只记入 `docs/superpowers/specs/2026-09-24-shape-cleanups.md`。
  原来吞错误（`unwrap_or_default()` 之类）的地方**继续吞**，同样记入清单。
- **错误响应体形状必须是 `{"error": "..."}`**（`src-tauri/src/error.rs` 模块注释）。
- **不得改动 `src-tauri/migrations/` 下任何文件。** 本轮没有 schema 变更。
- **`openapi.json` 的 `info.version` 固定为 `"1"`**，不写 App 版本号。
- **worker 不提交 `src-tauri/openapi.json` 与 `frontend/src/api/schema.d.ts`**；由审查者在批次合并后统一重新生成。
- **全仓 `any` 为零**；`@ts-expect-error` 必须写原因，且只接受第三方类型缺陷一类理由。
- **金额在 TS 侧是 `Cents`**（`frontend/src/utils/money.ts`），唯一的 `as Cents` 在 `money.ts` 里。
- **每个 task / worker 结束前必须全绿才提交**：Rust 侧 fmt + clippy + test；前端侧 lint + test:unit + build（阶段 3 起加 typecheck 对自己文件零错误）。
- **提交信息用 gitmoji 中文格式**，末尾带 `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`（worker 提交同样带这一行）。

---

## Review Focus

spec 没点名、但最可能咬到真实用户的五类输入。每一条都已指派给拥有那段代码的 task，并写成了该 task 里的测试：

1. **后端没起来 / 网络断了**（`fetch` 直接 reject 成 `TypeError`）→ 期望：包成 `ApiRequestError(status 0)`，
   `errorMessage(e, '加载失败')` 给出调用方的兜底文案，`useConnectionCheck` 判为断线而不是崩。
   → **Task 4** 的 `network failure becomes status 0`。
2. **错误响应不是 JSON**（axum 的 413 `DefaultBodyLimit`、`DebugJson` 的纯文本 400、Json 提取器的 422）→
   期望：`serverMessage` 为 `undefined`，走调用方兜底文案，而不是把整段 HTML/文本或 `undefined` 显示给用户；
   上传场景 413 仍弹「文件可能超过当前限制」。→ **Task 4** 的 `plain-text error body` 与 `413 upload`。
3. **调用方自己传了 `signal`**（`useConnectionCheck` 的 3 秒探测）→ 期望：调用方的 abort 与 30 秒全局超时**都**生效，
   谁先到谁中止。→ **Task 4** 的 `caller signal and global timeout both abort`。
4. **成功响应没有 body**（`StatusCode::OK` / `NO_CONTENT` 的删除、reconcile）→ 期望：`unwrap` 返回 `undefined`，
   不抛 JSON 解析错。→ **Task 4** 的 `empty success body`。
5. **已在 `/login` 页上收到 401**（登录失败本身就是 401「密码错误」）→ 期望：不跳转、不死循环，
   错误照常抛给登录页显示「密码错误」。→ **Task 4** 的 `401 on /login does not redirect`。

---

## 文件结构

**新建：**

| 文件 | 职责 |
|---|---|
| `src-tauri/openapi.json` | 契约快照，生成物，入库 |
| `src-tauri/src/api/openapi.rs` | `ApiDoc`（info / security scheme / 公共 schema）、`ApiErrorBody`、快照测试 |
| `frontend/tsconfig.json` `tsconfig.app.json` `tsconfig.node.json` `tsconfig.vitest.json` | TS 工程配置 |
| `frontend/env.d.ts` | vite client 类型 + `window.__TAURI_INTERNALS__` |
| `frontend/scripts/gen-api.mjs` | openapi.json → schema.d.ts，带 Money transform |
| `frontend/src/api/schema.d.ts` | 生成物，入库，不许手改 |
| `frontend/src/api/client.ts` | openapi-fetch client、传输切换、三个 middleware、`ApiRequestError`、`unwrap`、`errorMessage` |
| `frontend/src/api/client.spec.ts` | client 单测（含 Review Focus 1～5） |
| `scripts/worktree-new.sh` `scripts/worktree-rm.sh` | 并行 worker 的独立 worktree 起停 |
| `docs/superpowers/plans/3b/brief-backend.md` `brief-frontend-lower.md` `brief-store.md` `brief-vue.md` | worker brief 模板（附录 A～D 的落盘版） |
| `docs/superpowers/specs/2026-09-24-shape-cleanups.md` | 阶段 2 记下的「想改的形状」 |

**修改（主要）：** `src-tauri/Cargo.toml`、`src-tauri/src/api/*.rs`（全部）、`src-tauri/src/server.rs`、
`src-tauri/src/domain/money.rs`、`src-tauri/src/test_support.rs`、`src-tauri/src/utils/security.rs`、
`frontend/package.json`、`frontend/vite.config.js → .ts`、`frontend/eslint.config.js → .ts`、
`frontend/src/**/*.js → .ts`、`frontend/src/**/*.vue`、`.github/workflows/ci.yml`。

**删除：** `frontend/src/services/socketService.js`、`frontend/src/services/api.js`（Task 12）。

---

## 并行派发规程（Task 5/6/7/9/11 共用）

批次 task 的正文只写「派哪些 worker、各自变量是什么、批次验收」。怎么派、怎么审、怎么合，统一按这里：

1. **开 worktree**（每个 worker 一个，基于当前 `1.2-dev` HEAD）：
   ```bash
   ./scripts/worktree-new.sh <name>        # → /data/sunyunbo/www/ABL-wt/<name>，分支 3b/<name>
   ```
2. **落 brief**：把对应模板（`docs/superpowers/plans/3b/brief-*.md`）的 `{{变量}}` 替换后写到
   `<wt>/.3b/BRIEF.md`（`.3b/` 已在 worktree 的 `info/exclude` 里，不会被提交）。
3. **并行启动**（每个 worker 一条后台命令，**不要**用 `--batch`——它只支持一个 `--cwd`）：
   ```bash
   WT=/data/sunyunbo/www/ABL-wt/<name>
   dsh-flash --cwd "$WT" --timeout 3600 --log "$WT/.3b/worker.log" "$(cat "$WT/.3b/BRIEF.md")" > "$WT/.3b/final.md"
   ```
   不指定 `--model`（用默认的 deepseek-flash）。等全部退出。
4. **审查**：审查者对每个 worker 读 `<wt>/.3b/REPORT.md` 和 `git -C <wt> diff 1.2-dev...HEAD`，按 brief 里的
   「验收清单」逐条过。审查意见按 worker 汇总成一份 `<wt>/.3b/REVIEW.md`。
5. **修复最多一轮**：有 Important 以上问题的 worker，带 `BRIEF.md` + `REPORT.md` + `REVIEW.md` 重新派一次
   （dsh-flash 无法 resume，REPORT 就是它的记忆）。**一轮之后仍存在的问题由审查者直接在该 worktree 里改掉**，不再派第二轮。
   Minor 问题不发回，审查者合并时顺手改或记入 Task 13 的遗留清单。
6. **合并**：回到主仓库，逐个 `git merge --no-ff 3b/<name> -m "🔀 merge: ③b <name>"`（文件不相交，不应冲突；
   若冲突说明切分出错，停下来查）。全部合并后跑**批次验收**，然后 `./scripts/worktree-rm.sh <name>` 清理。

---

## Task 1: axum 0.8 升级

**Files:**
- Modify: `src-tauri/Cargo.toml`（axum / axum-server / tower / tower-http）
- Modify: `src-tauri/src/api/*.rs`（41 处 `/:x` 路径）
- Modify: `src-tauri/src/api/guard.rs:8-24,79`、`src-tauri/src/api/auth.rs:1-60`（去 `#[async_trait]`）
- Modify: `src-tauri/src/test_support.rs`（如有 0.7 专属 API）

**Interfaces:**
- Produces: 所有路由改用 `/{param}` 语法；`Claims` / `AdminOnly` / `DebugJson<T>` 行为与签名不变。

- [ ] **Step 1: 改依赖**

```toml
axum = { version = "0.8", features = ["multipart"] }
axum-server = { version = "0.7", features = ["tls-rustls"] }   # 若与 axum 0.8 不兼容，升到支持 0.8 的最低版本
tower = { version = "0.5", features = ["full"] }
tower-http = { version = "0.6", features = ["cors", "fs", "trace"] }
```

跑 `cd src-tauri && tauri-env linux cargo update -p axum -p tower -p tower-http`，看编译错误清单。

- [ ] **Step 2: 路径语法**

```bash
cd src-tauri
grep -rnE '"[^"]*/:[a-z_]+' src --include=*.rs     # 应为 41 处
sed -i -E 's#/:([a-z_]+)#/{\1}#g' $(grep -rlE '"[^"]*/:[a-z_]+' src --include=*.rs)
grep -rnE '"[^"]*/:[a-z_]+' src --include=*.rs     # 应为 0
```

sed 之后人工过一遍 diff：只允许改路由字符串，**不许**误伤注释里的 URL 或 `format!` 字符串以外的东西
（`format!("/api/events/{event_id}")` 这类测试里的 URL 本来就是 `{}` 插值，不受影响）。

- [ ] **Step 3: 去 `#[async_trait]`**

axum 0.8 的 `FromRequestParts` / `FromRequest` 是原生 async trait。`guard.rs` 两处、`auth.rs` 一处：
删掉 `#[async_trait]` 和 `use axum::async_trait`，其余不动。例：

```rust
impl FromRequestParts<AppState> for Claims {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        // …原样…
    }
}
```

`DebugJson` 的 `where S: Send + Sync` 保持。

- [ ] **Step 4: 其余编译错误**

已知可能遇到的：
- `vision.rs:678` 的 `Option<Json<RebuildRequest>>`：0.8 要求 `OptionalFromRequest`，`Json` 已实现，
  语义变为「无 `Content-Type: application/json` 时为 `None`，有但解析失败时 **400**」（0.7 是 `None`）。
  前端（`VisionModelPanel.vue`）若有「发空 body」的调用，确认它不带 JSON content-type，否则保持原行为需改成手动解析。在 REPORT 里写清结论。
- `nest("/events", event::router())` 与 `nest("/events", stats::router())` 同前缀两次 nest：若 0.8 panic，
  改为 `.nest("/events", event::router().merge(stats::router()))`。
- `Router::nest` 下 `"/"` 子路由：0.8 中 `nest("/events", r)` 里 `r` 的 `"/"` 匹配 `/api/events`（**不带**尾斜杠），
  与 0.7 相同；前端调用点无尾斜杠（已 grep 确认），无需处理。

- [ ] **Step 5: 全量门禁**

```bash
cd src-tauri
tauri-env linux cargo fmt
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features          # 期望：226 passed
tauri-env win cargo xwin check --target x86_64-pc-windows-msvc
python3 ../scripts/check-event-guards.py           # 守卫门禁会读路由字符串，确认它认得 {x} 语法
```

守卫门禁脚本若用正则匹配 `:event_id`，同步改成认 `{event_id}`，并跑一遍确认它**仍能抓到**一个故意漏守的 handler
（临时删一处 `require_event_open`，看脚本红，再恢复）。

- [ ] **Step 6: 提交**

```bash
git commit -am "⬆️ deps: axum 0.8 / tower 0.5 / tower-http 0.6，路由改 {param} 语法"
```

---

## Task 2: utoipa 骨架 + 契约快照

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/api/openapi.rs`
- Modify: `src-tauri/src/api/mod.rs`、**每个** `src-tauri/src/api/<模块>.rs` 的 `router()`
- Modify: `src-tauri/src/server.rs`、`src-tauri/src/test_support.rs`
- Modify: `src-tauri/src/domain/money.rs`（`ToSchema`）
- Create: `src-tauri/openapi.json`

**Interfaces:**
- Produces:
  - 每个模块 `pub fn router() -> OpenApiRouter<AppState>`——**本 task 之后 `mod.rs` 再也不需要改**，
    阶段 2 的 worker 只改自己模块的文件。
  - `crate::api::router() -> OpenApiRouter<AppState>`；`server.rs` / `test_support.rs` 用 `.split_for_parts().0` 取 `Router`。
  - `crate::api::openapi::ApiErrorBody`（`{ error: String }`，`ToSchema`）——所有错误响应引用它。
  - `crate::api::openapi::BEARER`：security scheme 名 `"bearer"`。
  - `Money: ToSchema`，schema `{ "type": "integer", "format": "cents" }`。

- [ ] **Step 1: 依赖**

```toml
utoipa = { version = "6", features = ["axum_extras", "chrono", "uuid"] }
utoipa-axum = "0.3"
```

- [ ] **Step 2: 过渡形态的模块 router**

每个模块只改 `router()` 的签名与最外层包装，**handler 一个不动**：

```rust
use utoipa_axum::router::OpenApiRouter;

pub fn router() -> OpenApiRouter<AppState> {
    // ③b 过渡：路由还没标注解，先整体包进来。阶段 2 迁移本模块时改为 routes!(...)。
    OpenApiRouter::from(
        Router::new()
            .route("/", get(list_events))
            // …原样…
    )
}
```

`#[cfg(feature = "vision")]` 的 `vision` 同样处理。

- [ ] **Step 3: `api/openapi.rs`**

```rust
//! OpenAPI 文档的公共部分与契约快照测试。
//!
//! 路径与 schema 由各模块的 `routes!(...)` 自动收集，这里只放全局的东西。

use serde::Serialize;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi, ToSchema};

pub const BEARER: &str = "bearer";

/// 所有错误响应的形状。见 `crate::error::ApiError` 的模块注释。
#[derive(Serialize, ToSchema)]
pub struct ApiErrorBody {
    pub error: String,
}

#[derive(OpenApi)]
#[openapi(
    info(title = "摊盒 Booth-Kernel API", version = "1"),
    components(schemas(ApiErrorBody)),
    modifiers(&BearerScheme)
)]
pub struct ApiDoc;

struct BearerScheme;
impl Modify for BearerScheme {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let c = openapi.components.get_or_insert_with(Default::default);
        c.add_security_scheme(
            BEARER,
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).bearer_format("JWT").build()),
        );
    }
}

#[cfg(test)]
mod tests {
    /// 契约快照。`openapi.json` 与当前代码生成的文档逐字节一致，否则失败。
    /// 更新：`UPDATE_OPENAPI=1 tauri-env linux cargo test --all-features openapi_snapshot`
    #[test]
    #[cfg(feature = "vision")]
    fn openapi_snapshot() {
        let doc = crate::api::router().into_openapi();
        let actual = serde_json::to_string_pretty(&doc).unwrap() + "\n";
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");
        if std::env::var_os("UPDATE_OPENAPI").is_some() {
            std::fs::write(path, &actual).unwrap();
            return;
        }
        let expected = std::fs::read_to_string(path).unwrap_or_default();
        assert!(
            actual == expected,
            "openapi.json 与代码不一致。确认契约变更是有意的，然后跑：\n  UPDATE_OPENAPI=1 tauri-env linux cargo test --all-features openapi_snapshot"
        );
    }

    #[test]
    #[cfg(not(feature = "vision"))]
    fn openapi_snapshot() {
        eprintln!("openapi_snapshot 只在 --all-features 下断言：openapi.json 按含 vision 的路由集生成");
    }
}
```

（utoipa 6 若 API 名有出入以编译器为准；**产出 JSON 的语义**必须满足上面 Interfaces 的约定。）

- [ ] **Step 4: `mod.rs` 合并**

```rust
pub fn router() -> OpenApiRouter<AppState> {
    let router = OpenApiRouter::with_openapi(openapi::ApiDoc::openapi())
        .nest("/auth", auth::router())
        .nest("/events", event::router())
        .nest("/events", stats::router())   // Task 1 若改成 merge，这里照 Task 1 的写法
        // …其余与原来一一对应…
        .nest("/legacy", legacy::router());
    #[cfg(feature = "vision")]
    let router = router.nest("/vision", vision::router());
    router
}
```

`server.rs` 与 `test_support.rs` 里的 `crate::api::router()` 改成 `crate::api::router().split_for_parts().0`
（或 `Router::from(crate::api::router())`，二选一全仓统一）。

- [ ] **Step 5: `Money` 的 schema**

`domain/money.rs` 末尾：

```rust
/// OpenAPI：金额在 JSON 里是裸整数分。`format: cents` 是给前端生成器的记号，
/// `frontend/scripts/gen-api.mjs` 把它映射成 branded `Cents` 类型。
impl utoipa::PartialSchema for Money {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::Integer)
            .format(Some(utoipa::openapi::SchemaFormat::Custom("cents".into())))
            .description(Some("金额，单位：分"))
            .into()
    }
}
impl utoipa::ToSchema for Money {}
```

加单测（同文件 `mod tests`）：

```rust
#[test]
fn money_schema_is_integer_cents() {
    use utoipa::PartialSchema;
    let v = serde_json::to_value(Money::schema()).unwrap();
    assert_eq!(v["type"], "integer");
    assert_eq!(v["format"], "cents");
}
```

- [ ] **Step 6: 生成首版快照并验证门禁有牙**

```bash
cd src-tauri
UPDATE_OPENAPI=1 tauri-env linux cargo test --all-features openapi_snapshot
tauri-env linux cargo test --all-features openapi_snapshot          # PASS
jq '.paths | length, .components.schemas | keys, .components.securitySchemes | keys' openapi.json
# 期望：paths 为 0；schemas 含 ApiErrorBody；securitySchemes 含 bearer
jq '.info.version = "2"' openapi.json > /tmp/o && cp /tmp/o openapi.json
tauri-env linux cargo test --all-features openapi_snapshot          # 期望 FAIL，提示信息可读
git checkout openapi.json
```

- [ ] **Step 7: 全量门禁（fmt / clippy / test 226+1 / xwin check）后提交**

```bash
git add -A src-tauri && git commit -m "✨ feat: 接入 utoipa，路由改 OpenApiRouter，openapi.json 快照门禁"
```

---

## Task 3: 样板模块 settlement + 并行基建

这个 task 的产物是阶段 2 所有后端 worker 的**模板**。它必须把最难的情况都走一遍：
`SettlementReport` 三层嵌套、`Money` 字段、`Option<Money>`、带状态码的创建响应、无 body 的删除、
二进制下载（xlsx）、冻结例外的描述、`Claims` 守卫。

**Files:**
- Modify: `src-tauri/src/api/settlement.rs`、`src-tauri/src/domain/settlement.rs`（给报表类型加 `ToSchema`）
- Modify: `src-tauri/src/test_support.rs`（加 `shape_of`）
- Modify: `src-tauri/openapi.json`（重新生成）
- Create: `scripts/worktree-new.sh`、`scripts/worktree-rm.sh`
- Create: `docs/superpowers/plans/3b/brief-backend.md`（附录 A 落盘，按本 task 的实际经验修订）
- Create: `docs/superpowers/specs/2026-09-24-shape-cleanups.md`（空表头）

**Interfaces:**
- Produces:
  - `test_support::shape_of(&Value) -> Value`：把 JSON 映射成「键 + 类型名」的骨架（见 Step 1）。
  - `scripts/worktree-new.sh <name> [base]`、`scripts/worktree-rm.sh <name>`。
  - `brief-backend.md`：附录 A 的定稿。

- [ ] **Step 1: `shape_of`**

`test_support.rs`：

```rust
/// 把 JSON 值映射成它的「形状」：对象保留键、值换成类型名；数组取第一个元素的形状
/// （空数组记为 `["empty"]`）。形状快照测试用它断言「字段和类型没变」，而不关心具体数值。
pub fn shape_of(v: &serde_json::Value) -> serde_json::Value {
    use serde_json::{json, Value};
    match v {
        Value::Null => json!("null"),
        Value::Bool(_) => json!("bool"),
        Value::Number(n) if n.is_i64() || n.is_u64() => json!("int"),
        Value::Number(_) => json!("float"),
        Value::String(_) => json!("string"),
        Value::Array(a) => match a.first() {
            Some(x) => json!([shape_of(x)]),
            None => json!(["empty"]),
        },
        Value::Object(m) => Value::Object(m.iter().map(|(k, x)| (k.clone(), shape_of(x))).collect()),
    }
}
```

带单测：嵌套对象、空数组、null、int/float 区分各一条。

- [ ] **Step 2: settlement 的形状快照测试（先写，先绿）**

在 `api/settlement.rs` 的 `mod tests` 里，为 8 个路由各写一条 `shape_*` 测试。以结算单为例：

```rust
#[tokio::test]
async fn shape_get_settlement() {
    let (router, _dir, pool) = test_router_with().await;
    let (event_id, _, _) = seed_event_and_product(&pool).await;
    let res = router
        .oneshot(json_request("GET", &format!("/api/events/{event_id}/settlement"), Some(&admin_token()), json!(null)))
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body = read_json(res).await;
    assert_eq!(shape_of(&body), json!({ /* 把当前实际输出的形状原样抄下来 */ }));
}
```

写法：先 `println!("{}", serde_json::to_string_pretty(&shape_of(&body)).unwrap())` 跑一次拿到实际形状，
**人工确认它符合前端现在的读法**（`AdminEventSettlement.vue` / `settlementStore.js`），再抄进断言。
种子数据要让每个可选字段、每个数组至少有一种非空情况（结算单：至少下一单、登记一笔垫付），否则形状里全是 `"null"` / `["empty"]`，钉不住东西。
错误路径至少一条（如不存在的展会 → 404 + `{"error": "string"}`）。xlsx 路由断言状态码与 `content-type`。

跑：`tauri-env linux cargo test --all-features shape_` → 全绿，提交一次（`✅ test: settlement 形状快照`）。

- [ ] **Step 3: 类型化与注解**

1. `domain/settlement.rs` 里所有出现在响应中的类型加 `ToSchema`（`SettlementReport`、`SocietyBlock`、`ChannelLine`、`GoodsLine`、`Entry`…按实际）。
2. 请求类型（`AdvanceRequest`、`AdjustmentRequest`…）加 `ToSchema`；响应行（`LedgerEntryRow`、`CreatedEntry`）加 `ToSchema`。
3. 每个 handler 标注解。样例：

```rust
/// 结算单。页面与 xlsx 导出渲染的是同一个 `SettlementReport`。
#[utoipa::path(
    get,
    path = "/events/{event_id}/settlement",
    tag = "settlement",
    params(("event_id" = i64, Path, description = "展会 id")),
    security((BEARER = [])),
    responses(
        (status = 200, body = SettlementReport),
        (status = 401, body = ApiErrorBody),
        (status = 403, body = ApiErrorBody),
        (status = 404, body = ApiErrorBody, description = "展会不存在"),
    ),
)]
async fn get_settlement(/* 原样 */) -> ApiResult<Json<SettlementReport>> { /* 原样 */ }

/// 登记垫付。**展会已结算后仍可调用**（冻结例外，spec 偏离 3）。
#[utoipa::path(
    post,
    path = "/events/{event_id}/advances",
    tag = "settlement",
    description = "展会已结算后仍可调用。冻结例外之一：垫付、结算调整、收摊清点。",
    params(("event_id" = i64, Path)),
    request_body = AdvanceRequest,
    security((BEARER = [])),
    responses(
        (status = 201, body = CreatedEntry),
        (status = 400, body = ApiErrorBody),
        (status = 404, body = ApiErrorBody),
    ),
)]
async fn create_advance(/* 原样 */) -> ApiResult<(StatusCode, Json<CreatedEntry>)> { /* 原样 */ }

#[utoipa::path(
    get,
    path = "/events/{event_id}/settlement.xlsx",
    tag = "settlement",
    params(("event_id" = i64, Path)),
    security((BEARER = [])),
    responses((status = 200, content_type = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", body = Vec<u8>)),
)]
async fn download_settlement_xlsx(/* 原样 */) -> Response { /* 原样 */ }
```

注意：`path` 写的是**模块内**路径（相对于 `mod.rs` 里的 nest 前缀；settlement 是 `merge` 进来的，所以就是完整的 `/events/...`）。
nest 的模块（如 `/events` 下的 event.rs），`path` 写 `"/{id}"`，utoipa-axum 会把 nest 前缀拼上——**以生成的 openapi.json 为准核对**。

4. `router()` 改为：

```rust
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_channels))
        .routes(routes!(list_advances, create_advance))
        .routes(routes!(delete_advance))
        .routes(routes!(list_adjustments, create_adjustment))
        .routes(routes!(delete_adjustment))
        .routes(routes!(get_settlement))
        .routes(routes!(reconcile))
        .routes(routes!(download_settlement_xlsx))
}
```

（`routes!` 里同一路径的不同方法放一起。）

- [ ] **Step 4: 验证**

```bash
cd src-tauri
tauri-env linux cargo test --all-features shape_          # Step 2 的测试一行没改，照样绿
UPDATE_OPENAPI=1 tauri-env linux cargo test --all-features openapi_snapshot
jq '.paths | keys' openapi.json                             # 8 个 settlement 路径，前缀完整
jq '.components.schemas.SettlementReport.properties.actual_total' openapi.json   # 指向 Money
jq '.components.schemas.Money' openapi.json                 # {type: integer, format: cents, …}
jq '.paths["/events/{event_id}/advances"].post.description' openapi.json         # 含「已结算后仍可调用」
```

然后 fmt / clippy / 全量 test，提交（`✨ feat: settlement 模块 OpenAPI 化（样板）`）。

- [ ] **Step 5: worktree 脚本**

`scripts/worktree-new.sh`：

```bash
#!/usr/bin/env bash
# 给并行 worker 开独立 worktree：独立分支、独立 target、带齐不入库的构建前置物。
# 为什么要独立 target：同一个 target 下多个 cargo 并发会互相编译到对方改了一半的文件。
set -euo pipefail
name=${1:?用法: worktree-new.sh <name> [base]}
base=${2:-HEAD}
root=$(git rev-parse --show-toplevel)
wt="$(dirname "$root")/ABL-wt/$name"
[ -e "$wt" ] && { echo "已存在: $wt" >&2; exit 1; }

git -C "$root" worktree add -b "3b/$name" "$wt" "$base"

# include_dir! 编译期需要 frontend/dist
cp -r "$root/frontend/dist" "$wt/frontend/dist"
# 完整复制而非硬链接：vitest/vite 会原地改 node_modules 里的缓存文件
cp -r "$root/frontend/node_modules" "$wt/frontend/node_modules"
# 不入库的 ONNX 原生库（cargo check 只需要它们存在）
for f in src-tauri/resources/onnxruntime.dll src-tauri/resources/DirectML.dll; do
  [ -e "$root/$f" ] && cp "$root/$f" "$wt/$f"
done
# 复用依赖的编译产物，免去冷编译（完整复制，~8G）
mkdir -p "$wt/src-tauri/target"
[ -d "$root/src-tauri/target/debug" ] && cp -r "$root/src-tauri/target/debug" "$wt/src-tauri/target/debug"

# worker 的 brief / 报告 / 日志放 .3b/，不进 git
mkdir -p "$wt/.3b"
echo ".3b/" >> "$(git -C "$wt" rev-parse --git-path info/exclude)"
echo "$wt"
```

`scripts/worktree-rm.sh`：

```bash
#!/usr/bin/env bash
set -euo pipefail
name=${1:?用法: worktree-rm.sh <name>}
root=$(git rev-parse --show-toplevel)
wt="$(dirname "$root")/ABL-wt/$name"
git -C "$root" worktree remove --force "$wt"
git -C "$root" branch -d "3b/$name"   # 未合并会拒绝删除——那正是要停下来看的时候
```

验一次：`./scripts/worktree-new.sh probe && cd ../ABL-wt/probe/src-tauri && time tauri-env linux cargo test --all-features shape_`，
记下耗时写进 REPORT（判断 target 复用是否生效：应远小于冷编译），然后 `worktree-rm.sh probe`。

- [ ] **Step 6: brief 模板定稿**

把附录 A 写到 `docs/superpowers/plans/3b/brief-backend.md`，**按 Step 2～4 的实际经验修订**：
nest 前缀的 `path` 写法、utoipa 6 的实际 API 名、`shape_of` 的种子数据技巧。建空的 `shape-cleanups.md`：

```markdown
# ③b 形状清理清单

阶段 2 各模块迁移时发现的「想改但不许改」的 JSON 形状问题。Task 10 统一处理。

| 模块 | 路由 | 现状 | 建议 | 理由 | 前端消费方 |
|---|---|---|---|---|---|
```

提交（`🔧 chore: ③b 并行基建——worktree 脚本与后端 worker brief`）。

---

## Task 4: 前端 TS 地基 + client

**Files:**
- Modify: `frontend/package.json`
- Create: `frontend/tsconfig.json`、`tsconfig.app.json`、`tsconfig.node.json`、`tsconfig.vitest.json`、`frontend/env.d.ts`
- Rename+Modify: `frontend/vite.config.js → vite.config.ts`、`frontend/eslint.config.js → eslint.config.ts`
- Create: `frontend/scripts/gen-api.mjs`、`frontend/src/api/schema.d.ts`（生成）
- Rename+Modify: `frontend/src/utils/money.js → money.ts`、`money.spec.js → money.spec.ts`
- Create: `frontend/src/api/client.ts`、`frontend/src/api/client.spec.ts`
- Modify: `.github/workflows/ci.yml`（frontend job）

**Interfaces:**
- Consumes: `src-tauri/openapi.json`（Task 3 后含 settlement 的 8 个路径）
- Produces（阶段 3 所有 worker 依赖这些名字）：
  - `import type { paths, components } from '@/api/schema'`
  - `type Schemas = components['schemas']`（在 `client.ts` 导出）
  - `api`：openapi-fetch client，`api.GET/POST/PUT/PATCH/DELETE(path, init)`
  - `unwrap<T>(p): Promise<T>`：成功返回 `data`，失败抛 `ApiRequestError`
  - `class ApiRequestError extends Error { status: number; body: unknown; serverMessage?: string; get response(): { status: number; data: unknown } /* @deprecated */ }`
  - `errorMessage(e: unknown, fallback: string): string`：`serverMessage` 优先，否则 `fallback`——与旧代码 `err.response?.data?.error || '…'` 语义一致
  - `createApiClient(opts)`：测试用工厂
  - `money.ts`：`type Cents`、`fromCents(c: Cents | null | undefined): number`、`toCents(yuan: number | string): Cents`、`formatCents(c: Cents | null | undefined): string`、`formatYuan(c: Cents | null | undefined): string`

- [ ] **Step 1: 依赖**

```bash
npm --prefix frontend install -D openapi-typescript@^7.13 typescript-eslint
npm --prefix frontend install openapi-fetch@^0.17
```

`package.json` scripts 增加：

```json
"gen:api": "node scripts/gen-api.mjs",
"typecheck": "vue-tsc --build --force"
```

- [ ] **Step 2: tsconfig 四件套 + env.d.ts**

`tsconfig.json`：

```json
{
  "files": [],
  "references": [
    { "path": "./tsconfig.node.json" },
    { "path": "./tsconfig.app.json" },
    { "path": "./tsconfig.vitest.json" }
  ]
}
```

`tsconfig.app.json`：

```json
{
  "extends": "@vue/tsconfig/tsconfig.dom.json",
  "include": ["env.d.ts", "src/**/*", "src/**/*.vue"],
  "exclude": ["src/**/*.spec.*"],
  "compilerOptions": {
    "tsBuildInfoFile": "./node_modules/.tmp/tsconfig.app.tsbuildinfo",
    "strict": true,
    "allowJs": true,
    "checkJs": false,
    "paths": { "@/*": ["./src/*"] }
  }
}
```

（`allowJs` 是过渡期设置——阶段 3 前仍有 `.js` 被 `.ts` import。Task 12 删掉。）

`tsconfig.node.json`：

```json
{
  "extends": "@tsconfig/node22/tsconfig.json",
  "include": ["vite.config.*", "eslint.config.*", "scripts/**/*"],
  "compilerOptions": {
    "noEmit": true,
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "types": ["node"],
    "tsBuildInfoFile": "./node_modules/.tmp/tsconfig.node.tsbuildinfo"
  }
}
```

`tsconfig.vitest.json`：

```json
{
  "extends": "./tsconfig.app.json",
  "include": ["src/**/*.spec.*", "env.d.ts"],
  "exclude": [],
  "compilerOptions": {
    "tsBuildInfoFile": "./node_modules/.tmp/tsconfig.vitest.tsbuildinfo",
    "types": ["node", "jsdom"]
  }
}
```

`env.d.ts`：

```ts
/// <reference types="vite/client" />

interface Window {
  /** Tauri 注入；浏览器（LAN 顾客端）里不存在。用它判断运行环境。 */
  __TAURI_INTERNALS__?: unknown
}
```

- [ ] **Step 3: vite / vitest / eslint 转 TS**

`git mv frontend/vite.config.js frontend/vite.config.ts`，内容不变，只改：

```ts
    test: {
      environment: 'jsdom',
      include: ['src/**/*.spec.{js,ts}'],   // 过渡期两者都认；Task 12 改成只认 .ts
      globals: false,
    },
```

`git mv frontend/eslint.config.js frontend/eslint.config.ts`，重写为：

```ts
import { globalIgnores } from 'eslint/config'
import { defineConfigWithVueTs, vueTsConfigs, configureVueProject } from '@vue/eslint-config-typescript'
import pluginVue from 'eslint-plugin-vue'
import skipFormatting from '@vue/eslint-config-prettier/skip-formatting'

// 过渡期允许 .vue 里仍是 JS；Task 12 收口时改成只允许 ts。
configureVueProject({ scriptLangs: ['ts', 'js'] })

export default defineConfigWithVueTs(
  { name: 'app/files-to-lint', files: ['**/*.{js,mjs,ts,mts,vue}'] },
  globalIgnores(['**/dist/**', '**/dist-ssr/**', '**/coverage/**', 'src/api/schema.d.ts']),
  pluginVue.configs['flat/essential'],
  vueTsConfigs.recommended,
  {
    name: 'app/ts-rules',
    rules: {
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/ban-ts-comment': [
        'error',
        { 'ts-expect-error': 'allow-with-description', 'ts-ignore': true, 'ts-nocheck': true },
      ],
    },
  },
  skipFormatting,
)
```

原 `eslint.config.js` 里手写的 `globals`（window / document / …）不再需要——TS 规则集不开 `no-undef`。
若原配置里还有其它项目专属规则，原样搬进一个 `app/legacy-rules` 块。

`npm --prefix frontend run lint`：现存 JS 代码可能因 `vueTsConfigs.recommended` 冒出新错误（典型是 `no-unused-vars` 变为 TS 版）。
**只修真问题**；规则误报对 JS 文件的，在 `app/legacy-rules` 里对 `**/*.js` 关掉并注明「Task 12 删」。

- [ ] **Step 4: `gen-api.mjs` + 首次生成**

```js
// openapi.json → src/api/schema.d.ts。生成物入库，CI 会重跑本脚本并 diff。
//
// 唯一的定制：`format: cents` 的 schema（Rust 的 Money）映射成 branded `Cents`，
// 让「把分当成元显示」在 TS 里报错。见 src/utils/money.ts。
import { writeFileSync } from 'node:fs'
import openapiTS, { astToString } from 'openapi-typescript'
import ts from 'typescript'

const CENTS = ts.factory.createTypeReferenceNode(ts.factory.createIdentifier('Cents'))

const ast = await openapiTS(new URL('../../src-tauri/openapi.json', import.meta.url), {
  transform(schemaObject) {
    if (schemaObject.format === 'cents') return CENTS
  },
})

const header = `/* eslint-disable */
// 由 frontend/scripts/gen-api.mjs 从 src-tauri/openapi.json 生成。不要手改——改后端，再跑 npm run gen:api。
import type { Cents } from '@/utils/money'

`
writeFileSync(new URL('../src/api/schema.d.ts', import.meta.url), header + astToString(ast))
```

跑 `npm --prefix frontend run gen:api`，确认 `schema.d.ts` 里 `SettlementReport` 的 `actual_total` 类型是 `Cents`
（`Option<Money>` 是 `Cents | null`）。`.prettierignore` 加 `src/api/schema.d.ts`。

- [ ] **Step 5: `money.ts`**

`git mv money.js money.ts`、`money.spec.js money.spec.ts`，加类型：

```ts
/** 金额，单位：分。只能由后端响应（经 schema.d.ts）或 toCents() 产生。 */
export type Cents = number & { readonly __brand: 'Cents' }

export function fromCents(cents: Cents | null | undefined): number { /* 原实现 */ }
export function toCents(yuan: number | string): Cents {
  const n = Number(yuan)
  return (Number.isFinite(n) ? Math.round(n * 100) : 0) as Cents   // 全仓唯一的 as Cents
}
export function formatCents(cents: Cents | null | undefined): string { /* 原实现 */ }
export function formatYuan(cents: Cents | null | undefined): string { /* 原实现 */ }
```

spec 里构造测试输入用 `toCents(19.9)` 或 `1990 as Cents`（**spec 文件里**允许 `as Cents`，在测试头注释说明）。
JS 调用方（还没迁的 `.vue` / store）继续正常 import，不受影响。

- [ ] **Step 6: 先写 client 的失败测试**

`frontend/src/api/client.spec.ts`：

```ts
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { createApiClient, unwrap, ApiRequestError, errorMessage } from './client'

type Handler = (req: Request) => Response | Promise<Response>
function mockFetch(h: Handler) {
  return vi.fn(async (input: Request) => h(input))
}
const json = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } })

let onUnauthorized: ReturnType<typeof vi.fn>
let onUploadError: ReturnType<typeof vi.fn>
let currentPath: string
function make(h: Handler, timeoutMs = 30_000) {
  onUnauthorized = vi.fn()
  onUploadError = vi.fn()
  return createApiClient({
    baseUrl: 'http://127.0.0.1:5140/api',
    fetch: mockFetch(h),
    timeoutMs,
    getToken: () => 'tok',
    currentPath: () => currentPath,
    onUnauthorized,
    onUploadError,
  })
}
beforeEach(() => { currentPath = '/admin' })

describe('client', () => {
  it('成功时 unwrap 返回 data，并带上 Bearer', async () => {
    let auth: string | null = null
    const api = make((req) => { auth = req.headers.get('authorization'); return json(200, ['微信']) })
    await expect(unwrap(api.GET('/channels'))).resolves.toEqual(['微信'])
    expect(auth).toBe('Bearer tok')
  })

  it('JSON 错误体：serverMessage 取 error 字段，旧式 response 兼容读取仍可用', async () => {
    const api = make(() => json(409, { error: '展会已冻结' }))
    const e = await unwrap(api.GET('/channels')).catch((x) => x)
    expect(e).toBeInstanceOf(ApiRequestError)
    expect(e.status).toBe(409)
    expect(e.serverMessage).toBe('展会已冻结')
    expect(e.response.data.error).toBe('展会已冻结')
    expect(errorMessage(e, '加载失败')).toBe('展会已冻结')
  })

  it('plain-text error body：serverMessage 为空，走调用方兜底', async () => {
    const api = make(() => new Response('JSON Parse Error: expected value', { status: 400 }))
    const e = await unwrap(api.GET('/channels')).catch((x) => x)
    expect(e.serverMessage).toBeUndefined()
    expect(errorMessage(e, '保存失败')).toBe('保存失败')
  })

  it('network failure becomes status 0', async () => {
    const api = make(() => { throw new TypeError('Failed to fetch') })
    const e = await unwrap(api.GET('/channels')).catch((x) => x)
    expect(e).toBeInstanceOf(ApiRequestError)
    expect(e.status).toBe(0)
    expect(errorMessage(e, '加载失败')).toBe('加载失败')
  })

  it('empty success body 返回 undefined 而不是抛错', async () => {
    const api = make(() => new Response(null, { status: 200, headers: { 'content-length': '0' } }))
    await expect(unwrap(api.GET('/channels'))).resolves.toBeUndefined()
  })

  it('401 跳登录', async () => {
    const api = make(() => json(401, { error: '无效的令牌' }))
    await unwrap(api.GET('/channels')).catch(() => {})
    expect(onUnauthorized).toHaveBeenCalledOnce()
  })

  it('401 on /login does not redirect，错误照常抛出', async () => {
    currentPath = '/login'
    const api = make(() => json(401, { error: '密码错误' }))
    const e = await unwrap(api.GET('/channels')).catch((x) => x)
    expect(onUnauthorized).not.toHaveBeenCalled()
    expect(errorMessage(e, '登录失败')).toBe('密码错误')
  })

  it('413 upload：multipart 请求失败触发上传弹窗', async () => {
    const api = make(() => new Response('length limit exceeded', { status: 413 }))
    const fd = new FormData()
    fd.append('f', new Blob(['x']), 'a.png')
    // @ts-expect-error settlement 里没有 multipart 路由；这里只测 middleware，路径类型无关紧要
    await unwrap(api.POST('/master-products', { body: fd })).catch(() => {})
    expect(onUploadError).toHaveBeenCalledOnce()
    const [url, errLike] = onUploadError.mock.calls[0]
    expect(url).toContain('/master-products')
    expect(errLike.response.status).toBe(413)
  })

  it('caller signal and global timeout both abort', async () => {
    const hang: Handler = (req) =>
      new Promise((_, reject) => req.signal.addEventListener('abort', () => reject(req.signal.reason)))
    // 全局超时先到
    const e1 = await unwrap(make(hang, 20).GET('/channels')).catch((x) => x)
    expect(e1.status).toBe(0)
    // 调用方 signal 先到
    const ctl = new AbortController()
    const p = unwrap(make(hang, 60_000).GET('/channels', { signal: ctl.signal })).catch((x) => x)
    ctl.abort()
    expect((await p).status).toBe(0)
  })
})
```

（`/master-products` 那条用 `@ts-expect-error` 是因为此时 openapi.json 只有 settlement 路径；Task 8 之后路径存在了，
这条注释会变成「未使用的 expect-error」而报错——届时删掉它，这正是预期的提醒。）

`npm --prefix frontend run test:unit -- client` → 全部 FAIL（模块不存在）。

- [ ] **Step 7: 实现 `client.ts`**

```ts
/**
 * 后端 API 的唯一入口。类型来自 schema.d.ts（由 openapi.json 生成）。
 *
 * 调用写法：
 *   const report = await unwrap(api.GET('/events/{event_id}/settlement', { params: { path: { event_id } } }))
 *   try { … } catch (e) { toast.error(errorMessage(e, '加载失败')) }
 *
 * 传输层沿用旧 axios adapter 的切换逻辑：Tauri 内请求 localhost 走 WebView 原生 fetch
 * （绕开 plugin-http 的 IPC 序列化，大 body 时 ~1.5 MB/s 瓶颈），其它走 plugin-http（保留跨域能力）。
 */
import createClient, { type Middleware } from 'openapi-fetch'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'
import type { paths, components } from './schema'

export type Schemas = components['schemas']

export class ApiRequestError extends Error {
  readonly status: number
  readonly body: unknown
  /** 后端 `{"error": "..."}` 里的那句话；错误体不是这个形状时为 undefined。 */
  readonly serverMessage?: string

  constructor(status: number, body: unknown, message?: string) {
    const server =
      body && typeof body === 'object' && typeof (body as { error?: unknown }).error === 'string'
        ? (body as { error: string }).error
        : undefined
    super(server ?? message ?? `请求失败（${status}）`)
    this.name = 'ApiRequestError'
    this.status = status
    this.body = body
    this.serverMessage = server
  }

  /** @deprecated 兼容旧的 axios 风格读取 `err.response?.data?.error`。Task 12 删除。 */
  get response(): { status: number; data: unknown } {
    return { status: this.status, data: this.body }
  }
}

/** 与旧写法 `err.response?.data?.error || fallback` 语义一致：只有后端给了话才用后端的。 */
export function errorMessage(e: unknown, fallback: string): string {
  return (e instanceof ApiRequestError && e.serverMessage) || fallback
}

type Result<T> = { data: T; error?: never; response: Response } | { data?: never; error: unknown; response: Response }

export async function unwrap<T>(p: Promise<Result<T>>): Promise<T> {
  let r: Result<T>
  try {
    r = await p
  } catch (e) {
    const timeout = e instanceof DOMException && e.name === 'TimeoutError'
    throw new ApiRequestError(0, null, timeout ? '请求超时' : '网络错误')
  }
  if (r.error !== undefined || !r.response.ok) throw new ApiRequestError(r.response.status, r.error)
  return r.data as T
}

export interface ApiClientOptions {
  baseUrl: string
  fetch: (req: Request) => Promise<Response>
  timeoutMs: number
  getToken: () => string | null
  currentPath: () => string
  onUnauthorized: () => void
  /** 参数与旧 `normalizeUploadError(error, …)` 期望的 error 形状兼容。 */
  onUploadError: (url: string, errLike: { message: string; response: { status: number; data: unknown } }) => void
}

export function createApiClient(o: ApiClientOptions) {
  const client = createClient<paths>({
    baseUrl: o.baseUrl,
    fetch: o.fetch,
    // FormData / 原始字节原样透传；其余 JSON
    bodySerializer: (body) =>
      body instanceof FormData || body instanceof Blob || body instanceof Uint8Array || body instanceof ArrayBuffer
        ? (body as BodyInit)
        : JSON.stringify(body),
  })

  const mw: Middleware = {
    onRequest({ request }) {
      const token = o.getToken()
      if (token) request.headers.set('Authorization', `Bearer ${token}`)
      const signal = AbortSignal.any([request.signal, AbortSignal.timeout(o.timeoutMs)])
      return new Request(request, { signal })
    },
    async onResponse({ request, response }) {
      if ((response.status === 401 || response.status === 403) && o.currentPath() !== '/login') {
        o.onUnauthorized()
      }
      const isUpload = (request.headers.get('content-type') ?? '').startsWith('multipart/form-data')
      if (isUpload && !response.ok) {
        const text = await response.clone().text()
        let data: unknown = text
        try { data = JSON.parse(text) } catch { /* 纯文本错误体，原样保留 */ }
        o.onUploadError(request.url, { message: text, response: { status: response.status, data } })
      }
      return response
    },
  }
  client.use(mw)
  return client
}

// ---- 默认实例 ----

const isTauri = window.__TAURI_INTERNALS__ !== undefined
const API_PORT = 5140
const baseUrl = isTauri ? `http://127.0.0.1:${API_PORT}/api` : '/api'

const isLocalhostUrl = (u: string) => {
  try {
    const h = new URL(u).hostname
    return h === '127.0.0.1' || h === 'localhost' || h === '::1'
  } catch {
    return false
  }
}

let reqSeq = 0
async function transportFetch(req: Request): Promise<Response> {
  const id = ++reqSeq
  const t0 = performance.now()
  const native = !isTauri || isLocalhostUrl(req.url)
  const res = native ? await window.fetch(req) : await tauriFetch(req)
  console.log(`[Req #${id}] ${req.method} ${req.url} -> ${res.status} (${(performance.now() - t0).toFixed(2)}ms) [${native ? 'native' : 'plugin-http'}]`)
  return res
}

export const api = createApiClient({
  baseUrl,
  fetch: transportFetch,
  timeoutMs: 30_000,
  getToken: () => sessionStorage.getItem('access_token'),
  currentPath: () => router.currentRoute.value.path,
  onUnauthorized: () => { router.push('/login').catch(() => {}) },
  onUploadError: (url, errLike) => {
    if (url.includes('/sync/import-products')) showUploadDialog('商品包导入失败', normalizeUploadError(errLike, SYNC_IMPORT_LIMIT_MB))
    else if (url.includes('/events')) showUploadDialog('付款二维码上传失败', normalizeUploadError(errLike, IMAGE_UPLOAD_LIMIT_MB))
    else if (url.includes('/master-products')) showUploadDialog('商品预览图上传失败', normalizeUploadError(errLike, IMAGE_UPLOAD_LIMIT_MB))
  },
})
```

（文件顶部补 `import router from '@/router'` 与 `@/utils/upload` 的四个名字。`client.spec.ts` 里 `vi.mock('@/router')`、
`vi.mock('@tauri-apps/plugin-http')`、`vi.mock('naive-ui', …)`（同 `upload.spec.js` 的写法），只测 `createApiClient` 工厂。）

**实现时核对**（openapi-fetch 0.17 源码为准，REPORT 里写结论）：
- `bodySerializer` 签名；FormData 时 openapi-fetch 是否会自行删掉 `Content-Type` 让浏览器补 boundary——**必须**让浏览器补，否则后端 multipart 解析失败。
- 中间件 `onRequest` 返回新 `Request` 是否被采用。
- 空 body 成功响应时 `data` 的值（Review Focus 4）。

`npm --prefix frontend run test:unit` → 全绿。

- [ ] **Step 8: CI**

`.github/workflows/ci.yml` 的 frontend job，在 `Lint` 之前加：

```yaml
      - name: Contract drift (schema.d.ts must match openapi.json)
        run: |
          npm run gen:api --prefix frontend
          git diff --exit-code frontend/src/api/schema.d.ts

      - name: Typecheck (non-blocking until ③b Task 12)
        run: npm run typecheck --prefix frontend
        continue-on-error: true
```

- [ ] **Step 9: 门禁与提交**

```bash
npm --prefix frontend run lint && npm --prefix frontend run format:check \
  && npm --prefix frontend run test:unit && npm --prefix frontend run build
npm --prefix frontend run typecheck; echo "typecheck exit=$?"   # 允许非零，记下错误数作为基线
git add -A frontend .github && git commit -m "✨ feat: 前端 TS 地基——tsconfig/eslint/vite 转 TS、gen:api、openapi-fetch client"
```

---

## Task 5: 批次 B1——后端 5 个已用 `ApiResult` 的模块 ＋ 前端底层

按「并行派发规程」执行。

| worker | 模板 | 变量 |
|---|---|---|
| `be-closing` | 附录 A | `MODULE=closing`，前端消费方：`stores/closingStore.js`、`components/vendor/ClosingWizard.vue` |
| `be-inventory` | 附录 A | `MODULE=inventory`，消费方：`stores/inventoryLogStore.js`、`components/vendor/InventoryLogModal.vue` |
| `be-lot` | 附录 A | `MODULE=lot`，消费方：`stores/lotStore.js`、`views/AdminEventLots.vue`、`utils/quote.js` |
| `be-order` | 附录 A | `MODULE=order`，消费方：`stores/orderStore.js`、`stores/customerStore.js`、`components/order/OrderCard.vue` |
| `be-refund` | 附录 A | `MODULE=refund`，消费方：`components/vendor/RefundModal.vue`、`utils/refund.js` |
| `fe-lower` | 附录 B | 文件：`utils/*.js`（除 `money.js`，Task 4 已迁）、`config/*.js`、`composables/*.js` |

**批次验收**（合并后在主仓库）：

```bash
cd src-tauri
UPDATE_OPENAPI=1 tauri-env linux cargo test --all-features openapi_snapshot
jq '.paths | keys | length' openapi.json                  # 较 Task 3 增加的数 = 五个模块的路由数
tauri-env linux cargo fmt --check && tauri-env linux cargo clippy --all-targets --all-features -- -D warnings \
  && tauri-env linux cargo test --all-features
cd .. && npm --prefix frontend run gen:api && npm --prefix frontend run lint && npm --prefix frontend run test:unit \
  && npm --prefix frontend run build
git add src-tauri/openapi.json frontend/src/api/schema.d.ts
git commit -m "📝 docs: 批次 B1 合并后重新生成 openapi.json / schema.d.ts"
```

审查者额外过一遍 `openapi.json` 的 diff：每个新路径都有 `tag`、`security`（有守卫的）、错误响应引用 `ApiErrorBody`、
金额字段是 `$ref: Money`。

---

## Task 6: 批次 B2——society / product / info / auth / stats

| worker | 模板 | 变量 |
|---|---|---|
| `be-society` | 附录 A | `MODULE=society`，消费方：`stores/societyStore.js`、`views/AdminSocieties.vue` |
| `be-product` | 附录 A | `MODULE=product`，消费方：`stores/productStore.js`、`views/AdminEventProducts.vue`、`components/customer/ProductGrid.vue` |
| `be-info` | 附录 A | `MODULE=info`，消费方：`composables/useConnectionCheck.ts`、`views/About.vue` |
| `be-auth` | 附录 A | `MODULE=auth`，消费方：`stores/authStore.js`、`views/LoginView.vue`、`views/AdminControlPanel.vue`。**额外**：`DebugJson` 的 400 纯文本响应形状不变；登录失败 401 的 `{"error":"密码错误"}` 不变 |
| `be-stats` | 附录 A | `MODULE=stats`，消费方：`stores/eventStatStore.js`、`views/AdminEventStat.vue`、`components/vendor/LiveStats.vue`、`components/stats/*` |

**批次验收**：同 Task 5（提交信息换成 B2）。

---

## Task 7: 批次 B3——39 个 `impl IntoResponse` 的重灾区

这批的模块大多是 v1.1 时代的代码：`json!` 现拼响应、`unwrap_or_default()` 吞错误、错误体有时不是 `{"error"}`。
**K3 在这里最容易被违反**，brief 里的约束对这批加粗重复一次。

| worker | 模板 | 变量 |
|---|---|---|
| `be-master-product` | 附录 A | `MODULE=master_product`，消费方：`views/AdminMasterProducts.vue`、`components/product/*`。含 multipart 上传 |
| `be-event` | 附录 A | `MODULE=event`，消费方：`stores/eventStore.js`、`stores/eventDetailStore.js`、`components/event/*`、`views/AdminDashboard.vue`、`views/EventPortalView.vue`。含 multipart 上传；`qrcode_url` 与 `qrcode_urls` 两个字段都要保留 |
| `be-admin` | 附录 A | `MODULE=admin`，消费方：`views/AdminControlPanel.vue`、`views/ThemeSetting.vue` |
| `be-sync` | 附录 A | `MODULE=sync`，消费方：`stores/syncStore.js`、`components/product/BoothpackSyncPanel.vue`。含 multipart 与 `application/octet-stream`（`import_products_raw`）|
| `be-legacy` | 附录 A | `MODULE=legacy`，消费方：`utils/legacyExport.ts`、`views/MigrationNotice.vue` |
| `be-vision` | 附录 A | `MODULE=vision`，消费方：`services/vision.js`、`components/shared/VisionSearch.vue`、`components/product/VisionModelPanel.vue`。**额外**：只类型化与注解，**不拆文件、不改任何识别逻辑**；`#[cfg(feature = "vision")]` 保持；`cargo check --no-default-features` 也要过 |

**批次验收**：同 Task 5，另加：

```bash
cd src-tauri && tauri-env linux cargo check --no-default-features
```

---

## Task 8: 阶段 2 收口

**Files:** `src-tauri/src/api/*.rs`（只读检查）、`src-tauri/openapi.json`、`frontend/src/api/client.spec.ts`、
`docs/superpowers/specs/2026-09-24-shape-cleanups.md`

- [ ] **Step 1: 完成标准**

```bash
cd src-tauri
grep -rn "OpenApiRouter::from(" src/api                       # 期望 0
grep -rnE "^\s*(pub )?async fn .*\)\s*->\s*impl IntoResponse" src/api   # 期望 0
grep -rn "impl IntoResponse" src/api                           # 只剩非 handler 的类型实现（若有），逐条确认
jq '[.paths[] | keys[]] | length' openapi.json                  # 期望 = 路由×方法总数；与 grep 出的 routes! 数核对
jq '[.paths[][] | select(.tags == null)] | length' openapi.json # 期望 0
```

路由×方法总数的核对方法：`grep -c` 各模块 `routes!(` 里的 handler 数之和，vision 包含在内。**不等就是有 handler 没进文档**，查到补上。

- [ ] **Step 2: 删掉 `client.spec.ts` 里那条已无用的 `@ts-expect-error`**（`/master-products` 现已存在于 schema），跑 `typecheck` 确认它不再报「unused」。

- [ ] **Step 3: 通读 `shape-cleanups.md`**，去重、补全「前端消费方」列，给每条标上 Task 10 做 / 不做（不做的写理由，典型是「改了收益小、牵动面大」）。

- [ ] **Step 4: 全量门禁 + 真实数据冒烟**

在 VNC 外可做的冒烟：把 `~/.local/share/com.abl.BoothKernel-dev/sale_system.db` **复制**到临时目录，
写一个 `#[ignore]` 测试或小脚本用真库起 `api::router()`，对每个 GET 路由跑一遍，确认 200 且 `shape_of` 与各模块形状测试一致。
（真库里有测试夹具造不出的历史数据，比如 v1.0 时代的空字段。）结果写进本 task 的执行记录。

- [ ] **Step 5: 提交**（`✅ test: ③b 阶段 2 收口——全部 68 路由进入 openapi.json`）

---

## Task 9: 批次 F1——services + 14 个 store 切到新 client

store 之间**互不 import**（已确认，只有 `authStore` import `@/router`），可以并行。组件在这一批里仍是 JS，
组件里 `err.response?.data?.error` 的读取依靠 `ApiRequestError.response` 兼容 getter 继续工作。

| worker | 模板 | 文件 |
|---|---|---|
| `st-core` | 附录 C | `stores/authStore.js`、`alertStore.js`、`themeStore.js`、`services/useAlert.js`、`services/clipboard.js`、`services/url.js`；`composables/useConnectionCheck.ts`、`utils/legacyExport.ts` 切到新 client（Task 5 已转 TS，只换调用）；**删除** `services/socketService.js` 并从 `package.json` 移除 `socket.io-client` |
| `st-event` | 附录 C | `stores/eventStore.js`、`eventDetailStore.js`、`eventStatStore.js`、`societyStore.js`、`syncStore.js` |
| `st-sale` | 附录 C | `stores/productStore.js`、`customerStore.js`、`orderStore.js`、`lotStore.js` |
| `st-close` | 附录 C | `stores/closingStore.js`、`inventoryLogStore.js`、`settlementStore.js`、`services/vision.js` |

`package.json` 只有 `st-core` 改；其它 worker 若发现要加依赖，写进 REPORT，不改。

**批次验收**：

```bash
npm --prefix frontend run lint && npm --prefix frontend run test:unit && npm --prefix frontend run build
npm --prefix frontend run typecheck 2>&1 | grep -E "src/(stores|services)/" | wc -l    # 期望 0
grep -rn "from '@/services/api'\|from './api'" frontend/src/stores frontend/src/services frontend/src/composables frontend/src/utils   # 期望 0
ls frontend/src/stores/*.js frontend/src/services/*.js 2>/dev/null | grep -v api.js      # 期望空
```

---

## Task 10: 形状清理

**Files:** `shape-cleanups.md` 里标「做」的各条涉及的后端模块与前端 store、组件。

- [ ] **Step 1:** 逐条改后端：改类型 → 同步改该路由的 `shape_*` 测试断言（**这是本轮唯一允许改形状快照的地方**，
  每处改动在提交信息里点名）→ 重新生成 `openapi.json` 与 `schema.d.ts`。
- [ ] **Step 2:** `npm --prefix frontend run typecheck`，按报错修 store（已是 TS）。**组件仍是 JS，编译器看不到**——
  对每条清理项，用表里的「前端消费方」列逐个 grep 字段名修掉。
- [ ] **Step 3:** 全量门禁，每条清理项一个提交（`♻️ refactor: <路由> 响应形状统一——<一句话>`）。

---

## Task 11: 批次 F2——49 个 `.vue` → `lang="ts"`

按目录切，同批不共享子组件（子组件与父组件放同一个 worker，或子组件先于父组件——这里都在同一批，靠「子组件的 props 类型由子组件自己定义」解耦）。

| worker | 模板 | 文件 |
|---|---|---|
| `vue-admin-a` | 附录 D | `views/AdminLayout.vue`、`AdminDashboard.vue`、`AdminControlPanel.vue`、`AdminSocieties.vue`、`AdminMasterProducts.vue`、`ThemeSetting.vue`、`components/product/*`（5 个） |
| `vue-admin-b` | 附录 D | `views/AdminEventProducts.vue`、`AdminEventOrders.vue`、`AdminEventStat.vue`、`AdminEventLots.vue`、`AdminEventSettlement.vue`、`components/stats/*`（2 个）、`components/event/*`（3 个） |
| `vue-vendor` | 附录 D | `views/VendorView.vue`、`VendorEventSelection.vue`、`components/vendor/*`（5 个）、`components/order/OrderCard.vue`、`components/shared/ChannelSelect.vue` |
| `vue-customer` | 附录 D | `views/CustomerView.vue`、`EventPortalView.vue`、`components/customer/*`（3 个）、`components/shared/VisionSearch.vue`、`ImageCropper.vue`、`ImageUploader.vue` |
| `vue-misc` | 附录 D | `App.vue`、`views/LoginView.vue`、`MigrationNotice.vue`、`NotFound.vue`、`ServerError.vue`、`components/GlobalAlert.vue`、`components/shared/AppModal.vue`、`CollapsibleSection.vue`、`EmptyGuide.vue`、`HelpBubble.vue`、`MainHeader.vue`；`main.js → main.ts`、`router/index.js → index.ts` |

（Help / About / UpdateModal 已是 `lang="ts"`，归 `vue-misc` 顺带做 strict 检查。）

分完后审查者核对：`find frontend/src -name '*.vue' | wc -l` = 各 worker 文件数之和 + 已是 TS 的 3 个，一个不漏。

**批次验收**：

```bash
npm --prefix frontend run typecheck        # 期望 0 错误（这一步之后它就是硬门禁了）
npm --prefix frontend run lint && npm --prefix frontend run test:unit && npm --prefix frontend run build
grep -rL 'lang="ts"' $(find frontend/src -name '*.vue')      # 期望空
grep -rn "response?.data?.error\|\.response\.data" frontend/src --include=*.vue --include=*.ts   # 期望 0
```

---

## Task 12: 收口

**Files:** `frontend/src/services/api.js`（删）、`frontend/package.json`、`frontend/src/api/client.ts`、
`frontend/tsconfig.app.json`、`frontend/eslint.config.ts`、`frontend/vite.config.ts`、`.github/workflows/ci.yml`、
`frontend/package.json` 的 `format*` 脚本、`.prettierignore`

- [ ] **Step 1: 删旧**：`git rm frontend/src/services/api.js`；`npm --prefix frontend uninstall axios`；
  删 `ApiRequestError.response` 兼容 getter 与 `client.spec.ts` 里对它的断言；`tsconfig.app.json` 删 `allowJs/checkJs`；
  `eslint.config.ts` 改 `scriptLangs: ['ts']`、删 `app/legacy-rules`；`vite.config.ts` 的 vitest include 改为只认 `.spec.ts`。
- [ ] **Step 2: 残留检查**

```bash
find frontend/src -name '*.js'                                      # 期望空
grep -rn "axios" frontend/src frontend/package.json                 # 期望 0
grep -rn "@ts-expect-error" frontend/src                            # 逐条列进 Task 13 的执行后记，每条必须有原因
grep -rnE ":\s*any\b|as any\b|<any>" frontend/src                   # 期望 0
```

- [ ] **Step 3: CI 硬门禁**：`ci.yml` 里 `Typecheck` 去掉 `continue-on-error`、改名去掉 "non-blocking"。
- [ ] **Step 4: prettier 扩范围（单独提交）**：`package.json` 的 `format` / `format:check` 改为覆盖
  `src/ scripts/ *.ts *.json` 于 frontend 内；仓库根新增 `format:check` 覆盖 `docs/ scripts/ .github/`
  （根 `package.json` 已有 prettier 则用之，否则只在 CI 里 `npx prettier@3.6.2 --check`）。
  先跑 `--write`，**只含重排的**一个提交（`🎨 style: prettier 覆盖 docs/scripts/.github，全量重排`），然后 CI 加检查一个提交。
- [ ] **Step 5: 全量门禁**：Rust fmt/clippy/test、守卫门禁、前端 lint/format:check/test:unit/typecheck/build、docs:build。
- [ ] **Step 6: 提交**（`🔥 remove: 删除 axios 与旧 api.js，vue-tsc 转为 CI 硬门禁`）

---

## Task 13: 文档与交接

**Files:** `docs/superpowers/specs/2026-09-22-v1.2-roadmap.md`（追加附录四）、本 plan 末尾（执行后记）、
`docs/BUILD.md`（如有前端构建相关的新命令）、仓库根 `CLAUDE.md`（本机笔记，不入库）

- [ ] **Step 1: 路线图附录四**：现在能做什么（契约门禁、TS strict）；没验过的真机清单（spec §5 的真机清单原样抄）；
  本轮暴露的值得记住的事；留给 ④ 的（`vue-tsc` 已硬门禁后 UI 重做的注意事项、`Cents` 在表单输入处的用法）；
  「CI 仍未覆盖」一节更新（`cargo check --no-default-features` 若 Task 7 已加进 CI 则划掉）。
- [ ] **Step 2: 执行后记**：每批次的 worker 数、修复轮情况、审查者直接改掉的问题、`@ts-expect-error` 清单、
  target 复用的实测耗时、「计划错了」而非「代码错了」的地方。
- [ ] **Step 3: `CLAUDE.md`（本机）** 加两条：`openapi.json` 更新命令；`scripts/worktree-new.sh` 的用途与磁盘占用。
- [ ] **Step 4: 提交**（`📝 docs: ③b 收尾——路线图附录四、plan 执行后记`）

---

## 完成标准

- [ ] `openapi.json` 含全部路由×方法，每个都有 tag、正确的 security、错误响应引用 `ApiErrorBody`、金额字段为 `Money`（`format: cents`）
- [ ] 冻结例外的端点 description 含「展会已结算后仍可调用」
- [ ] `api/` 下无 `OpenApiRouter::from(`、无返回 `impl IntoResponse` 的 handler
- [ ] 每个模块每个路由至少一条 `shape_*` 测试
- [ ] 前端无 `.js` 源文件、无 axios、无 `any`；每处 `@ts-expect-error` 有原因且列在执行后记
- [ ] CI：`openapi_snapshot`（cargo test 内）、`schema.d.ts` diff、`typecheck` 硬门禁、prettier 扩范围后的检查，全绿
- [ ] Rust 测试数 ≥ 226 + 新增形状测试；前端测试数 ≥ 47 + client 单测
- [ ] 路线图附录四与执行后记已写

---

## 附录 A：后端模块 worker brief（`brief-backend.md`）

> Task 3 Step 6 会按样板的实际经验修订这份模板并落盘。以下是初稿。

```markdown
# 任务：把 `src-tauri/src/api/{{MODULE}}.rs` 迁移到 utoipa（③b 阶段 2）

你在一个独立的 git worktree 里工作（当前目录就是仓库根，分支 3b/{{NAME}}）。只改下面「允许改的文件」。
完成后把报告写到 `.3b/REPORT.md`，然后提交（不要 push）。

## 背景（一段话）
这个项目的后端是跑在 Tauri 进程里的 axum 0.8 HTTP server。我们在用 utoipa 6 + utoipa-axum 0.3 给每个路由生成
OpenAPI 文档，前端再从文档生成 TS 类型。`src-tauri/src/api/settlement.rs` 已经迁移完，**它就是你的模板，先通读它**。

## 允许改的文件
- `src-tauri/src/api/{{MODULE}}.rs`
- `src-tauri/src/domain/` 下**只被本模块用到**的类型文件（加 `ToSchema` derive）；被多个模块共用的类型，只加 derive，不改字段
- `docs/superpowers/specs/2026-09-24-shape-cleanups.md`（只追加行）
- 不许改：`api/mod.rs`、`api/openapi.rs`、`test_support.rs`、`src-tauri/openapi.json`、任何迁移文件、任何前端文件

## 硬约束
1. **JSON 形状一个字节都不许变**：字段名、字段有无、null 还是缺省、数字还是字符串、状态码、错误体。
   想改的写进 `shape-cleanups.md`（模块 | 路由 | 现状 | 建议 | 理由 | 前端消费方），**不动手**。
2. **原来吞错误的继续吞**（如 `.unwrap_or_default()`、`.unwrap_or(None)`），同样记入清单。
3. 错误响应一律是 `{"error": "..."}`。已经是 `ApiError` 的保持；手拼 `json!({"error": …})` 的，
   **状态码与文字完全相同**时可以换成对应的 `ApiError` 变体（400 BadRequest / 404 NotFound / 409 Conflict / 403 Forbidden），否则保留原样。
4. 不许改任何业务逻辑、SQL、权限检查、展会守卫（`require_event_open`）。
5. 本模块的前端消费方是：{{CONSUMERS}}。你**不改**它们，但要读它们，确认你钉住的形状就是它们在读的形状。

## 步骤
1. **先写形状快照测试，先绿。** 在本模块的 `#[cfg(test)] mod tests` 里，为**每个路由**写一条 `shape_<handler名>` 测试，
   用 `crate::test_support::{test_router_with, seed_event_and_product, admin_token, vendor_token, json_request, read_json, shape_of}`。
   先 `println!` 实际的 `shape_of(&body)`，确认合理后抄进 `assert_eq!(shape_of(&body), json!({...}))`。
   种子数据要让每个可选字段和数组至少出现一次非空。每个路由至少一条成功路径；有错误分支的至少一条错误路径（断言状态码 + `{"error": "string"}`）。
   multipart 路由用 `axum::body::Body` 手写 multipart body（参考 settlement 或本模块已有测试）。
   跑绿，**单独提交一次**：`✅ test: {{MODULE}} 形状快照`。
2. **类型化。** `impl IntoResponse` 的 handler 改成 `ApiResult<Json<T>>` / `ApiResult<(StatusCode, Json<T>)>` / `ApiResult<StatusCode>`；
   `json!` 现拼的成功响应换成具名 `#[derive(Serialize, ToSchema)]` 结构体（字段名、顺序无关，**序列化结果必须相同**——
   注意 `Option` 字段：原来输出 `null` 的要保持 `null`，不要加 `skip_serializing_if`）。
   二进制下载可以保持返回 `Response`。请求体、查询参数类型加 `ToSchema` / `IntoParams`。金额字段必须是 `Money` 类型。
3. **标注解。** 每个 handler 加 `#[utoipa::path(...)]`，照 settlement.rs 的写法：method、path（模块内路径）、`tag = "{{MODULE}}"`、
   params、request_body、`security((BEARER = []))`（handler 参数里有 `Claims` / `AdminOnly` 才写）、
   responses（成功类型 + 每个可能的错误码，错误 body 用 `ApiErrorBody`）。doc 注释第一行写这个接口干什么；需要管理员的写明「需要管理员」。
4. **router 改 `routes!`**：`OpenApiRouter::new().routes(routes!(a, b))…`，删掉 `OpenApiRouter::from(Router::new()…)` 的过渡包装。
   原来 router 上的 `.layer(...)`（如 `DefaultBodyLimit`）照原样挂在 `OpenApiRouter` 上。
5. **验证**（必须全部通过）：
   ```bash
   cd src-tauri
   tauri-env linux cargo fmt
   tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
   tauri-env linux cargo test --all-features            # 第 1 步的 shape_ 测试一行没改照样绿；openapi_snapshot 会红——这是预期的，见下
   UPDATE_OPENAPI=1 tauri-env linux cargo test --all-features openapi_snapshot   # 只为了看生成结果
   jq '.paths | with_entries(select(.value[]?.tags? // [] | index("{{MODULE}}")))' openapi.json
   git checkout openapi.json                             # 不提交 openapi.json
   ```
   检查生成的路径前缀完整、金额字段是 `$ref: Money`、有守卫的路由有 security。
6. **提交**：`✨ feat: {{MODULE}} 模块 OpenAPI 化`，末尾一行 `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`。

## 不要做
- 不要起 dev server（`npx tauri dev`、`vite`），不要开 VNC，不要截图。
- 不要 `cargo update`、不要改 `Cargo.toml`。
- 不要重构、拆文件、改名、"顺手"修 bug（发现 bug 写进 REPORT）。

## `.3b/REPORT.md` 格式
- 改了哪些文件；每个 handler 的前后签名
- 形状测试列表（handler → 测试名）
- 记入 shape-cleanups 的条目
- 你不确定的地方、发现的疑似 bug
- 第 5 步各命令的结果（通过 / 失败 + 摘要）
```

## 附录 B：前端底层 worker brief（`brief-frontend-lower.md`）

```markdown
# 任务：把以下前端文件从 JS 迁到 strict TypeScript（③b 阶段 2）

独立 worktree，分支 3b/{{NAME}}。只改下列文件（及其同名 `.spec.js`）：
{{FILES}}

## 规则
- `git mv x.js x.ts`（spec 同理 `.spec.js → .spec.ts`），保留历史。
- tsconfig 是 strict。**禁止 `any`**（eslint 会报错）；确实拿不到类型的外部值用 `unknown` 再收窄。
  `@ts-expect-error` 只允许用于第三方库类型缺陷，且必须写原因。
- 金额用 `import type { Cents } from '@/utils/money'`；把 number 变成 Cents 只能调 `toCents()`。
- 函数签名要写全参数与返回类型；导出的对象/配置写 `interface` 或 `as const`。
- **行为零变化**：不改逻辑、不改文案、不改导出名。调用这些模块的 `.vue` / store 仍是 JS，它们照常 import（Vite 会解析 `.ts`），你不用改它们。
- 调用 `@/services/api`（旧 axios client）的文件（如 `composables/useConnectionCheck.js`、`utils/legacyExport.js`）：
  **这一批不切换 client**——它们调的路由此时可能还没进 `openapi.json`。保留 `import api from '@/services/api'`，
  只给其余部分加类型；切换在阶段 3 的 store 批次里做。
- 不要起 dev server / vitest watch。

## 验证（全部通过才提交）
    npm --prefix frontend run lint
    npm --prefix frontend run test:unit
    npm --prefix frontend run build
    npm --prefix frontend run typecheck 2>&1 | grep -E "src/(utils|config|composables)/"    # 期望无输出

提交：`🏷️ types: {{SCOPE}} 迁移到 TypeScript`，末尾带 Co-Authored-By 行。报告写 `.3b/REPORT.md`（改了什么、不确定的地方、验证结果）。
```

## 附录 C：store / services worker brief（`brief-store.md`）

```markdown
# 任务：把以下 store / service 迁到 TypeScript，并切换到新的带类型 API client（③b 阶段 3）

独立 worktree，分支 3b/{{NAME}}。只改：{{FILES}}

## 新 client 用法（`frontend/src/api/client.ts`，先通读）
    import { api, unwrap, type Schemas } from '@/api/client'
    const report = await unwrap(api.GET('/events/{event_id}/settlement', { params: { path: { event_id: id } } }))
    await unwrap(api.POST('/events/{event_id}/advances', { params: { path: { event_id: id } }, body: payload }))
- 路径就是 `src-tauri/openapi.json` / `src/api/schema.d.ts` 里的路径（**不带** `/api` 前缀）。路径、参数、body 都有类型检查——报错说明你调用写错了或后端契约与前端期望不符，**不要**用 `as` 压掉，写进 REPORT。
- 旧写法 `const res = await api.get(url); x = res.data` → 新写法 `x = await unwrap(api.GET(...))`。
- FormData 直接作为 `body` 传；二进制下载加 `parseAs: 'blob'`。
- 状态类型：`ref<Schemas['EventResponse'][]>([])` 这类，直接用生成的 schema 类型，不要手写重复的 interface。

## 规则
- 错误对象现在是 `ApiRequestError`。**store 里**读错误信息一律 `errorMessage(e, '<原来的兜底文案>')`，文案一字不改。
  store 若把错误继续 throw 给组件：原样 throw，不要包装——组件（仍是 JS）靠 `ApiRequestError.response` 兼容 getter 继续读 `err.response.data.error`。
- 禁止 `any`；`@ts-expect-error` 只限第三方类型缺陷并写原因。
- 行为零变化：不改 state 结构、action 名、getter 名、文案。
- 空 `catch {}` 至少改成 `console.warn('<store 名>.<action 名>', e)`。
- 不要起 dev server。

## 验证
    npm --prefix frontend run lint && npm --prefix frontend run test:unit && npm --prefix frontend run build
    npm --prefix frontend run typecheck 2>&1 | grep -E "{{FILE_REGEX}}"    # 期望无输出

提交：`🏷️ types: {{SCOPE}} 迁移到 TS 并切换到带类型 client`，末尾带 Co-Authored-By 行。报告写 `.3b/REPORT.md`。
```

## 附录 D：`.vue` worker brief（`brief-vue.md`）

```markdown
# 任务：把以下 `.vue` 文件迁到 `<script setup lang="ts">`，strict 零错误（③b 阶段 3）

独立 worktree，分支 3b/{{NAME}}。只改：{{FILES}}

## 规则
- `<script setup>` → `<script setup lang="ts">`；只改 script 段，**不碰 template 与 style**（除非 template 里的表达式因类型需要调整，比如 `?.`）。
- `defineProps` / `defineEmits` 用类型参数写法：`defineProps<{ eventId: number; report: Schemas['SettlementReport'] }>()`，
  有默认值用 `withDefaults`。
- API 数据类型用 `import type { Schemas } from '@/api/client'`，不要手写重复 interface。
- 组件里直接调 API 的，改用 `api` + `unwrap`（见 `src/api/client.ts` 顶部注释）。
- 错误信息：`err.response?.data?.error || '文案'` → `errorMessage(err, '文案')`，**文案一字不改**。
- 金额：`Cents` 类型；显示走 `formatYuan` / `formatCents`；用户输入 → `toCents()`。
- naive-ui 的类型从 `naive-ui` 导入（`DataTableColumns<Row>`、`FormInst`、`SelectOption` 等）。模板 ref：`ref<FormInst | null>(null)`。
- 禁止 `any`；`@ts-expect-error` 只限第三方类型缺陷并写原因。
- 行为零变化。不要重构、不要拆组件、不要改样式。不要起 dev server。

## 验证
    npm --prefix frontend run lint && npm --prefix frontend run test:unit && npm --prefix frontend run build
    npm --prefix frontend run typecheck 2>&1 | grep -E "{{FILE_REGEX}}"    # 期望无输出

提交：`🏷️ types: {{SCOPE}} 的 .vue 迁移到 TS`，末尾带 Co-Authored-By 行。报告写 `.3b/REPORT.md`（每个文件的 props/emits 类型、不确定的地方、`@ts-expect-error` 清单）。
```

---

## 执行后记（2026-09-25 完成）

82 个提交（54 个非合并）。串行 task 由 Claude 在会话内实现；并行批次由 dsh-flash（默认 deepseek-flash）实现、
Claude 审查。台账在 `.superpowers/sdd/2026-09-24-api-contract-and-ts/progress.md`（不入库）。

### 批次

| 批次 | worker | 修复轮 | 审查者直接改的 |
|---|---|---|---|
| B1 后端 closing/inventory/lot/order/refund | 5 | 0 | — |
| B2 后端 society/product/info/auth/stats + 前端 utils、config/composables | 7 | 0 | `money.ts` 加 `cents()`；`quote.ts` 改用契约类型 |
| B3 后端 master_product/event/admin/sync/legacy/vision | 6 | 0 | 守卫门禁认 `method(post, put)`；operationId 全局唯一 |
| F1 services + 14 个 store | 4 | 0 | client 层两处类型缺陷，删掉 22 处绕过与 51 处显式泛型 |
| F2 52 个 `.vue`（6 组） | 6 | 0 | 跨组组件边界的 4 个类型错误 |

**28 个 worker，没有一个需要发回修复。** brief 写得足够具体（样板模块 + 硬约束 + 验证命令 + 报告格式）
之后，flash 模型的产出质量足够直接合并；审查的时间主要花在跨 worker 的汇合点上。

### 计划错了的地方（而不是代码错了）

1. **worker 提交不了。** dsh 沙箱只能写 `--cwd`，而 worktree 的 git 元数据在主仓库的 `.git/worktrees/` 下。
   B1 起改为「worker 不提交，审查者代为提交」。
2. **共享的 `shape-cleanups.md` 每次合并都冲突。** 改为 worker 写进自己的 REPORT，审查者汇总。
3. **同时启动 5 个 dsh-flash 会撞配置文件**（`~/.dsh/profiles/headless/cordis.yml` 每次启动都重写），
   一个 worker 启动即失败。改为间隔 5 秒启动。
4. **守卫门禁会随注册写法静默失效。** `routes!` 之后找不到写 handler（Task 3 发现），`method(post, put)`
   之后漏掉 3 个（B3 发现）。最终加了对 `openapi.json` 的交叉校验。
5. **openapi-typescript 要求 operationId 全局唯一**，而 utoipa 默认用 handler 名。
6. **openapi-fetch 的 `Readable<T>` 与 branded 类型不兼容**，`unwrap` 的推断也有缺陷——
   F1 的 3 个 worker 各自独立报告。修在 `types/openapi-typescript-helpers.d.ts`（tsconfig paths）与 `core.ts`。
7. **Node 里 `AbortSignal.any` 派生的信号、以及 Request 跟随的信号都不派发 abort 事件**（Task 4 测试发现）。
   超时与调用方 signal 手工合并，并显式传给底层 fetch。
8. **「形状清理」几乎都不是形状问题**，Task 10 缩成「值域固定的字符串声明为枚举」一类，并提前到 Task 9 之前。
9. **`--no-default-features` 在 ③b 之前就编不过**，修了并进 CI。
10. **prettier 没有接管 `docs/`**：markdown 表格会按列宽补齐，中文文档大面积变难读（偏离 spec §3）。

### 真 bug

- 销售汇总导出并发 500（固定临时文件名），改为内存生成。
- `normalizeUploadError` 如果不认识 `ApiRequestError`，删掉兼容 getter 后上传超限的提示会悄悄退化（测试先证实，再修）。

### 过程上的失误

- 两次提交时门禁的某一步失败了却没注意到（一次是 `gen:api` 失败导致提交了旧的 `schema.d.ts`，一次是
  本地门禁脚本自己的 `&&` 漏洞放过了 clippy）。都在下一个提交里修了。之后所有提交都经过逐步检查
  退出码的门禁脚本。

### 逃生口清单（收口时）

- `any`：0
- `@ts-expect-error`：2，都在 `frontend/src/utils/money.spec.ts`，是故意的类型断言（裸 number 不能当 `Cents`）
- `as never`：7，全部是 multipart / 原始字节请求体（契约里是字段结构体，`FormData` 赋不上去）
- `as Cents`：2，`money.ts` 的 `toCents()` 与 `cents()`

### 并行基建的实测

`scripts/worktree-new.sh` 约 65 秒（主要是复制 8G 的 `target/debug`）；worktree 里第一次 `cargo test`
墙钟约 1 分 50 秒、CPU 约 30 分钟——复制的 target 大半没被复用（路径变了）。112 核上可以接受；
纯前端的 worker 其实不需要 target，下次可以加个开关跳过。

### 整支分支审查（2026-09-25）

一个没参与实现的审查者（Claude Opus）看了整支分支：0 Critical、3 Important、9 Minor。三条 Important 都修了，
每条先写能复现的测试：

1. **store 失败时的提示文案变了**（约 34 处）。旧 store 一律 `throw new Error(后端原文 || 中文兜底)`，组件读
   `err.message`；F1 改成原样抛 `ApiRequestError`，它的 message 永远非空，于是断网时点「完成配货」从
   「更新订单状态失败。」变成「网络错误」。根因在 brief-store 的一句假设（「组件读 `err.response.data.error`」，
   实际读的是 `err.message`）——**worker 忠实执行了一条错误的规则，批次审查也照着同一条规则审，看不出来。**
   现在由 `storeErrors.spec.ts` 表驱动钉住每个写操作的旧文案。
2. **Tauri 里多了 30 秒超时。** 旧版在 Tauri 里挂的是自定义 axios adapter，axios 的 timeout 只在内置 adapter
   里实现，所以桌面 / Android 以前从不超时。大号 `.boothpack` 导入会在前端报超时而后端仍在写。
3. **登录页输错密码被带到 404**（旧 bug）。登录路由是 `/login/:role`，401 处理却判断 `!== '/login'` 并跳去
   不存在的 `/login`。Review Focus 第 5 条的测试用的是字面量 `'/login'`，一直在空转。

9 条 Minor 记在台账里没有修，其中值得在真机上看一眼的是：`EditEventForm` 的日期绑定改成了
`formatted-value`（大概率是修了一个 bug，但属于未声明的行为变化），以及旧 iOS 上识图的超时提示。
测试：前端 67 → **170**（主要是 store 文案的表驱动测试）。
