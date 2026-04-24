# 自动更新功能设计

**日期**: 2026-04-24
**作者**: Renko_6626（与 Claude 协作）
**状态**: Draft — 初版设计

## 背景

当前「检查更新」功能位于 `frontend/src/composables/useUpdateCheck.js` + `frontend/src/components/shared/UpdateModal.vue`，能力仅限：

1. 调用 GitHub REST `releases/latest` 接口
2. 用本地 semver 比对，判断是否有新版本
3. 若有新版本，弹窗展示 changelog
4. 「前往下载」按钮只是复制链接 + 打开 GitHub releases 网页，**需要用户手动下载、手动运行安装器**

目标：改造成真正的**下载 → 校验签名 → 就地安装 → 重启**闭环自动更新。

仓库根目录已有 `secret-key.txt`（rsign 加密私钥），说明此前已经生成过 Tauri updater 签名密钥，但对应公钥尚未配入 `tauri.conf.json`，也没安装 updater 插件。等于地基在，房子还没盖。

## 范围

### 覆盖

- **Windows** 桌面版：完整自动更新闭环（下载 → 验签 → 替换 → 重启）
- 现有手动"检查更新"入口复用，不新增入口
- 发布流程文档（开发者侧）+ 用户文档（使用者侧）

### 不覆盖（YAGNI）

- **Android** 自动更新：继续当前的「打开 GitHub releases 网页」fallback。Tauri updater 官方 Android 支持不稳定，且 Android 原生安装需要「安装未知来源」权限弹窗，UX 不如当前的下载安装器方案。留待后续独立项目。
- **启动时静默自检**：漫展现场（典型使用场景）不能随便弹框干扰接待，保留「用户主动点击检查」。
- **增量/差分更新**：安装包 < 50 MB，全量下载完全够用。
- **回滚机制**：若新版安装后出问题，用户可从 GitHub releases 手动下载旧版重装。增加回滚逻辑复杂度远大于收益。
- **多通道（beta/stable）**：单通道足够，当前没有测试用户群体基础。

## 架构

### 组件图

```
┌──────────────────────┐     ┌─────────────────────────────────────┐
│ UpdateModal.vue      │────▶│ useUpdateCheck.js (改造)            │
│  (UI, 已有)          │     │  - checkUpdate()    (走 updater 插件)│
│  + 下载进度条        │◀────│  - downloadAndInstall()  (新)       │
│  + 确认重启对话框    │     │  - restartApp()          (新)       │
└──────────────────────┘     └──────┬──────────────────────────────┘
                                    │
                                    ▼
            ┌───────────────────────────────────────────────┐
            │ @tauri-apps/plugin-updater (前端 binding)      │
            │ @tauri-apps/plugin-process  (前端 binding)     │
            └───────┬───────────────────────────────────────┘
                    │  (IPC)
                    ▼
            ┌───────────────────────────────────────────────┐
            │ Rust: tauri-plugin-updater + -process (后端)   │
            │  - 下载 NSIS installer                          │
            │  - rsign 公钥验签                               │
            │  - 调起 installer（静默 / 交互）                │
            │  - 通知应用关闭                                 │
            └───────┬───────────────────────────────────────┘
                    │  (HTTPS GET)
                    ▼
            ┌───────────────────────────────────────────────┐
            │ GitHub Releases                                │
            │  - 每版 release 附 3 个文件:                   │
            │    1. <product>_x64-setup.exe    (NSIS 安装器) │
            │    2. <product>_x64-setup.exe.sig  (rsign 签名)│
            │    3. latest.json               (version 清单) │
            └───────────────────────────────────────────────┘
```

### 关键接口

#### 1. 前端 composable 新 API

`useUpdateCheck.js` 扩展后对外暴露：

