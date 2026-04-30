# LAN HTTPS 自签证书 设计

**日期**: 2026-04-29
**作者**: Renko_6626（与 Claude 协作）
**状态**: Draft — 排期 v1.1.2
**前置阅读**: 无

## 背景

当前后端 axum 监听 `0.0.0.0:5140` 单一 HTTP 端口，所有 LAN 访问也走 HTTP。这导致：

- `navigator.mediaDevices.getUserMedia()` 在 LAN 浏览器（`http://192.168.x.x:5140`）下**完全不可用** —— 浏览器规定它只在 secure context（HTTPS / localhost / file://）启用
- 实际症状：摊主把摊位平板放在摊位前面后，平板浏览器打开顾客点单页（`<VisionSearch camera-mode>`），调 getUserMedia 时报 `Cannot read properties of undefined (reading 'getUserMedia')`
- v1.1 引入的"顾客拍照识别商品加购物车"功能在 LAN 浏览器场景**完全失效**

唯一有效的解决路径是让 LAN 也走 HTTPS。Public CA 不可能给私有 IP 签证书，所以必须自签。

## 关键场景前提（决定方案选择）

**所有扫码均由摊主本人完成**：摊主用自己的设备扫"顾客入口"二维码挂在摊位平板上、扫"摊主入口"挂在自己手机上、扫"管理员入口"挂在管理设备上。**真正的顾客根本不接触 URL，只接触摊主放好的平板**。

这一前提关键地排除了"微信内置浏览器拒绝自签证书"这个原本会致命的约束。摊主能用 Chrome / Edge / 系统浏览器接受证书警告，微信扫码仅作为获得 URL 的手段，初次进入后摊主一次性完成接受。

## 范围

### 覆盖

- 后端 axum 同时监听 HTTP（仅回环）和 HTTPS（LAN）两个端口
- 启动时自动用 `rcgen` 生成自签证书并缓存到磁盘，10 年有效期，到期自动重生成
- 证书 SAN 包含所有本机 LAN IP（IPv4） + `127.0.0.1` + `localhost`
- `server_info` API 返回的 QR URL 全部改为 HTTPS
- `VisionSearch.vue` 在 secure context 不存在时给清晰引导（兜底，万一某台设备首次未接受证书）
- 摊主侧文档：每台设备一次性接受证书的截图引导（Chrome / Safari / Edge / 小米/华为系统浏览器）

### 不覆盖（YAGNI）

- **HTTP/2 / HTTP/3**：HTTPS 默认 HTTP/1.1 即可，性能足够漫展场景
- **证书自动轮换 / ACME**：Let's Encrypt 不签私有 IP，10 年自签到期再说
- **CA 模式**（生成本地 CA + 信任 CA + 用 CA 签每台主机证书）：每台设备需安装 CA 根证书，摊主很难配
- **mDNS `xxx.local` 域名**：增加复杂度，对摊主无明显收益
- **强制 HTTPS（关 HTTP 5140）**：不关，Tauri webview 仍走 HTTP 本地最快
- **自动检测证书是否被某些定制 ROM 拒绝**：超出可控范围

## 架构

```
┌────────────────────────────────────────────────────────────────────┐
│ 摊盒主机进程                                                        │
│                                                                    │
│  ┌──────────────────┐                                              │
│  │ Tauri webview    │── HTTP ──► 127.0.0.1:5140  axum HTTP listener│
│  │ (admin 客户端)   │             (仅回环，无 LAN 暴露)             │
│  └──────────────────┘                                              │
│                                                                    │
│  axum HTTPS listener: 0.0.0.0:5141                                 │
│         ▲                                                          │
│         │ HTTPS（自签证书）                                         │
└─────────┼──────────────────────────────────────────────────────────┘
          │
   ┌──────┴───────┬──────────────┬────────────────┐
   │              │              │                │
摊位平板       摊主手机     管理员设备       临时来连的笔记本
(顾客点单)    (查订单)    (后台管理)         (大屏管理)

每台设备首次访问 → 浏览器红屏警告 → 摊主点"高级" → "继续访问" → ✅
```

两个 listener 共享同一个 `Router`、同一个 `AppState`、同一个 SQLite Pool；只是绑定不同协议+地址的端口。

## 组件分解

### 1. 证书管理模块（新文件 `src-tauri/src/utils/cert.rs`）

职责：生成、缓存、加载自签证书。

接口：
```rust
/// 加载缓存证书。如不存在 / 已过期 / SAN 不匹配当前 LAN IP，则重新生成并缓存。
/// 返回 (cert_pem, key_pem) 字节流，供 RustlsConfig 直接消费。
pub async fn load_or_generate_cert(
    app_data_dir: &Path,
    additional_sans: &[IpAddr],
) -> Result<(Vec<u8>, Vec<u8>), CertError>;
```

