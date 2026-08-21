---
type: project_topic
status: active
summary: "Windows 上钉版 MSVC v14.50 的前提、三个必知的坑与验证流程（SDK 缺失假象、18.6+ workload 自带 Latest、卸载残留 stub）"
tags: [mdor, windows, msvc, toolchain, vs-build-tools, rust]
contains: [lesson, procedure, decision]
created: "2026-08-16"
updated: "2026-08-21"
related: [env.md]
authoring_mode: ai_generated
---
# Windows MSVC 工具链（v14.50 钉版）

## 背景

mdor 桌面目标为 `x86_64-pc-windows-msvc`，需要 VS Build Tools 提供 `link.exe`。本机实测于 2026-08，完整安装步骤与版本矩阵见 `doc/env.md` §1/§2.1。cairn 侧只留经验与坑。

## 当前结论

- **MSVC 版本与 VS 解耦**：VS 2026 中 MSVC 组件 ID 带独立版本号。本项目钉 **v14.50**（`VC.14.50.18.0.x86.x64`），与 `rust-toolchain.toml` 钉 1.97.1 的策略一致；不随 VS 更新自动跳变。
- **14.50 是 LTS 非最新**（支持至 2028-11）：钉它 = 最长支持窗口 + 稳定性，非追最新。
- **Rust ≥ 1.93 才能识别 VS 2026**：`find-msvc-tools 0.1.5` 于 Rust 1.93 合入；1.97.1 满足。
- **MSVC 仅约束 host 目标**：Android 交叉编译走 NDK 自带 clang/lld，与 MSVC 解耦，可并存。

## 教训

1. **`Windows11SDK.26100` 不可省**：漏装 SDK 组件时，即使手动能找到 `link.exe`，rustc 仍报「找不到 link.exe」（2026-01 社区真实案例）。`link.exe` 在、缺的是 SDK——排障先看 SDK。
2. **18.6+ 的 C++ workload 默认带上 Latest 编译器**（当前 14.51）：VS 2026 18.0 时代「只 `--add` 工作负载不会装编译器」，18.6 起默认装 Latest，与钉版 14.50 并存。装法 A 含卸载流程；全新安装直接走「纯组件」装法 B，从源头避免 Latest。
3. **卸载 Latest 后残留 stub 目录与 default 标记**：无论 UI 取消勾选还是 `--remove`，`VC\Tools\MSVC\14.51.xxx` 都剩 2 个 props 残壳（无 `link.exe`），且 `Microsoft.VCToolsVersion.v145.default.txt` 仍指向已删版本。需手动删空壳目录、把 `v145.default.txt` 改为 `14.50.35717`；只剩 14.50 时 find-msvc-tools 会回退到目录扫描。
4. **`Microsoft.VCRedistVersion.default.txt` 不要改**：它指向最新 redist（14.51）是刻意设计（redist 跨工具集共享、v14x ABI 兼容）。

## 实践指南

- 验证：`rustup show` 确认 host 为 msvc；`Get-ChildItem "D:\VS\BuildTools\VC\Tools\MSVC"` 应只有 14.50.35717；新建临时工程 `cargo build` 做真实链接验证。
- 排障速查：`cargo build` 报找不到 link.exe → 重跑 §2.1 安装 + `rustup show` 确认 host；link.exe 找得到但仍报错 → 确认 `Windows11SDK.26100` 已 `--add`。
- 细节：完整两阶段安装（M0 桌面 / M6 Android 补装）、体积估算、依赖升级策略见 `doc/env.md`。
