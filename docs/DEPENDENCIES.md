# 依赖维护笔记

记录当前有意保留在旧版本的依赖，以及为什么。**不要**凭「有新版」就升——
这个项目目前没有任何自动化测试（Rust 侧 0 个 `#[test]`，前端无 e2e），
每一次跨大版本升级的验证成本都要自己掏。

## 怎么查

```bash
cd src-tauri
tauri-env linux cargo tree -p BoothKernel -e normal,build --depth 1 --target all
```

注意别直接从 `Cargo.lock` 里 grep 版本号：同一个 crate 常有多个版本共存
（不同依赖各自拉的），grep 会拿到错的那个。

安全公告用 RustSec 的库比对：

```bash
git clone --depth 1 https://github.com/rustsec/advisory-db
# 拿 advisory-db/crates/<name>/RUSTSEC-*.md 里的 patched 区间去比 Cargo.lock 的版本
```

## 当前状态（2026-09-22）

RustSec 扫描 752 个包：**漏洞 0 条**，提示类 21 条。

21 条提示里绝大多数是 `gtk` / `gdk` / `atk` / `glib` 这套 GTK3 绑定被标记
unmaintained——它们只在 **Linux** 构建里出现（webkit2gtk 那条链），而这个项目
发布的是 Windows（WebView2）和 Android（系统 WebView），产物里根本没有它们。
要动只能等上游 tauri 换掉 GTK3，不是这边能解决的。

其余几条：`dirs` unmaintained（直接依赖，见下）、`ring` unmaintained（rustls 的
底座，rustls 官方仍在用）、`paste` / `proc-macro-error` / `unic-*` 是编译期宏依赖。

## 有意锁住的版本

| crate | 锁在 | 原因 |
| --- | --- | --- |
| `tauri-plugin-http` | `2.6` | npm 侧 `@tauri-apps/plugin-http` 只发到 2.6.1，crate 已到 2.7.0。tauri CLI 2.11+ 会校验 JS 包与 Rust crate 的 major/minor 一致，放开就构建失败。**等 npm 发出 2.7 再解开**，解开时顺带处理 `reqwest` 的重复（见下）。 |
| `ort` | `=2.0.0-rc.11` | 必须与打包的 ONNX Runtime 1.23.x 动态库严格匹配，版本不符是**运行时 crash** 而不是编译错误。连带把 `ndarray` 钉在 0.16。 |
| `tauri-plugin-app` | `2.0.0-alpha.2` | 上游至今没有稳定版。 |

## 暂缓的跨大版本升级

下面 17 个都**没有安全驱动**，纯维护。按风险从低到高排，将来要做的话建议
一次只动一两个，每次都跑一遍 `scripts/build-windows.sh` + `build-android.sh`，
并在 VNC 里真跑一次 `tauri-env vnc npx tauri dev` 打一遍 API。

### 低风险（改动面小，编译期能拦住错误）

| crate | 当前 → 最新 | 要注意的 |
| --- | --- | --- |
| `thiserror` | 1.0.69 → 2.0.20 | 派生宏语法基本兼容，主要是 `#[source]`/`#[from]` 的一些边角规则收紧 |
| `sha2` | 0.10.9 → 0.11.0 | 跟随 `digest` 0.11，`Digest` trait 的 import 路径有变 |
| `dirs` | 6.0.0 → 7.0.0 | 已被标记 unmaintained；顺手可考虑换 `dirs-next` 或 `etcetera` |
| `local-ip-address` | 0.5.7 → 0.6.13 | 错误类型有调整；这个 crate 用在 LAN IP 枚举，改完要在真机验二维码地址 |
| `tower` / `tower-http` | 0.4/0.5 → 0.5/0.7 | 要和 axum 的版本配套动，见下 |

### 中风险（运行期才暴露）

