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
cargo tauri dev

# 发布（NSIS 安装包）
cargo tauri build
```

输出：`src-tauri/target/release/bundle/nsis/*.exe`

### Android

```bash
# 开发（连接设备或模拟器）
cargo tauri android dev

# 发布 APK
cargo tauri android build
```

> `gradle.properties` 已配置 `targetList=aarch64`，默认只编译 arm64-v8a，无需额外指定 `--target`。

输出：`src-tauri/gen/android/app/build/outputs/apk/universal/release/*.apk`

## 安装包内容

### Windows NSIS 安装包

| 文件 | 大小 | 说明 |
|------|------|------|
| 应用程序主体 | ~30MB | Rust + 前端 |
| `onnxruntime.dll` | ~17MB | ONNX Runtime (DirectML) |
| `DirectML.dll` | ~18MB | DirectML 独立分发版 |
| `models/vision/convnextv2_pico_fp32/model.onnx` | ~34MB | 内嵌默认 AI 模型 |

### Android APK

| 文件 | 大小 | 说明 |
|------|------|------|
| 应用程序主体 | ~25MB | Rust .so + 前端 |
| `libonnxruntime.so` | ~19MB | ONNX Runtime (NNAPI) |
| 内嵌模型 | ~34MB | ConvNeXtV2-Pico FP32 |

## 模型分发

内嵌默认模型：`convnextv2_pico_fp32` (34MB)，安装后开箱即用。

可选下载模型托管在 [GitHub Release `models-v1`](https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases/tag/models-v1)：

| 模型 | 大小 | 说明 |
|------|------|------|
| `mobileclip_s0_fp32_tract.onnx` | 43MB | CLIP 图像编码器，均衡之选 |
| `dinov2_small_fp32_tract.onnx` | 83MB | ViT 自监督模型，检索质量最佳 |

用户在 管理后台 → 控制台 → AI 视觉识别 面板中下载安装。

## 推理设备

| 平台 | 自动模式 | 可选 |
|------|---------|------|
| Windows | DirectML (GPU) → CPU | 指定 GPU 设备 / 仅 CPU |
| Android | NNAPI (NPU/GPU) → CPU | NNAPI / 仅 CPU |

设备选择在 管理后台 → 控制台 → AI 视觉识别 面板的"设备选择"下拉框中配置，设置持久化到 `vision_model.json`。
