# ① 安全网 设计

**日期**: 2026-09-22
**作者**: Renko_6626（与 Claude 协作）
**状态**: Draft — 排期 v1.2.0（子项目 ①，四个中的第一个）
**前置阅读**: `docs/superpowers/specs/2026-09-22-v1.2-roadmap.md`

## 背景

v1.2 四个子项目中的第一个。它本身不改变任何用户可见的行为，作用是给后面三个子项目
（③a handler 类型化、② 领域模型、③b 契约与 TS、④ 外观）铺一张网。

必要性来自 v1.1.0 的教训：那次"跨越数月、上百处改动"是一个 squash 巨型提交，
发布两天后 `ee8b02e` 就得紧急修管理员改密码，05-01 当天连打 7 个补丁。
**直接原因是没有增量记录、没有 CI、没有测试。** ② 的改动面比 v1.1.0 更大，
在没有安全网的情况下重做领域模型是在重演同一个剧本。

### 现状测量（2026-09-22 实测，非估计）

| 项 | 实测值 |
|---|---|
| Rust 测试 | `#[test]` / `#[cfg(test)]` / `tests/` 全为 **0**，无 `[dev-dependencies]` |
| 前端测试 | 唯一的 `frontend/src/utils/dateFormatter.test.js` 是 `console.log("✓ 通过")` 脚本，**无断言、无退出码、未接入任何 npm script** |
| CI | 只有 `.github/workflows/deploy-docs.yml`，**只管文档站**，不碰 `src-tauri/` 和 `frontend/` |
| 配置文件 | `frontend/tsconfig.json`、`frontend/eslint.config.js`、`frontend/playwright.config.ts` **三个都不存在**（依赖全装了） |
| clippy 基线 | 本项目源码 **17 条 warning**，其中 13 条 `clippy --fix` 可自动修 |
| 五类静音警告打开后 | 本项目源码 **41 条 warning**：14 未用变量 / 9 未用 import / 1 多余 `mut` / 17 死代码项 |
| rustc | 本机 1.98.1，仓库**无 `rust-toolchain.toml`** |

**结论：债务总量是几十条，不是几千条。CI 可以一步到位上 `-D warnings`，不需要过渡期。**

### 两个调研中发现、此前未记录的问题

**A. 前端依赖清单不诚实。** 以下 5 个包被 `frontend/src` 直接 import，但**没有声明在
`frontend/package.json`**，只存在于仓库根的 `node_modules`：

| 包 | 引用点 |
|---|---|
| `vuedraggable` | `frontend/src/components/customer/ProductGrid.vue:110` |
| `@tauri-apps/plugin-http` | `frontend/src/services/api.js:2`、`frontend/src/views/AdminEventStat.vue:162` |
| `@tauri-apps/plugin-fs` | `frontend/src/views/AdminEventStat.vue:161` |
| `@tauri-apps/plugin-dialog` | `frontend/src/views/AdminEventStat.vue:160` |
| `@tauri-apps/plugin-shell` | `frontend/src/composables/useUpdateCheck.js:3` |

现在能构建，纯粹因为 Node/Vite 的模块解析会向上走到仓库根。其中 `@tauri-apps/plugin-http`
是 `services/api.js` 的核心依赖。这是一颗潜在的构建炸弹。

**B. 仓库根的 `dev` / `build` / `preview` 三个 npm script 是死的。** 仓库根没有
`index.html`、没有 `vite.config.*`、没有 `src/`，纯脚手架残留。root 与 frontend 的
`vite`（^6.0.3 vs ^7.0.6）和 `@vitejs/plugin-vue`（^5.2.1 vs ^6.0.1）版本也不一致。

## 范围

### 覆盖

1. PR 触发的 CI gate（Rust + 前端两个 job）
2. Rust 测试脚手架 + 首批测试
3. 前端测试脚手架（vitest）+ 首批测试
4. lint / 格式化配置接线，并把实测的 41 + 17 条警告清零
5. 数据库迁移前自动快照 + 失败回滚
6. 零风险清扫（死依赖、死代码、依赖泄漏、文档站 bug）

