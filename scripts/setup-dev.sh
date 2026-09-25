#!/usr/bin/env bash
#
# 一次性把工作副本配成「可构建」状态。
#
# 换机器 / 新 clone 之后跑这一个脚本就够了：它负责所有**不入库但构建必需**的东西
# （node_modules、ONNX Runtime 原生库），并在最后清点还缺哪些签名材料。
#
#   ./scripts/setup-dev.sh
#
# 幂等：已经就位且校验通过的东西会跳过。

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# ONNX Runtime 必须是 1.23.x，与 Cargo.toml 里 ort 2.0.0-rc.11 匹配；版本不符会在运行时 crash。
ORT_VERSION=1.23.0
DIRECTML_VERSION=1.15.4

SHA_ORT_DLL=f5131591edac6b0a8090d0e329040a49319d7a689cb5b465235fbf7030fa8027
SHA_DIRECTML_DLL=9c9e6d822561c6c41b90e6994b3e8857cf1d66dbfb1e0c4c799c7c89b4e92da1
SHA_ORT_SO=cd1285f8955f3abcb0127d1ffaf1e5da7893d2547872a831b4717c0bfe388328

# onnxruntime.dll 依赖 VC++ 运行库（MSVCP140 / VCRUNTIME140 及其 _1），而 BoothKernel.exe
# 静态链接了 CRT、自己不需要——于是没装 VC++ 运行库的机器上，一加载模型就闪退。
# 按微软允许的 app-local 方式跟 exe 放在同一目录。来源是 conda-forge 的 vc14_runtime
# （微软签名的原版 DLL 重新打包，版本号即 MSVC 运行库版本），先校验整包再逐个校验。
VCRT_PKG=vc14_runtime-14.51.36247-habf1de7_41
SHA_VCRT_PKG=4e4cb599cdc41bf2109d1464c127b5bcbddf548ce3e322e612afb691338b48f8
VCRT_DLLS=(
  "vcruntime140.dll   d1f4225df2cd877dbf130d5668a021dce3f94118455ff5ec952061c30afc9ce7"
  "vcruntime140_1.dll a7146c08f89fe5b04541ab507cdb59ff7b44534d4ba3c668a426c6450a03434e"
  "msvcp140.dll       7c26614e1d733892c2deac7e245ce115504b1d80592dd0a01b08e3e5a55f89ca"
  "msvcp140_1.dll     206c931bf90fdad8816de3b5e2ef80b2bcaa9406c89ecc05fe6fddffe251e982"
)

cd "$REPO_ROOT"

info "检查基础工具"
need_cmd node   "先装 Node 20+（本机用 nvm）。"
need_cmd npm    "跟 node 一起装。"
need_cmd curl   "系统包管理器装一下。"
need_cmd unzip  "系统包管理器装一下。"
need_cmd zstd   "系统包管理器装一下（解 conda 包里的 VC++ 运行库用）。"
need_cmd python3 "脚本用它读 tauri.conf.json。"
need_cmd rustup "装 Rust 工具链：https://rustup.rs"
ok "node $(node -v) / npm $(npm -v)"

info "安装 npm 依赖（根 + frontend）"
npm ci --no-audit --no-fund
npm ci --no-audit --no-fund --prefix frontend
ok "依赖安装完成"

# --- Rust 交叉编译 target --------------------------------------------------
# rust-toolchain.toml 只钉 channel（1.98.1），故意不列 targets（见该文件注释：
# 写在 toml 里会让 rustup 每次认到它就把全部 target 急切下载一遍，CI 里三个 job
# 各自只用得上其中一个，会白下几百 MB）。target 安装挪到这里，只在真正需要
# 交叉编译的本机上跑一次。显式 --toolchain 1.98.1，不依赖 cwd 命中 override
# （虽然上面已经 `cd "$REPO_ROOT"`，这里有 rust-toolchain.toml，两者本该一致，
# 显式指定更保险，也和这段命令改版本号时要同步改的地方对齐）。
info "安装非宿主 Rust target（Windows / Android 交叉编译用）"
# 显式先装工具链本体：`rustup target add --toolchain <未安装的版本>` 目前会自动
# 补装该工具链，但已经打出 `warn: auto-installation is deprecated for most
# rustup commands`——这个隐式行为将来会被移除，届时新克隆的机器就装不起来了。
# 幂等：已装的话这一步只会说 up to date。
rustup toolchain install 1.98.1
rustup target add --toolchain 1.98.1 \
  x86_64-pc-windows-msvc \
  aarch64-linux-android \
  armv7-linux-androideabi \
  i686-linux-android \
  x86_64-linux-android
ok "target 就位"

# --- ONNX Runtime 原生库 ------------------------------------------------------
# 三个文件都在 .gitignore 里，迁移机器时不会跟着 git 过来，必须重新下载。
# 出处与原理见 docs/BUILD.md。

info "Windows 用 ONNX Runtime（DirectML 版）"
fetch_verified \
  "https://www.nuget.org/api/v2/package/Microsoft.ML.OnnxRuntime.DirectML/${ORT_VERSION}" \
  "$REPO_ROOT/src-tauri/resources/onnxruntime.dll" \
  "$SHA_ORT_DLL" \
  "runtimes/win-x64/native/onnxruntime.dll"

