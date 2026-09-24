# 构建与打包指南

> **在 Linux 开发机上构建？** 这台机器已经配好 Windows 交叉编译和 Android 工具链，
> 所有步骤都封装成了脚本，不用手抄下面的命令：
>
> ```bash
> ./scripts/setup-dev.sh          # 新 clone / 换机器后跑一次
> ./scripts/build-windows.sh      # Windows NSIS 安装包（含 .sig）
> ./scripts/build-android.sh      # Android release APK
> ./scripts/make-latest-json.sh   # 自动更新清单
> ```
>
> 本机特有的坑（NSIS 插件缓存冲突、代理、`--bundles` 不可用等）见仓库根 `CLAUDE.md`。
> 下面这一篇讲的是**原理和手工步骤**，两边内容一致，脚本只是把它自动化了。

## 前置要求

**通用：**
- Rust toolchain (stable)
- Node.js 20+
- Tauri CLI (`cargo install tauri-cli`)

**Windows 额外：**
- Visual Studio Build Tools (C++ 工具链)

**Android 额外：**
- Android SDK (API 36)
- Android NDK
- JDK 17+

## ONNX Runtime 动态库

> 三个文件都可以用 `./scripts/setup-dev.sh` 带 sha256 校验地自动下载，下面是手工步骤。

Vision 功能依赖 ONNX Runtime 动态库。**版本必须是 1.23.x**（与 ort-sys 2.0.0-rc.11 匹配），版本不一致会导致运行时 crash。

### Windows (x64) — DirectML GPU 加速版

需要两个 DLL，放入 `src-tauri/resources/`：

**1. onnxruntime.dll (DirectML 版，~17MB)**
1. 下载 NuGet 包：`https://www.nuget.org/api/v2/package/Microsoft.ML.OnnxRuntime.DirectML/1.23.0`
2. 改 `.nupkg` 为 `.zip` 解压
3. 复制 `runtimes/win-x64/native/onnxruntime.dll` → `src-tauri/resources/onnxruntime.dll`

**2. DirectML.dll (独立分发版，~18MB)**
1. 下载 NuGet 包：`https://www.nuget.org/api/v2/package/Microsoft.AI.DirectML/1.15.4`
2. 改 `.nupkg` 为 `.zip` 解压
3. 复制 `bin/x64-win/DirectML.dll` → `src-tauri/resources/DirectML.dll`

> 打包独立的 DirectML.dll 可以避免依赖用户系统自带的旧版本（Windows 自带最高只有 1.8，ORT 1.23 需要更高版本）。

**⚠ 关键：确保 `tauri.windows.conf.json` 的 `bundle.resources` 声明了这两个 DLL**，否则 `tauri build` 不会把它们打进安装包：

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "bundle": {
    "resources": {
      "resources/onnxruntime.dll": "./",
      "resources/DirectML.dll": "./"
    }
  }
}
```

- 配置写在独立的 `tauri.windows.conf.json`（不在基础 `tauri.conf.json`）**是有意的**：`bundle.resources` 是跨平台统一配置，写在基础配置会让 Android 也把 DLL 同步到 APK assets（多 35MB 无用废料）。用平台专属配置可以让 DLL 只在 Windows 构建时生效。
- `./` 目的地表示放在 Tauri 的 resource_dir 根下，与 `src-tauri/src/lib.rs` 里 `res_dir.join("onnxruntime.dll")` 的查找路径一致。
- 不设此项，安装包只有 ~15MB（只含 Rust 主体+前端），AI 视觉功能装完即失效。

### Android (arm64-v8a) — NNAPI 加速版

**libonnxruntime.so (~19MB)**
1. 下载：`https://repo1.maven.org/maven2/com/microsoft/onnxruntime/onnxruntime-android/1.23.0/onnxruntime-android-1.23.0.aar`
2. 改 `.aar` 为 `.zip` 解压
3. 复制 `jni/arm64-v8a/libonnxruntime.so` → `src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a/libonnxruntime.so`

