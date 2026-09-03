---
type: project_topic
status: active
summary: "Crush bash 权限门 guard.py 三档分类模型与漏勺修复：三类字面量绕过 + 无害重定向误判为写文件的降级问题"
tags: [crush, bash-gate, permission, guard, security]
contains: [lesson, decision]
created: "2026-09-02"
updated: "2026-09-03"
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
- `READONLY` 目录（ls/cat/grep/rg/find/...）→ allow，但若带 write flag 或**写文件重定向**（`>`/`>>` 到真实路径）转 confirm；丢弃式重定向（`2>/dev/null`/`2>&1`/`>/dev/null`）不转。
- 构建/工具链（cargo/go/gofmt/black/ruff/make/just/mkdir/touch）→ allow（路径逃逸时 confirm）。
- 默认（npm install/publish、pip、python -c、curl/wget 等）→ confirm；`curl|sh` → deny。

## 教训

对破坏性/写操作做「字面量白名单」匹配天然漏勺，三类实测绕过：

1. **rm flag 变体**：原判 `if any(a in ("-rf","-r","-f","-R") ...)` 只认裸短旗标字面量，`rm --force foo`（等价 `-f` 的长格式）与组合旗标（如 `-rfx`）全漏。**修法**：rm 一律 confirm（项目决策：rm 不自动放行，任何形态都要人确认），彻底消除变体。
2. **`find -delete` / `find -exec rm {} \;`**：`find` 在 READONLY 里被放行，`-exec rm` 是其参数而非命令头，`classify` 不识别 → 绕过 rm 门。**修法**：新增 `find_mutates()`，`find` 带 `-delete/-exec/-execdir/-ok` 时转 confirm；纯读 `find . -type f` 仍 allow。
3. **git 读写形式未分**：`git config`（写 key+value / 写 flag）、`git branch -D`、`git remote add|set-url`、`git tag -d` 等破坏性/注入形态被当只读放行。**修法**：把 `rm/restore/reset` 归为必然改工作树 → confirm；把 `branch/remote/tag/config` 做成「按子命令写形态判断」——只有写形态才 confirm，纯读（`git branch`、`git remote -v`、`git tag`、`git config --list`、`git config user.email` 读值）仍 allow。

核心教训：**判定写/破坏应基于语义（是否带不可逆/写 action），而非字面量 flag 名单**；且需按命令子形态区分「有读写两态的命令」——纯读保持 allow，避免把常见只读操作误伤成 confirm。

4. **无害重定向误判为写文件**：`READONLY`（ls/cat/grep/...）带重定向时统一转 confirm，原 `has_redirection()` 只要命中 bashlex `redirect` 节点就返回 True，**不区分目标**——于是 `ls ... 2>/dev/null`、`ls ... 2>&1`、`ls ... > /dev/null`、`cat foo > /dev/null` 这类「丢弃输出/dup fd、无写盘副作用」的常见只读命令全被降级成 confirm，实测 `ls plan.todo 2>/dev/null; ls doc/*.md 2>/dev/null | head` 整条复合命令挡在权限询问前。**修法**：`has_redirection` 改写为 `has_writing_redirect`——只对**真实写文件**的重定向返回 True：`type` 为 `>/>>/>|/<>` 且目标是非 `/dev/null`/`NUL`/`null`/`-` 的路径；输入型（`<`/`<<`/`<<<`）、fd dup（`2>&1`，output 为 int）、fd 关闭（`2>&-`，output 为 `-`）全部视为无写副作用、保持 allow。修正后实测：`2>/dev/null`/`2>&1`/`>/dev/null`/`&>/dev/null` → allow，`> out.txt`/`>> out.txt`/`2> err.txt`/`>| out.txt` → 仍 confirm（安全锁保留），真实写文件混在复合命令里也依旧拦截。
   - 细分教训：**写副作用判定要按「重定向目标」而非「是否存在重定向」**；`/dev/null`/fd dup/fd close 是「消费或丢弃」，不对持久存储产生副作用，禁与 `> filename` 混为一谈。
   - **边界**：heredoc 无终止符时 bashlex 抛 `ParsingError` → `decide` 走 `unparseable` 分支返回 confirm（安全，不误放行）；输入型重定向 `<`/`<<`/`<<<` 只读不写，保持 allow 合理。
   - **判定基座仍是 AST 语义（bashlex）而非正则**：`echo hi > /dev/null`、`git status 2>/dev/null`、`find . -type f 2>/dev/null` 均正确 allow——印证「语义判定」这条路对读写两态命令有效。
   - **实测暴露的边界（写测试时核对实际行为）**：`git reset --hard` / 裸 `git reset` 落 **deny**（`reset` 在 `GIT_DESTRUCTIVE`，非 confirm）；`rm -rf /` 与裸 `rm` 同判 **confirm**（项目决策 rm 一律不自动放行）；`apt-get install` 落 **confirm**（`apt-get` 未列入 `DESTRUCTIVE` 前缀集，仅 `mkfs.ext4` 等走前缀匹配）；`git add .` / `git commit` 在路径不逃逸仓库时落 **allow**（`GIT_SAFE_WRITE`）。回归测试见 `crush-guard/test_guard.py`（allow/confirm/deny 三档覆盖）。