info "Windows 用 DirectML（独立分发版）"
fetch_verified \
  "https://www.nuget.org/api/v2/package/Microsoft.AI.DirectML/${DIRECTML_VERSION}" \
  "$REPO_ROOT/src-tauri/resources/DirectML.dll" \
  "$SHA_DIRECTML_DLL" \
  "bin/x64-win/DirectML.dll"

info "Windows 用 VC++ 运行库（onnxruntime.dll 的依赖，app-local）"
fetch_vcrt() {
  local need=0 name sha
  for entry in "${VCRT_DLLS[@]}"; do
    read -r name sha <<<"$entry"
    local dest="$REPO_ROOT/src-tauri/resources/$name"
    if [ ! -f "$dest" ] || [ "$(sha256sum "$dest" | cut -d' ' -f1)" != "$sha" ]; then need=1; fi
  done
  if [ "$need" = 0 ]; then ok "VC++ 运行库 ${#VCRT_DLLS[@]} 个 DLL 已就位且校验通过"; return 0; fi

  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  info "下载 $VCRT_PKG …"
  curl -fsSL --retry 3 -o "$tmp/pkg.conda" \
    "https://conda.anaconda.org/conda-forge/win-64/$VCRT_PKG.conda" || die "下载 $VCRT_PKG 失败"
  local got; got="$(sha256sum "$tmp/pkg.conda" | cut -d' ' -f1)"
  [ "$got" = "$SHA_VCRT_PKG" ] || die "$VCRT_PKG 校验不符：期望 $SHA_VCRT_PKG，实际 $got"
  unzip -o -q "$tmp/pkg.conda" "pkg-$VCRT_PKG.tar.zst" -d "$tmp" || die "解压 $VCRT_PKG 失败"
  zstd -d -q -f "$tmp/pkg-$VCRT_PKG.tar.zst" -o "$tmp/pkg.tar" || die "解压 $VCRT_PKG 失败"
  for entry in "${VCRT_DLLS[@]}"; do
    read -r name sha <<<"$entry"
    tar -xf "$tmp/pkg.tar" -C "$tmp" "$name" || die "包里没有 $name"
    got="$(sha256sum "$tmp/$name" | cut -d' ' -f1)"
    [ "$got" = "$sha" ] || die "$name 校验不符：期望 $sha，实际 $got"
    mv "$tmp/$name" "$REPO_ROOT/src-tauri/resources/$name"
  done
  ok "VC++ 运行库 ${#VCRT_DLLS[@]} 个 DLL 下载完成并校验通过"
}
fetch_vcrt

info "Android 用 ONNX Runtime（NNAPI 版）"
fetch_verified \
  "https://repo1.maven.org/maven2/com/microsoft/onnxruntime/onnxruntime-android/${ORT_VERSION}/onnxruntime-android-${ORT_VERSION}.aar" \
  "$REPO_ROOT/src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a/libonnxruntime.so" \
  "$SHA_ORT_SO" \
  "jni/arm64-v8a/libonnxruntime.so"

# --- Android SDK 路径（给 IDE / 直接跑 gradlew 用；tauri 自己看 ANDROID_HOME）-----
ANDROID_SDK="${ANDROID_HOME:-/data/sunyunbo/android-sdk}"
if [ -d "$ANDROID_SDK" ]; then
  LOCAL_PROPS="$REPO_ROOT/src-tauri/gen/android/local.properties"
  if [ ! -f "$LOCAL_PROPS" ]; then
    printf '## 本机生成，不入库\nsdk.dir=%s\n' "$ANDROID_SDK" > "$LOCAL_PROPS"
    ok "写入 $(realpath --relative-to="$REPO_ROOT" "$LOCAL_PROPS")"
  else
    ok "local.properties 已存在"
  fi
else
  warn "没找到 Android SDK（$ANDROID_SDK），Android 构建会失败"
fi

# --- 清点签名材料 --------------------------------------------------------------
echo
info "签名材料清点（这些东西不入库，换机器必须手动带过来）"

missing=0

if [ -f "$REPO_ROOT/src-tauri/gen/android/keystore.properties" ]; then
  ok "Android keystore.properties 已就位"
else
  warn "缺 src-tauri/gen/android/keystore.properties —— 只能出 debug APK，release 会直接失败"
  missing=1
fi

if [ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  ok "TAURI_SIGNING_PRIVATE_KEY 已设置"
elif [ -f "$REPO_ROOT/src-tauri/updater-key.key" ]; then
  ok "src-tauri/updater-key.key 已就位（构建时用 scripts/build-windows.sh 自动读取）"
else
  warn "缺 updater 私钥 —— 能出安装包，但签不出 .sig，老用户的自动更新用不了"
  missing=1
fi

echo
if [ "$missing" = 0 ]; then
  ok "全部就位。构建：./scripts/build-windows.sh / ./scripts/build-android.sh"
else
  warn "签名材料不全。日常开发和验证构建不受影响；正式发版前必须补齐，详见 docs/BUILD.md。"
fi
