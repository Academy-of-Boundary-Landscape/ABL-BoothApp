# 任务：把 `src-tauri/src/api/{{MODULE}}.rs` 迁移到 utoipa（③b 阶段 2）

你在一个独立的 git worktree 里工作：当前目录就是仓库根，分支 `3b/{{NAME}}`。
只改下面「允许改的文件」。完成后把报告写到 `.3b/REPORT.md`。
**不要 `git commit`**（沙箱写不了 worktree 的 git 元数据，试了会报 Read-only file system）——改动留在工作树里，审查者会代为提交。

## 背景
后端是跑在 Tauri 进程里的 axum 0.8 HTTP server。我们用 utoipa 6 + utoipa-axum 0.3 给每个路由生成
OpenAPI 文档，前端再从文档生成 TS 类型。**`src-tauri/src/api/settlement.rs` 已经迁移完，它就是你的模板——
动手前先通读它的 `router()`、每个 `#[utoipa::path]` 注解、类型上的 derive，以及文件末尾的 `mod shape_tests`。**

## 允许改的文件
- `src-tauri/src/api/{{MODULE}}.rs`
- `src-tauri/src/domain/`、`src-tauri/src/db/` 下被本模块响应/请求用到的类型：**只许加 derive 和 `#[schema(...)]` 属性**，不改字段、不改逻辑
- **不许改**：`api/mod.rs`、`api/openapi.rs`、`test_support.rs`、`src-tauri/openapi.json`、`Cargo.toml`、迁移文件、任何前端文件

## 硬约束
1. **JSON 形状一个字节都不许变**：字段名、字段有无、null 还是缺省、数字还是字符串、HTTP 状态码、错误体。
   想改的写进 `.3b/REPORT.md` 的「形状清理」一节（表格列：模块 | 路由 | 现状 | 建议 | 理由 | 前端消费方），**不动手**。
   （不要直接改 `docs/…/shape-cleanups.md`：几个 worker 同时追加同一个文件，合并时必冲突。）
2. **原来吞错误的继续吞**（`.unwrap_or_default()`、`.unwrap_or(None)`、`let _ =` 等），同样记进「形状清理」一节。
3. 错误响应一律是 `{"error": "..."}`。手拼的 `(StatusCode::X, Json(json!({"error": msg})))`，**只有**状态码与文字
   完全一致时才可以换成对应的 `ApiError` 变体（400 `BadRequest` / 404 `NotFound` / 409 `Conflict`；
   `ApiError::Forbidden` 的文字固定是「权限不足」，`ApiError::Db` 固定是 500「数据库错误」）；否则保留原样的手拼响应。
4. 不许改任何业务逻辑、SQL、权限检查、展会守卫（`require_event_open` 和 `// 不需要展会守卫：…` 注释都原样保留）。
5. 本模块的前端消费方：{{CONSUMERS}}。你**不改**它们，但要读它们，确认你钉住的形状就是它们在读的形状。

## 步骤

### 1. 先写形状快照测试，先绿
在本模块文件末尾新建 `#[cfg(test)] mod shape_tests`，照 settlement.rs 的写法：一个 `seeded()` 造数据，一个 `call()` 发请求，
然后**每个路由×方法至少一条** `shape_<handler 名>` 测试，断言 `(状态码, shape_of(&body))`。

- 夹具在 `crate::test_support`：`test_router_with()`（返回 router、TempDir、pool）、`seed_event_and_product(&pool)`
  （一场进行中的展会 + 两个商品，返回 `(event_id, ep_a, ep_b)`）、`admin_token()`、`vendor_token(event_id)`、
  `json_request(method, uri, token, body)`、`read_json(res)`（空 body 返回 `Value::Null`）、`place(&router, event_id, items)`、`shape_of(&v)`。
- `shape_of` 把数组**所有元素**的形状合并（键取并集，类型冲突记为 `"null|string"`），空数组是 `["empty"]`。
- **种子数据要让每个可选字段和每个数组至少出现一次非空**，否则形状里全是 `"null"` / `["empty"]`，钉不住东西。
- 写法：先 `println!("{}", serde_json::to_string(&shape_of(&body)).unwrap())`，`cargo test … -- --nocapture` 看实际形状，
  对照前端消费方确认合理，再抄进 `assert_eq!`。
- 有错误分支的路由至少一条错误路径（断言状态码 + `json!({"error": "string"})`，或原样的非标准错误体）。
- 二进制响应断言状态码 + `content-type`。multipart 路由手写 multipart body：
  ```rust
  let boundary = "X-BOUNDARY";
  let body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\n值\r\n--{boundary}--\r\n");
  let req = axum::http::Request::builder().method("POST").uri(uri)
      .header("authorization", format!("Bearer {}", admin_token()))
      .header("content-type", format!("multipart/form-data; boundary={boundary}"))
      .body(axum::body::Body::from(body)).unwrap();
  ```
- **自检这些测试有牙**：临时给某个响应字段加 `#[serde(rename = "xxx")]`，确认对应测试变红，再改回来。

跑绿后再进入第 2 步（不要提交）。

### 2. 类型化
- `-> impl IntoResponse` 的 handler 改成 `ApiResult<Json<T>>` / `ApiResult<(StatusCode, Json<T>)>` / `ApiResult<StatusCode>`。
  确实返回非标准形状的（如 `(StatusCode::OK, Json(json!({"message": "..."})))`），为它建具名结构体，保持同样的序列化结果。
  二进制下载、原样透传文件等可以保持返回 `Response`。