5. **git 只读子命令白名单缺漏（审会确认清单时发现）**：`GIT_READONLY` 只列了 status/log/diff/show/branch/... 等少量，`check-ignore` / `show-ref` / `for-each-ref` / `cat-file` / `ls-tree` / `diff-tree` / `name-rev` / `merge-base` / `rev-list` / `diff-index` / `diff-files` / `check-ref-format` 这些**无副作用只读/查询子命令**全落入默认 confirm——与本会话修的「无害重定向误判」同类：纯读操作被误伤成需要确认。**修法**：把这些常用 git 只读/查询子命令补进 `GIT_READONLY`（`has_write_flag`/重定向守卫仍让带 write flag 的变体转 confirm，如 `git show --output=.git/config HEAD`）。
   - **审查方法论**：审查「哪些命令需要确认」时，应把会话用过的每一条命令喂给 `decide()` 对照预期——真正应 confirm 的是**执行任意代码的工具**（`uv`/`python`/`pytest`/`npm install`）与**写操作**（`git rm`/写文件重定向/rm）；无副作用的**纯读**（ls/cat/find/git 查询/丢弃式重定向）误伤成 confirm 即是 gap。经此审查，本会话误伤的仅 git 只读子命令一类，已修复。

6. **Crush 内置权限控制无法命令级（决定自定义 hook 是唯一路径）**：调研 charmbracelet/crush 源码与文档确认——内置 `permissions allow/deny` 只匹配**完整工具名**（`bash`/`view`/`mcp_xxx`），**不能**匹配子命令/参数/路径；内置 `safeCommands`/`bannedCommands` 列表**写死在源码、不可配置**，且是**前缀匹配 + 禁链式字符**（`` ; | && $( ` ``）实现，天然存在三类绕过（`echo hi && rm -rf`、`find -delete`、`git show --output=.git/config`）——正是本 guard 要补的洞。**结论**：命令级粒度只能靠自定义 PreToolUse hook；`guard.py` hook 已是官方推荐范式（文档明确示例自定义 `safe-bash.sh`）。Crush 扩展机制是 **hook（子进程，读 stdin/env，输出 allow/deny/none）非插件**，跟撰写语言无关。
7. **性能瓶颈在 Python 冷启动，不与启动形态相关（决定重写动机）**：实测（2026-09-03）单次调 `guard.py` 时延——venv python 直呼 ~98ms、`uv tool install` console script ~96ms、`uv run --directory` ~147ms（uv 多一层项目解析，更慢）、`uv run --project` 的 49ms 是假的（未真正执行，`program not found`）。瓶颈是 **Python 解释器启动 + `import bashlex`**（`import bashlex` 约 99ms、`import sys` 38ms、`import json` 40ms）。**结论**：`uv tool install` 出的 console script **不更快**，「装成环境工具」不解决延迟；若要把时延从 ~100ms 降到个位数 ms，唯一彻底方案是**重写**（Rust + tree-sitter-bash 已确认可用，启动 ~2ms、单 `.exe` 部署、无 venv/解析开销）。Go「插件」不成立——hook 与语言无关，Go 不会让集成更简单。
