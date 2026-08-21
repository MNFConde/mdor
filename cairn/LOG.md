# Project Cairn 日志

本文件按时间倒序记录实质进展——最新条目在最上、紧跟本行。每条保持精简——摘要 + 指针；结论沉淀进 `cairn/<主题>.md`。

## 2026-08-21 · 小节标题统一为中文（zh-glossary）+ 毕业候选盘点

- 14 篇知识专题文档的英文小节标题按 zh-glossary 固定词统一：Lessons→教训、Current Conclusions→当前结论、Practice Guide→实践指南、Open Questions→开放问题；文件名保持英文 slug（skill 规则：`language: zh` 只约束正文与标题，不改文件名）。
- 过程坑：PowerShell 5.1 `Get-Content -Raw` 对无 BOM UTF-8 按 ANSI/GBK 误读，回写造成字节级损坏；从 `experiment/cairn-track` 分支（cairn/ 已跟踪、版本仅差一晚）按 blob 字节恢复后以 .NET UTF-8 无 BOM I/O 重做。已沉淀 [powershell-encoding.md](powershell-encoding.md)。
- 毕业候选盘点完成（未写入知识库）：Tier A 四篇（cargo-audit-behavior / windows-scripts / metadata-write-reliability / cargo-config-toml）、Tier B 六篇（gix-windows-pitfalls 仅已验证部分 / windows-msvc-toolchain / android-cross-compile-rust / local-resource-channel / mcp-doc-retrieval / dioxus-cli-0.7-config）；Tier C 四篇不标记。毕业时需重跑 obsidian-preflight + INDEX 查重。
- 更正：恢复源实为 master 自身历史——cairn/ 与 `.cairn/config.yaml` 均已被 master 跟踪（自 `68398e2` 起），并非依赖 `experiment/cairn-track` 分支；该分支与 master 完全同步、无独有内容（保留不删）。

## 2026-08-20 · manifest 收敛 workspace 层 + D-14 单人仓库协作决策定稿

