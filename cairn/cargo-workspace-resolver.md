---
type: project_topic
status: active
summary: "Cargo resolver 版本（1/2/3）与 virtual workspace 必须显式设 resolver 的原因；mdor 取值建议"
tags: [cairn, cargo, resolver, workspace]
contains: [lesson, reference]
created: "2026-08-19"
updated: "2026-08-19"
related: [env.md, decisions.md]
authoring_mode: ai_generated
---
# Cargo resolver 与 virtual workspace 显式声明

## 背景

M1 workspace 重构把根 `Cargo.toml` 变为 virtual workspace（删根 `src/`、根只剩 `[workspace]`、无 `[package]`）。此时 Cargo 无法从 `edition` 推断 resolver 版本，必须在根 `[workspace]` 显式声明。本文档整理自 Cargo book「Dependency Resolution / Workspaces」章节。

## 参考：resolver 版本

| resolver | 含义 | 默认适用 |
|---|---|---|
| `"1"` | 旧算法：feature 无条件全局合并 | edition 2018 及以前 |
| `"2"` | 按目标平台拆分 feature 合并：`cfg(windows)` 专属依赖的 feature 不串到别的平台；build/dev 依赖的 feature 不污染普通依赖（Cargo 1.51+） | edition 2021 |
| `"3"` | 同 `"2"`，另将 `resolver.incompatible-rust-versions` 默认从 `allow` 改为 `fallback`（解析时优先选与项目 Rust 版本兼容的依赖）（Cargo 1.84+） | edition 2024 |

## Lessons

1. **resolver 是全局项**：只取 workspace 顶层（根 `[workspace]` 或根包 `[package]`）声明的值，member 里写无效。
2. **普通 package 由 `edition` 推断**（2018→1 / 2021→2 / 2024→3），一般无需显式；**virtual workspace 没有 edition，必须显式**，否则 Cargo 直接报错。
3. **mdor 建议 `resolver = "3"`**：crate 均 edition 2024、工具链 1.97.1 满足（需 1.84+），取值与项目其余配置最一致。
4. **members 声明与 crates/ 落地必须原子**：workspace `members` 指向不存在的 manifest 目录时，cargo 加载直接失败（仓库处于不可构建中间态）；根 Cargo.toml 的 members 改动与对应 `crates/` 目录须同次提交，避免提交出中间态（M1 重构开工时即遇此态）。

## Current Conclusions

- M1 根 `[workspace]` 写 `resolver = "3"`；规范落点 `env.md` §4.1「版本约束落根工作流」，任务落点 `plan.todo` M1 workspace 重构条目。