| crate | 当前 → 最新 | 要注意的 |
| --- | --- | --- |
| `axum` + `axum-server` | 0.7 → 0.8 | 路径参数语法从 `/:id` 改成 `/{id}`，**53 个 handler 的路由全要改**；改漏了是 404 而不是编译错误。同时要配套升 `tower-http`。 |
| `jsonwebtoken` | 9.3.1 → 11.1.0 | 校验 API 重构；改错了的后果是鉴权失效，必须逐条过登录/权限路径 |
| `bcrypt` | 0.15.1 → 0.19.3 | **注意**：已有用户的密码 hash 必须仍能校验通过，升级后务必拿旧库的 hash 实测 |
| `rcgen` | 0.13.2 → 0.14.10 | 自签证书生成，影响 LAN HTTPS；改完要在手机上重新走一遍「首次接受证书」 |
| `rust_xlsxwriter` | 0.84.1 → 0.99.1 | 跨度极大，导出报表的 API 大概率要重写 |
| `zip` | 0.6.6 → 8.6.0 | 跨了 8 个大版本；`.boothpack` 导入导出全依赖它 |
| `rand` | 0.8.8 → 0.10.3 | `thread_rng()` → `rng()` 等一系列重命名 |
| `reqwest` | 0.12.28 → 0.13.5 | 现在二进制里**已经有两份**：tauri 2.11.6 用 0.13.5，而我们的直接依赖和被钉住的 `tauri-plugin-http` 2.6.1 用 0.12.28。等 npm 发出 plugin-http 2.7、把上面那个 pin 解开之后，连同本项目一起升到 0.13 就能收敛成一份。**顺序别反了**，单独升会让 plugin-http 那份继续留着。 |
| `windows` | 0.58.0 → 0.62.2 | 只影响 `#[cfg(windows)]` 的 DXGI 显卡枚举。交叉编译能验编译，**运行时行为必须在真 Windows 上看**。 |
| `ndarray` | 0.16.1 → 0.17.2 | 被 `ort` 锁死，只能跟着 `ort` 走 |

## 升级时的验证清单

没有测试兜底，所以这几步不能省：

1. `tauri-env linux cargo check` —— 编译期能拦的先拦掉
2. `./scripts/build-windows.sh` + `./scripts/build-android.sh` —— 交叉编译和打包
3. `tauri-env vnc npx tauri dev`，然后打 API：登录 → 建商品/场次 → 下单 →
   `GET /events/:id/orders`（走 `IN (?,?,?)` 动态 SQL）→ `GET /events/:id/sales_summary`
   （带 `product_code` / `start_date` / `end_date` 三个筛选，走另一条动态 SQL）→
   `PUT /admin/reset-database`（走 `DELETE FROM {table}` 循环）
4. 动了 `sqlx` / `bcrypt` / `jsonwebtoken` 的话，额外拿**升级前就存在的**数据库文件
   跑一遍，确认老数据和老密码 hash 仍然可用

## sqlx 的一个约定

sqlx 0.9 起，`query()` / `query_as()` 只接受 `&'static str`。项目里有 6 处需要按
运行时条件拼 SQL（`IN (?,?,?)` 的占位符个数、可选筛选条件、写死的表名列表），
它们用 `AssertSqlSafe(...)` 包了一层。

**每一处旁边都写了审计结论。新增这种写法时必须同样先审计再包**——
`AssertSqlSafe` 的字面意思就是「我看过了，这里没有把用户输入拼进 SQL」。
用户数据一律走 `.bind()`。

## naive-ui 钉在 2.44.1（2026-09-25）

`frontend/package.json` 里写的是精确版本 `"naive-ui": "2.44.1"`，**不带 `^`**。

2.45.0 起 naive-ui 改用 vue-jsx-vapor 编译，产物里的 block 元数据不稳定。以 `n-space` 为例，
每个子项的包裹 div 都是同一个 `key: 1`，外层 Fragment 却被标成 STABLE_FRAGMENT（「子节点结构不会变」）。
子节点里只要有 `v-if` 在变，就会留下旧 DOM：Windows 真机上，控制台的「历史数据（v1）」卡片
被复制了四份，全挤在页面最上方（浏览器和 jsdom 里都复现不了，要 WebView2 下的请求时序才触发）。
上游同类问题：tusen-ai/naive-ui#8218（批量卸载/重建崩溃，2.44.1 正常）、#8207（异步填充后内容冻结）。

**升级前先确认**：上游修掉这类回归（看 CHANGELOG 里有没有提到 vapor / block 相关的修复），
而且要在真 Windows 上打开控制台，从别的页面来回切几次，确认没有重复卡片。
