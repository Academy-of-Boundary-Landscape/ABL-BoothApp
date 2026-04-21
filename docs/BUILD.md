# 构建与打包指南

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
