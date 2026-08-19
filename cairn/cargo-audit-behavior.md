---
type: project_topic
status: active
summary: "cargo-audit 0.22.x 无漏洞时静默成功：不打印任何结语、退出码 0 才是通过判据；质量门禁中 audit 的确认方式"
tags: [cairn, cargo-audit, quality-gate, tooling]
contains: [lesson, experience]
created: "2026-08-19"
updated: "2026-08-19"
related: [env.md, decisions.md]
authoring_mode: ai_generated
---
# cargo-audit 无漏洞时静默成功

## 背景

质量门禁首跑（`fmt --check` → `clippy -D warnings` → `test` → `audit`）时，`cargo audit` 输出停在 `Scanning Cargo.lock for vulnerabilities (N crate dependencies)` 后没有后续结语，连续两次如此，易被误判为「卡住 / 未完成 / 缺结果」。

## Lessons

1. **cargo-audit 0.22.x 无漏洞时静默成功**：`presenter.rs` 只在 `report.vulnerabilities.found == true` 时打印 `N vulnerabilities found` 结语（对应 `commands/audit.rs` 退出码 1）；无漏洞时**不打印任何「通过」结语**、直接 `exit(0)`。判定判据 = **退出码**：`0` = 通过（无漏洞）、`1` = 发现漏洞、`2` = 运行出错。
2. **审计确认方式**：PowerShell 下 `cargo audit; Write-Output "exit=$LASTEXITCODE"` 看退出码；**不能以「有没有结语输出」判断成败**——旧版本才有 `No vulnerabilities found` 类收尾，0.22.x「无洞就闭嘴」。

## Current Conclusions

- 质量门禁中 audit 的通过判据 = **退出码 0**，而非输出结语。命令无报错跑完 + `exit 0` 即过。