### 不覆盖（YAGNI）

- **E2E / Playwright 配置** → ④。现在对着一个 ④ 会重构掉的 UI 写 E2E 是一次性投入。
- **`tsconfig.json` 与 `vue-tsc` 门禁** → ③b。当前零 TS 代码，空门禁没有意义。
- **发版自动化**（tag 触发交叉编译 / 签名 / 建 Release）→ 不做。
  需要把 Android keystore 和 updater 私钥传进 GitHub Secrets，而这两把密钥在
  `CLAUDE.md` 里被标记为"丢了找不回来"。发版继续用 `scripts/` 在本机跑。
- **`booth-core` crate 拆分**（把业务逻辑剥离 tauri 依赖以加速测试）→ ②/③a。
  在还不知道领域边界时拆是瞎拆。
- **测试覆盖率门槛** → 先有测试再谈。
- **修改任何用户可见行为** —— ① 是纯工程改动。

## 设计

### 1. CI

新增 `.github/workflows/ci.yml`。触发：PR → `main` 或 `1.2-dev`；push → `1.2-dev`。
现有 `deploy-docs.yml` 不动。

**job `rust`**（ubuntu-latest）

```
apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
            libjavascriptcoregtk-4.1-dev librsvg2-dev libappindicator3-dev patchelf
Swatinem/rust-cache
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

`--all-features` 会带上 `vision` 从而拉进 `ort`，但 `ort` 配的是 `load-dynamic`
（见 `src-tauri/Cargo.toml`），**编译期不需要 onnxruntime 的 .so/.dll**，
所以 CI 不必下载那三个原生库（它们在 `.gitignore` 里，由 `scripts/setup-dev.sh` 取）。

**job `frontend`**（ubuntu-latest，node 22 —— `frontend/package.json` 的 `engines`
要求 `^20.19.0 || >=22.12.0`）

```
npm ci --prefix frontend        # 只装 frontend，故意不装 root
npm run lint --prefix frontend
npm run format:check --prefix frontend
npm run test:unit --prefix frontend
npm run build --prefix frontend
```

**"只装 frontend、不装 root"是刻意设计**：补齐上面问题 A 的 5 个包之后，
这一步就成为"`frontend/package.json` 必须是一份诚实清单"的永久守卫——
一旦有人再引入未声明的包，CI 立刻红。

### 2. Rust 测试脚手架

**新增 dev-dependency 只有 `tempfile`。** `tower`（0.4，features = full）、
`tokio`（features = full）、`serde_json` 都已经是正式依赖，
所以 `tower::ServiceExt::oneshot` 直接可用，不必新增。axum 是 0.7。

新增 `src-tauri/src/test_support.rs`（整个模块 `#[cfg(test)]`），提供：

- `async fn test_pool() -> SqlitePool` —— 连 `sqlite::memory:`，跑 `sqlx::migrate!()`。
  可行的前提是项目用的是**运行时** `migrate!()` 而非编译期 `query!` 宏，因此不需要 `DATABASE_URL`。
- `async fn test_app() -> (Router, TempDir)` —— 用临时目录作 `upload_dir`、
  上面的 pool 作 `db`，组出 `AppState` 与 `Router`。
  `VisionRuntime::new()` 不触碰 ONNX 运行时（`ort` 的 `load-dynamic` 只在真正建 session 时 `dlopen`），
  所以这条链路无需任何模型文件。

**首批测试**（目的是证明脚手架可用，不追覆盖率）：

| 目标 | 内容 |
|---|---|
| `src-tauri/src/utils/security.rs` | `hash_password` 往返、同一明文两次哈希不相同（bcrypt salt）、错误密码校验失败 |
| `src-tauri/src/utils/ip.rs` | LAN IP 筛选/排序的纯函数行为 |
| `src-tauri/src/utils/file.rs` | 文件名清洗（路径穿越、非法字符） |
| handler 级集成测试样例 | 建展会 → 列展会能拿到它。**一个就够**——目的是把 `oneshot` + 内存库 + 迁移这条链路走通一次，供后续子项目照抄 |

