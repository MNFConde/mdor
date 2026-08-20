# 贡献指南（CONTRIBUTING）

mdor —— 移动端 mdBook 离线阅读器（Android · Rust + Dioxus）。本文件说明本仓库的协作模式与参与方式。

> 协作模式决策记录见 [doc/decisions.md D-14](doc/decisions.md#d-14-单人仓库协作与外部贡献流程)。

## 协作模式（一句话）

- **维护者日常**：直接提交到 `master`，保持 fast-forward 线性历史；分支仅用于隔离实验/长周期工作（`experiment/*`，用完即弃）。
- **外部贡献**：`fork + Pull Request`，PR 一律 **squash-and-merge** 合入（PR 标题即落进历史的 commit message）。

## 环境准备

- 工具链由 `rust-toolchain.toml` 自动钉版（Rust 1.97.1，minimal profile）；Windows 需 VS Build Tools（MSVC 14.50 + Win11 SDK），详见 [doc/env.md](doc/env.md)。
- 一次性安装提交钩子（本地配置，不入库）：
  ```sh
  git config core.hooksPath .githooks
  ```
- 提交信息格式遵循 [Conventional Commits](.agents/rules/commit.md)（钩子强制校验）。

## 质量门禁（提交/合并前必须全绿）

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test -p mdor-core
cargo audit          # 需 cargo install cargo-audit --locked
```

## 外部贡献流程

1. `Fork` 本仓库并 clone；**不要直接 push 到本仓库 master**。
2. 新建分支：`feat/xxx`、`fix/xxx`、`docs/xxx` 等（命名参考提交类型）。
3. 提交遵循 Conventional Commits，本地先跑上面的质量门禁。
4. 发起 PR：标题遵循同一规范（squash 合并后即成为 commit message）；描述写清「改了什么 / 为什么 / 怎么验证」；关联 issue 用 `Closes #xxx`。
5. 维护者 review + CI 全绿后 squash-and-merge。

## 合并策略

- `master`：维护者直推，**fast-forward 线性**，不做 merge commit。
- 外部 PR：**squash-and-merge**，每个 PR 合成一个原子 commit。
- 不使用 Git Flow / develop 分支；无分支保护硬卡（fork 天然隔离写权限）。

## 相关文档

- 架构总纲：[doc/project.md](doc/project.md)
- 决策记录：[doc/decisions.md](doc/decisions.md)
- 提交规范：[.agents/rules/commit.md](.agents/rules/commit.md)