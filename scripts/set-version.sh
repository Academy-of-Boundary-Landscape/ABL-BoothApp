#!/usr/bin/env bash
#
# 把版本号同步到所有该改的地方。
#
#   ./scripts/set-version.sh 1.1.2
#   ./scripts/set-version.sh          # 只打印当前各处版本号，检查有没有跑偏
#
# 版本号散在四个文件里，手改极容易漏：
#   src-tauri/tauri.conf.json   ← 真正的权威。安装包文件名、Android versionCode、
#                                 updater 比对的版本都来自它
#   src-tauri/Cargo.toml        ← 连带 Cargo.lock 里的同名条目
#   frontend/package.json
#   package.json                ← 仓库根
#
# Android 的 versionCode 由 tauri 从 version 派生，**必须单调递增**，所以别往回改。

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cd "$REPO_ROOT"
NEW="${1:-}"

python3 - "$NEW" <<'PY'
import json, re, sys, pathlib

new = sys.argv[1] if len(sys.argv) > 1 and sys.argv[1] else None
if new and not re.fullmatch(r"\d+\.\d+\.\d+", new):
    sys.exit(f"版本号格式不对：{new}（要 x.y.z）")

def rw_json(path, get_only):
    p = pathlib.Path(path)
    s = p.read_text(encoding="utf-8")
    cur = json.loads(s)["version"]
    if not get_only:
        s2, n = re.subn(r'("version"\s*:\s*)"[^"]+"', rf'\1"{new}"', s, count=1)
        assert n == 1, path
        p.write_text(s2, encoding="utf-8")
    return cur

def rw_cargo_toml(get_only):
    p = pathlib.Path("src-tauri/Cargo.toml")
    s = p.read_text(encoding="utf-8")
    cur = re.search(r'^version\s*=\s*"([^"]+)"', s, re.M).group(1)
    if not get_only:
        s2, n = re.subn(r'^(version\s*=\s*)"[^"]+"', rf'\1"{new}"', s, count=1, flags=re.M)
        assert n == 1
        p.write_text(s2, encoding="utf-8")
    return cur

def rw_cargo_lock(get_only):
    p = pathlib.Path("src-tauri/Cargo.lock")
    s = p.read_text(encoding="utf-8")
    m = re.search(r'(\[\[package\]\]\nname = "BoothKernel"\nversion = ")([^"]+)(")', s)
    cur = m.group(2)
    if not get_only:
        p.write_text(s[:m.start(2)] + new + s[m.end(2):], encoding="utf-8")
    return cur

targets = [
    ("src-tauri/tauri.conf.json", lambda g: rw_json("src-tauri/tauri.conf.json", g)),
    ("src-tauri/Cargo.toml",      rw_cargo_toml),
    ("src-tauri/Cargo.lock",      rw_cargo_lock),
    ("frontend/package.json",     lambda g: rw_json("frontend/package.json", g)),
    ("package.json",              lambda g: rw_json("package.json", g)),
]

before = {name: fn(True) for name, fn in targets}

if new is None:
    width = max(len(n) for n in before)
    for name, v in before.items():
        print(f"  {name:<{width}}  {v}")
    distinct = set(before.values())
    consistent = len(distinct) == 1
    print()
    print("一致 ✓" if consistent else f"不一致 ✗ —— 出现了 {sorted(distinct)}")
    sys.exit(0 if consistent else 1)

for name, fn in targets:
    fn(False)
    print(f"  {name}: {before[name]} → {new}")
PY

if [ -n "$NEW" ]; then
  echo
  ok "版本号已同步到 $NEW"
  info "别忘了往 CHANGELOG-v1.x.md 里追加这一版的条目"
fi