```js
{
  // 已有
  loading, error, hasUpdate,
  currentVersion, latestVersion, releaseNote, releaseDate,
  checkUpdate,
  goToDownload,  // 保留：Android 用，Windows 用作 "去网页看" 的备选

  // 新增
  isDownloading,        // Ref<boolean>
  downloadProgress,     // Ref<{ downloaded: number, total: number, percent: number }>
  downloadAndInstall,   // async () => void ; 走 plugin-updater
  restartApp,           // async () => void ; 走 plugin-process
}
```

底层改用 `@tauri-apps/plugin-updater`：

```js
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

const update = await check()  // 从 endpoint 拉 latest.json
if (update) {
  await update.downloadAndInstall((event) => {
    // 更新 downloadProgress
  })
  // installer 已就位 / 已安装（视 installer mode 配置）
  await relaunch()
}
```

#### 2. `tauri.conf.json` 新配置

```jsonc
{
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases/latest/download/latest.json"
      ],
      "pubkey": "<从 secret-key.txt 提取的对应公钥>",
      "windows": {
        "installMode": "passive"
        // passive: 显示进度窗口但不需要用户点击，安装后自动退出
        // quiet:  完全静默（适合后台更新，但本项目要用户确认，不用）
        // basicUi: 标准 installer UI（最保守，用户点 Next 点到底）
      }
    }
  }
}
```

选择 `passive`：用户已经点过"下载并安装"并确认重启，再走 basicUi 多此一举；quiet 又太黑盒（装完后用户看不到结果）。passive 折中。

#### 3. Rust 侧改动（最小化）

`src-tauri/src/lib.rs` 注册两个插件：

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_process::init())
    // ...既有 builder chain