实现要点：
- 缓存路径：`<app_data_dir>/cert.pem` + `<app_data_dir>/key.pem`
- SAN 列表构建：传入的 LAN IP（来自 `local-ip-address` crate）+ `127.0.0.1` + `::1` + `localhost`
- 重生成触发条件（任一）：文件缺失 / 解析失败 / 已过期 / 文件存在但 SAN 与当前 LAN IP 集合不交（换 WiFi 时常见）
- CN：`booth-kernel-self-signed`
- Subject Alternative Names: 上面的 IP + DNS 列表
- Validity: 现在起 10 年
- 算法：ECDSA P-256（小、快、广泛兼容）

### 2. axum 双 listener（修改 `src-tauri/src/server.rs`）

当前 `start_server` 只起一个 HTTP listener。改成同时起两个：

```rust
pub async fn start_server(state: AppState, app_data_dir: PathBuf) -> Result<(), Error> {
    let app = build_router(state);

    let http_listener = TcpListener::bind("127.0.0.1:5140").await?;
    let https_addr: SocketAddr = "0.0.0.0:5141".parse()?;

    let lan_ips = collect_local_ipv4_addrs();
    let (cert_pem, key_pem) = cert::load_or_generate_cert(&app_data_dir, &lan_ips).await?;
    let tls_cfg = RustlsConfig::from_pem(cert_pem, key_pem).await?;

    let app_clone = app.clone();
    let http_task = tokio::spawn(async move {
        axum::serve(http_listener, app_clone.into_make_service()).await
    });

    let https_task = tokio::spawn(async move {
        axum_server::bind_rustls(https_addr, tls_cfg)
            .serve(app.into_make_service())
            .await
    });

    tokio::try_join!(http_task, https_task)?;
    Ok(())
}
```

注意：HTTP 端口仍叫 5140 不变（向后兼容 Tauri webview 现有 baseURL）；HTTPS 占用 5141。

### 3. server_info API 调整（修改 `src-tauri/src/api/info.rs`）

现在返回：
```json
{
  "order_url":  "http://192.168.x.x:5140/order/...",
  "vendor_url": "http://192.168.x.x:5140/vendor/...",
  "admin_url":  "http://192.168.x.x:5140/admin"
}
```

改为：
```json
{
  "order_url":  "https://192.168.x.x:5141/order/...",
  "vendor_url": "https://192.168.x.x:5141/vendor/...",
  "admin_url":  "https://192.168.x.x:5141/admin"
}
```

scheme + 端口都换。前端 `AdminControlPanel.vue` 渲染二维码的代码无需任何改动——它只是 join URL 字符串。

### 4. 前端 axios baseURL 不变

Tauri 客户端继续 `http://127.0.0.1:5140/api`（HTTP 回环）。LAN 浏览器从 `https://...:5141/...` 加载页面，axios 默认走相对路径 `/api/...` —— 同源 HTTPS 请求，零 CORS 麻烦。

### 5. VisionSearch.vue 防御兜底（修改 `frontend/src/components/shared/VisionSearch.vue`）

哪怕走了 HTTPS，仍然可能有边角 case：
- 摊主首次访问 https URL 但还没接受证书警告就尝试拍照
- 极少数定制 ROM / 浏览器对自签证书的 secure context 判定不一致

兜底：调 `getUserMedia` 前先检查 `window.isSecureContext` 和 `navigator.mediaDevices`，不可用时显示清晰引导卡片：

> 当前页面不是安全连接，浏览器禁止访问摄像头。
> 检查 URL 是否以 https 开头、是否首次接受过浏览器的证书警告。
> 如仍无法解决，请在主机的摊盒桌面应用内直接操作。

### 6. 文档（新增 `docs/guide/lan-https.md`）

摊主第一次部署的引导：
- 为什么有红屏警告（一句解释）
- Chrome on Windows / Android 截图：高级 → 继续访问
- Safari on iPad 截图：显示详细信息 → 浏览此网站
- Edge 截图：高级 → 继续转到此页面
- 小米 / 华为系统浏览器：基本同 Chrome
- 接受过一次后，**该设备永不再问**

## 数据流

```
启动序列：
1. 主进程启动 axum
2. 调 cert::load_or_generate_cert():
   a. 检查 cert.pem / key.pem 是否存在
   b. 若不存在 → rcgen 生成 → 写盘
   c. 若存在 → 检查过期、SAN 是否覆盖当前 LAN IP
   d. 任一条件不通过 → 重新生成
3. 用证书构造 RustlsConfig
4. 同时 spawn HTTP（127.0.0.1:5140）和 HTTPS（0.0.0.0:5141）两个 task
5. 任一 task 退出 → 整个进程退出（保持原有 try_join 行为）

请求路径：
- Tauri webview                    → http://127.0.0.1:5140/api/* → axum
- LAN 顾客平板（摊位）             → https://192.168.x.x:5141/order/...
- LAN 摊主手机                      → https://192.168.x.x:5141/vendor/...
- LAN 管理员设备                    → https://192.168.x.x:5141/admin
```

