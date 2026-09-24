#!/usr/bin/env python3
"""每个非 GET 的 API handler 要么守展会状态，要么写明为什么不需要。

②-1 列出 4 个「不查 events.status」的敞口，②-2 一个没补，②-3 又新增了
十来个写入口——靠记是记不住的，所以做成门禁。

判据：凡是被 post()/put()/patch()/delete() 包起来的 handler 函数，以及（③b 起）
标了 `#[utoipa::path(post|put|patch|delete, ...)]` 的 handler 函数，
**先剥掉注释**，剩下的函数体里必须出现 require_event_open，或者出现豁免标记注释
    // 不需要展会守卫：<理由>
理由必须非空。

守卫可以委托给共用函数，但必须显式声明；脚本会去找那个函数并验证它真的调了守卫：
    // 展会守卫在 <函数名>：<理由>
找不到那个函数、或者它里面没有调用，都判失败。
"""
import json
import re
import sys
from pathlib import Path

API_DIR = Path(__file__).resolve().parent.parent / "src-tauri" / "src" / "api"
# 契约快照：非 GET 操作的 operationId 就是 handler 名（utoipa 默认），用来交叉校验
# 脚本有没有漏认某种注册写法。openapi.json 本身由 openapi_snapshot 测试保证是最新的。
OPENAPI = Path(__file__).resolve().parent.parent / "src-tauri" / "openapi.json"
EXEMPT_RE = re.compile(r"//\s*不需要展会守卫：\s*\S+")
# // 展会守卫在 <函数名>：<理由>
DELEGATE_RE = re.compile(r"//\s*展会守卫在\s+([A-Za-z_][A-Za-z0-9_]*)\s*：\s*(\S[^\n]*)")
# 宽松地找出所有注册点，参数是什么交给后面判——`post(handler::<T>)`、`post(|| ...)`
# 这类写法解析不出裸标识符，要当成问题报出来，而不是当作不存在。
METHOD_TOKEN_RE = re.compile(r"\b(?:post|put|patch|delete)\s*\(")
IDENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
# ③b：路由改成 `routes!(a, b)` 注册后，方法写在 handler 的 utoipa 注解里，
# 上面那条正则就再也看不到它们了——不加这一条，门禁会静默地变成空转。
UTOIPA_WRITE_RE = re.compile(
    # 方法既可以单写（`post,`），也可以是 `method(post, put)` 这种多方法写法——
    # 后者漏掉的话，同时挂 POST/PUT 的 handler 会静默脱离门禁。
    r"#\[utoipa::path\(\s*(?:(?:post|put|patch|delete)\b|method\([^)]*\b(?:post|put|patch|delete)\b[^)]*\))"
    r".*?\)\]\s*(?:#\[[^\]]*\]\s*)*"
    r"(?:pub\s+)?async fn ([A-Za-z_][A-Za-z0-9_]*)\s*\(",
    re.S,
)


def strip_comments(src: str) -> str:
    """把行注释 `// ...` 整段去掉。

    只跳过普通 `"..."` 字符串（含 `\\"` 转义），不处理原始字符串——这个代码库
    没有把 `//` 放进字符串字面量的 handler。行尾换行保留，行号不会漂。
    """
    out = []
    i = 0
    n = len(src)
    in_string = False
    while i < n:
        c = src[i]
        if in_string:
            out.append(c)
            if c == "\\" and i + 1 < n:
                out.append(src[i + 1])
                i += 2
                continue
            if c == '"':
                in_string = False
            i += 1
            continue
        if c == '"':
            in_string = True
            out.append(c)
            i += 1
            continue
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            if j == -1:
                break
            i = j
            continue
        out.append(c)
        i += 1
    return "".join(out)


def handler_body(src: str, name: str) -> str | None:
    m = re.search(rf"^async fn {re.escape(name)}\s*\(", src, re.M)
    if not m:
        return None
    # handler 一律是顶层函数，结束于第一个位于第 0 列的 '}'
    end = src.find("\n}\n", m.start())
    return src[m.start() : end if end != -1 else len(src)]


