#!/usr/bin/env python3
"""check-markers.py — 校验 doc/ 方案状态标记（行内简记 + 块记）一致性。

用法:
  uv run --directory script check-markers.py
  uv run --directory script check-markers.py --doc-root <doc目录>

对 project.md / decisions.md / env.md / diff.md 检查:

1. 块记（独立小标题 + admonition callout）：
   - 每个 callout 上方最近的非空行为 `#` 标题（锚点，标题不含标记）
   - callout 标题行 `> [!TYPE] 【标签】 方案标题` 的类型↔标签映射正确：
     当前→IMPORTANT、备选→NOTE、已否决→CAUTION、已替换→WARNING
   - 备选/已否决/已替换：块内第一内容行以 `触发：` / `原因：` 开头，随后一行 `> ` 空行
   - 当前：无触发/原因行，标题行后直接 `> ` 空行
2. 行内简记 = 链接：标记必须以 `[【标签】 方案标题](#锚点)` 形式出现；
   既不在链接内、也不在块记标题行内的裸标签 → 报错

退出码：0 = 全部通过；1 = 存在违规（逐条列出）
"""

import argparse
import re
import sys
from pathlib import Path

LABELS = ("【当前】", "【备选】", "【已否决】", "【已替换】")
TYPE_TO_LABEL = {
    "IMPORTANT": "【当前】",
    "NOTE": "【备选】",
    "CAUTION": "【已否决】",
    "WARNING": "【已替换】",
}
PREFIX_BY_LABEL = {"【备选】": "触发：", "【已否决】": "原因：", "【已替换】": "原因："}
FILES = ("project.md", "decisions.md", "env.md", "diff.md")

FENCE_RE = re.compile(r"^\s*(```|~~~)")
BLOCK_TITLE_RE = re.compile(r"^> \[!(IMPORTANT|NOTE|CAUTION|WARNING)\] (【当前】|【备选】|【已否决】|【已替换】) (.+)$")
INLINE_LINK_RE = re.compile(r"\[(【当前】|【备选】|【已否决】|【已替换】)\s+[^\]\n]+\]\([^)]*\)")
LABEL_RE = re.compile("|".join(re.escape(l) for l in LABELS))
EMPTY_QUOTE_RE = re.compile(r"^>\s*$")


def check_block(lines: list[str], i: int, bt: re.Match, prev_nonblank: str) -> list[str]:
    type_, label, title = bt.group(1), bt.group(2), bt.group(3)
    out = []
    if TYPE_TO_LABEL.get(type_) != label:
        out.append(f"callout [!{type_}] 与标签 {label} 不匹配（应为 {TYPE_TO_LABEL[type_]}）")
    if not prev_nonblank.lstrip().startswith("#"):
        out.append(f"块记缺少独立小标题作锚点（{label} {title}）")

    prefix = PREFIX_BY_LABEL.get(label)
    j = i + 1
    if prefix:
        if j >= len(lines) or not lines[j].startswith(f"> {prefix}"):
            out.append(f"{label} 块第一内容行应为 `> {prefix}…`")
            return out
        j += 1
    if j >= len(lines) or not EMPTY_QUOTE_RE.match(lines[j]):
        out.append(f"{label} 块在标题行/触发原因行后缺 `> ` 空行")
    return out


def check_file(path: Path, issues: list[str]) -> None:
    lines = path.read_text(encoding="utf-8").splitlines()
    prev_nonblank = ""
    in_fence = False

    for i, line in enumerate(lines):
        if FENCE_RE.match(line):
            in_fence = not in_fence
            prev_nonblank = line
            continue
        if in_fence:
            continue

        bt = BLOCK_TITLE_RE.match(line)
        if bt:
            for it in check_block(lines, i, bt, prev_nonblank):
                issues.append(f"{path}:{i + 1}: {it}")
        else:
            link_spans = [m.span() for m in INLINE_LINK_RE.finditer(line)]
            for m in LABEL_RE.finditer(line):
                if not any(s <= m.start() and m.end() <= e for s, e in link_spans):
                    issues.append(f"{path}:{i + 1}: 裸标签 {m.group()} 未用行内简记 [【标签】 方案标题](#锚点)")

        if line.strip():
            prev_nonblank = line


def main() -> int:
    parser = argparse.ArgumentParser(description="校验 doc/ 方案状态标记一致性")
    parser.add_argument(
        "--doc-root",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "doc",
        help="doc 目录（默认仓库根下 doc/）",
    )
    args = parser.parse_args()

    issues: list[str] = []
    for file in FILES:
        check_file(args.doc_root / file, issues)

    if not issues:
        print("OK: markers consistent")
        return 0
    for it in issues:
        print("VIOLATION " + it)
    return 1


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    sys.exit(main())