- `json!` 现拼的成功响应换成 `#[derive(Serialize, ToSchema)]` 结构体。**序列化结果必须相同**：
  原来输出 `null` 的 `Option` 字段保持输出 `null`，不要加 `skip_serializing_if`；原来缺省的，保留缺省（用 `skip_serializing_if`）。
- 请求体类型加 `ToSchema`，查询参数类型加 `IntoParams`（`#[derive(Deserialize, IntoParams)]`，注解里写 `params(QueryType)`）。
- **金额**：已经是 `crate::domain::money::Money` 的不动。还是 `i64`/`f64` 的金额字段**不改类型**，
  加 `#[schema(value_type = Money)]`（i64 分；`Option<i64>` 用 `value_type = Option<Money>`）——`f64` 元的**不要**标 Money，记进「形状清理」（它是元不是分）。
- **schema 名全局唯一**：`Entry`、`Row`、`Item`、`Request`、`Response` 这类通用名，用 `#[schema(as = {{PREFIX}}Entry)]` 加模块前缀。
  不同模块的同名类型会在 openapi.json 里互相覆盖，而且不报错。
- 一个 multipart 请求体，写一个只用于文档的结构体描述它的字段：
  ```rust
  /// 仅用于 OpenAPI 文档：multipart 表单的字段。
  #[derive(ToSchema)]
  #[allow(dead_code)]
  struct CreateXxxForm {
      name: String,
      #[schema(value_type = Option<String>, format = Binary)]
      image: Option<Vec<u8>>,
  }
  // 注解里：request_body(content = CreateXxxForm, content_type = "multipart/form-data")
  ```
  原始字节：`request_body(content = Vec<u8>, content_type = "application/octet-stream")`。

### 3. 标注解
每个 handler 在 doc 注释之后、`async fn` 之前加 `#[utoipa::path(...)]`，照 settlement.rs：
- 方法、`path`：**相对于 `api/mod.rs` 里本模块的 nest 前缀**（`merge` 进来的模块写完整路径；`nest("/events", …)` 的模块写 `"/{id}"` 这类）。
  路径参数用 `{name}`，和路由字符串一致。
- `tag = "{{MODULE}}"`；`params(("id" = i64, Path, description = "…"))`；`request_body = T`。
- `security(("bearer" = []))`——**必须是字面量 `"bearer"`**（宏不接受常量）。handler 参数里有 `Claims` 或 `AdminOnly` 才写。
- `responses(...)`：成功类型 + 每个可能的错误码，错误 body 用 `crate::api::openapi::ApiErrorBody`。
  有 `Claims`/`AdminOnly` 的都有 401；`AdminOnly` 的还有 403。
- doc 注释第一行写这个接口干什么；需要管理员的在 doc 里写「需要管理员」。没有 doc 注释的 handler 补一行。

### 4. router 改 `routes!`
删掉 `legacy_router()` 和 `OpenApiRouter::from(...)` 过渡包装，改成：
```rust
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_xxx, create_xxx))   // 同一路径的不同方法放在同一个 routes! 里
        .routes(routes!(get_xxx, update_xxx, delete_xxx))
}
```
原来 router 上的 `.layer(...)`（如 `DefaultBodyLimit::max(...)`）原样挂到 `OpenApiRouter` 上。删掉不再用的 import。

### 5. 验证（全部通过才算完成）
```bash
cd src-tauri
tauri-env linux cargo fmt
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features 2>&1 | grep -E "test result|FAILED|panicked"
#   ↑ 除了 openapi_snapshot 之外全绿；第 1 步的 shape_ 测试一行没改照样绿。openapi_snapshot 红是预期的。
python3 ../scripts/check-event-guards.py        # 必须通过
UPDATE_OPENAPI=1 tauri-env linux cargo test --all-features openapi_snapshot   # 只为看生成结果
jq -r '.paths | to_entries[] | select([.value[].tags[]?] | index("{{MODULE}}")) | .key + " " + (.value | keys | join(","))' openapi.json
git checkout openapi.json                        # 不提交 openapi.json
```
检查：本模块每个路由×方法都出现、路径前缀完整（和 `api/mod.rs` 的 nest 拼起来一致）、金额字段是 `$ref: Money`、有守卫的路由有 security。

第一次 `cargo test` 要编译 1～3 分钟，属正常。

### 6. 不要提交
改动留在工作树（`git status` 里应只有 `src-tauri/` 下的文件）。审查者会代为提交。

## 不要做
- 不要起 dev server（`npx tauri dev`、`vite`），不要开 VNC，不要截图。
- 不要 `cargo update`、不要改 `Cargo.toml`、不要 `npm install`。
- 不要重构、拆文件、改名、"顺手"修 bug（发现 bug 写进 REPORT）。

## `.3b/REPORT.md` 格式
1. 改了哪些文件
2. 每个 handler：前后签名、对应的形状测试名
3. 「形状清理」一节（表格，没有就写「无」）
4. 不确定的地方、发现的疑似 bug
5. 第 5 步各命令的结果（通过 / 失败 + 摘要），以及 jq 列出的路径
