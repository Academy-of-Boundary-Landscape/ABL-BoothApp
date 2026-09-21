# ① 安全网 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 给 v1.2 后续三个子项目铺一张网——PR 触发的 CI gate、Rust 与前端的测试脚手架、lint 接线并清零现有警告、数据库迁移前自动快照，以及一批零风险清扫。

**Architecture:** 纯工程改动，**不改变任何用户可见行为**。顺序上先清债再建门禁——把实测的 41 条 rustc + 17 条 clippy 警告清零、把前端依赖清单修诚实之后，CI 才加进来，这样 CI 从第一次运行就是绿的，不存在"先红着等人修"的窗口期。

**Tech Stack:** Rust 1.98.1 / axum 0.7 / sqlx 0.9 (sqlite, 运行时 migrate) / tower 0.4 / Vue 3 + Vite 7 / vitest / ESLint 9 flat config / GitHub Actions

**Spec:** `docs/superpowers/specs/2026-09-22-safety-net-design.md`
**Umbrella:** `docs/superpowers/specs/2026-09-22-v1.2-roadmap.md`

## Global Constraints

- **分支**：全部工作在 `1.2-dev` 上。不要合并到 `main`。
- **不改用户可见行为**。① 是纯工程改动；任何会改变界面、接口语义或数据内容的改动都超出范围。
- **rustc 钉死 1.98.1**（`rust-toolchain.toml`）。本机是 1.98.1，CI 默认 stable 会漂移，`-D warnings` 会在本机绿、CI 红。
- **Node 22**。`frontend/package.json` 的 `engines` 要求 `^20.19.0 || >=22.12.0`。
- **本机构建必须走 `tauri-env`**（见 `CLAUDE.md`）：`tauri-env linux cargo <...>`。裸跑 `cargo` 会因为缺 webkit2gtk 失败。
- **`cargo` 命令一律带 `--manifest-path src-tauri/Cargo.toml`**，或先 `cd src-tauri`。仓库根没有 `Cargo.toml`。
- **不碰这两个文件**：`my-release-key.jks`、`src-tauri/updater-key.key`。
- **不引入新的运行时依赖**。允许新增的只有 dev-dependency（Rust: `tempfile`）和 devDependencies（前端: `vitest` / `@vue/test-utils` / `jsdom`）。
- **不做**：E2E / Playwright 配置、`tsconfig.json`、`vue-tsc` 门禁、发版自动化、crate 拆分、覆盖率门槛。
- 提交信息用仓库现有的 gitmoji 风格（`git log` 可见），结尾带
  `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>`。

---

## File Structure

**新建**

| 文件 | 职责 |
|---|---|
| `rust-toolchain.toml` | 钉 rustc 版本 |
| `rustfmt.toml` | rustfmt 配置 |
| `.editorconfig`（仓库根） | 覆盖 `src-tauri/`，现有的只在 `frontend/` |
| `src-tauri/src/db/snapshot.rs` | 数据库快照与恢复，唯一职责 |
| `src-tauri/src/test_support.rs` | 测试夹具：内存库、AppState、Router |
| `frontend/eslint.config.js` | ESLint 9 flat config |
| `frontend/.prettierrc.json` | prettier 配置 |
| `frontend/src/utils/dateFormatter.spec.js` | 取代假测试 |
| `frontend/src/utils/upload.spec.js` | 纯函数测试 |
| `.github/workflows/ci.yml` | PR gate |

**修改**

| 文件 | 改什么 |
|---|---|
| `src-tauri/Cargo.toml` | 删 `[lints.rust]` allow 段；加 `[dev-dependencies] tempfile` |
| `src-tauri/src/lib.rs` | 挂 `mod test_support`（`#[cfg(test)]`） |
| `src-tauri/src/db/mod.rs` | 拆出 `seed_defaults`；`init_db` 接入快照；`reset_database` 接入快照 |
| `src-tauri/src/utils/{security,file,ip}.rs` | 各加 `#[cfg(test)] mod tests` |
| `src-tauri/src/api/event.rs` | 加 `#[cfg(test)] mod tests`（handler 级集成测试样例） |
| `frontend/package.json` | 补 5 个漏声明依赖、删 2 个死依赖、加 devDeps 与 scripts |
| `package.json`（根） | 删死 script、对齐 vite 版本 |
| `frontend/vite.config.js` | 加 vitest 配置段 |
| `frontend/src/services/url.js` | 导出 `SERVER_ORIGIN` 与 `toAbsoluteApiUrl` |
| `frontend/src/components/order/OrderCard.vue` | 用 `services/url.js` |
| `frontend/src/views/AdminEventStat.vue` | 用 `services/url.js` |
| `frontend/src/services/socketService.js` | 用 `services/url.js`；查明注释矛盾 |
| `frontend/src/router/index.js` | 删调试注释 |
| `docs/support/contact.md` 等 | 文档站修复 |

**删除**：`frontend/src/components/WebSocketTester.vue`、`frontend/src/utils/dateFormatter.test.js`

---

## Task 1: Rust lint 接线与清债

**Files:**
- Create: `rust-toolchain.toml`, `rustfmt.toml`, `.editorconfig`
- Modify: `src-tauri/Cargo.toml`（删 `[lints.rust]` 段），以及 clippy/rustc 报出的各源文件

**Interfaces:**
- Consumes: 无
- Produces: 一个 `cargo clippy --all-targets --all-features -- -D warnings` 能通过的代码库。后续所有任务都依赖这个前提。

**背景数据（已实测，不必重新测量）**：clippy 17 条（13 条可 `--fix`）；`[lints.rust]` 五项从 allow 放开后额外 41 条 = 14 未用变量 / 9 未用 import / 1 多余 `mut` / 17 死代码项。

- [ ] **Step 1: 建三个配置文件**

`rust-toolchain.toml`：
```toml
[toolchain]
channel = "1.98.1"
components = ["rustfmt", "clippy"]
```

`rustfmt.toml`：
```toml
edition = "2021"
```

`.editorconfig`（仓库根）：
```ini
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true

[*.rs]
indent_style = space
indent_size = 4

[*.{js,ts,vue,json,css,html,yml,yaml}]
indent_style = space
indent_size = 2

[*.md]
trim_trailing_whitespace = false
```

- [ ] **Step 2: 放开被静音的五类警告**

在 `src-tauri/Cargo.toml` 中**整段删除**：
```toml
[lints.rust]
unused_variables = "allow"
unused_imports = "allow"
unused_mut = "allow"
unused_assignments = "allow"
dead_code = "allow"
```
（rustc 对这五项的默认行为就是 warn，删掉即可，不要改成 `= "warn"` 再留一段冗余配置。）

- [ ] **Step 3: 跑 clippy 拿到完整清单**

Run: `tauri-env linux cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --message-format=short 2>&1 | grep '^src/' | sort -u`
Expected: 约 58 行（41 + 17，可能有重叠）。把输出存下来当作待办清单。

- [ ] **Step 4: 自动修可自动修的**

Run: `tauri-env linux cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --fix --allow-dirty`
Expected: 13 条左右被自动修掉（`if` 折叠进 `match`、多余 `return`、`&PathBuf` → `&Path`、`== []` → `is_empty()`、`chunks_exact` → `as_chunks` 等）。

- [ ] **Step 5: 手工清剩下的**

按这三条规则处理，**不要无差别加 `#![allow(...)]` 糊弄**：

