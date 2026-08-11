# AGENTS.md

mdor —— 移动端 mdBook 离线阅读器（Android · Rust + Dioxus）。当前为规划期：`src/` 只有 hello world，仓库主体是 `doc/` 规划文档。

## 协作约定

- 与用户交流一律使用中文
- 指令若与仓库文档（`doc/`）或既有约定不符，先指出冲突点、说明取舍，再执行
- 项目状态发生变化（如 M1 落地、workspace 建立、工具链变更）时，同步更新本文件对应的状态描述，避免误导后续会话
- 提交格式规范见 `@.agents/rules/commit.md`，仅在准备 commit 时读取

## 文档即源码

- `doc/README.md` 是文档地图；project/decisions/diff/env 四篇为规划稿，随实现持续更新
- `doc/` 只收与项目架构、实现相关的文档；提交规范、工作流等约束/约定不入 doc/（提交格式放 `.agents/rules/`）；归属拿不准时先确认再放
- 引用一律 markdown 链接（跨文件 `[x](file.md#锚点)` / 站内 `[x](#锚点)`），不用裸编号
- `decisions.md` 的 ADR 编号 `D-xx` 连续递增，标题不含 `、（）+` 等标点
- 改完 doc 必须跑锚点一致性检查，退出码非 0 = 有不匹配：
  `uv run --directory script check-links.py`

## 文档存档（结构调整/精简前）

- 对 `doc/` 做结构重组、精简、整篇重写前，先整体快照：新建 `doc/archive_doc_v{num}/`，`num` = 现有存档最大编号 + 1（当前 `archive_doc_v1` → 下个 `v2`）
- 快照 = 复制 `doc/` 顶层全部文件（README、四篇文档、mdor.c4），**不含任何 `archive_doc_v*` 存档目录**，复制完再在顶层调整
- 旧存档永不删除；`archive_doc_v1/` 为历史归档，勿改

## script/ 目录约定

- 需要持久化、本地运行的脚本一律放 `script/`，由 uv 管理环境（根 pyproject.toml，共享单环境）
- 简单脚本：单文件 `script/{name}.py`，本身即入口
- 复杂脚本：主体放 `script/{name}/`（作为包，含 `__init__.py`），入口 `script/{name}.py` 只做薄封装（import 子包 + 触发）
- 统一触发：`uv run --directory script {name}.py`
- 当前共享单环境；若日后多个复杂脚本依赖互相干扰，将冲突脚本拆成独立 uv 项目隔离

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
- 组件关系改动时更新 `doc/mdor.c4`（LikeC4 源），再生成：`likec4 gen mermaid doc -o <输出>`
