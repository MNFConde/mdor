# Project Cairn 日志

本文件按时间倒序记录实质进展——最新条目在最上、紧跟本行。每条保持精简——摘要 + 指针；结论沉淀进 `cairn/<主题>.md`。

## 2026-08-18 · doc/ L2 扩「确定信息单源」+ README 补抽象层级说明

- doc/AGENTS.md L2 新增「确定信息同样单源」bullet：具体易变的确定信息（依赖版本、当前方案）只一处落值、别处链接引用防漏改；层级关系链接 README。
- README「文档间引用关系」补抽象层级说明（规范 > 论证 > 差异 > 操作；上层稳定、下层易变）——四层模型唯一源在 README，L2 不复制。
- 审查确认：decisions 为论证层（非纯具体）、diff 含背景知识，维持既有四层模型不重构。
- 门禁：`check-links.py` 通过（222 anchors）。

## 2026-08-18 · 全量 cargo install 统一 --locked + CI 补 dx 钉版

- env.md §4.1 三命令（cargo-outdated / cargo-edit / cargo-audit）补 `--locked`；根 AGENTS.md 门禁行的 cargo-audit 安装同步补；project.md §12.3 CI 缓存行 dx 安装命令改链接 env.md §2.3、不内联版本号（版本号单源 env.md §1，避免升级漏改）。
- env.md §1「版本钉版边界」加澄清条：`--locked`（依赖图可复现）≠ `--version`（钉工具自身版本），「不钉版本」只指后者，`--locked` 对全部 cargo install 一律启用。
- 核查：全仓库 27 处 cargo install，archive_doc_v* 存档不改，其余已全部带 `--locked`。
- 门禁：`check-links.py` 通过（219 anchors）。

## 2026-08-18 · doc/ 版本钉版边界：不钉清单 + 通用规则 + crates 钉版时机

- env.md §1 新增「版本钉版边界」：不钉版本清单（cargo-audit/outdated/edit、WebView2、随 toolchain 自动锁定项）；通用规则（钉 = 影响构建/运行行为或与项目库配对，否则不钉）；crates 依赖版本不在文档确认、M1 建 workspace 时于根 workspace.dependencies 钉下，dioxus 库跟随 dx（0.7.10 ↔ 0.7.x）。
- 门禁：`check-links.py` 通过（219 anchors）。

## 2026-08-18 · doc/ 版本号落点约定 + dx 钉 0.7.10 + --locked 机制说明

- 新增约定（doc/AGENTS.md「版本号事实落点约定」）：版本号事实源 = env.md §1 矩阵；安装命令内联具体号是命令参数、与矩阵同一事实；升级两处同步改。
- env.md：dx 钉 0.7.10（§1 矩阵 + §2.3 `cargo install dioxus-cli --locked --version 0.7.10`）；§4.1 新增「版本锁定机制」（--locked 含义 / 加不加区别 / 与项目 Cargo.lock 区分）。
- 门禁：`check-links.py` 通过（218 anchors）。

## 2026-08-16 · doc/ A 类单源化 + 引用方向原则

- 版本钉版单源化到 `env.md` §1：project §12.3 CI 表改「钉版」+ 指针、§12 文件树注释指针化、diff §6.4 指针化。
- fsync 分层映射单源化到 project §6.7：diff §7.2 删完整表，保留平台结论 + 链接。
- 版本清理策略单源化到 §7.4：§11「版本历史的存储占用」行改链接。
- doc/AGENTS.md「L2 单详述源」追加引用方向原则（单一事实源 + 反向链接 + 互引不算环）。
- 门禁：`check-links.py` 通过（211 anchors）。

## 2026-08-16 · doc/ 决策状态单表化 + §11 补孤儿清理行

- project.md §12.1「决策摘要表」删除（decisions 决策总览的复述，违反 L2 单详述源），改为一行指针链接 `decisions.md#决策总览`；ADR 状态单一事实源收敛到 decisions。
- project.md §11 新增「孤儿 `books/<id>/` 目录清理」行（非 ADR 的实现待定项，只进 §11 不进 decisions）。
- 门禁：`check-links.py` 通过（207 anchors）。

## 2026-08-16 · doc/ 一致性审计与修正

- 双轨检索（MCP 覆盖矩阵 + 全文核读）审查 doc/（排除 `archive_doc_v*/`）：结构自洽、无存档信息丢失（v1 diff §9 WebView 清单全量迁入 §10.1 且 B7 已闭环）。
- 修正：project.md §12.3「无跨平台交叉编译需求」→「无引入 Zig 的 C 交叉编译需求」；§9 `site/` 注释「webview 直接读文件」→「经本地 http 服务读取」；§6.9 httpmock 引用 §12.2 → §10 M2/M4；hash 术语对齐 D-08（§7.3/§7.2/§4.1/§5）；env.md §2.5 示例 android24 → android30。
- 决策：`.bak` 备份机制否决（原子写已消除半写态），删除 project.md §6.7 对应行，论证记入 decisions.md D-03、diff.md §7.2 留短块链接。
- 门禁：`check-links.py` 通过（218 anchors）。

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