### 3. 前端测试脚手架

`frontend/package.json` 新增 devDependencies：`vitest`、`@vue/test-utils`、`jsdom`。
新增 scripts：`test:unit`、`lint`、`format`、`format:check`。
vitest 配置合并进 `frontend/vite.config.js`（共用 alias 与插件，不另起 `vitest.config.js`）。

**首批测试**：

- 把 `frontend/src/utils/dateFormatter.test.js` 从 `console.log("✓ 通过")` 脚本
  **改写成真 vitest 用例**。它本来就写好了期望值，缺的只是断言和退出码。
- 补 `frontend/src/utils/upload.js` 的测试。

### 4. lint / 格式化接线与清债

新增配置文件：

| 文件 | 内容 |
|---|---|
| `frontend/eslint.config.js` | ESLint 9 flat config：`eslint-plugin-vue` + `@vue/eslint-config-prettier`。**暂不接 `@vue/eslint-config-typescript`**（装了，但 TS 在 ③b） |
| `frontend/.prettierrc.json` | 与现有 `frontend/.editorconfig` 一致 |
| `rustfmt.toml` | 最小配置，用默认规则，只固定 edition |
| `.editorconfig`（仓库根） | 当前只有 `frontend/.editorconfig`，覆盖不到 `src-tauri/` |
| `rust-toolchain.toml` | **钉到 1.98.1**。本机是 1.98.1，CI 用 actions 默认 stable，clippy lint 集随版本漂移会让 `-D warnings` 在本机绿、CI 红 |

`src-tauri/Cargo.toml` 的 `[lints.rust]` 五项（`unused_variables` / `unused_imports` /
`unused_mut` / `unused_assignments` / `dead_code`）由 `"allow"` 改为默认（删除该段）。

然后清零实测的 **41 条 rustc + 17 条 clippy** 警告：

- 13 条 clippy 由 `cargo clippy --fix` 自动处理（`if` 折叠进 `match`、多余 `return`、
  `&PathBuf` → `&Path`、`== []` → `is_empty()`、`chunks_exact` → `as_chunks` 等）
- 24 条琐碎项（未用变量加 `_` 前缀或删除、删未用 import、去掉多余 `mut`）
- 17 条死代码项**逐个判断**：确属遗留的删除；属于将被 ③a/② 用到的 API 表面则加
  `#[allow(dead_code)]` 并写明原因，不得无差别加 allow 糊弄过去

### 5. 数据库迁移前快照

修改 `src-tauri/src/db/mod.rs` 的 `init_db`，在 `sqlx::migrate!().run(&pool)`（当前 41 行）之前插入：

1. 判断是否**既有待跑迁移、数据库文件又已存在**（即不是全新安装）。全新安装不快照。
2. 用 **`VACUUM INTO 'sale_system.db.bak-v<当前版本>-<时间戳>'`** 落快照。
   **必须用 `VACUUM INTO` 而不是 `fs::copy`**：库跑在 WAL 模式下
   （`src-tauri/src/db/mod.rs:31`），直接拷 `.db` 会漏掉 `-wal` 中尚未 checkpoint 的数据，
   拷出来的快照可能是残缺的。`VACUUM INTO` 产出的是事务一致的完整副本。
3. 迁移失败则从快照恢复，并返回一个 UI 能显示的错误，而不是让应用带着半迁移的库启动。
4. 保留最近 3 份快照，更旧的删除，避免无限堆积。

这条在 D1（历史数据知情清零）之后不再是 ② 的卡脖子前置条件，但仍必须有——
② 之后的每一次 schema 变更都要靠它兜底。

`reset_database`（`src-tauri/src/db/mod.rs:81-103`，当前直接 `drop_database` + `remove_file`）
同样接入快照：删库前先落一份。

### 6. 清扫

**前端依赖**

- 把问题 A 的 5 个包补进 `frontend/package.json` 的 `dependencies`
- 删除 `@headlessui/vue`（`frontend/src` 零引用）与 `vicons@0.0.1`
  （真正在用的是 `@vicons/ionicons5`，是不同的包）
