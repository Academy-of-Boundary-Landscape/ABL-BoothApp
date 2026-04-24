# Android 自动更新设计

**日期**: 2026-04-24
**作者**: Renko_6626（与 Claude 协作）
**状态**: Draft — 初版设计
**依赖**: 基于已落地的 Windows 自动更新（见 `2026-04-24-auto-update-design.md`），复用其前端模态框和 `useUpdateCheck` composable

## 背景

Windows 侧已经用 `tauri-plugin-updater` 完成真正的自动更新闭环。Android 侧目前的行为仍是"打开 GitHub 发布页让用户手动下载 APK" —— 等同于没做。

Android 上做"自动更新"的现实约束：

- Sideload 应用（非 Play Store 分发）无法像桌面那样原地替换二进制；Android 安全模型强制走系统 PackageInstaller
- Android 8+ 每个 app 要单独开"允许安装未知应用"权限（首次触发跳系统设置页，无法跳过）
- Android 11+ 需要在 AndroidManifest 的 `<queries>` 里声明才能 resolve PackageInstaller intent
- 定制 ROM（小米 MIUI、华为 EMUI 等）可能额外拦截 `ACTION_VIEW` + APK mime
- `tauri-plugin-updater` 本身对 Android 支持有限，不能直接用

目标：在这些约束下做出"**对大多数设备几乎一键**、对少数老/定制设备**有降级兜底**"的更新体验。

## 范围

### 覆盖

- Android（minSdk 24 / Android 7 起，与当前 build.gradle.kts 设置对齐），三层降级的 APK 一键更新
- 首次权限引导（跳系统设置页 + 返回重试）
- 中文错误文案 + 对"定制 ROM 可能挂掉"的显式兜底
- 复用 Windows 侧已有的 `UpdateModal.vue` / `useUpdateCheck.js`，仅扩展不重写

### 不覆盖（YAGNI）

- **Play Store 上架**：业务决策，且漫展主力用户在国内 Play Store 不可用
- **多应用商店分发**（应用宝/小米/华为）：需要运营流程，非本次技术 scope
- **后台静默更新**：sideload app 拿不到 `INSTALL_PACKAGES` 系统权限
- **差分 / 增量 APK**：Android 无 sideload 标准差分机制
- **F-Droid 适配**：需要开源 + 可复现构建，scope 外
- **iOS**：没有相关 issue，签名体系完全不同，单开专案

## 架构

三层降级，上层失败自动尝试下层：

```
前端点「下载并安装」
     │
     ▼
 下载 APK 到 app cache（plugin-http + 进度事件）
     │
     ▼
 [Tier 1] JNI → Kotlin → Intent 安装（> 90% 设备的主路径）
     │
     ├─ 抛异常 / NoActivity 降级 ─► [Tier 2] plugin-shell open(path)
     │                                │
     │                                ├─ 成功 ─► 系统安装器弹框
     │                                └─ 失败 ─► [Tier 3] 显示路径 + 前往 GitHub
     │
     ├─ PermissionDenied            ─► 引导用户去系统设置，回来后重试 Tier 1（不降级）
     │
     └─ 成功                         ─► 系统安装器弹框 → 装完用户自己从桌面重开
```

**为什么权限拒绝不自动降级**：权限问题降到 Tier 2/3 也会被挡，应该让用户主动去设置页处理。静默降级会让用户困惑"怎么每次都装不上"。

## 组件

### 1. Kotlin 侧：`MainActivity.kt` 扩展

现有 `MainActivity.kt` 只有 7 行（`TauriActivity` + `enableEdgeToEdge`）。扩展为：

- `fun installApk(apkPath: String): String` —— 主入口。返回状态码字符串：
  - `"launched"`：intent 已发出，系统安装器接管
  - `"no_activity"`：`PackageManager.resolveActivity` 返回 null（需 `<queries>` 或 ROM 拦截）
  - `"permission_denied"`：Android 8+ 且 `canRequestPackageInstalls()` false
  - `"error:<message>"`：其他异常