1. **未用变量**（14 条）：确实不用的加 `_` 前缀；是遗漏的 bug 就修好它。
2. **未用 import**（9 条）+ **多余 `mut`**（1 条）：直接删。
3. **死代码**（17 条：5 个结构体从未构造、4 个字段从未读、若干未用方法/函数）：**逐个判断**。
   - 确属历史遗留、后续子项目也用不到的 → 删除。
   - 属于 ③a / ② 将要用到的 API 表面（例如 `db::models` 里对应表字段但当前没读的字段）→ 加
     `#[allow(dead_code)]` 并在**同一行上方写一句注释说明为什么保留**，例如：
     ```rust
     // 对应 order_items.product_code 列；③a 把响应类型化之后会用到。
     #[allow(dead_code)]
     pub product_code: String,
     ```
   - 拿不准的一律保留 + 注释，**宁可留也不要删**——删错了后续子项目要重写。

注意 `src-tauri/src/api/guard.rs:15` 已有一处 `#[allow(dead_code)]`（在 `AdminOnly` 上），保持不动。

- [ ] **Step 6: 格式化**

Run: `tauri-env linux cargo fmt --manifest-path src-tauri/Cargo.toml --all`

- [ ] **Step 7: 验证全绿**

Run:
```
tauri-env linux cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
tauri-env linux cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```
Expected: 两条都退出码 0，无任何输出的 warning。

- [ ] **Step 8: Commit**

```bash
git add rust-toolchain.toml rustfmt.toml .editorconfig src-tauri/
git commit -m "$(cat <<'EOF'
chore: :wrench: 放开被静音的 lint 并清零全部警告

删掉 Cargo.toml 里 allow 掉 unused_variables/unused_imports/unused_mut/
unused_assignments/dead_code 的那一段，清掉随之出现的 41 条 rustc 警告和
17 条 clippy 警告，之后 clippy -D warnings 可以通过。

rust-toolchain.toml 钉到 1.98.1：clippy 的 lint 集随版本漂移，不钉的话
-D warnings 会在本机绿、CI 红。

保留的死代码一律带注释说明为什么留。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Rust 测试脚手架与首批测试

**Files:**
- Create: `src-tauri/src/test_support.rs`
- Modify: `src-tauri/Cargo.toml`（加 `[dev-dependencies]`）、`src-tauri/src/lib.rs`（挂模块）、`src-tauri/src/db/mod.rs`（拆出 `seed_defaults`）、`src-tauri/src/utils/{security,file,ip}.rs`、`src-tauri/src/api/event.rs`

**Interfaces:**
- Consumes: Task 1 的 clippy 绿状态。
- Produces:
  - `crate::test_support::test_pool() -> sqlx::SqlitePool` —— 内存库，已跑完迁移并种好默认密码
  - `crate::test_support::test_state() -> (crate::state::AppState, tempfile::TempDir)` —— TempDir 必须由调用方持有，drop 掉目录就没了
  - `crate::test_support::test_router() -> (axum::Router, tempfile::TempDir)` —— 已 `with_state` 的完整 `/api` 路由
  - `crate::db::seed_defaults(pool: &SqlitePool) -> Result<(), sqlx::Error>` —— 从 `init_db` 里拆出来的默认管理员/摊主密码种子

- [ ] **Step 1: 加 dev-dependency**

在 `src-tauri/Cargo.toml` 的 `[build-dependencies]` 之前插入：
```toml
[dev-dependencies]
tempfile = "3"
```

`tower`（0.4, features=full）、`tokio`（features=full）、`serde_json` 已经是正式依赖，`tower::ServiceExt::oneshot` 直接可用，**不要重复添加**。

- [ ] **Step 2: 从 init_db 拆出 seed_defaults**

在 `src-tauri/src/db/mod.rs`，把现在 `init_db` 里第 43–74 行那两段"检查并初始化默认密码"抽成独立函数：

```rust
/// 种入默认的管理员 / 摊主密码。已存在则跳过。
/// 从 init_db 拆出来，让测试夹具能用同一份逻辑建库，避免两处漂移。
pub async fn seed_defaults(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let admin_exists: (i64,) =
        sqlx::query_as("SELECT count(*) FROM settings WHERE key = 'admin_password'")
            .fetch_one(pool)
            .await?;
    if admin_exists.0 == 0 {
        sqlx::query("INSERT INTO settings (key, value) VALUES ('admin_password', ?)")
            .bind(hash_password("admin123"))
            .execute(pool)
            .await?;
    }

    let vendor_exists: (i64,) =
        sqlx::query_as("SELECT count(*) FROM settings WHERE key = 'vendor_password'")
            .fetch_one(pool)
            .await?;
    if vendor_exists.0 == 0 {
        sqlx::query("INSERT INTO settings (key, value) VALUES ('vendor_password', ?)")
            .bind(hash_password("vendor123"))
            .execute(pool)
            .await?;
    }

    Ok(())
}
```

然后 `init_db` 里原来那两段替换成一行 `seed_defaults(&pool).await?;`。原来的两句 `println!` 一并移除（种子是否发生对用户无意义，且会污染测试输出）。

- [ ] **Step 3: 写测试夹具**

新建 `src-tauri/src/test_support.rs`：

```rust
//! 测试夹具。整个模块只在 cfg(test) 下编译。
//!
//! 业务逻辑跑在内嵌 axum server 上而不是 tauri IPC，所以 handler 可以脱离
//! Tauri runtime 测试：内存 SQLite + tower::ServiceExt::oneshot 就够，不需要开窗口。

use crate::state::AppState;
use crate::vision::VisionRuntime;
use axum::Router;
use sqlx::SqlitePool;
use std::sync::Arc;
use tempfile::TempDir;

/// 内存库，跑完迁移并种好默认密码。
///
/// 可行的前提是项目用的是运行时 `sqlx::migrate!()` 而不是编译期 `query!` 宏，
/// 所以不需要 DATABASE_URL。
pub async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite");
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("run migrations on in-memory db");
    crate::db::seed_defaults(&pool)
        .await
        .expect("seed default passwords");
    pool
}

/// AppState + 它依赖的临时目录。
///
/// **TempDir 必须由调用方持有到测试结束**：它一旦 drop，目录就被删掉了。
pub async fn test_state() -> (AppState, TempDir) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let upload_dir = dir.path().join("uploads");
    std::fs::create_dir_all(&upload_dir).expect("create upload dir");

    let pool = test_pool().await;

    // VisionRuntime::new 不触碰 ONNX 运行时（ort 用 load-dynamic，只在真正建
    // session 时才 dlopen），所以这里不需要任何模型文件。
    let vision_runtime = Arc::new(VisionRuntime::new(
        dir.path().to_path_buf(),
        upload_dir.clone(),
        pool.clone(),
    ));

    let state = AppState {
        db: pool,
        upload_dir,
        jwt_secret: "test-secret".to_string(),
        vision_runtime,
    };

    (state, dir)
}

