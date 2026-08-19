---
type: project_topic
status: active
summary: "dioxus-cli 0.7 的 Dioxus.toml 格式（name + default_platform，区别于 0.6 的 app_id）、workspace 中 dx serve 必须进 member 目录运行的原因、dx build 产物路径与窗口标题行为（M1 实测）"
tags: [cairn, dioxus, dx, dioxus-cli, workspace]
contains: [lesson, reference]
created: "2026-08-19"
updated: "2026-08-19"
related: [env.md, project.md]
authoring_mode: ai_generated
---
# dioxus-cli 0.7 配置与 workspace 运行方式

## 背景

M1 workspace 重构把 `mdor-app`（Dioxus 桌面 bin）放进 Cargo workspace 成员 `crates/mdor-app/`。骨架期实测 dx 0.7.10 的配置格式、运行目录与产物路径，结论供后续维护与版本升级参考。

## Dioxus.toml 0.7 格式

- 必填字段（`dx config init` 或最小模板）：`[application] name` + `default_platform`（web / desktop）。
- **与 0.6 不同**：0.6 用 `app_id`，0.7 用 `name`；0.7 的 bundle/权限等扩展字段走 `[bundle]` / `[permissions]` 等段（见 dioxus 仓库 `notes/architecture/12-MANIFEST-SYSTEM.md`）。
- mdor-app 骨架（无静态资源）最小可用：

  ```toml
  [application]
  name = "mdor"
  default_platform = "desktop"
  ```

- `public_dir`（静态目录复制）可选，未配 `public/` 目录也不报错；M1 骨架无 `assets/`。

## workspace 中运行 dx serve

- **dx serve 无 `--project` / `-p` 选项**（0.7.10 `dx serve --help` 实测），从当前工作目录查找 Dioxus.toml；workspace 中必须 `cd` 进成员目录再跑（Dioxus.toml 所在处），从仓库根跑会找不到配置。
- `dx check`（项目体检）同样需要从成员目录运行。
- 项目可纯 bin（仅 `src/main.rs` + `dioxus::launch(App)`），官方 barebones 模板与 tutorial 即此结构，无需 lib.rs。

## 构建产物与窗口行为

- `dx build --platform desktop` 产物在 **workspace 根 `target/dx/<crate>/debug/windows/app/<crate>.exe`**（非成员目录内）。
- 直接运行该 exe 即弹桌面窗口（无需 dx serve）；骨架未设窗口标题时默认为 **"Dioxus App"**——`[application] name` 不直接映射为桌面窗口标题（需 dioxus-desktop 窗口配置显式设置）。

## Lessons

1. **dx 版本升级时核对 Dioxus.toml 字段**：0.6 `app_id` → 0.7 `name` + `default_platform`，大版本迁移必须先看官方模板/`dx config init`。
2. **workspace member 必须 cd 进入后运行 dx 命令**：dx serve/check 从 CWD 找配置，无 project 定位参数。
3. **`dx build` 可作自动化验收**：dx serve 是常驻交互进程，在 agent/自动化 shell 中无法驻留（opencode bash 工具会清理其启动的进程树，后台启动同样被杀），故骨架期验收改用 `dx build --platform desktop` 一次编译产出 exe、直接运行弹窗验证。**人工终端下仍用 `dx serve`**（常驻开发），两者不冲突。
4. **crates 依赖版本统一走根 `[workspace.dependencies]`**：member 侧 `dioxus = { workspace = true, features = ["desktop"] }`，desktop feature 在 member 追加（features additive，见 env.md §4.1）。
5. **dx 日志走 stderr，PowerShell 输出判读用 `$LASTEXITCODE`**：dx 的 tracing 日志（INFO/warning）写 stderr；PowerShell 里 `dx ... 2>&1` 会把正常日志包装成红色 ErrorRecord 块（`NativeCommandError`），业务正常时也显示成「报错外观」。判据看退出码 `$LASTEXITCODE`（0=成功），不看输出颜色——与 cargo-audit 的静默成功判据同理。

## Current Conclusions

- mdor-app 保持纯 bin + Dioxus.toml（`name` + `default_platform = "desktop"`）；dx serve/check 一律在 `crates/mdor-app/` 运行。
- 规范落点：`doc/env.md` §3（验收命令 + 工作目录）；任务落点：`plan.todo` M1（mdor-app UI 壳，已完成）。