> Gradle 自动将 jniLibs 下的 .so 打包进 APK。AAR 已内置 NNAPI 支持。

## 构建命令

### Windows

```bash
# 开发
npx tauri dev

# 发布（NSIS 安装包）
npx tauri build
```

输出：`src-tauri/target/release/bundle/nsis/*.exe`

### Android

```bash
# 开发（连接设备或模拟器）
npx tauri android dev

# 发布 APK（推荐，最常用）
npx tauri android build --apk true -t aarch64

# 发布 AAB（Google Play 上架格式）
npx tauri android build --aab true -t aarch64

# 同时生成 APK + AAB
npx tauri android build --apk true --aab true -t aarch64
```

参数说明：
- `--apk true` — 生成 APK（独立分发、侧载用）
- `--aab true` — 生成 AAB（Android App Bundle，Google Play 专用）
- `-t aarch64` — 只编译 arm64-v8a 架构（现代设备主流）

> `gradle.properties` 已配置 `targetList=aarch64`，默认只编译 arm64-v8a。
> `-t aarch64` 可以省略，但显式写出更清晰。

输出路径：
- APK：`src-tauri/gen/android/app/build/outputs/apk/universal/release/*.apk`
- AAB：`src-tauri/gen/android/app/build/outputs/bundle/universalRelease/*.aab`

### 本地测试

```bash
cd src-tauri && cargo test --workspace
```

Rust 测试用 `tempfile::tempdir()` 建临时目录（`test_support::test_state` 等夹具），
`tempfile` 会读 `TMPDIR` 作为临时目录的父目录。**如果 `TMPDIR` 指向一个不存在的路径，
`tempdir()` 会直接 panic**，症状是：

```
create temp dir: NotFound
... path: "<TMPDIR>/.tmpXXXX"
```

这不是代码问题，不要去查 `tempfile` 或测试夹具。跑测试前确保 `TMPDIR` 指向一个**真实
存在**的目录即可（`export TMPDIR=/path/that/exists`，或给单条命令前置
`TMPDIR=/path/that/exists cargo test ...`）。在沙箱 / 容器里 `/tmp` 有时是每次命令
一份的临时挂载，机器默认的 `TMPDIR=/tmp/<user>` 可能并不存在。

更坑的是：**某些沙箱环境里 `/tmp` 不跨 shell 调用保留**——上一条命令里 `mkdir` 出来的
目录，下一条命令里可能又没了。于是 150+ 个用 `tempfile` 的测试会一起挂在
`create temp dir: NotFound`，而报错完全看不出是环境问题。可照抄的做法是把
`mkdir -p "$TMPDIR"` 和 cargo 命令写在**同一条命令**里，让建目录和跑测试发生在同一个
shell、同一份挂载视图内：

```bash
TMPDIR=/path/to/real/dir sh -c 'mkdir -p "$TMPDIR" && cd src-tauri && tauri-env linux cargo test --workspace'
```

## 安装包内容

### Windows NSIS 安装包

| 文件 | 大小 | 说明 |
|------|------|------|
| 应用程序主体 | ~30MB | Rust + 前端 |
| `onnxruntime.dll` | ~17MB | ONNX Runtime (DirectML) |
| `DirectML.dll` | ~18MB | DirectML 独立分发版 |

> v1.1.0 起安装包**不再内嵌模型**，首次启动会在 AI 视觉识别面板中提示下载。
> 默认下载项：`convnextv2_pico_fp16` (~17MB)。

### Android APK

| 文件 | 大小 | 说明 |
|------|------|------|
| 应用程序主体 | ~25MB | Rust .so + 前端 |
| `libonnxruntime.so` | ~19MB | ONNX Runtime (NNAPI) |

## 模型分发

所有模型均为**首次启动后下载**，托管在 [GitHub Release `models-v1`](https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases/tag/models-v1)。

