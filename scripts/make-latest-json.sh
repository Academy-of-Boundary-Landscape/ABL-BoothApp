#!/usr/bin/env bash
#
# 从 dist/ 里已经签好名的安装包生成 latest.json（自动更新清单）。
#
#   ./scripts/make-latest-json.sh ["这一版的更新说明"]
#
# 客户端 endpoint 写死的是
#   https://github.com/.../releases/latest/download/latest.json
# 所以上传到 Release 时**文件名必须精确是 latest.json**。
#
# 发 Release 时三个 asset 一起传，少一个自动更新就用不了：
#   BoothKernel-Windows-<版本>-x64-release-setup.exe
#   BoothKernel-Windows-<版本>-x64-release-setup.exe.sig
#   latest.json

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cd "$REPO_ROOT"

VERSION="$(app_version)"
NOTES="${1:-摊盒 $VERSION}"
EXE="BoothKernel-Windows-${VERSION}-x64-release-setup.exe"
REPO_URL="https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp"

[ -f "dist/$EXE" ]     || die "没有 dist/$EXE，先跑 ./scripts/build-windows.sh"
[ -f "dist/$EXE.sig" ] || die "没有 dist/$EXE.sig —— 构建时没拿到 updater 私钥，签名缺失。
     补上私钥后重新 ./scripts/build-windows.sh。注意 .sig 必须和最终上传的那个 exe
     是同一次构建的产物，否则客户端会报 invalid signature。"

python3 - "$VERSION" "$NOTES" "$EXE" "$REPO_URL" <<'PY'
import json, sys, pathlib, datetime

version, notes, exe, repo_url = sys.argv[1:5]
dist = pathlib.Path("dist")
sig = (dist / f"{exe}.sig").read_text(encoding="utf-8").strip()

manifest = {
    "version": version,
    "notes": notes,
    "pub_date": datetime.datetime.now(datetime.timezone.utc)
                  .replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    "platforms": {
        "windows-x86_64": {
            "signature": sig,
            # 用 ASCII 文件名。中文文件名在部分浏览器 / CDN 上会变成 ???，下载直接坏掉。
            "url": f"{repo_url}/releases/download/v{version}/{exe}",
        }
    },
}
out = dist / "latest.json"
out.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(f"写入 {out}")
PY

ok "dist/latest.json"
echo
info "打 tag v$VERSION，建 Release，把下面三个文件传上去："
echo "    dist/$EXE"
echo "    dist/$EXE.sig"
echo "    dist/latest.json"
echo "  Android 的 APK 单独传：dist/BoothKernel-${VERSION}-arm64-release.apk"
