#!/usr/bin/env python3
"""check-commit-msg.py — 校验 git 提交信息是否符合 Conventional Commits 格式。

用法:
  uv run --directory script check-commit-msg.py <提交信息文件>

由 .githooks/commit-msg 钩子调用；仅校验本地提交，绝不自动推送。
不符合时逐条输出原因并返回 exit code 1。
"""

import re
import sys
from pathlib import Path

TYPES = {"feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore", "revert"}
SUBJECT_RE = re.compile(r"^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([^()\s]+\))?:\s+(\S.*)$")
CLOSES_RE = re.compile(r"^Closes\s+#\d+(,\s*#\d+)*$")
EXEMPT_PREFIX = ("Merge ", "Revert ", "fixup! ", "squash! ")
MAX_LINE = 72


def check(msg: str) -> list[str]:
    lines = [ln for ln in msg.splitlines() if not ln.startswith("#")]
    while lines and lines[-1] == "":
        lines.pop()
    while lines and lines[0] == "":
        lines.pop(0)

    if not lines:
        return ["空提交信息，已拒绝"]

    subject = lines[0]
    if subject.startswith(EXEMPT_PREFIX):
        return []

    problems: list[str] = []
    m = SUBJECT_RE.match(subject)
    if not m:
        problems.append(
            f"主题行格式不符：`{subject}`；应为 `<类型>(<可选范围>): <主题>`，"
            f"类型仅限 {'/'.join(sorted(TYPES))}，冒号后必须带空格"
        )
        return problems

    subj = m.group(3)
    if subj.endswith("."):
        problems.append("主题结尾不能加句号")
    if subj and subj[0].isascii() and subj[0].isupper():
        problems.append("主题英文首字母不要大写")

    rest = lines[1:]
    if rest:
        if rest[0] != "":
            problems.append("主题与正文之间需空行分隔")
        for i, ln in enumerate(rest[1:], start=2):
            if len(ln) > MAX_LINE:
                problems.append(f"正文第 {i} 行超过 {MAX_LINE} 字符（当前 {len(ln)}）")
        for ln in rest:
            if ln.startswith("BREAKING CHANGE:"):
                if not ln[len("BREAKING CHANGE:"):].strip():
                    problems.append("BREAKING CHANGE 需带描述")
            elif ln.startswith("Closes"):
                if not CLOSES_RE.match(ln):
                    problems.append(f"Closes 格式不符：`{ln}`；应为 `Closes #123, #456`")

    return problems


def main() -> int:
    sys.stdout.reconfigure(encoding="utf-8")
    if len(sys.argv) < 2:
        print("用法：check-commit-msg.py <提交信息文件>")
        return 2
    msg = Path(sys.argv[1]).read_text(encoding="utf-8", errors="replace")
    problems = check(msg)
    if not problems:
        print("提交信息格式校验通过")
        return 0
    for p in problems:
        print(f"格式错误：{p}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