默认推荐下载的模型是 `convnextv2_pico_fp16` (17MB)，17 MB 极小体积、速度最快，识别质量与 FP32 基线几乎无差别。

可选的完整模型清单：

| 模型 | 大小 | 定位 |
|------|------|------|
| `convnextv2_pico_fp16.onnx` | 17MB | ⭐ 默认推荐，体积最小 |
| `dinov2_small_fp16.onnx` | 43MB | ⭐ 高精度推荐，ViT 自监督 |
| `mobileclip_s0_fp32.onnx` | 46MB | CLIP 编码器，语义理解强 |
| `convnextv2_pico_fp32.onnx` | 34MB | 参考精度版本（一般选 FP16 即可）|
| `dinov2_small_fp32.onnx` | 87MB | 参考精度版本（一般选 FP16 即可）|

用户在 管理后台 → 控制台 → AI 视觉识别 面板中下载安装。

> **注**：MobileCLIP 只有 FP32 版本，已验证其 FP16 / INT8 变体会产生错误 embedding。

## 推理设备

| 平台 | 自动模式 | 可选 |
|------|---------|------|
| Windows | DirectML (GPU) → CPU | 指定 GPU 设备 / 仅 CPU |
| Android | NNAPI (NPU/GPU) → CPU | NNAPI / 仅 CPU |

设备选择在 管理后台 → 控制台 → AI 视觉识别 面板的"设备选择"下拉框中配置，设置持久化到 `vision_model.json`。

## 发布带自动更新的新版本

从 v1.1.1 起，客户端会从 GitHub Releases 拉取 `latest.json` 清单判断更新。每次发布需要把三个 artifact 一起传上去：安装器、签名文件、清单。

### updater 签名密钥（已生成，不要重新生成）

密钥对在 v1.1.1 发布时就已经定下来了：

- `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey` 内联的是 key ID **`C56CE43C94863207`**
  的公钥，和仓库里的 `src-tauri/updater-key.key.pub` 一致。
- 配对的私钥是 `src-tauri/updater-key.key`（加密，**绝不进 git**），密码在密码管理器里；
  Linux 开发机上另存了一份在仓库根 `.env.local`（已 gitignore）。

> ⚠️ **不要重新跑 `signer generate`。** 换了密钥对，所有已经装了旧版的用户点「检查更新」
> 都会永久失败——客户端校验的是内嵌在**旧版安装包**里的那个公钥，改 config 追不回来。
> 唯一的补救是让用户手动重新下载安装。

换机器之后想确认手上这把私钥是不是对的，签个探针文件比对 key ID：

```bash
npx tauri signer sign -f "$PWD/src-tauri/updater-key.key" -p "<密码>" /tmp/probe.txt
```

把 `/tmp/probe.txt.sig` 和 `src-tauri/updater-key.key.pub` 都 base64 解开，取第二行 base64 的
第 3~10 字节（小端序）就是 key ID，两边必须都是 `C56CE43C94863207`。

**私钥和密码务必在仓库之外另存备份。** 丢了就等于永久失去给老用户推更新的能力。

### 发布前必查：sqlx 迁移文件一旦发布就**永久冻结**

`sqlx::migrate!()` 在 `_sqlx_migrations` 表里记的是每个迁移文件的**校验和**。
改动一个已经被应用过的迁移文件，下次启动就是

```
Database initialization failed: Migrate(VersionMismatch(<版本号>))
```

**App 直接 panic，起不来。** 而且「迁移前自动快照 + 失败回滚」那套基建救不了它——
那是防「迁移跑失败」的，而这是「迁移还没开始跑就被拒」。

2026-09-24 真踩过一次：`202609230001_double_entry_schema.sql` 在开发期间被改过两次，
开发机上的 dev 库停在中间某一版，于是 App 起不来。开发库可以改名留存让它重建，
**真实用户的库不行**。

所以：

