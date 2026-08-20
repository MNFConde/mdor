# AGENTS.md

mdor —— 移动端 mdBook 离线阅读器（Android · Rust + Dioxus）。当前为骨架期：Cargo workspace（`mdor-core` + `mdor-app`）已建，core 业务模块按 plan.todo M1 推进；仓库另含 `doc/` 规划文档。

> 本项目使用 Project Cairn 组织项目知识：Cairn 全套规则（初始化配置/阅读顺序/文档职责/冲突仲裁/知识库消费反射/知识沉淀规则）见 `cairn/AGENTS.md`。
> 本机装有 project-cairn skill 且仓库存在 `cairn/` 时生效；否则视为不适用，跳过。

## 阅读顺序

1. 先读本文件（AGENTS.md）。
2. 若存在 `cairn/AGENTS.md`，先读其中 Cairn 规则（含 ROADMAP / LOG 的阅读顺序）。
3. 按需阅读相关 `cairn/` 知识专题文档与 `doc/` 规划文档。

## 文档协作规则

- 改动前判断用户要「讨论/建议」还是「直接改文档」；说「先看看/先评估」时先给分析，别直接重写正式文档。
- 纠正过往判断时追加更正说明，不静默覆盖。
- 未经确认的判断不写成既成事实。

## 协作约定

- 与用户交流一律使用中文
- 指令若与仓库文档（`doc/`）或既有约定不符，先指出冲突点、说明取舍，再执行
- 项目状态发生变化（如 M1 落地、workspace 建立、工具链变更）时，同步更新本文件对应的状态描述，避免误导后续会话
- 提交格式规范见 `@.agents/rules/commit.md`，仅在准备 commit 时读取
- 协作模式（维护者直推 master 保线性 + 外部 PR squash 合入）见仓库根 `CONTRIBUTING.md` / [doc/decisions.md D-14](doc/decisions.md#d-14-单人仓库协作与外部贡献流程)
- 目录级专属约束见 `doc/AGENTS.md`（doc/ 写作约定）与 `script/AGENTS.md`（script/ 目录约定），读取对应目录下文件时自动生效

## 检索约定（MCP）

- doc→cairn 提炼用 MCP 检索时：先确认索引新鲜度；显式排除 `archive_doc_v*/`（存档 ⊆ 当前 doc，避免旧副本稀释覆盖判断）
- MCP 对表格型（env.md 故障排查表/§8 记录）与长篇论证（decisions.md rationale）召回弱——须对 env.md / decisions.md / project.md 补手工核读；MCP 结果只作覆盖矩阵与排序，不替代全文召回
- 坑与实证见 `cairn/mcp-doc-retrieval.md`

## M0 桌面构建（唯一当前可用目标）

- `cargo run -p mdor-app`（workspace bin）；桌面 UI：`dx serve --platform desktop`（需在 `crates/mdor-app/` 目录运行，Dioxus.toml 所在处；需 dioxus-cli 0.7.x + Win11 WebView2）
- Windows 走 MSVC：`cargo build` 报「找不到 link.exe」= VS Build Tools（钉版 v14.50）问题，与代码无关
- Android 侧 M6 前不碰：本机未装 JDK / SDK / NDK；toolchain 已带 android targets（rust-std-{aarch64,x86_64}-linux-android，M6 打包时直接用）

## 质量门禁（计划进 CI core-quality，本地保持一致）

- `cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test` → `cargo audit`
- `cargo audit`（需 `cargo install cargo-audit --locked`）是硬性要求（D-12），退出码非 0 即失败
- 单测重点在 mdor-core（平台无关库），桌面直接 `cargo test -p mdor-core`

## 工具链钉版

- `rust-toolchain.toml` 钉 1.97.1（minimal profile）；本地 MSVC 钉 14.50；勿随手升级
- 版本约束只钉在根 `[workspace.dependencies]` 一处；升级后必须 `cargo test` + `cargo audit`
- Android 工具链便携隔离：`.cargo/config.toml`（已提交，相对路径 `[env]` + include 本地覆盖）注入环境；`dev/` 工具树与 `config.local.toml` gitignored；机制见 [doc/env.md §2.6](doc/env.md#26-环境注入机制cargo-配置与-dev-envps1)（M6 生效）

## 架构约定（改代码前先读 project.md）

- 目标结构：Cargo workspace = `mdor-core`（纯 Rust、平台无关，桌面直测）+ `mdor-app`（Dioxus UI 壳）
- 业务逻辑/渲染管线全在 core，app 层只做「拿 HTML 注入 + 交互」，不掺业务编排
- project.md §3.2 C4 图由 `doc/mdor.c4` 生成：`likec4 gen mermaid . -o tmp`（工作区必须是仓库根、即 likec4.config.json 所在目录；否则 `**/archive_doc_v*/**` 排除不生效，`doc/archive_doc_v*/` 里的 mdor.c4 会被扫进来报重复定义）
- §3.2 节点注记（如 `(v1 默认)` / `(M5 开放)`）必须写进组件 name：likec4 的 mermaid 标签只取 name（title/description/technology 不进标签），组件名规范为「类名 + (注记)」，id 与代码类名一致
