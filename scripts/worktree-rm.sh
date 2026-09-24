#!/usr/bin/env bash
# 清理 worktree-new.sh 开的 worktree。分支未合并时 `branch -d` 会拒绝——那正是该停下来看的时候。
set -euo pipefail
name=${1:?用法: worktree-rm.sh <name>}
root=$(git rev-parse --show-toplevel)
wt="$(dirname "$root")/ABL-wt/$name"
git -C "$root" worktree remove --force "$wt"
git -C "$root" branch -d "3b/$name"
