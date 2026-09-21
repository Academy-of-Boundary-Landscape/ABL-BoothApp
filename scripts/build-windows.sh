#!/usr/bin/env bash
#
# 在这台 Linux 机器上交叉编译 Windows x64 安装包（NSIS）。
#
#   ./scripts/build-windows.sh              # 出安装包；有私钥就顺带签出 .sig
#   ./scripts/build-windows.sh --no-sign    # 明确跳过签名（产物不可用于发布）
#
# 产物会被改成和历史 Release 一致的 ASCII 文件名，放到 dist/ 下：
#   dist/BoothKernel-Windows-<版本>-x64-release-setup.exe[.sig]
#
# 交叉编译只证明「编得过、打得出包」。COM / DXGI / WebView2 这些东西的运行时行为
# 仍然必须在真 Windows 上验。

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

NO_SIGN=0
[ "${1:-}" = "--no-sign" ] && NO_SIGN=1

cd "$REPO_ROOT"
load_env_local
need_tauri_env
need_cmd curl "下载 NSIS 插件要用。"

TARGET=x86_64-pc-windows-msvc
VERSION="$(app_version)"
OUT_NAME="BoothKernel-Windows-${VERSION}-x64-release-setup.exe"

# --- NSIS 插件缓存隔离 ---------------------------------------------------------
# tauri 的 NSIS 插件缓存是**按机器全局**一份（~/.cache/tauri/NSIS/...），路径里不带版本号。
# 本机还有别的 Tauri 项目用更新的 CLI，它们要的 nsis_tauri_utils 版本不一样，
# 会把同一个文件反复覆盖 → 另一边就报 "NSIS directory contains mis-hashed files" 然后重下。
# 而本机全局 http_proxy 下 tauri 内置的下载器会失败（protocol: http response missing version），
# 于是构建直接挂掉。这里给本项目单独开一份缓存，并用 curl（走得通代理）预先塞好插件。
export XDG_CACHE_HOME="$HOME/.cache/boothkernel-tauri"
# 但 cargo-xwin 的 Windows SDK/CRT 缓存有 1.1G，跟着一起隔离等于白下一遍。
# 它有自己的环境变量，显式指回全局那份共享缓存。
export XWIN_CACHE_DIR="${XWIN_CACHE_DIR:-$HOME/.cache/cargo-xwin}"
NSIS_PLUGIN_DIR="$XDG_CACHE_HOME/tauri/NSIS/Plugins/x86-unicode/additional"
NSIS_PLUGIN="$NSIS_PLUGIN_DIR/nsis_tauri_utils.dll"

# 每次都从 CLI 二进制里读它写死的插件版本 —— CLI 一升级要的版本就变，光判断「文件在不在」
# 会拿旧版本糊弄过去，结果还是 mis-hash。用一个 .version 边车文件记住缓存里躺的是哪一版。
CLI_NODE="$REPO_ROOT/node_modules/@tauri-apps/cli-linux-x64-gnu/cli.linux-x64-gnu.node"
[ -f "$CLI_NODE" ] || die "找不到 tauri CLI 原生模块，先跑 ./scripts/setup-dev.sh"
PLUGIN_TAG="$(strings "$CLI_NODE" | grep -o 'nsis_tauri_utils-v[0-9.]*' | sort -u | head -1)"
[ -n "$PLUGIN_TAG" ] || die "没能从 tauri CLI 里解析出 nsis_tauri_utils 版本"

if [ ! -f "$NSIS_PLUGIN" ] || [ "$(cat "$NSIS_PLUGIN.version" 2>/dev/null)" != "$PLUGIN_TAG" ]; then
  info "预置 NSIS 插件 $PLUGIN_TAG（本项目专属缓存）"
  mkdir -p "$NSIS_PLUGIN_DIR"
  curl -fsSL --retry 3 -o "$NSIS_PLUGIN" \
    "https://github.com/tauri-apps/nsis-tauri-utils/releases/download/${PLUGIN_TAG}/nsis_tauri_utils.dll" \
    || die "NSIS 插件下载失败"
  printf '%s' "$PLUGIN_TAG" > "$NSIS_PLUGIN.version"
  ok "已放到 $NSIS_PLUGIN"
fi

# --- updater 签名 --------------------------------------------------------------
# 有私钥才加 createUpdaterArtifacts —— 否则 tauri 会因为「有公钥没私钥」直接报错。
EXTRA_ARGS=()
if [ "$NO_SIGN" = 1 ]; then
  warn "--no-sign：不生成 .sig，产物只能自己装着看，不能发布。"
elif [ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  ok "用环境变量里的 updater 私钥"
  EXTRA_ARGS+=(--config '{"bundle":{"createUpdaterArtifacts":true}}')
elif [ -f "$REPO_ROOT/src-tauri/updater-key.key" ]; then
  export TAURI_SIGNING_PRIVATE_KEY="$(cat "$REPO_ROOT/src-tauri/updater-key.key")"
  [ -n "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" ] || die \
    "找到 src-tauri/updater-key.key，但没设 TAURI_SIGNING_PRIVATE_KEY_PASSWORD。
     export TAURI_SIGNING_PRIVATE_KEY_PASSWORD='<生成密钥时的密码>' 再跑一次。"
  ok "用 src-tauri/updater-key.key"
  EXTRA_ARGS+=(--config '{"bundle":{"createUpdaterArtifacts":true}}')
else
  warn "没有 updater 私钥 —— 能出安装包，但签不出 .sig。"
  warn "这种包发上去，老用户点「检查更新」会失败。发版前务必补上私钥。"
fi

info "交叉编译 Windows x64（版本 $VERSION）"
# --bundles nsis 需要 CLI >= 2.11：更早的版本（如 2.9.6）只接受宿主平台（linux）的取值，
# 给 Windows 目标传 nsis 会被 clap 拒掉。真要回退 CLI 的话，去掉这个参数即可 ——
# 打包目标 tauri.conf.json 的 bundle.targets 里也写了 ["nsis"]。
tauri-env win npx tauri build --runner cargo-xwin --target "$TARGET" --bundles nsis "${EXTRA_ARGS[@]}"

# --- 收拢产物 ------------------------------------------------------------------
BUNDLE_DIR="$REPO_ROOT/src-tauri/target/$TARGET/release/bundle/nsis"
SRC_EXE="$(find "$BUNDLE_DIR" -maxdepth 1 -name '*-setup.exe' | head -1)"
[ -n "$SRC_EXE" ] || die "没找到 NSIS 安装包，看上面的构建输出"

mkdir -p "$REPO_ROOT/dist"
cp -f "$SRC_EXE" "$REPO_ROOT/dist/$OUT_NAME"
ok "dist/$OUT_NAME  ($(du -h "$REPO_ROOT/dist/$OUT_NAME" | cut -f1))"

if [ -f "$SRC_EXE.sig" ]; then
  cp -f "$SRC_EXE.sig" "$REPO_ROOT/dist/$OUT_NAME.sig"
  ok "dist/$OUT_NAME.sig"
  echo
  info "下一步：./scripts/make-latest-json.sh 生成 latest.json"
else
  echo
  warn "没有 .sig。自动更新清单（latest.json）做不出来。"
fi