- `fun canRequestInstall(): Boolean` —— Android 8+ 返回 `PackageManager.canRequestPackageInstalls()`；Android 7 直接返 true
- `fun openInstallSettings()` —— 跳 `Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES`（Android 8+）或 `Settings.ACTION_SECURITY_SETTINGS`（Android 7）

返回类型用字符串而不是 enum 是因为 JNI 传 enum 很烦，字符串前缀匹配够用。

**关键实现点**：用 `FileProvider.getUriForFile(this, "${applicationContext.packageName}.fileprovider", File(apkPath))` 生成 content URI，intent 设 `FLAG_GRANT_READ_URI_PERMISSION`，type 设 `application/vnd.android.package-archive`。

### 2. Rust JNI 桥接

新文件 `src-tauri/src/android_install.rs`，仅在 `cfg(target_os = "android")` 下编译。暴露三个 `#[tauri::command]`：

- `install_apk(path: String) -> Result<String, String>`
- `can_request_install() -> Result<bool, String>`
- `open_install_settings() -> Result<(), String>`

实现使用 `jni` crate（Tauri 2 Android 已经依赖它）：拿到 `JavaVM` → `attach_current_thread` → 拿 Activity 实例 → 调 Kotlin 方法 → 解析返回值。

`lib.rs` 注册这些命令时 cfg-gate：

```rust
#[cfg(target_os = "android")]
{
    builder = builder.invoke_handler(tauri::generate_handler![
        get_backend_url,
        android_install::install_apk,
        android_install::can_request_install,
        android_install::open_install_settings,
    ]);
}
#[cfg(not(target_os = "android"))]
{
    builder = builder.invoke_handler(tauri::generate_handler![get_backend_url]);
}
```

### 3. AndroidManifest 修改

`src-tauri/gen/android/app/src/main/AndroidManifest.xml`：

- 加权限 `<uses-permission android:name="android.permission.REQUEST_INSTALL_PACKAGES" />`
- 加 `<queries>` 块允许 resolve APK install intent：

```xml
<queries>
  <intent>
    <action android:name="android.intent.action.VIEW" />
    <data android:mimeType="application/vnd.android.package-archive" />
  </intent>
</queries>
```

- `FileProvider` 和 `file_paths.xml` 已经存在（为相机功能配置过），复用即可。需要确认 `file_paths.xml` 里的 `cache-path` 能覆盖 `getCacheDir()` 返回的路径 —— 当前配置的 `<cache-path name="my_cache_images" path="." />` 意思是整个 cache 目录都允许 share，已经足够。

### 4. 前端 composable 扩展

`frontend/src/composables/useUpdateCheck.js` 在现有 `downloadAndInstall` 旁边加 `downloadAndInstallAndroid`。`canAutoUpdate()` 改为 Android 时也返回 `true`：

```js
const canAutoUpdate = async () => {
  if (!isTauri()) return false;
  const p = await detectPlatform();
  platform.value = p;
  return p === 'windows' || p === 'macos' || p === 'linux' || p === 'android';
};
```

`downloadAndInstall` 改为根据 platform 分派：

```js
const downloadAndInstall = async () => {
  if (platform.value === 'android') {
    return downloadAndInstallAndroid();
  }
  return downloadAndInstallDesktop();  // 现有逻辑改名
};
```