/// 挂在 /api 下的完整路由，已 with_state。
pub async fn test_router() -> (Router, TempDir) {
    let (state, dir) = test_state().await;
    let router = Router::new()
        .nest("/api", crate::api::router())
        .with_state(state);
    (router, dir)
}
```

在 `src-tauri/src/lib.rs` 的模块声明处加上：
```rust
#[cfg(test)]
mod test_support;
```

- [ ] **Step 4: 跑一次确认夹具能编译**

Run: `tauri-env linux cargo test --manifest-path src-tauri/Cargo.toml --all-features 2>&1 | tail -20`
Expected: 编译通过，`running 0 tests`。若报 `crate::api::router` 不可见，把 `api/mod.rs` 的 `pub fn router` 所在模块在 `lib.rs` 里的声明确认为 `pub mod api;` 或 `mod api;`（同 crate 内 `mod` 即可访问）。

- [ ] **Step 5: 写 utils 的单元测试**

在 `src-tauri/src/utils/security.rs` 末尾追加：
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrip() {
        let h = hash_password("admin123");
        assert!(verify_password("admin123", &h));
    }

    #[test]
    fn verify_rejects_wrong_password() {
        let h = hash_password("admin123");
        assert!(!verify_password("admin124", &h));
    }

    #[test]
    fn same_plaintext_hashes_differently() {
        // bcrypt 每次用新的 salt，两次哈希不该相同
        assert_ne!(hash_password("same"), hash_password("same"));
    }

    #[test]
    fn verify_rejects_malformed_hash() {
        // verify 内部是 unwrap_or(false)，畸形 hash 不能 panic
        assert!(!verify_password("anything", "not-a-bcrypt-hash"));
    }
}
```

在 `src-tauri/src/utils/file.rs` 末尾追加：
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn save_upload_bytes_writes_file_and_returns_url() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();

        let url = save_upload_bytes(&base, b"hello", Some("a.png"), Some("products"))
            .await
            .unwrap();

        assert!(url.starts_with("/uploads/products/"));
        assert!(url.ends_with(".png"), "扩展名应沿用原始文件名: {url}");

        let on_disk = base.join(url.trim_start_matches("/uploads/"));
        assert_eq!(std::fs::read(on_disk).unwrap(), b"hello");
    }

    #[tokio::test]
    async fn save_upload_bytes_defaults_extension_to_jpg() {
        let dir = tempfile::tempdir().unwrap();
        let url = save_upload_bytes(&dir.path().to_path_buf(), b"x", None, None)
            .await
            .unwrap();
        assert!(url.ends_with(".jpg"), "无原始文件名时应回落到 jpg: {url}");
    }

    #[tokio::test]
    async fn delete_file_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let err = delete_file(&dir.path().to_path_buf(), "uploads/../../etc/passwd")
            .await
            .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[tokio::test]
    async fn delete_file_is_noop_for_empty_and_missing() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        assert!(delete_file(&base, "").await.is_ok());
        assert!(delete_file(&base, "uploads/nope.png").await.is_ok());
    }
}
```

在 `src-tauri/src/utils/ip.rs` 末尾追加（测纯函数，不碰真实网卡——真实网卡在 CI 上不可控）：
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotspot_adapter_is_included_despite_blacklist() {
        // "Microsoft Wi-Fi Direct Virtual Adapter" 同时含 "virtual"（黑名单）
        // 和 "wi-fi direct"（热点关键字），热点判断必须先生效。
        assert!(should_include_iface("Microsoft Wi-Fi Direct Virtual Adapter #2"));
    }

    #[test]
    fn virtual_adapters_are_excluded() {
        for name in ["VMware Network Adapter VMnet8", "docker0", "tailscale0", "vEthernet (WSL)"] {
            assert!(!should_include_iface(name), "{name} 应被排除");
        }
    }

    #[test]
    fn physical_adapters_are_included() {
        for name in ["wlan0", "eth0", "Wi-Fi", "以太网 Ethernet"] {
            assert!(should_include_iface(name), "{name} 应被包含");
        }
    }

    #[test]
    fn windows_hotspot_subnet_detection() {
        use std::net::Ipv4Addr;
        assert!(is_windows_hotspot_subnet(&Ipv4Addr::new(192, 168, 137, 1)));
        assert!(!is_windows_hotspot_subnet(&Ipv4Addr::new(192, 168, 1, 1)));
    }

    #[test]
    fn name_matching_is_case_insensitive() {
        assert!(name_matches("DOCKER0", &["docker"]));
    }
}
```

- [ ] **Step 6: 写 handler 级集成测试样例**

在 `src-tauri/src/api/event.rs` 末尾追加。**这一个测试的目的是把 `oneshot` + 内存库 + 迁移 + 路由这条链路走通一次，供后续子项目照抄**，不追求覆盖 event 模块：

```rust
#[cfg(test)]
mod tests {
    use crate::test_support::test_router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt; // for oneshot

    #[tokio::test]
    async fn list_events_returns_empty_array_on_fresh_db() {
        let (router, _dir) = test_router().await;

        let res = router
            .oneshot(
                Request::builder()
                    .uri("/api/events/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json.as_array().map(|a| a.len()), Some(0));
    }

    #[tokio::test]
    async fn admin_only_route_rejects_request_without_token() {
        // 证明 guard.rs 的 AdminOnly 提取器确实挂在链路上
        let (router, _dir) = test_router().await;

        let res = router
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/events/1/status")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"status":"active"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
```

若 `/api/events/` 的尾斜杠导致 404，改用 `/api/events`——`event::router()` 里注册的是 `.route("/", get(list_events))` 且被 `.nest("/events", ...)`，两种写法取能通的那个，**不要改路由定义**。

- [ ] **Step 7: 跑全部测试**

Run: `tauri-env linux cargo test --manifest-path src-tauri/Cargo.toml --all-features`
Expected: 全部通过，用例数 ≥ 14。

- [ ] **Step 8: 确认 clippy 仍然绿（测试代码也要过 -D warnings）**

Run: `tauri-env linux cargo fmt --manifest-path src-tauri/Cargo.toml --all && tauri-env linux cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
Expected: 退出码 0。

- [ ] **Step 9: Commit**

```bash
git add src-tauri/
git commit -m "$(cat <<'EOF'
test: :white_check_mark: Rust 测试脚手架与首批测试

业务逻辑跑在内嵌 axum 上而不是 tauri IPC，所以 handler 可以脱离 Tauri
runtime 测：内存 SQLite + tower oneshot 就够，不用开窗口；sqlx 用的是运行时
migrate! 而非编译期宏，也不需要 DATABASE_URL。

test_support 提供 test_pool / test_state / test_router 三个夹具。
为了让夹具和 init_db 用同一份种子逻辑，把默认密码那段拆成 db::seed_defaults。

首批覆盖 utils 的 security / file / ip，外加一个 handler 级集成测试样例，
供后续子项目照抄这条链路。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: 数据库迁移前快照

**Files:**
- Create: `src-tauri/src/db/snapshot.rs`
- Modify: `src-tauri/src/db/mod.rs`

**Interfaces:**
- Consumes: Task 2 加的 `tempfile` dev-dependency（本任务的测试自建 WAL 文件库，不走 `test_support` 的内存库——快照行为必须在真实文件 + WAL 上验）
- Produces:
  - `crate::db::snapshot::take(pool: &SqlitePool, db_path: &Path, tag: &str) -> Result<PathBuf, sqlx::Error>`
  - `crate::db::snapshot::restore(db_path: &Path, snap: &Path) -> std::io::Result<()>`
  - `crate::db::snapshot::prune(db_path: &Path, keep: usize)`

**为什么必须是 `VACUUM INTO` 而不是 `fs::copy`**：库跑在 WAL 模式下（`src-tauri/src/db/mod.rs:31`），直接拷 `.db` 会漏掉 `-wal` 中尚未 checkpoint 的数据，拷出来可能是残缺的。`VACUUM INTO` 产出的是事务一致的完整副本。

- [ ] **Step 1: 写失败的测试**

