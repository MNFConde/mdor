---
type: project_topic
status: archived
summary: "crush-guard（Crush bash 权限门 guard.py）基建化改造的评估结论：抽取独立仓库、Rust 重写可行性、fail-safe 默认确认设计。A（抽取）/B（Rust 重写）均未执行，仅评估沉淀。"
tags: [crush, bash-gate, permission, guard, retrofit, rust, fail-safe]
contains: [decision, lesson]
created: "2026-09-03"
updated: "2026-09-06"
related: [crush-guard-bash-gate]
authoring_mode: ai_generated
---
# crush-guard 基建化改造评估（抽取独立仓库 / Rust 重写 / fail-safe）
> **已退役（2026-09-06）**：crush-guard 已由独立 Rust 重写 [crush_tether](../../crush_tether) 取代（mdor 侧 `crush-guard/` 目录同日删除）。本文保留作历史知识；现行实现见 crush_tether 仓库 `doc/design.md`。

## 背景

本项目 `.crushrc` 用 PreToolUse hook 挂 `crush-guard/guard.py`（bashlex AST）对 bash 命令做三档分类（allow/confirm/deny）。用户提出：`crush-guard/` 能否抽成独立仓库、能否作环境工具被 `.crushrc` 调用、若重写是否更好、以及审计行数确认守卫失效时「默认确认而非默认通过」如何限制。本评估结论沉淀于此；**A（抽独立仓库）/ B（Rust 重写）均未执行**，仅记录分析与待办清单。

## 当前结论

### 结论 1：`crush-guard/` 是自包含的，可抽独立仓库
- 仅依赖 `bashlex`；无任何 mdor 代码/路径耦合（读 `CRUSH_PROJECT_DIR`/`CRUSH_TOOL_INPUT_COMMAND`/stdin 判项目边界），分类表全通用（cargo/go/git/npm…）。
- 抽出后真正要处理的耦合仅两处：`.crushrc` 挂载路径（`crush-guard/.venv/Scripts/python.exe` 硬编码 Windows）+ `.venv` 不入库（要 `uv sync` 现场生成）。
- 方案对比：**子模块**（`.crushrc` 相对路径正好落到子模块、少改）vs **全局工具**（`~/.config/crush/crushrc` 挂所有仓库生效、mdor 少一块基建、但 cairn/LOG 沉淀绑定 mdor 上下文需跟随迁移）。

### 结论 2：性能瓶颈在 Python 冷启动，与启动形态无关
- 实测单次时延：venv python ~98ms、`uv tool install` console script ~96ms、`uv run --directory` ~147ms。
- 瓶颈 = Python 解释器启动 + `import bashlex`（~99ms）。`uv run` 反而更慢。故「装成环境工具」不解决延迟。

### 结论 3：路由重写走 Rust，不用 Go「插件」
- Crush 扩展机制是 **hook**（子进程、读 stdin/env、输出 allow/deny/none），**非插件**，与语言无关——Go 不会让集成更简单。
- `tree-sitter-bash` 存在（已被 InnerWarden 用于同类 shell AST 安全分析），Rust 重写可行：启动 ~2ms、单 `.exe`、依赖编译期确定。`guard.py` 的 `classify()` 语义表（DESTRUCTIVE/READONLY/GIT_*/写 flag/路径逃逸/管道 sink）是纯语义可 1:1 平移；仅 AST 遍历层（`gather_commands`/`has_writing_redirect`/`reject_pipe`/`find_mutates`/`path_escapes`）需重写。

### 结论 4：Crush 内置权限控制做不到命令级
- `permissions allow/deny` 只匹配完整工具名；内置 `safeCommands`/`bannedCommands` 写死不可配且为前缀匹配，存在三类绕过（`echo hi && rm -rf`、`find -delete`、`git show --output=.git/config`）。
- 结论：命令级粒度只能靠自定义 hook；`guard.py` 已是官方推荐范式。

### 结论 5：fail-safe（默认确认而非默认通过）不能靠 `.crushrc`，必须靠 wrapper
- `.crushrc` 是装载期脚本，只管注册 hook，管不到每次调用时的工具可用性。
- hook 命令失败时 Crush 把非 0/2 退出当**非阻塞错误** → 工具照常放行 = **fail-open**，恰好要避免。
- 正确做法三层 fail-safe：guard.py 内部兜底（永不崩成非 0/2）+ wrapper 兜底（唤不起 guard 输出 `{"decision":"none"}`）+ `.crushrc` 指向 wrapper。`confirm` = 输出 `none`/空，非 `deny`。

## 决策日志

- **2026-09-03**：评估 crush-guard 基建化，方向为「A 抽取独立仓库 + B Rust 重写」，但**当前仅落实 C（cairn 沉淀），A/B 未执行**，待用户确认后按方案推进。

## 经验

- 守卫类工具最忌讳「静默失效」：任一环节崩了都该**保守降级为确认**，而不是放行。hook 失败 = fail-open 是安全上的反模式，必须用「输出 none」而非「依赖失败」来兜底。
- 语言选择影响的是**部署方式**（venv/uv vs 单二进制）与**性能**（冷启动 100ms vs 2ms），不影响判定逻辑，也不影响「能否被 Crush 集成」（hook 与语言无关）。

## 开放问题

1. **抽取方式**未定：子模块（保留 `crush-guard/` 路径、`.crushrc` 少改）vs 纯全局工具（mdor 删目录、所有仓库共用、需迁 cairn/LOG）。
2. **是否重写**未定：现在就重写 Rust，还是先保持 Python（依赖 `uv sync` + venv）、重写留作独立里程碑。
3. `pyproject.toml` 本会话实验性新增 `[project.scripts] crush-guard = "guard:main"`（配合全局 `uv tool install`）：保留或回滚待定。