新函数 `downloadAndInstallAndroid`：
1. 拉 GitHub release API 拿最新 APK 的 browser_download_url（沿用现有 `checkViaGithubApi` 拿到的信息，额外筛选 asset 名字匹配 `.apk`）
2. `plugin-http` 流式 `fetch()` 下载到 `app.path().appCacheDir() + "/update.apk"`，进度通过 `ReadableStream` 读取 chunk 更新 `downloadProgress`
3. 调 `can_request_install` 检查权限
4. 若 false：设置一个新的 ref `needsPermissionGrant = true`，暂停（不自动降级），UI 弹"去设置"按钮；用户回来后可重新点下载
5. 若 true：调 `install_apk(path)`
6. 根据返回值：
   - `"launched"` → `isInstalled = true`，UI 提示"请在桌面重新打开摊盒"
   - `"no_activity"` / `"error:..."` → 尝试 Tier 2：`open(apkPath)` via `plugin-shell`
   - `"permission_denied"` → 同步骤 4

Tier 2 失败则设置 `androidFallbackPath = apkPath`，UI 显示路径+"前往 GitHub"按钮。

### 5. UI 改动（UpdateModal.vue）

最小化改动。Android 场景下多暴露两个状态：

- `needsPermissionGrant === true` → 显示 alert + "去授权" 按钮 → 点了调 `open_install_settings()`，用户返回后按钮变回"下载并安装"让他们重试
- `androidFallbackPath !== null` → 显示 alert "自动安装失败，APK 已保存到 <path>" + "前往 GitHub" 备选

Windows 的 `confirmRestart()` 对话框在 Android 上永远不会触发（Android 安装 APK 时系统自动杀进程，不走 `relaunch`），所以无需修改 restart 分支。

## 数据流（时序）

```
用户点「下载并安装」
  │
  ├── canRequestInstall() ── false ──► 标记 needsPermissionGrant ──► 结束，等用户授权后重试
  │
  └── true
       │
       ├── fetch GitHub release → 挑 .apk asset → stream download 到 cache
       │    │
       │    ├── 下载失败 (network) ─► 设 error，结束
       │    │
       │    └── 下载完成
       │         │
       │         └── invoke('install_apk', { path })
       │              │
       │              ├── "launched" ─► isInstalled = true, UI 提示手动重开
       │              │
       │              ├── "permission_denied" ─► 标记 needsPermissionGrant，结束
       │              │
       │              └── "no_activity" | "error:*" ─► Tier 2
       │                   │
       │                   └── shell.open(path)
       │                        │
       │                        ├── 成功 ─► isInstalled = true, UI 提示手动重开
       │                        │
       │                        └── 失败 ─► androidFallbackPath = path, UI 显示兜底按钮
```

## 错误处理矩阵

| 场景 | 原因 | 前端表现 | 后续路径 |
|---|---|---|---|
| 下载失败 | 网络 / GitHub 限流 | "下载失败：<原因>" + 重试 | 用户重试 |
| 选不到 APK asset | release 里没有 `.apk` 文件 | "该版本没有 Android 安装包" | 显示去 GitHub 页面按钮（备选仍可能有 apk） |
| `canRequestInstall` false | Android 8+ 未授权 | 暂停 + "去设置"按钮 | 用户授权回来重试 |
| Tier 1 launched | 正常 | "请在桌面重新打开摊盒" | 系统 PackageInstaller 接管 |
| Tier 1 no_activity | 定制 ROM 拦截 / `<queries>` 未生效 | 静默切 Tier 2 | 见下 |
| Tier 2 shell open 成功 | 某些文件管理器接收了 | "请在桌面重新打开摊盒" | 用户操作 |
| Tier 2 失败 | 系统完全找不到 handler | "自动安装失败，APK 在 <path>" + 备选按钮 | 手动 |
| 安装器被用户取消 | 用户在系统对话框点"取消" | 无感知（Android 不通知） | 用户再点"下载并安装"（APK 已缓存可复用？见开放项 A）|

## 测试策略

**不加单元测试**，同 Windows spec 的理由——plugin 边界清晰，实际需要真机验证。

手动 smoke test 至少覆盖：