新建 `src-tauri/src/db/snapshot.rs`，先只写测试（实现留空会编译失败，这是预期的）：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::SqlitePool;
    use std::str::FromStr;

    /// 建一个 WAL 模式的文件库，跑完迁移，插一行 settings。
    async fn wal_db(dir: &std::path::Path) -> (SqlitePool, std::path::PathBuf) {
        let db_path = dir.join("sale_system.db");
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
            .unwrap()
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        let pool = SqlitePool::connect_with(opts).await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        sqlx::query("INSERT INTO settings (key, value) VALUES ('probe', 'v1')")
            .execute(&pool)
            .await
            .unwrap();
        (pool, db_path)
    }

    #[tokio::test]
    async fn take_produces_readable_consistent_copy() {
        let dir = tempfile::tempdir().unwrap();
        let (pool, db_path) = wal_db(dir.path()).await;

        let snap = take(&pool, &db_path, "premigrate").await.unwrap();
        assert!(snap.exists(), "快照文件应存在: {snap:?}");

        // 关键：不 checkpoint 直接打开快照，也必须读得到刚写入的那一行。
        // 这正是 fs::copy 会失败而 VACUUM INTO 能通过的地方。
        let snap_pool = SqlitePool::connect(&format!("sqlite://{}", snap.display()))
            .await
            .unwrap();
        let v: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'probe'")
            .fetch_one(&snap_pool)
            .await
            .unwrap();
        assert_eq!(v, "v1");
    }

    #[tokio::test]
    async fn prune_keeps_only_the_newest_n() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        std::fs::write(&db_path, b"").unwrap();

        for stamp in ["20260101000001", "20260101000002", "20260101000003", "20260101000004"] {
            std::fs::write(
                dir.path().join(format!("sale_system.db.bak-premigrate-{stamp}")),
                b"",
            )
            .unwrap();
        }
        // 无关文件不该被碰
        std::fs::write(dir.path().join("unrelated.txt"), b"").unwrap();

        prune(&db_path, 2);

        let mut left: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".bak-"))
            .collect();
        left.sort();
        assert_eq!(
            left,
            vec![
                "sale_system.db.bak-premigrate-20260101000003",
                "sale_system.db.bak-premigrate-20260101000004"
            ]
        );
        assert!(dir.path().join("unrelated.txt").exists());
        assert!(db_path.exists(), "主库不能被 prune 删掉");
    }

    #[tokio::test]
    async fn restore_replaces_db_and_clears_wal_sidecars() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        let snap = dir.path().join("snap.db");
        std::fs::write(&db_path, b"broken").unwrap();
        std::fs::write(dir.path().join("sale_system.db-wal"), b"stale").unwrap();
        std::fs::write(dir.path().join("sale_system.db-shm"), b"stale").unwrap();
        std::fs::write(&snap, b"good").unwrap();

        restore(&db_path, &snap).unwrap();

        assert_eq!(std::fs::read(&db_path).unwrap(), b"good");
        // 旧的 -wal/-shm 必须删掉，否则新库会被过期的 WAL 污染
        assert!(!dir.path().join("sale_system.db-wal").exists());
        assert!(!dir.path().join("sale_system.db-shm").exists());
    }
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `tauri-env linux cargo test --manifest-path src-tauri/Cargo.toml --all-features snapshot 2>&1 | tail -20`
Expected: 编译失败，报 `cannot find function take/prune/restore`。

- [ ] **Step 3: 实现**

在 `src-tauri/src/db/snapshot.rs` 顶部（`mod tests` 之前）写入：

```rust
//! 数据库快照与恢复。
//!
//! ② 之后每一次 schema 变更都靠这里兜底：迁移前落一份事务一致的副本，
//! 迁移失败就还原，不让应用带着半迁移的库启动。

use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

/// 保留的快照份数。更旧的在每次 take 之后删掉，避免无限堆积。
pub const KEEP: usize = 3;

fn bak_prefix(db_path: &Path) -> String {
    let stem = db_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "sale_system.db".to_string());
    format!("{stem}.bak-")
}

/// 用 `VACUUM INTO` 落一份事务一致的快照。
///
/// **必须是 VACUUM INTO 而不是 fs::copy**：库跑在 WAL 模式下，直接拷 .db
/// 会漏掉 -wal 中尚未 checkpoint 的数据，拷出来可能是残缺的。
pub async fn take(pool: &SqlitePool, db_path: &Path, tag: &str) -> Result<PathBuf, sqlx::Error> {
    let stamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let name = format!("{}{}-{}", bak_prefix(db_path), tag, stamp);
    let dest = db_path.with_file_name(name);

    // VACUUM INTO 要求目标文件不存在
    if dest.exists() {
        std::fs::remove_file(&dest).ok();
    }

    // VACUUM INTO 不接受绑定参数，只能拼字符串；转义单引号防止路径里的引号破坏语句
    let escaped = dest.to_string_lossy().replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{escaped}'"))
        .execute(pool)
        .await?;

    prune(db_path, KEEP);
    Ok(dest)
}

/// 用快照覆盖主库。调用前 pool 必须已经关闭。
pub fn restore(db_path: &Path, snap: &Path) -> std::io::Result<()> {
    // 过期的 -wal/-shm 必须删掉，否则会污染刚还原的库
    for suffix in ["-wal", "-shm"] {
        let side = db_path.with_file_name(format!(
            "{}{}",
            db_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            suffix
        ));
        if side.exists() {
            std::fs::remove_file(side).ok();
        }
    }
    std::fs::copy(snap, db_path)?;
    Ok(())
}

/// 只保留最新的 `keep` 份快照。文件名里的时间戳是定长的，按名字排序即按时间排序。
pub fn prune(db_path: &Path, keep: usize) {
    let Some(dir) = db_path.parent() else { return };
    let prefix = bak_prefix(db_path);

    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut baks: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().starts_with(&prefix))
                .unwrap_or(false)
        })
        .collect();

    if baks.len() <= keep {
        return;
    }

    baks.sort();
    let drop_count = baks.len() - keep;
    for p in baks.into_iter().take(drop_count) {
        std::fs::remove_file(p).ok();
    }
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `tauri-env linux cargo test --manifest-path src-tauri/Cargo.toml --all-features snapshot`
Expected: 3 个测试全部 PASS。

- [ ] **Step 5: 接进 init_db**

在 `src-tauri/src/db/mod.rs` 顶部加 `pub mod snapshot;`，然后把 `init_db` 里那行 `sqlx::migrate!().run(&pool).await?;`（原第 41 行）替换为：

```rust
    // 5. 迁移前快照 —— 只在「库已存在 且 确有待跑迁移」时落，全新安装不落。
    let migrator = sqlx::migrate!();
    let snap = if db_existed && has_pending(&pool, &migrator).await {
        match snapshot::take(&pool, &db_path, "premigrate").await {
            Ok(p) => {
                println!("[Booth Tool] pre-migration snapshot: {}", p.display());
                Some(p)
            }
            Err(e) => {
                // 快照失败不阻断启动，但要吼出来
                eprintln!("[Booth Tool] WARNING: pre-migration snapshot failed: {e}");
                None
            }
        }
    } else {
        None
    };

    // 6. 运行迁移 (使用运行时方式避免编译时需要 DATABASE_URL)
    if let Err(e) = migrator.run(&pool).await {
        if let Some(snap) = snap {
            eprintln!("[Booth Tool] migration failed ({e}); restoring snapshot");
            pool.close().await;
            if let Err(re) = snapshot::restore(&db_path, &snap) {
                eprintln!("[Booth Tool] FATAL: restore also failed: {re}");
            }
        }
        return Err(e.into());
    }
