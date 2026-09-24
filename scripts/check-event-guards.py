#!/usr/bin/env python3
"""每个非 GET 的 API handler 要么守展会状态，要么写明为什么不需要。

②-1 列出 4 个「不查 events.status」的敞口，②-2 一个没补，②-3 又新增了
十来个写入口——靠记是记不住的，所以做成门禁。

判据：凡是被 post()/put()/patch()/delete() 包起来的 handler 函数，
函数体里必须出现 require_event_open，或者出现豁免标记注释
    // 不需要展会守卫：<理由>
理由必须非空。
"""
import re
import sys
from pathlib import Path

API_DIR = Path(__file__).resolve().parent.parent / "src-tauri" / "src" / "api"
ROUTE_RE = re.compile(r"\b(?:post|put|patch|delete)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)")
EXEMPT_RE = re.compile(r"//\s*不需要展会守卫：\s*\S+")

def handler_body(src: str, name: str) -> str | None:
    m = re.search(rf"^async fn {re.escape(name)}\s*\(", src, re.M)
    if not m:
        return None
    # handler 一律是顶层函数，结束于第一个位于第 0 列的 '}'
    end = src.find("\n}\n", m.start())
    return src[m.start() : end if end != -1 else len(src)]

def main() -> int:
    problems = []
    for path in sorted(API_DIR.glob("*.rs")):
        src = path.read_text(encoding="utf-8")
        # 只看 router() 里的注册，避免把 `post(` 的其它用法算进来
        for name in sorted(set(ROUTE_RE.findall(src))):
            body = handler_body(src, name)
            if body is None:
                problems.append(f"{path.name}: 路由注册了 {name}，但找不到 `async fn {name}(`")
                continue
            if "require_event_open" in body or EXEMPT_RE.search(body):
                continue
            problems.append(
                f"{path.name}:{name} 既没调用 require_event_open，"
                f"也没写 `// 不需要展会守卫：<理由>`"
            )
    if problems:
        print("展会守卫门禁未通过：", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        print(
            "\n写入口必须守住 events.status（spec 3.1）。"
            "确实不需要的（登录、全局商品库、展会本身的 CRUD 等），"
            "在函数体里写一行 `// 不需要展会守卫：<理由>`。",
            file=sys.stderr,
        )
        return 1
    print("展会守卫门禁通过")
    return 0

if __name__ == "__main__":
    sys.exit(main())
