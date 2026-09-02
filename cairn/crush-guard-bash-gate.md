---
type: project_topic
status: active
summary: "Crush bash 权限门 cr 的 rg_guard.py 三档分类模型，以及字面量匹配导致的三类绕过漏洞与修复"
tags: [crush, bash-gate, permission, guard, security]
contains: [lesson, decision]
created: "2026-09-02"
updated: "2026-09-02"
related: []
authoring_mode: ai_generated
---
# Crush bash 权限门（guard.py 分类模型与漏勺修复）

## 背景

`.crushrc` 用 PreToolUse hook 挂 `crush-guard/guard.py`（bashlex AST 解析），对 bash 命令做三档分类后返回给 Crush 决定是否放行。另在 `.crushrc` 用 `permissions allow` 直接放行若干工具（不经 hook）：`view ls grep glob` + `edit write multiedit` + `fetch agentic_fetch download`，由 `crush_info` 的 `allowed_tools` 确认。

## 当前结论

三档语义：
- **allow**：只读 / 仓库内安全写 → 放行（exit 0 输出 `{"decision":"allow"}`）。
- **confirm**：有风险、可逆、需人工确认 → exit 0 不输出 JSON（走正常权限提示）。
- **deny**：不可逆 / 破坏性 → exit 2（硬阻断）。

分类对象是 AST 拉平后的每条简单命令（`gather_commands`），因此 `echo hi && rm -rf /` 这类复合命令会被拆开逐条分类，不会因前缀匹配 `echo` 而漏掉 `rm`。这也是本门核心洞见：**用 AST/语义而非正则前缀匹配**，专门堵住两个已知洞——`echo hi && rm -rf /` 与 `git show --output=.git/config HEAD`。

## 经验

分类表关键点（`guard.py`）：
- `DESTRUCTIVE`（sudo/rm/mkfs/...）→ deny；`GIT_DESTRUCTIVE`（reset/clean/rebase/push/pull/...）→ deny。
- `GIT_READONLY`（status/log/diff/show/...）→ allow，但若带 write flag（`--output`/`--pretty`/`-c` 等）转 confirm。
- `GIT_SAFE_WRITE`（add/commit/checkout/switch/mv/rm）→ allow（仅当路径不逃逸仓库）。
- `READONLY` 目录（ls/cat/grep/rg/find/...）→ allow，但若带 write flag 或重定向（`>`/`>>`）转 confirm。
- 构建/工具链（cargo/go/gofmt/black/ruff/make/just/mkdir/touch）→ allow（路径逃逸时 confirm）。
- 默认（npm install/publish、pip、python -c、curl/wget 等）→ confirm；`curl|sh` → deny。

## 教训

对破坏性/写操作做「字面量白名单」匹配天然漏勺，三类实测绕过：

1. **rm flag 变体**：原判 `if any(a in ("-rf","-r","-f","-R") ...)` 只认裸短旗标字面量，`rm --force foo`（等价 `-f` 的长格式）与组合旗标（如 `-rfx`）全漏。**修法**：rm 一律 confirm（项目决策：rm 不自动放行，任何形态都要人确认），彻底消除变体。
2. **`find -delete` / `find -exec rm {} \;`**：`find` 在 READONLY 里被放行，`-exec rm` 是其参数而非命令头，`classify` 不识别 → 绕过 rm 门。**修法**：新增 `find_mutates()`，`find` 带 `-delete/-exec/-execdir/-ok` 时转 confirm；纯读 `find . -type f` 仍 allow。
3. **git 读写形式未分**：`git config`（写 key+value / 写 flag）、`git branch -D`、`git remote add|set-url`、`git tag -d` 等破坏性/注入形态被当只读放行。**修法**：把 `rm/restore/reset` 归为必然改工作树 → confirm；把 `branch/remote/tag/config` 做成「按子命令写形态判断」——只有写形态才 confirm，纯读（`git branch`、`git remote -v`、`git tag`、`git config --list`、`git config user.email` 读值）仍 allow。

核心教训：**判定写/破坏应基于语义（是否带不可逆/写 action），而非字面量 flag 名单**；且需按命令子形态区分「有读写两态的命令」——纯读保持 allow，避免把常见只读操作误伤成 confirm。