- 仓库根 `package.json`：删除死的 `dev` / `build` / `preview` 三个 script；
  与 frontend 对齐 `vite` 和 `@vitejs/plugin-vue` 版本

**前端代码**

- 删除 `frontend/src/components/WebSocketTester.vue`（105 行，零引用）
- `frontend/src/components/order/OrderCard.vue:49` 与
  `frontend/src/views/AdminEventStat.vue:223` 的硬编码 `http://127.0.0.1:5140` 收口到
  `frontend/src/services/url.js`；`frontend/src/services/socketService.js:3` 的
  `import.meta.env.VITE_API_URL || 'http://127.0.0.1:5140'` fallback 一并收口
- 删除 `frontend/src/router/index.js:130-146` 成对的调试 `console.log` 注释
- **`frontend/src/services/socketService.js:8-9` 先查明再改**：注释写
  "【核心改动】强制只使用 'polling'，禁用 'websocket'"，下一行却是
  `transports: ['websocket', 'polling']`。不知道哪个是当初的本意，
  盲改可能弄坏局域网下的实时订单推送。**查不清就只留一条 issue 注释说明矛盾，不动代码。**

**文档站**（线上用户看得到）

- `docs/support/contact.md` 内容是日语（简体中文默认 locale 的页面被错放覆盖），恢复中文
- `docs/faq/community.md:17` 仍写"当前版本不支持一键自动更新，需要手动下载安装包"，
  但 v1.1.1 起 Windows 已支持（见 `docs/guide/auto-update.md`）。中/英/日三版同批订正
- `docs/guide/export.md` 全文只有"这个页面还在建设中，敬请期待！"，
  但 `docs/.vitepress/config.js` 侧边栏挂着入口。**导出能力本来就有**
  （Excel / CSV 报表见 `src-tauri/src/api/stats.rs`，`.boothpack` 见 `src-tauri/src/api/sync.rs`），
  缺的只是描述，补实即可
- `CHANGELOG-v1.1.md` 只含 v1.1.0，v1.1.1 的变更单独在 `docs/releases/v1.1.1.md`，
  两处记录分裂。在 `CHANGELOG-v1.1.md` 中指明 v1.1.1 的位置

## 测试策略

① 自身的验收标准是**可执行的**，不靠人工确认：

1. `cargo fmt --all --check` 通过
2. `cargo clippy --all-targets --all-features -- -D warnings` 通过（即 41 + 17 条清零）
3. `cargo test --all-features` 通过，且用例数 > 0
4. `npm ci --prefix frontend` 之后**在不存在 root `node_modules` 的情况下**
   `npm run build --prefix frontend` 成功（依赖诚实性的验证）
5. `npm run test:unit --prefix frontend` 通过，且用例数 > 0
6. `npm run lint --prefix frontend` 与 `format:check` 通过
7. 快照逻辑有测试覆盖：对一个已存在且有待跑迁移的库，`init_db` 之后
   快照文件存在且能被独立打开；迁移失败时原库未被破坏

第 4 条在本机验证时需要临时移走仓库根的 `node_modules`，CI 里天然满足。

## 风险

- **CI 首次运行慢**：`--all-features` 会编译 `ort` / `ndarray` / `image` / `reqwest`
  等重依赖。靠 `Swatinem/rust-cache` 缓解，首次冷跑预计十余分钟，之后命中缓存。
- **`-D warnings` 的版本敏感性**：已用 `rust-toolchain.toml` 钉到 1.98.1 消除。
  升级 toolchain 时需要同步处理新增 lint，这是有意的成本。
- **17 条死代码的判断可能误删**：删掉的若是 ③a/② 将要用到的 API 表面，
  后续子项目要重写。缓解：宁可加 `#[allow(dead_code)]` 附原因注释，也不删拿不准的。
- **`socketService.js` 的矛盾可能查不清**：那种情况下保持现状 + 注释说明，
  不为了"清扫干净"而冒险改动局域网实时推送。
