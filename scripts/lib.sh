#!/usr/bin/env bash
# 被 scripts/ 下其它脚本 source。不要直接执行。

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TAURI_CONF="$REPO_ROOT/src-tauri/tauri.conf.json"

info()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m ✓ \033[0m %s\n' "$*"; }
warn()  { printf '\033[1;33m ! \033[0m %s\n' "$*" >&2; }
die()   { printf '\033[1;31m ✗ \033[0m %s\n' "$*" >&2; exit 1; }

# 本机私密配置（updater 私钥密码等）。文件在 .gitignore 里，不入库。
load_env_local() {
  local f="$REPO_ROOT/.env.local"
  [ -f "$f" ] || return 0
  set -a; source "$f"; set +a
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "找不到命令 '$1'。$2"
}

# 这台机器上四套 Tauri 工具链的统一入口，见 ~/.claude/CLAUDE.md
need_tauri_env() {
  command -v tauri-env >/dev/null 2>&1 || die \
    "找不到 tauri-env。它是本机 Tauri 工具链的统一入口（/data/sunyunbo/bin/tauri-env），
     换机器的话先按 /data/sunyunbo/www/tauri-build-handbook.md 把工具链配起来。"
}

app_version() {
  python3 -c "import json;print(json.load(open('$TAURI_CONF',encoding='utf-8'))['version'])"
}

# fetch_verified <url> <目标路径> <sha256> [解压时从 zip 里取的成员路径]
fetch_verified() {
  local url="$1" dest="$2" want="$3" member="${4:-}"
  if [ -f "$dest" ] && [ "$(sha256sum "$dest" | cut -d' ' -f1)" = "$want" ]; then
    ok "$(basename "$dest") 已就位且校验通过"
    return 0
  fi
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  info "下载 $(basename "$dest") …"
  curl -fsSL --retry 3 -o "$tmp/pkg" "$url" || die "下载失败：$url"
  mkdir -p "$(dirname "$dest")"
  if [ -n "$member" ]; then
    unzip -o -j "$tmp/pkg" "$member" -d "$tmp/out" >/dev/null || die "解压失败：$member"
    mv "$tmp/out/$(basename "$member")" "$dest"
  else
    mv "$tmp/pkg" "$dest"
  fi
  local got; got="$(sha256sum "$dest" | cut -d' ' -f1)"
  [ "$got" = "$want" ] || die "$(basename "$dest") 校验不符：期望 $want，实际 $got"
  ok "$(basename "$dest") 下载完成并校验通过"
}