- **发布之前**改迁移文件是免费的（顶多自己的开发库要重建）。
- **一旦某个版本发出去**，`src-tauri/migrations/` 里的既有文件就不准再动一个字节。
  任何修正都必须是**一个新的迁移文件**。
- 装过 beta.N 的测试者升级到 beta.N+1 时同样会踩，不只是正式版用户。

改之前先确认它有没有被应用过（路径按平台换）：

```bash
sqlite3 <app data dir>/sale_system.db \
  "SELECT version, description FROM _sqlx_migrations ORDER BY version DESC LIMIT 3;"
```

### 每次发布


1. 更新版本号：
   - `src-tauri/tauri.conf.json` → `"version"`
   - `frontend/package.json` → `"version"`
   - 追加 `CHANGELOG-v1.x.md`

2. 设置签名环境变量。

   **Linux 开发机**：密码放在仓库根 `.env.local`（已 gitignore），
   `scripts/build-windows.sh` 会自己读私钥和密码，这一步不用手动做。

   **Windows（PowerShell）**：

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content src-tauri\updater-key.key -Raw)
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "<生成密钥时设置的那个密码>"
```

3. 构建。

   **Linux 开发机**：

```bash
./scripts/build-windows.sh      # 前端 dist 由 beforeBuildCommand 自动构建
./scripts/build-android.sh
```

   **Windows**：

```powershell
npm run tauri build             # beforeBuildCommand 会先构建 frontend/dist
```

产出物在 `src-tauri/target/release/bundle/nsis/`：
- `摊盒_x.y.z_x64-setup.exe` — NSIS 安装器
- `摊盒_x.y.z_x64-setup.exe.sig` — rsign 签名文件

4. 生成 `latest.json`。内容模板：

```json
{
  "version": "1.1.1",
  "notes": "摊盒 1.1.1 —— 支持一键自动更新",
  "pub_date": "2026-05-01T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<.sig 文件的完整内容，换行改成 \\n>",
      "url": "https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases/download/v1.1.1/摊盒_1.1.1_x64-setup.exe"
    }
  }
}
```

Linux 开发机上直接 `./scripts/make-latest-json.sh "更新说明"`，产物在 `dist/latest.json`。

Windows 上手工做的话，这个 PowerShell one-liner 可以生成 `signature` 字段（输出需要再塞到 JSON 字符串里）：

```powershell
(Get-Content "src-tauri/target/release/bundle/nsis/摊盒_1.1.1_x64-setup.exe.sig" -Raw) -replace "`r`n", "\n"
```

5. 在 GitHub 打 tag `v1.1.1` → 建 Release → 上传三个 asset：
   - `摊盒_1.1.1_x64-setup.exe`
   - `摊盒_1.1.1_x64-setup.exe.sig`
   - `latest.json`（**文件名必须就叫 `latest.json`**，客户端 endpoint 写死了这个名字）
6. Publish Release。

### 验收

在一台装着旧版（比如 v1.1.0，或者你故意装的 v1.1.99）的机器上点「检查更新」。应当能看到新版本、下载进度、重启后版本真的变了。如果任一环节失败，看 `docs/guide/auto-update.md`「什么时候用不了自动更新」章节排障。

### 常见问题

- **"invalid signature" 错误**：`.sig` 和 `.exe` 不匹配。多半是重新构建了 `.exe` 但忘了重新生成 `.sig`，或上传顺序弄错。把三个文件重新做一遍。
- **"No version available"**：`latest.json` 没上传，或文件名不对。客户端 endpoint 写的是 `/releases/latest/download/latest.json`，GitHub 要求 asset 名字**精确**是 `latest.json`。
- **中文文件名下载后变 `???`**：部分浏览器 / CDN 对中文 URL 处理有坑。考虑把 `productName` 改成 ASCII-only 名称重新构建，或在上传到 release 时重命名 `.exe` 为英文再更新 `latest.json` 里的 `url`。