```

`db_existed` 由第 25 行那段改出来——把
```rust
    if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        Sqlite::create_database(&db_url).await?;
    }
```
改成
```rust
    let db_existed = Sqlite::database_exists(&db_url).await.unwrap_or(false);
    if !db_existed {
        Sqlite::create_database(&db_url).await?;
    }
```

并在文件里加上辅助函数：
```rust
/// 是否有尚未应用的迁移。表不存在（全新库）时返回 false——那种情况不需要快照。
async fn has_pending(pool: &SqlitePool, migrator: &sqlx::migrate::Migrator) -> bool {
    let applied: Option<i64> = sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .flatten();
    match (applied, migrator.iter().map(|m| m.version).max()) {
        (Some(a), Some(latest)) => latest > a,
        _ => false,
    }
}
```

注意 `init_db` 的返回类型是 `Result<SqlitePool, sqlx::Error>`，而 `migrator.run()` 返回的是 `MigrateError`。`sqlx::Error` 有 `From<MigrateError>`，所以 `e.into()` 可用；若编译器不认，改成 `return Err(sqlx::Error::Migrate(Box::new(e)));`。

- [ ] **Step 6: reset_database 也落快照**

在 `src-tauri/src/db/mod.rs` 的 `reset_database` 里，第 91 行 `if Sqlite::database_exists(...)` 那段**之前**插入——删库是整个应用里最危险的操作，删之前必须留一份：

```rust
    // 删库之前先落一份快照。这是全应用最危险的操作，没有第二次机会。
    if Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        match SqlitePool::connect(&db_url).await {
            Ok(p) => {
                match snapshot::take(&p, &db_path, "prereset").await {
                    Ok(s) => println!("[Booth Tool] pre-reset snapshot: {}", s.display()),
                    Err(e) => eprintln!("[Booth Tool] WARNING: pre-reset snapshot failed: {e}"),
                }
                p.close().await;
            }
            Err(e) => eprintln!("[Booth Tool] WARNING: cannot open db for pre-reset snapshot: {e}"),
        }
    }
```

- [ ] **Step 7: 验证**

Run:
```
tauri-env linux cargo test --manifest-path src-tauri/Cargo.toml --all-features
tauri-env linux cargo fmt --manifest-path src-tauri/Cargo.toml --all
tauri-env linux cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```
Expected: 测试全过，clippy 退出码 0。

- [ ] **Step 8: Commit**

```bash
git add src-tauri/
git commit -m "$(cat <<'EOF'
feat: :lock: 迁移前自动快照与失败回滚

init_db 在跑迁移之前先落一份事务一致的快照，迁移失败就还原，不让应用
带着半迁移的库启动。reset_database 删库之前同样留一份。

用 VACUUM INTO 而不是 fs::copy：库跑在 WAL 模式下，直接拷 .db 会漏掉 -wal
里尚未 checkpoint 的数据，拷出来可能是残缺的。测试里专门验了这一点——
不 checkpoint 直接打开快照也要读得到刚写入的行。

只在「库已存在且确有待跑迁移」时落快照，全新安装不落；保留最近 3 份。

② 之后每次 schema 变更都靠这个兜底。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: 前端清扫

**Files:**
- Modify: `frontend/package.json`, `package.json`（根）, `frontend/src/services/url.js`, `frontend/src/services/socketService.js`, `frontend/src/components/order/OrderCard.vue`, `frontend/src/views/AdminEventStat.vue`, `frontend/src/router/index.js`
- Delete: `frontend/src/components/WebSocketTester.vue`

**Interfaces:**
- Produces: `frontend/src/services/url.js` 新增两个具名导出
  - `SERVER_ORIGIN: string`
  - `toAbsoluteApiUrl(url: string): string`

- [ ] **Step 1: 修依赖清单的诚实性**

`frontend/package.json` 的 `dependencies` 中**新增**这 5 个（它们已被 `frontend/src` 直接 import，但只声明在仓库根，现在靠 Node 的模块解析向上走才能构建——版本与根 `package.json` 保持一致）：
```json
"@tauri-apps/plugin-dialog": "^2.7.3",
"@tauri-apps/plugin-fs": "^2.5.2",
"@tauri-apps/plugin-http": "^2.6.1",
"@tauri-apps/plugin-shell": "^2.3.6",
"vuedraggable": "^4.1.0"
```

**删除**这 2 个死依赖：`"@headlessui/vue"`（`frontend/src` 零引用）、`"vicons"`（真正在用的是 `@vicons/ionicons5`，是完全不同的包）。

注意 `@tauri-apps/plugin-http` 的版本**必须停在 `^2.6.1`**：`src-tauri/Cargo.toml` 里 `tauri-plugin-http` 被钉在 `"2.6"`，而 tauri CLI 2.11+ 会校验 npm 包与 Rust crate 的 major/minor 一致。

- [ ] **Step 2: 清理根 package.json**

删除 `scripts` 里的 `dev`、`build`、`preview` 三项——仓库根没有 `index.html`、没有 `vite.config.*`、没有 `src/`，这三个脚本跑不起来。保留 `tauri` 和三个 `docs:*`。

把 `devDependencies` 里的 `"@vitejs/plugin-vue": "^5.2.1"` 改为 `"^6.0.1"`、`"vite": "^6.0.3"` 改为 `"^7.0.6"`，与 `frontend/package.json` 对齐。

- [ ] **Step 3: 重装并验证依赖诚实性**

Run:
```bash
rm -rf frontend/node_modules frontend/package-lock.json
npm ci --prefix frontend 2>/dev/null || npm install --prefix frontend
mv node_modules /tmp/root_nm_backup
npm run build --prefix frontend
mv /tmp/root_nm_backup node_modules
```
Expected: **在仓库根 `node_modules` 被移走的情况下** `vite build` 成功。这一步失败说明还有未声明的依赖，把报错里的包名补进 `frontend/package.json` 再试。

（第一条 `npm ci` 在删掉 lock 后会失败，所以给了 `npm install` 的回退。装完记得把新生成的 `frontend/package-lock.json` 一起提交。）

- [ ] **Step 4: 把 URL 收口进 services/url.js**

在 `frontend/src/services/url.js` 中，把第 9 行的 `const SERVER_ORIGIN` 改为导出，并新增一个函数：

```js
export const SERVER_ORIGIN = `http://127.0.0.1:${API_PORT}`;

/**
 * 把后端返回的相对地址补成绝对地址。
 * 和 getImageUrl 的区别：这个不判断 Tauri 环境，任何相对路径都补全，
 * 用于 fetch / 下载链接这类必须拿到绝对地址的场景。
 * @param {string} url
 * @returns {string}
 */
