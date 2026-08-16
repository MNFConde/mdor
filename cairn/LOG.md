# Project Cairn 日志

本文件按时间倒序记录实质进展——最新条目在最上、紧跟本行。每条保持精简——摘要 + 指针；结论沉淀进 `cairn/<主题>.md`。

## 2026-08-16 · doc→cairn 第二批沉淀（4 新 2 扩）+ Cairn 规则拆分

- 新建专题：[webview-host-differences.md](webview-host-differences.md)（diff §2.1–2.4 宿主/线程/内核/DevTools，纯落空）、[data-directory-platform.md](data-directory-platform.md)（diff §3 + D-13）、[service-orchestration.md](service-orchestration.md)（D-07：薄门面 + 命令串行 = 单写者、为何不引全局中介者）、[mcp-doc-retrieval.md](mcp-doc-retrieval.md)（MCP 检索实证：archive 稀释/表格召回弱/形式取信陷阱）。
- 扩充：[gix-windows-pitfalls.md](gix-windows-pitfalls.md)（+§4.3 保留设备名/Path 分隔符/fixtures 大小写）、[android-cross-compile-rust.md](android-cross-compile-rust.md)（+Zig 评估不采纳、dx/dioxus 同步升、旧版 dx 链接 bug、min_sdk=30、cmdline-tools 结构）。
- 规则层：Cairn 规则集中到新 `cairn/AGENTS.md`（根 `AGENTS.md` 瘦身至 ≤60 行 + 新增「检索约定（MCP）」节）；三次模板偏离固化为项目级约束（不用 Claude Code / cairn 私有不分发 / 规则集中子文件）。
- 判定：D-01/D-12/D-10 偏规范留 doc 不沉淀；测试策略已进 AGENTS.md 不重复。

## 2026-08-16 · 从 doc/ 提炼知识，新建 5 篇专题文档

- 对比 `doc/` 及 `archive_doc_v1/`、`archive_doc_v2/`：存档是同一批文档的更早状态，内容 ⊆ 当前 doc/，无被丢弃知识（v1 diff.md 详细评估已收口进 decisions.md D-11）。
- 按「经验/坑为主 + 决策摘要」约定新建专题（规范细节一律链接回 doc/）：[windows-msvc-toolchain.md](windows-msvc-toolchain.md)、[gix-windows-pitfalls.md](gix-windows-pitfalls.md)、[android-cross-compile-rust.md](android-cross-compile-rust.md)、[local-resource-channel.md](local-resource-channel.md)、[metadata-write-reliability.md](metadata-write-reliability.md)。
- 涉及 D-08/D-09/D-11/D-04/D-05/D-06/D-02/D-03 与 env.md §2.1、diff.md §1/§2/§4/§6、project.md §6.5/§6.7/§6.8。

## 2026-08-16 · 实验分支：放开 cairn 分发

- 建分支 `experiment/cairn-track`（commit 3dffd8e）：`.gitignore` 移除 cairn/ 与 .cairn/ 忽略规则，`git_policy` 翻转为 `track`，cairn 知识层与配置随仓库分发。
- master 保持 cairn 私有策略不变；实验评估后决定合并或丢弃该分支。
- 详情：见 `.cairn/config.yaml`。

## 2026-08-16 · Cairn 调整：守卫声明 + `.cairn/` 忽略 + CLAUDE.md 删除 + 坑沉淀

- AGENTS.md：Cairn 引言加守卫声明（无 skill 或无 `cairn/` 时 Cairn 规则不适用），并删除 CLAUDE.md 相关引用。
- `.cairn/` 与 `cairn/` 一并忽略：Cairn 层完全私有不分发。对 Cairn「永不忽略 `.cairn/config.yaml`」默认的刻意偏离，取舍：配置仅 5 行、无密，本地文件仍在磁盘，功能不受影响。
- `CLAUDE.md` 删除：不用 Claude Code（对 Cairn 模板的偏离，AGENTS.md 引用已同步清理）。
- 坑沉淀：Windows 下运行 Cairn 脚本的调用姿势 → [windows-scripts.md](windows-scripts.md)。
- 详情：见 `AGENTS.md` 与 `.gitignore`。

## 2026-08-16 · Project Cairn 初始化

- 初始化 Project Cairn 结构（语言 zh、git_policy=ignore、migration_mode=inventory_only）。
- 毕业目标：Obsidian（vault: SecondBrain → Cairn/INDEX.md），预检通过。
- doc/ 保持架构规范不动，cairn/ 专注经验沉淀，链接互补（doc 规范 ↔ cairn 经验）。
- 详情：见 `AGENTS.md` 与 `.cairn/config.yaml`。
