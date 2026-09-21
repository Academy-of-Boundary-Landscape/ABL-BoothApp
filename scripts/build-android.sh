#!/usr/bin/env bash
#
# 构建 Android arm64-v8a APK。
#
#   ./scripts/build-android.sh           # release（需要 keystore.properties）
#   ./scripts/build-android.sh --debug   # debug，不需要签名材料，用来验工具链
#
# 产物：dist/BoothKernel-<版本>-arm64-release.apk
#
# --apk 在 CLI 2.11 是布尔开关；2.9.x 时它要求显式写 --apk true。别乱改。
# 只出 arm64-v8a：gradle.properties 已经把 abiList/archList/targetList 都限死了。
# 真机调试这台服务器插不了 USB，只能 adb connect 走局域网，或者把 APK 拷到手机上装。

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

MODE=release
[ "${1:-}" = "--debug" ] && MODE=debug

cd "$REPO_ROOT"
need_tauri_env

VERSION="$(app_version)"
ANDROID_DIR="$REPO_ROOT/src-tauri/gen/android"
JNI_SO="$ANDROID_DIR/app/src/main/jniLibs/arm64-v8a/libonnxruntime.so"

[ -f "$JNI_SO" ] || die "缺 $JNI_SO —— 先跑 ./scripts/setup-dev.sh（这文件不入库）"

if [ "$MODE" = release ] && [ ! -f "$ANDROID_DIR/keystore.properties" ]; then
  die "缺 src-tauri/gen/android/keystore.properties，出不了可发布的 release APK。
     内容格式：
       storeFile=/abs/path/to/boothkernel-upload.keystore
       storePassword=<store 密码>
       keyAlias=<alias>
       keyPassword=<key 密码>
     必须是签过历史版本（v1.0.0 起）的那把 keystore，换密钥会让老用户装不上。
     只是想验工具链的话：./scripts/build-android.sh --debug"
fi

info "构建 Android APK（$MODE，版本 $VERSION）"
if [ "$MODE" = debug ]; then
  tauri-env android npx tauri android build --apk --target aarch64 --debug
  OUT_DIR="$ANDROID_DIR/app/build/outputs/apk/universal/debug"
else
  tauri-env android npx tauri android build --apk --target aarch64
  OUT_DIR="$ANDROID_DIR/app/build/outputs/apk/universal/release"
fi

APK="$(find "$OUT_DIR" -maxdepth 1 -name '*.apk' | head -1)"
[ -n "$APK" ] || die "没找到 APK（找过 $OUT_DIR），看上面的构建输出"

mkdir -p "$REPO_ROOT/dist"
cp -f "$APK" "$REPO_ROOT/dist/$(basename "$APK")"
ok "dist/$(basename "$APK")  ($(du -h "$APK" | cut -f1))"

if [ "$MODE" = debug ]; then
  warn "这是 debug 包，不能发布。"
fi