export function toAbsoluteApiUrl(url) {
  if (!url) return url;
  if (url.startsWith('http://') || url.startsWith('https://')) return url;
  if (url.startsWith('/')) return `${SERVER_ORIGIN}${url}`;
  return `${SERVER_ORIGIN}/${url}`;
}
```

顺手删掉 `getImageUrl` 里第 28 行那句 `console.log(...)`——它在每次渲染图片时都会打日志。

- [ ] **Step 5: 三个调用点改用 services/url.js**

`frontend/src/components/order/OrderCard.vue:49`：删掉 `const backendUrl = 'http://127.0.0.1:5140';`，改为从 `@/services/url` 引入 `SERVER_ORIGIN`，并把模板里用到 `backendUrl` 的地方替换成 `SERVER_ORIGIN`（先 `grep -n backendUrl frontend/src/components/order/OrderCard.vue` 找全）。

`frontend/src/views/AdminEventStat.vue:223-229`：删掉本地的 `const API_ORIGIN` 和 `function toAbsoluteApiUrl`，改为 `import { toAbsoluteApiUrl } from '@/services/url'`。两者实现逻辑完全一致，直接替换。

`frontend/src/services/socketService.js:3`：把 `const URL = import.meta.env.VITE_API_URL || 'http://127.0.0.1:5140';` 改为
```js
import { SERVER_ORIGIN } from './url';
const URL = import.meta.env.VITE_API_URL || SERVER_ORIGIN;
```

- [ ] **Step 6: 查明 socketService 的注释矛盾**

`frontend/src/services/socketService.js:8-9` 现在是：
```js
  // 【核心改动】强制只使用 'polling'，禁用 'websocket'
  transports: ['websocket', 'polling'],
```
注释与代码互相矛盾。**查明步骤**：`git log -p --follow frontend/src/services/socketService.js` 看这两行分别是哪个提交引入的、顺序如何。

- 若历史显示代码是后来改的、注释忘了删 → 删注释，保留 `['websocket', 'polling']`。
- 若历史显示注释和代码同时引入（即从来就矛盾）或查不出结论 → **不要改代码**。局域网下的实时订单推送是摊主端的关键路径，盲改有真实风险。把注释替换成：
  ```js
  // FIXME(1.2): 这里的意图不明——原注释写「强制只用 polling，禁用 websocket」，
  // 但 transports 把 websocket 排在第一位。改动会影响局域网实时订单推送，
  // 需要在真机上验证过再动。见 docs/superpowers/specs/2026-09-22-safety-net-design.md
  ```

- [ ] **Step 7: 删死代码**

```bash
git rm frontend/src/components/WebSocketTester.vue
```
（先 `grep -rn "WebSocketTester" frontend/src` 确认仍是零引用。）

删除 `frontend/src/router/index.js` 里两处注释掉的调试日志（约在 138-139 行和 155 行附近，内容是 `// console.log('--- Router Guard: ...')`）。用 `grep -n "Router Guard" frontend/src/router/index.js` 定位，把**被注释掉的** console.log 整行删除，不要动守卫逻辑本身。

- [ ] **Step 8: 验证构建仍然通过**

Run: `npm run build --prefix frontend`
Expected: 构建成功。

- [ ] **Step 9: Commit**

```bash
git add -A frontend package.json
git commit -m "$(cat <<'EOF'
chore: :broom: 前端清扫——依赖诚实性、URL 收口、死代码

frontend/package.json 补上 5 个实际 import 但从未声明的包（vuedraggable
和 4 个 tauri plugin）。它们此前只在仓库根声明，构建能过纯粹是因为 Node
的模块解析会向上走到根 node_modules——其中 @tauri-apps/plugin-http 还是
services/api.js 的核心依赖。删掉 @headlessui/vue 和 vicons 两个零引用依赖。

仓库根删掉 dev/build/preview 三个死脚本（根本没有 index.html / vite.config
/ src），并把 vite 与 @vitejs/plugin-vue 版本对齐到 frontend。

三处绕过统一封装的后端地址收口到 services/url.js。删掉孤儿组件
WebSocketTester.vue 和路由守卫里的调试日志注释。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: 前端 lint / format / test 脚手架

**Files:**
- Create: `frontend/eslint.config.js`, `frontend/.prettierrc.json`, `frontend/src/utils/dateFormatter.spec.js`, `frontend/src/utils/upload.spec.js`
- Modify: `frontend/package.json`, `frontend/vite.config.js`
- Delete: `frontend/src/utils/dateFormatter.test.js`

**Interfaces:**
- Consumes: Task 4 之后诚实的 `frontend/package.json`
- Produces: 四个 npm script —— `lint`、`format`、`format:check`、`test:unit`

- [ ] **Step 1: 装测试依赖并加 scripts**

Run: `npm install --prefix frontend -D vitest @vue/test-utils jsdom`

在 `frontend/package.json` 的 `scripts` 中加入：
```json
"test:unit": "vitest run",
"test:watch": "vitest",
"lint": "eslint . --max-warnings 0",
"format": "prettier --write src/",
"format:check": "prettier --check src/"
```

**不要加** `type-check` / `vue-tsc` —— TS 在 ③b。

- [ ] **Step 2: 配 vitest（合并进 vite.config.js，不另起文件）**

在 `frontend/vite.config.js` 返回的对象里，`build` 同级新增：
```js
    test: {
      environment: 'jsdom',
      include: ['src/**/*.spec.js'],
      globals: false,
    },
```

`include` 只认 `*.spec.js`，是为了和被删掉的旧 `*.test.js` 划清界限。

- [ ] **Step 3: 写 eslint flat config**

新建 `frontend/eslint.config.js`：
```js
import js from '@eslint/js'
import pluginVue from 'eslint-plugin-vue'
import skipFormatting from '@vue/eslint-config-prettier/skip-formatting'

export default [
  {
    name: 'app/files-to-lint',
    files: ['**/*.{js,mjs,jsx,vue}'],
  },
  {
    name: 'app/files-to-ignore',
    ignores: ['**/dist/**', '**/dist-ssr/**', '**/coverage/**', '**/node_modules/**'],
  },
  js.configs.recommended,
  ...pluginVue.configs['flat/essential'],
  skipFormatting,
  {
    name: 'app/language-options',
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
      globals: {
        window: 'readonly',
        document: 'readonly',
        console: 'readonly',
        navigator: 'readonly',
        fetch: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        URL: 'readonly',
        Blob: 'readonly',
        File: 'readonly',
        FormData: 'readonly',
        Image: 'readonly',
        localStorage: 'readonly',
        sessionStorage: 'readonly',
        alert: 'readonly',
        confirm: 'readonly',
      },
    },
  },
]
```

若 `@eslint/js` 未安装，先 `npm install --prefix frontend -D @eslint/js`。

新建 `frontend/.prettierrc.json`（与现有 `frontend/.editorconfig` 保持一致）：
```json
{
  "semi": false,
  "singleQuote": true,
  "printWidth": 100,
  "tabWidth": 2,
  "trailingComma": "es5"
}
```

- [ ] **Step 4: 把假测试改写成真 vitest 用例**

```bash
git rm frontend/src/utils/dateFormatter.test.js
```

新建 `frontend/src/utils/dateFormatter.spec.js`。**下面的期望值已在 Node 22 上实测过，不是猜的**：

```js
import { describe, it, expect } from 'vitest'
import {
  formatTimestamp,
  formatDate,
  formatTime,
  formatChartLabel,
  formatChartTooltip,
} from './dateFormatter'

describe('formatTimestamp', () => {
  it('把无时区标记的时间戳当作 UTC，转成 UTC+8', () => {
    expect(formatTimestamp('2024-01-19 08:30:00')).toBe('2024/01/19 16:30:00')
  })

  it('跨日转换正确', () => {
    expect(formatTimestamp('2024-01-19 16:00:00Z')).toBe('2024/01/20 00:00:00')
  })

  it('接受 ISO 格式', () => {
    expect(formatTimestamp('2024-01-19T08:30:00')).toBe('2024/01/19 16:30:00')
  })

  it('showSeconds=false 时不带秒', () => {
    expect(formatTimestamp('2024-01-19 08:30:00', false)).toBe('2024/01/19 16:30')
  })

  it('空值返回短横线', () => {
    expect(formatTimestamp('')).toBe('-')
    expect(formatTimestamp(null)).toBe('-')
    expect(formatTimestamp(undefined)).toBe('-')
  })

  it('无法解析时原样返回输入', () => {
    expect(formatTimestamp('not-a-date')).toBe('not-a-date')
  })
})

