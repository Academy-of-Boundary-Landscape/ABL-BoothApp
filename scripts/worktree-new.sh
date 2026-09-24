#!/usr/bin/env bash
# 给并行 worker 开独立 worktree：独立分支、独立 target、带齐不入库的构建前置物。
#
# 为什么 target 要独立：同一个 target 下多个 cargo 并发，会互相编译到对方改了一半的文件。
# 复制主仓库的 target/debug 是为了复用依赖的编译产物，免掉冷编译（~8G，完整复制）。
#
# 用法：scripts/worktree-new.sh <name> [base]   → ../ABL-wt/<name>，分支 3b/<name>
set -euo pipefail
name=${1:?用法: worktree-new.sh <name> [base]}
base=${2:-HEAD}
root=$(git rev-parse --show-toplevel)
wt="$(dirname "$root")/ABL-wt/$name"
[ -e "$wt" ] && { echo "已存在: $wt" >&2; exit 1; }

git -C "$root" worktree add -q -b "3b/$name" "$wt" "$base"

# include_dir! 编译期需要 frontend/dist
cp -r "$root/frontend/dist" "$wt/frontend/dist"
# 完整复制而非硬链接：vitest / vite 会原地改 node_modules 里的缓存文件
cp -r "$root/frontend/node_modules" "$wt/frontend/node_modules"
# 不入库的 ONNX 原生库（cargo check 只需要它们存在）
for f in src-tauri/resources/onnxruntime.dll src-tauri/resources/DirectML.dll; do
  if [ -e "$root/$f" ]; then mkdir -p "$(dirname "$wt/$f")"; cp "$root/$f" "$wt/$f"; fi
done
mkdir -p "$wt/src-tauri/target"
if [ -d "$root/src-tauri/target/debug" ]; then
  cp -r "$root/src-tauri/target/debug" "$wt/src-tauri/target/debug"
fi

# worker 的 brief / 报告 / 日志放 .3b/，不进 git
mkdir -p "$wt/.3b"
echo ".3b/" >> "$(git -C "$wt" rev-parse --git-path info/exclude)"
echo "$wt"