```

`src-tauri/Cargo.toml` 新增依赖：

```toml
tauri-plugin-updater = "2"
tauri-plugin-process = "2"
```

capabilities 里放行 updater + process 能力（`capabilities/default.json` 或 `desktop.json`）。

#### 4. `latest.json` 格式

```json
{
  "version": "1.2.0",
  "notes": "摊盒 1.2.0 — AI 识别模型库扩充、网络稳定性提升",
  "pub_date": "2026-05-01T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<base64-encoded .sig 文件内容>",
      "url": "https://github.com/.../releases/download/v1.2.0/摊盒_1.2.0_x64-setup.exe"
    }
  }
}
```

### UI 流程

1. 用户点"控制台 → 检查更新" → 打开 `UpdateModal`（现有流程）
2. `checkUpdate()` 触发；loading 状态
3. 结果状态：
   - **无更新**: 显示"当前已是最新版本 v{current}"（现有）
   - **检查失败**: 显示错误 + 重试按钮（现有）
   - **有更新**: 显示 changelog + 两个按钮
     - **"暂不更新"**（现有，保留）
     - **"下载并安装"**（替换原"前往下载"）
       - Android: 该按钮文字改回"前往下载"，调用 `goToDownload()`，行为等同现状
4. 点击"下载并安装"后：
   - 按钮变为进度条 + "下载中... 38%"
   - `downloadAndInstall` 完成后，按钮变为"立即重启以完成安装"
5. 点击重启按钮：
   - 二次确认对话框："即将关闭摊盒以完成安装，未保存的数据将丢失。确定吗?"
   - 用户确认 → `relaunch()`
   - 取消 → 回到步骤 4 末态，用户可随时再点

### 错误处理

| 场景 | 现象 | 处理 |
|---|---|---|
| GitHub API 超时 | `check()` 抛错 | 显示"检查失败: <原因>" + 重试按钮 |
| 下载中断（网络）| `downloadAndInstall` 抛错 | 重置 UI 到"发现更新"状态，显示错误，保留 changelog 不重新拉 |
| 签名校验失败 | updater 插件抛 `InvalidSignature` | 显示"更新包校验失败，请从官网重新下载"，提供"前往下载"按钮兜底 |
| 磁盘空间不足 | installer 写入失败 | 错误信息透传 + 文档说明典型原因 |
| 用户拒绝 UAC | Windows 会话级失败 | 显示"安装已取消"，保留 UI，用户可重试 |

### 测试策略

Tauri updater 插件本身由 Tauri 团队测试。本项目需要测试的：

1. **手动 smoke test**（每次 release 流程跑一次）：
   - 发布 v1.1.1 → 在装有 v1.1.0 的机器上点检查更新
   - 确认：能看到新版本 / 能下载 / 能重启 / 装完后版本真的变了
2. **签名错误路径**：
   - 故意上传错误的 .sig 文件
   - 确认客户端显示校验失败而不是装了坏包
3. **无新版路径**：
   - 当前版本 = latest 版本 → 显示"已是最新"

不引入单元测试基础设施——Tauri updater 插件边界清晰，且本项目的实际代码量（composable 改造 + 几个按钮）太少，单测投入产出比差。

## 发布流程（开发者侧）

1. 改 `src-tauri/tauri.conf.json` 里 `version`
2. 改 `CHANGELOG-v1.x.md` 写更新日志
3. 设置环境变量:
   ```
   set TAURI_SIGNING_PRIVATE_KEY=<secret-key.txt 内容>
   set TAURI_SIGNING_PRIVATE_KEY_PASSWORD=<用户生成时设置的密码>
   ```
4. `npm run build` （前端 dist）
5. `cargo tauri build` → 产出:
   - `src-tauri/target/release/bundle/nsis/摊盒_x.y.z_x64-setup.exe`
   - `src-tauri/target/release/bundle/nsis/摊盒_x.y.z_x64-setup.exe.sig`
6. 手工或脚本生成 `latest.json`（填 version、pub_date、url、signature 字段）
7. 在 GitHub 打 tag `vx.y.z`，建 release，上传上述 3 个文件
8. 把 `latest.json` 作为 asset 附上（文件名必须就是 `latest.json`）

后续可以写一个 PowerShell 脚本或用 `tauri-action` GitHub workflow 自动化第 5–8 步。本次 spec 先不做，手动跑通闭环即可。

## 文档交付清单

1. `docs/superpowers/specs/2026-04-24-auto-update-design.md`（本文）
2. `docs/guide/auto-update.md` — 用户视角：什么是自动更新、网络要求、常见问题
3. `docs/BUILD.md`（已存在，补内容） — 开发者视角：如何签名、发布、确认 updater 工作
4. README 简短提及（可选）

## 实施顺序（给 writing-plans 使用）

1. **后端骨架**：加 Cargo 依赖，注册插件，配 capabilities，配 `tauri.conf.json` 的 updater 段（先用占位公钥，后补）
2. **公钥提取**：从 `secret-key.txt` 提取 public key 补入 tauri.conf.json
3. **前端 composable 改造**：`useUpdateCheck.js` 加 `downloadAndInstall` 和 `restartApp`
4. **UI 更新**：`UpdateModal.vue` 按设计改进度条 + 重启流程，Android 分支保留旧行为
5. **本地 smoke test**：无远程 latest.json 时的错误路径；构造本地 `latest.json` 验证正常路径
6. **文档**：用户指南 + BUILD.md 补章节
7. **首个有 updater 的 release**：v1.2.0，作为里程碑发布，附完整 latest.json 和 .sig

## 风险与开放问题

- **用户 rsign 私钥密码**：已加密的 `secret-key.txt` 需要密码才能使用。若用户忘记密码，无法签名。**建议用户在密码管理器存一份**，并在 BUILD.md 里明确这一点。
- **无法验证"这个 secret-key.txt 对应什么公钥"**：需要用户手动跑一次 `tauri signer sign` 并从输出里提公钥，或从过去构建过的旧 sig 文件反推。首次实施时要处理这个。
- **v1.1.0 用户无法收到"使用 updater 的新版本"通知**：因为 v1.1.0 的检查更新逻辑不走 updater endpoint。这版用户第一次收到更新要么通过现有的"打开 GitHub 页面"流程手装一次 v1.2.0，从 v1.2.0 起后续版本才能真正自动更新。这是过渡期无法避免的。需要在 v1.2.0 的 changelog 里明说："此版本起支持一键更新，后续版本无需手动下载"。
