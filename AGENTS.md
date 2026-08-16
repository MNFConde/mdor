# AGENTS.md

mdor —— 移动端 mdBook 离线阅读器（Android · Rust + Dioxus）。当前为规划期：`src/` 只有 hello world，仓库主体是 `doc/` 规划文档。

> 本项目使用 Project Cairn 组织项目知识：本文件是规则与导航入口，`cairn/` 是项目知识/状态层。
> 本机装有 project-cairn skill 且仓库存在 `cairn/` 时，以下 Cairn 相关规则（阅读顺序/文档职责/冲突仲裁/知识库消费反射/知识沉淀规则）生效；否则视为不适用，跳过。

## 初始化配置

- 毕业 provider：Obsidian (vault: SecondBrain)
- 知识库索引：Obsidian → Cairn/INDEX.md
- 毕业目标：Obsidian → Cairn

## 进入项目后的阅读顺序

1. 先读本文件（AGENTS.md）。
2. 若存在 `cairn/ROADMAP.md`，阅读路线图、当前焦点与开放问题。
3. 阅读 `cairn/LOG.md` 最近条目（最新在上）了解近期进展与关键决策。
4. 按需阅读相关 `cairn/` 知识专题文档。

## 文档职责

| 文件 | 职责 | 维护 |
|---|---|---|
| `AGENTS.md`（根） | 规则与导航 | 极少改动，≤ 60 行 |
| `cairn/ROADMAP.md` | 路线图与进展 | 就地更新，保持精简 |
| `cairn/LOG.md` | 时间序日志 | 顶部新增条目（最新在前），每条 ≤ 20 行，摘要 + 指针 |
| `cairn/<主题>.md` | 知识专题文档（当前真相） | 就地更新；坑写入正文小节，经 `contains` 标记；修订留 LOG 指针 |
| `cairn/Reference/` | 外部原始输入 | 按需创建；只增不改 |
| `cairn/Cited.md` | 知识库引用清单 | 仅指针，绝不复制原文 |

> 其余内容只在有具体信号时才创建（需记录决策、坑已解决、目标超出单次会话）——不预建空壳。工程资产（代码/配置/规范消费的合同）不归本系统管理，留在代码树，不入 `cairn/`。

## 冲突仲裁规则

- 优先级：**知识专题文档 > LOG 历史**；规则级冲突由本文件裁定。
- 业务/设计结论以 `cairn/` 知识专题文档的最新记录为准，而非更早的 LOG 条目。

## 知识库消费反射

- 在开展可复用内核——其产出或依赖的任何结论——够格毕业的工作前，先查知识库索引（Obsidian → Cairn/INDEX.md）；只有实际影响产出的笔记才添加 `cairn/Cited.md` 条目（仅指针，不复制正文）。

## 文档协作规则

- 改动前判断用户要「讨论/建议」还是「直接改文档」；说「先看看/先评估」时先给分析，别直接重写正式文档。
- 纠正过往判断时追加更正说明，不静默覆盖。
- 未经确认的判断不写成既成事实。

## 知识沉淀规则

- 每有实质进展，在 `cairn/LOG.md` 顶部加一条（摘要 + 指针）；结论沉淀进 `cairn/` 知识专题文档。
- **完成回复门禁**：任何完成断言前——包括但不限于工作完成/已实现、定稿、已更新、已同步、已验证或测试通过、问题已修复/已解决、交付可用、声称工作已结束及语义等同措辞——先执行 `references/maintenance.md` 中的 Cairn 检查点；仅更新其触发矩阵要求的记录、验证后，再回复。明确的只读/不改动请求禁止 Cairn 写入。
- 跨项目可复用经验经毕业机制沉淀到知识库（Obsidian → Cairn）。

## 协作约定

- 与用户交流一律使用中文
- 指令若与仓库文档（`doc/`）或既有约定不符，先指出冲突点、说明取舍，再执行
- 项目状态发生变化（如 M1 落地、workspace 建立、工具链变更）时，同步更新本文件对应的状态描述，避免误导后续会话
- 提交格式规范见 `@.agents/rules/commit.md`，仅在准备 commit 时读取
- 目录级专属约束见 `doc/AGENTS.md`（doc/ 写作约定）与 `script/AGENTS.md`（script/ 目录约定），读取对应目录下文件时自动生效

## M0 桌面构建（唯一当前可用目标）

- `cargo run`（骨架）；桌面 UI：`dx serve --platform desktop`（需 dioxus-cli 0.7.x + Win11 WebView2）
- Windows 走 MSVC：`cargo build` 报「找不到 link.exe」= VS Build Tools（钉版 v14.50）问题，与代码无关
- Android 侧 M6 前不碰：本机未装 android targets / JDK / SDK / NDK

## 质量门禁（计划进 CI core-quality，本地保持一致）

- `cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test` → `cargo audit`
- `cargo audit`（需 `cargo install cargo-audit`）是硬性要求（D-12），退出码非 0 即失败
- 单测重点在 mdor-core（平台无关库），桌面直接 `cargo test -p mdor-core`

## 工具链钉版

- `rust-toolchain.toml` 钉 1.97.1（minimal profile）；本地 MSVC 钉 14.50；勿随手升级
- 版本约束只钉在根 `[workspace.dependencies]` 一处（规划中）；升级后必须 `cargo test` + `cargo audit`

## 架构约定（改代码前先读 project.md）

- 目标结构：Cargo workspace = `mdor-core`（纯 Rust、平台无关，桌面直测）+ `mdor-app`（Dioxus UI 壳）
- 业务逻辑/渲染管线全在 core，app 层只做「拿 HTML 注入 + 交互」，不掺业务编排
- project.md §3.2 C4 图由 `doc/mdor.c4` 生成：`likec4 gen mermaid . -o tmp`（工作区必须是仓库根、即 likec4.config.json 所在目录；否则 `**/archive_doc_v*/**` 排除不生效，`doc/archive_doc_v*/` 里的 mdor.c4 会被扫进来报重复定义）
- §3.2 节点注记（如 `(v1 默认)` / `(M5 开放)`）必须写进组件 name：likec4 的 mermaid 标签只取 name（title/description/technology 不进标签），组件名规范为「类名 + (注记)」，id 与代码类名一致
