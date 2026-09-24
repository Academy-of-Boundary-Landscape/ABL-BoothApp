#!/usr/bin/env bash
# 清理 worktree-new.sh 开的 worktree。分支未合并时 `branch -d` 会拒绝——那正是该停下来看的时候。
set -euo pipefail
# 用法：worktree-rm.sh [--prefix <p>] <name>   分支前缀默认 3b，与 worktree-new.sh 对应
prefix=3b
if [ "${1:-}" = "--prefix" ]; then prefix=${2:?--prefix 需要参数}; shift 2; fi
name=${1:?用法: worktree-rm.sh [--prefix <p>] <name>}
root=$(git rev-parse --show-toplevel)
wt="$(dirname "$root")/ABL-wt/$name"
git -C "$root" worktree remove --force "$wt"
git -C "$root" branch -d "$prefix/$name"