describe('formatDate / formatTime', () => {
  it('formatDate 只给日期', () => {
    expect(formatDate('2024-01-19 08:30:00')).toBe('2024/01/19')
  })

  it('formatTime 只给时间', () => {
    expect(formatTime('2024-01-19 08:30:00')).toBe('16:30:00')
  })

  it('都对空值返回短横线', () => {
    expect(formatDate(null)).toBe('-')
    expect(formatTime(null)).toBe('-')
  })
})

describe('图表格式化', () => {
  it('formatChartLabel 不含年和秒', () => {
    expect(formatChartLabel('2024-01-19 08:30:00')).toBe('01/19 16:30')
  })

  it('formatChartTooltip 含年不含秒', () => {
    expect(formatChartTooltip('2024-01-19 08:30:00')).toBe('2024/01/19 16:30')
  })
})
```

**如果 `formatChartLabel` / `formatChartTooltip` / `showSeconds=false` 这三条的实际输出与上面写的不一致**（它们的期望值是按同一套 `toLocaleString` 规则推的，未逐条实测），以实际输出为准修正**测试里的期望值**，不要去改 `dateFormatter.js` —— 这一轮的目的是把现有行为固定下来，不是改行为。

- [ ] **Step 5: 写 upload.js 的测试**

新建 `frontend/src/utils/upload.spec.js`。注意 `upload.js` 在模块顶层执行了 `createDiscreteApi(['dialog'])`，所以必须 mock 掉 naive-ui，否则 import 时就会炸：

```js
import { describe, it, expect, vi } from 'vitest'

vi.mock('naive-ui', () => ({
  createDiscreteApi: () => ({ dialog: { warning: vi.fn() } }),
}))

const { bytesFromMb, validateFileSize, normalizeUploadError, IMAGE_UPLOAD_LIMIT_MB } =
  await import('./upload')

describe('bytesFromMb', () => {
  it('按 1024 换算', () => {
    expect(bytesFromMb(1)).toBe(1048576)
    expect(bytesFromMb(10)).toBe(10485760)
  })
})

describe('validateFileSize', () => {
  it('没有文件时不通过', () => {
    expect(validateFileSize(null, 10)).toEqual({ ok: false, message: '未选择文件。' })
  })

  it('恰好等于上限时通过（边界是闭区间）', () => {
    expect(validateFileSize({ size: bytesFromMb(10) }, 10)).toEqual({ ok: true })
  })

  it('超过上限时不通过并带上限数字', () => {
    const r = validateFileSize({ size: bytesFromMb(10) + 1 }, 10)
    expect(r.ok).toBe(false)
    expect(r.message).toContain('10MB')
  })
})

describe('normalizeUploadError', () => {
  it('优先取后端返回的 error 字段', () => {
    const err = { response: { data: { error: '后端说不行' } } }
    expect(normalizeUploadError(err, 10)).toBe('后端说不行')
  })

  it('413 翻译成体积超限提示', () => {
    const err = { response: { status: 413, data: {} } }
    expect(normalizeUploadError(err, 10)).toContain('10MB')
  })

  it('multipart 解析错误也翻译成体积超限提示', () => {
    const err = { message: 'Error parsing `multipart/form-data` request' }
    expect(normalizeUploadError(err, 10)).toContain('10MB')
  })

  it('什么都没有时给兜底文案', () => {
    expect(normalizeUploadError({}, 10)).toBe('上传失败，请稍后重试。')
  })
})

describe('常量', () => {
  it('图片上限是 10MB', () => {
    expect(IMAGE_UPLOAD_LIMIT_MB).toBe(10)
  })
})
```

- [ ] **Step 6: 跑测试**

Run: `npm run test:unit --prefix frontend`
Expected: 全部通过，用例数 ≥ 18。若 `formatChartLabel` 等的期望值不对，按 Step 4 的说明修正期望值后重跑。

- [ ] **Step 7: 跑 lint 和 format，修掉报出来的问题**

Run:
```bash
npm run format --prefix frontend
npm run lint --prefix frontend
```

`lint` 若报错：**只修真问题**（未定义变量、未使用变量、Vue 模板错误）。如果某条规则在这份历史代码上大面积误报且价值不高，在 `eslint.config.js` 末尾的配置块里显式关掉它并写注释说明原因——**但不要整体降级为 `--max-warnings` 放宽**，那样门禁就没有意义了。

- [ ] **Step 8: 验证全绿**

Run:
```bash
npm run lint --prefix frontend
npm run format:check --prefix frontend
npm run test:unit --prefix frontend
npm run build --prefix frontend
```
Expected: 四条全部退出码 0。

- [ ] **Step 9: Commit**

```bash
git add -A frontend
git commit -m "$(cat <<'EOF'
test: :white_check_mark: 前端 lint / format / test 脚手架接线

eslint、prettier、typescript、playwright 这些依赖早就装在
frontend/package.json 里，但一个配置文件都没有，等于从未启用。这次把
eslint flat config 和 prettier 配起来，并加上 vitest。

把 dateFormatter.test.js 从 console.log("✓ 通过") 脚本改写成真的 vitest
用例——它本来就写好了期望值，缺的只是断言和退出码，CI 根本判不了成败。
补上 upload.js 纯函数的测试（naive-ui 要 mock，因为模块顶层就调了
createDiscreteApi）。

暂不接 vue-tsc / tsconfig：当前零 TS 代码，空门禁没有意义，跟着 ③b 一起落地。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: CI

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: Task 1–5 全部完成（代码已经是绿的，CI 从第一次运行就该是绿的）

- [ ] **Step 1: 写 workflow**

新建 `.github/workflows/ci.yml`：

```yaml
name: CI

on:
  pull_request:
    branches: [main, 1.2-dev]
  push:
    branches: [1.2-dev]

# 同一分支上新的 push 取消上一次未跑完的
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

jobs:
  rust:
    name: Rust (fmt / clippy / test)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # tauri crate 需要这些系统库才能编译。
      # 注意：不需要 onnxruntime —— ort 配的是 load-dynamic，编译期不链接它。
      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libgtk-3-dev \
            libsoup-3.0-dev \
            libjavascriptcoregtk-4.1-dev \
            librsvg2-dev \
            libayatana-appindicator3-dev \
            patchelf

      # rust-toolchain.toml 钉了 1.98.1，这里不指定版本，让它读文件
      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - name: cargo fmt
        working-directory: src-tauri
        run: cargo fmt --all --check

      - name: cargo clippy
        working-directory: src-tauri
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: cargo test
        working-directory: src-tauri
        run: cargo test --all-features

  frontend:
    name: Frontend (lint / test / build)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: frontend/package-lock.json

      # 故意只装 frontend、不装仓库根：
      # 这让「frontend/package.json 必须是一份诚实的清单」成为永久守卫。
      # 一旦有人 import 了未声明的包，这一步就会红。
      - name: Install frontend dependencies
        run: npm ci --prefix frontend

      - name: Lint
        run: npm run lint --prefix frontend

      - name: Format check
        run: npm run format:check --prefix frontend

      - name: Unit tests
        run: npm run test:unit --prefix frontend

      - name: Build
        run: npm run build --prefix frontend
```