1. **新手机 + 权限已开**：走 Tier 1 主路径
2. **未开"未知来源"**：验证 `needsPermissionGrant` 流程 → 跳系统设置 → 回来重试成功
3. **老平板 Android 7–8**：验证 Tier 1 能走通（FileProvider 是否有 quirk）
4. **国内 ROM（小米 / 华为）**：若有，验证 Tier 1 失败时 Tier 2 能兜住
5. **模拟 Tier 1 全挂**：临时把 `install_apk` 返回改成 `"error:test"`，验证代码确实切到 Tier 2

测试 4/5 如果真机缺位，至少跑 5 确认降级代码逻辑对。

## 已知限制 / 开放项

### 开放项（实施阶段首先确认）

A. **APK 缓存策略**：下载的 APK 放 `appCacheDir/update.apk`，每次覆盖同名文件。用户取消安装时是否复用？**默认：不复用，重新下**（简单；但浪费流量）。替代：加一个 SHA 校验，已缓存就跳过下载。**默认先选不复用，后续根据反馈优化**。

B. **`versionCode` 递增**：Tauri 2 Android bundle 是否自动把 `tauri.conf.json` 的 `1.1.1` 映射成单调递增的 versionCode？需要确认。如果不自动，需在 `build.gradle.kts` 里手工拼（例如 `major * 10000 + minor * 100 + patch`）。**实施阶段 Task 0 先查明**，若需要手工改则单独做一个 task。

C. **Tauri 对 Android app cache dir 的 API**：`@tauri-apps/api/path` 的 `appCacheDir()` 在 Android 上返回什么？需要确认返回的是 `/data/data/com.abl.BoothKernel/cache/` 之类应用私有目录（FileProvider 里 `cache-path` 覆盖到）。

### 明确限制

- **首次更新必经过系统设置页**：Android 8+ 硬性要求，无法跳过。用户体验"第一次多一步，之后都顺"
- **签名证书一致性**：`my-release-key.jks` 丢了 = 所有已装老版本用户升级断路，只能卸载重装。**BUILD.md 新章节必须强调**
- **装完无法自动返回摊盒**：Android 系统安装 APK 会杀当前进程，装完用户从桌面重新打开。跟 Windows 的 `relaunch` 不同，不要试图做等价功能
- **Play Store 风险**：如果这个 app 未来上 Play Store，`REQUEST_INSTALL_PACKAGES` 权限会触发 Play Store 的额外审核（Google 把它列为 "sensitive permission"）。在 sideload 分发阶段没问题

## 发布流程影响

Android APK 构建已经走 `cargo tauri android build`。自动更新上线后：

1. 每次 release 构建出 `app-<arch>-release.apk`（或 `app-universal-release.apk`，取决于 gradle 配置）
2. 用 `my-release-key.jks` 签名（现有流程）
3. **新增**：把签名后的 APK 上传到 GitHub release，文件名规范 `摊盒_<version>-android.apk`（或英文名避免 URL encoding 问题）
4. **更新 `latest.json`**：加 `android` 平台块：

```json
{
  "version": "1.1.2",
  "platforms": {
    "windows-x86_64": { ... },
    "android": {
      "url": "https://github.com/.../releases/download/v1.1.2/BoothTool_1.1.2-android.apk"
    }
  }
}
```

（Android 不需要 `signature` 字段，因为自动更新走 APK 签名验证而不是 rsign。但清单要包含 URL 让前端知道去哪下。）

5. BUILD.md 增补 "Android 发布" 一节，说明 keystore 使用、签名命令、上传步骤

## 风险

- **小米 / 华为 ROM 行为**：无法预先测试所有定制 ROM，Tier 2/3 是必需兜底，上线后要有用户反馈渠道
- **Tauri 2 Android 的 JNI 接入成熟度**：官方文档对"Rust → 自定义 Kotlin 方法"的例子少，实施时可能踩坑。预留一个"如果 JNI 走不通退化为完整 Tauri plugin"的备选方案
- **大陆 GitHub 下载速度**：现场网络不稳时下载 APK 可能失败；考虑未来加镜像地址（不在本次 scope）