def route_registrations(src: str):
    """产出每个 post|put|patch|delete 注册点的 (原文片段, 括号内参数)。"""
    for m in METHOD_TOKEN_RE.finditer(src):
        i = m.end()
        depth = 1
        while i < len(src) and depth:
            if src[i] == "(":
                depth += 1
            elif src[i] == ")":
                depth -= 1
            i += 1
        arg = src[m.end() : i - 1].strip()
        yield src[m.start() : i], arg


def write_handlers(code: str, path: Path, problems: list):
    """两种注册方式下的写 handler 名，去重。"""
    seen = []
    for snippet, arg in route_registrations(code):
        if not IDENT_RE.match(arg):
            problems.append(
                f"{path.name}: 无法解析的路由注册 `{snippet.strip()}`，"
                f"请改成具名 handler 或手工确认"
            )
            continue
        if arg not in seen:
            seen.append(arg)
    for m in UTOIPA_WRITE_RE.finditer(code):
        if m.group(1) not in seen:
            seen.append(m.group(1))
    return seen


def main() -> int:
    problems = []
    checked = 0
    found = set()
    for path in sorted(API_DIR.rglob("*.rs")):
        raw = path.read_text(encoding="utf-8")
        code = strip_comments(raw)
        for name in write_handlers(code, path, problems):
            checked += 1
            found.add(name)

            raw_body = handler_body(raw, name)
            if raw_body is None:
                problems.append(f"{path.name}: 路由注册了 {name}，但找不到 `async fn {name}(`")
                continue

            # 成员判断只看剥掉注释后的代码：注释里出现 require_event_open 不算数。
            if "require_event_open" in strip_comments(raw_body):
                continue
            if EXEMPT_RE.search(raw_body):
                continue

            delegates = DELEGATE_RE.findall(raw_body)
            if delegates:
                for target, _reason in delegates:
                    target_body = handler_body(raw, target)
                    if target_body is None:
                        problems.append(
                            f"{path.name}:{name} 委托给 {target}，"
                            f"但源码里找不到 `async fn {target}(`"
                        )
                    elif "require_event_open" not in strip_comments(target_body):
                        problems.append(
                            f"{path.name}:{name} 委托给 {target}，"
                            f"但 {target} 的函数体里没有调用 require_event_open"
                        )
                continue

            problems.append(
                f"{path.name}:{name} 既没调用 require_event_open，"
                f"也没写 `// 不需要展会守卫：<理由>` 或 `// 展会守卫在 <函数名>：<理由>`"
            )

    # 交叉校验：文档里的每个写操作，脚本都必须认出来。这条正则已经因为注册写法
    # 变化（routes!、method(post, put)）两次静默漏认过 handler。
    if OPENAPI.exists():
        doc = json.loads(OPENAPI.read_text(encoding="utf-8"))
        for p, item in doc.get("paths", {}).items():
            for method, op in item.items():
                if method in ("post", "put", "patch", "delete") and isinstance(op, dict):
                    op_id = op.get("operationId")
                    if op_id and op_id not in found:
                        problems.append(
                            f"openapi.json 里的写操作 {method.upper()} {p}（{op_id}）没被脚本认出来——"
                            f"注册写法变了，脚本需要跟着改"
                        )

    # 一个写入口都没找到，只可能是脚本认不出注册方式了，而不是真的没有写入口。
    if checked == 0:
        problems.append("一个写 handler 都没找到——路由注册写法变了，脚本需要跟着改")

    if problems:
        print("展会守卫门禁未通过：", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        print(
            "\n写入口必须守住 events.status（spec 3.1）。"
            "确实不需要的（登录、全局商品库、展会本身的 CRUD 等），"
            "在函数体里写一行 `// 不需要展会守卫：<理由>`；"
            "守卫委托给共用函数的，写 `// 展会守卫在 <函数名>：<理由>`，"
            "脚本会验证被委托的函数真的调了 require_event_open。",
            file=sys.stderr,
        )
        return 1
    print(f"展会守卫门禁通过（{checked} 个写 handler）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