- [ ] **Step 2: 本地预演 rust job**

Run:
```bash
cd src-tauri
tauri-env linux cargo fmt --all --check
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features
cd ..
```
Expected: 三条全绿。（本机必须走 `tauri-env`；CI 上系统依赖由 apt 装好，直接 `cargo` 即可，所以 yml 里不带 `tauri-env`——那是本机专用的包装器。）

- [ ] **Step 3: 本地预演 frontend job**

Run:
```bash
mv node_modules /tmp/root_nm_backup
npm ci --prefix frontend
npm run lint --prefix frontend
npm run format:check --prefix frontend
npm run test:unit --prefix frontend
npm run build --prefix frontend
mv /tmp/root_nm_backup node_modules
```
Expected: 全绿。**这一步就是 CI 环境的忠实模拟**——根 `node_modules` 不存在。

- [ ] **Step 4: 校验 yaml 语法**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml')); print('yaml ok')"`
Expected: `yaml ok`

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "$(cat <<'EOF'
ci: :construction_worker: PR gate —— Rust 与前端

此前仓库里唯一的 workflow 是 deploy-docs.yml，只管文档站，完全不碰
src-tauri/ 和 frontend/，应用代码没有任何 PR 检查。v1.1.0 发布即救火，
缺的就是这一层。

rust job：fmt --check / clippy -D warnings / test，全部 --all-features。
不需要下载 onnxruntime —— ort 配的是 load-dynamic，编译期不链接。

frontend job 故意只装 frontend、不装仓库根，把「frontend/package.json
必须是一份诚实的清单」变成永久守卫：再有人 import 未声明的包，CI 立刻红。

不做发版自动化：那需要把 Android keystore 和 updater 私钥传进 GitHub
Secrets，而这两把密钥丢了找不回来。发版继续用 scripts/ 在本机跑。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: 文档站修复

**Files:**
- Modify: `docs/support/contact.md`, `docs/faq/community.md`, `docs/en/faq/community.md`, `docs/ja/faq/community.md`, `docs/guide/export.md`, `CHANGELOG-v1.1.md`

这些是线上用户看得到的内容错误，与代码无关，所以单独一个 commit。

- [ ] **Step 1: 恢复 contact.md 的中文**

`docs/support/contact.md` 现在整篇是日语（简体中文是默认 locale，这一页被错放覆盖了，中文原文已丢失）。

先看 `docs/en/support/contact.md` 和 `docs/ja/support/contact.md` 的结构，然后**按同样的章节结构写一份中文版**。frontmatter 改成：
```yaml
---
title: 联系我们 & 支持
description: 遇到问题不知道该找谁？这里按场景列出最合适的反馈渠道。
---
```
正文按日语/英语版的章节一一对应翻译成中文（反馈渠道、issue 链接、社区入口等）。**链接和邮箱地址原样保留，不要改**。

- [ ] **Step 2: 订正自动更新的说法（三语）**

`docs/faq/community.md:14-17` 现在是：
```
## 我能自动更新吗？

当前版本不支持一键自动更新，需要手动下载安装包进行覆盖安装。
```

v1.1.1 起 Windows 已支持一键自动更新（见 `docs/guide/auto-update.md` 和
`docs/releases/v1.1.1.md`），Android 因系统限制仍需手动安装。改为：

```
## 我能自动更新吗？

Windows 版从 v1.1.1 起支持一键自动更新：App 会自己检查新版本，确认后下载并安装。
详见[自动更新](/guide/auto-update)。

Android 版受系统限制，仍然需要手动下载 APK 安装。
```

`docs/en/faq/community.md` 和 `docs/ja/faq/community.md` 里的对应条目同样订正（这三份是 `docs/translate_faq.py` 一次性批量翻译的产物，没有持续同步机制，所以是同批过时）。英/日版写对应语言。

- [ ] **Step 3: 补实 export.md**

`docs/guide/export.md` 全文只有"这个页面还在建设中，敬请期待！"，但 `docs/.vitepress/config.js` 的侧边栏挂着"导出与复盘"入口。

**导出能力本来就有，缺的只是描述。** 先读这两个文件确认实际能导出什么：
- `src-tauri/src/api/stats.rs` —— 销售统计的 Excel / CSV 导出
- `src-tauri/src/api/sync.rs` —— `.boothpack` 商品包导出/导入

再看前端入口：`frontend/src/views/AdminEventStat.vue`（统计页导出按钮）、
`frontend/src/components/product/BoothpackSyncPanel.vue`（商品包同步面板）。

然后写一页中文文档，覆盖：每种导出的入口在哪、产出什么格式的文件、典型用途
（销售报表交社团主催复盘 / `.boothpack` 换设备迁移商品库）。

**必须写清楚一条边界**：`.boothpack` 只包含商品库和图片，**不包含展会、订单和销售流水**。
这一点用户很容易误解成"导出了就等于全量备份"。

- [ ] **Step 4: 说明 CHANGELOG 的分裂**

`CHANGELOG-v1.1.md` 文件名暗示覆盖整个 v1.1.x，实际只有 `# v1.1.0 Changelog` 一节；
v1.1.1 的变更单独在 `docs/releases/v1.1.1.md`。

在 `CHANGELOG-v1.1.md` 的标题下方加一行指路，不要搬运内容：
```markdown
> v1.1.1 的变更记录在 [docs/releases/v1.1.1.md](docs/releases/v1.1.1.md)。
```

- [ ] **Step 5: 验证文档站能构建**

Run: `npm run docs:build`
Expected: vitepress 构建成功，无死链警告。

- [ ] **Step 6: Commit**

```bash
git add docs CHANGELOG-v1.1.md
git commit -m "$(cat <<'EOF'
docs: :memo: 修文档站里三个线上用户看得到的错误

- docs/support/contact.md 整篇是日语（简体中文默认 locale 的页面被错放
  覆盖，中文原文已丢失），重写中文版
- faq/community.md 仍写「不支持一键自动更新」，但 v1.1.1 起 Windows 已支持；
  中英日三版同批订正（translate_faq.py 是一次性批量翻译，没有同步机制）
- guide/export.md 是「建设中」空壳但侧边栏挂着入口。导出能力本来就有，
  补上描述，并写清 .boothpack 不含展会/订单/销售流水这条边界

CHANGELOG-v1.1.md 只含 v1.1.0，加一行指向 v1.1.1 的发布说明。

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## 完成标准

全部任务做完后，下面每一条都必须**实际跑过并看到通过**，不能凭印象声称：

```bash
# Rust
cd src-tauri
tauri-env linux cargo fmt --all --check
tauri-env linux cargo clippy --all-targets --all-features -- -D warnings
tauri-env linux cargo test --all-features        # 用例数 > 0
cd ..

# 前端（模拟 CI：根 node_modules 不可见）
mv node_modules /tmp/root_nm_backup
npm ci --prefix frontend
npm run lint --prefix frontend
npm run format:check --prefix frontend
npm run test:unit --prefix frontend              # 用例数 > 0
npm run build --prefix frontend
mv /tmp/root_nm_backup node_modules

# 文档站
npm run docs:build

# 工作区干净
git status --short
```

**不要声称 ① 完成，除非上面每条命令都跑过且退出码为 0。**