## 错误处理

| 场景 | 现象 | 处理 |
|---|---|---|
| 证书生成失败（磁盘只读 / 权限不足）| `rcgen` 或 fs 写入抛错 | 致命错误：log 后退出，UI 显示"无法启动 HTTPS 服务"。这是部署问题，不是运行时问题 |
| 证书已过期 | 启动检查发现 | 自动重生成，无需用户介入 |
| 5141 端口被占 | `bind_rustls` 失败 | 致命错误：log 提示用户检查端口冲突。考虑未来加配置项让用户自定义端口 |
| 5140 端口被占 | HTTP listener 失败 | 同上 |
| 设备拒绝自签证书（如某些 IoT 浏览器）| 用户访问 https URL 浏览器不允许继续 | UX 文档引导用户换浏览器；技术上无解 |

## 测试策略

**不引入 TLS 单元测试**——`rcgen` 和 `axum-server` 自身有充分上游测试，本项目的代码量太少不值得做。

跨设备 smoke test（每次发布前手测一遍）：

1. **Tauri 客户端 HTTP 路径仍正常**：在主机摊盒里点几个常规操作，看 Console 里 `[Req #N] -> 200 [native]` 都正常返回
2. **Windows Chrome → HTTPS**：扫管理员二维码，弹证书警告，接受后能进，拍照功能可用
3. **Android Chrome（朋友平板）→ HTTPS**：同上
4. **iPad Safari → HTTPS**：同上（Safari 路径与 Chrome 略有不同）
5. **小米 / 华为 系统浏览器 → HTTPS**：同上
6. **证书重生成**：手动改本机 IP（连不同 WiFi）→ 重启摊盒 → 检查 cert.pem 被重生成（mtime 更新）+ 新 IP 在 SAN 内
7. **证书过期模拟**：本地把 cert.pem 改成已过期的 → 重启 → 检查重生成

## 已知限制

- **首次访问每台设备**都要点过红屏警告（Chrome 系约 3 次点击，Safari 略多）。无法消除
- **证书"本机指纹"会变**：换网络环境（漫展现场和家里是不同 WiFi）→ LAN IP 变 → 证书会重生成 → 已经接受过老证书的设备会再看一次警告。可接受（漫展前一晚在现场连上 WiFi 重启一次即可）
- **HTTPS 比 HTTP 略慢**（TLS 握手）：每个请求约 +5-30ms，对 boothpack 大文件上传影响微乎其微（一次握手共用，不每包）
- **Tauri 安卓客户端无影响**：它通过 plugin-http 走外部网络，不通过本地后端 server

## 与已有特性的交互

- **boothpack 导入** (`POST /sync/import-products-raw`)：从 LAN 浏览器导时切到 HTTPS 端口，body 仍是原始字节，性能不变
- **AI 视觉识别** (`POST /vision/search`)：HTTPS 下 multipart 上传也正常工作
- **更新检查** (`useUpdateCheck`)：仍然 hit GitHub API（外部公网），与本地 HTTPS 无关
- **订单轮询** (`orderStore`)：3 秒一次的 GET，HTTPS 下小幅 TLS overhead，可忽略

## 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| 摊主第一次见红屏不敢点 | 高 | 用不起来 | 文档 + UI 内嵌 banner 引导 + 漫展前一天试机 |
| 某型号手机浏览器拒绝自签证书且无"继续"按钮 | 中 | 那台设备不能用 | 文档说明换浏览器；技术上无能为力 |
| 5141 端口被防火墙拦 | 低 | 全员连不上 | Windows 防火墙首次启动会弹询问，文档提示放行 |
| 证书生成竟然失败（罕见 OS 边界） | 低 | 启动崩 | 友好错误日志 + 重启可重试 |

## 实施顺序（给 writing-plans 用）

1. **依赖加入** + **证书模块** —— 加 `axum-server` `rcgen`，新增 `utils/cert.rs`
2. **server.rs 改造双 listener**
3. **server_info API URL 改 HTTPS**
4. **VisionSearch 兜底 UI**
5. **新文档 `docs/guide/lan-https.md`**
6. **跨设备 smoke test**

## 开放问题（实施时确认）

- A. `rcgen` 在 Windows 上 ARM64 构建是否有特殊依赖（OpenSSL 等）。预期纯 Rust 无 C 依赖，但实施第一步先验证
- B. `axum-server` 的 `bind_rustls` 与现有 `tower-http` middleware（cors / trace）是否完全兼容。预期 yes，需实施时跑一次完整请求链
- C. 是否需要把 5141 这个端口号写成可配置（环境变量 `BOOTH_HTTPS_PORT`）。**默认不可配置（YAGNI），实施时若发现冲突频发再加**