- `build(workspace)`：根加 `[workspace.package]`（version/edition/rust-version=1.97.1 统一）+ `[workspace.lints]`（unsafe_code=deny）；两成员 `version/edition/rust-version` 改 `workspace = true` 继承 + `publish = false`。门禁（fmt/clippy/test）全绿。
- `docs`：单人维护 + 偶尔外部贡献的协作模式定稿——维护者直推 master 保线性、外部 PR 一律 squash 合入；决策留痕 [decisions.md D-14](doc/decisions.md#d-14-单人仓库协作与外部贡献流程)，操作规范单源落仓库根 `CONTRIBUTING.md`（协作流程属约定不入 doc/，仅留痕不展开），AGENTS.md 补协作模式指针。
- 门禁：`check-links.py` 通过（231 anchors）。

## 2026-08-20 · 便携 Android 工具链方案定稿（dev/ + 提交相对路径 .cargo 配置）

- 评估 Podman6（仅 Linux VM、无 Windows 容器计划，GH #27842）与 Hyper-V Windows VM（过重）后，定「本机便携隔离」：Android SDK/NDK/JDK 全部 zip 便携装 `<repo>\dev\`（gitignore，M6 执行）；不写任何持久环境变量。
- 提交层落地：`.cargo/config.toml`（`relative=true` 的 `[env] ANDROID_HOME` + `include` 本地覆盖，无 force = 全局优先）、`.cargo/config.local.toml.example`（版本化 NDK + Windows linker 模板）、`dev/dev-env.ps1`（全局优先、便携兜底，dx/Gradle 用）、`.gitignore` 加 `/dev/*` + `!dev-env.ps1` + `config.local.toml`。
- 归档 `cairn/cargo-config-toml.md`：config 层级、`[env]` relative/force 语义、提交模式与坑（linker OS 相关、版本号不进提交文件、dx/Gradle 不读 cargo config）。
- JDK 定 Temurin 21（zip → `dev\jdk`），`dev-env.ps1` 保留 Scoop/既有 `JAVA_HOME` 兜底。
- 文档同步：env.md §1/§2.4/§2.5/新增 §2.6/§5/§6/§7/§8、project.md §12 文件树、根 AGENTS.md 一行。
- 门禁：`check-links.py` 通过。

## 2026-08-19 · M1 workspace 骨架落地 + 书架弹窗验收

- M1 workspace 重构落地：根 Cargo.toml 补 `[workspace.dependencies]`（serde 1.0.229 / serde_json 1.0.151 / thiserror 2.0.20 / dioxus 0.7.10，`cargo info` 验证均为当时稳定版）；删根 src/ hello world；建 `crates/mdor-core`（lib 空壳）+ `crates/mdor-app`（bin 骨架，中文书架占位）。
- 验收：门禁全绿（test / fmt / clippy / audit exit=0）；`dx build --platform desktop` 产物 exe 弹窗（标题 Dioxus App，MainWindowHandle 非零）——合并勾掉 plan.todo 14 + 20/21/22/48/49。
- 实证沉淀 `cairn/dioxus-cli-0.7-config.md`：Dioxus.toml 0.7 用 name + default_platform（非 0.6 app_id）、dx serve/check 无 --project 须 cd 进 member、dx build 产物在 `target/dx/<crate>/debug/windows/app`、name 不映射窗口标题。
- 过程实证：开工时仓库即处 workspace 中间态（Cargo.toml members 已改、`crates/` 未建，不可构建）；agent shell 无法驻留常驻 GUI 进程，弹窗验收改用 dx build + exe；PowerShell 下 dx 的红色 NativeCommandError 为 stderr 合并噪音，以 `$LASTEXITCODE` 判结果。
- audit 注记：dioxus-desktop 引入 14 条 unmaintained/unsound 级 advisory（gtk 系 Linux 侧 + rand 0.7.3 等传递依赖），退出码 0 不阻塞（D-12 判据）。
- 文档同步：plan.todo 勾选；根 AGENTS.md 状态/命令/单测/钉版四处更新；env.md §3 补工作目录；ROADMAP M0 勾掉、当前焦点转 M1。
- 门禁：`check-links.py` 通过。

## 2026-08-19 · AGENTS.md 状态修正 + resolver 纳入与归档

- AGENTS.md：android targets 状态修正（toolchain 已带 rust-std-{aarch64,x86_64}-linux-android，JDK/SDK/NDK 未装）；质量门禁补单 crate 骨架期命令（`cargo test`，workspace 落地后切 `-p mdor-core`）。
- resolver 纳入：plan.todo M1 workspace 重构补 `resolver = "3"`；env.md §4.1 补 virtual workspace 须显式设 resolver（全局项、member 写无效）。
- 归档 `cairn/cargo-workspace-resolver.md`：resolver 1/2/3 版本差异、virtual workspace 无 edition 必须显式的原因、mdor 取值建议（Cargo book 整理）。
- 门禁：`check-links.py` 通过。

## 2026-08-19 · 质量门禁首跑 + audit 静默成功实证 + 版本落根工作流定稿

- 质量门禁首跑四门全绿（fmt --check / clippy -D warnings / cargo test / cargo audit `exit=0`）；plan.todo 勾选，符号规范补 `@done(yy-mm-dd HH:MM)` 标注格式。
- 实证：cargo-audit 0.22.2 无漏洞时静默成功、不打印结语，**退出码 0 才是判据**（presenter.rs 仅在 found 时打输出）——沉淀 `cairn/cargo-audit-behavior.md`。
- env.md §4.1「版本约束落根工作流」：定稿 `cargo add` 不能钉根表（#11527 / #16797）、3 步流程、workspace 根不带 `-p` 直接报错、两个 Cargo 语义约束；decisions D-12 补依据（rustls/dioxus 根钉版为硬约束，推翻也绕不开）。
- 门禁：`check-links.py` 通过。

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
