---
type: project_topic
status: active
summary: "cargo-audit 0.22.x 无漏洞时静默成功：不打印任何结语、退出码 0 才是通过判据；allowed warnings 语义（非 deny 的 unmaintained/unsound 不失败）；质量门禁中 audit 的确认方式"
tags: [cairn, cargo-audit, quality-gate, tooling]
contains: [lesson, experience]
created: "2026-08-19"
updated: "2026-09-01"
related: [env.md, decisions.md]
authoring_mode: ai_generated
---
# cargo-audit 无漏洞时静默成功

## 背景

质量门禁首跑（`fmt --check` → `clippy -D warnings` → `test` → `audit`）时，`cargo audit` 输出停在 `Scanning Cargo.lock for vulnerabilities (N crate dependencies)` 后没有后续结语，连续两次如此，易被误判为「卡住 / 未完成 / 缺结果」。

## 教训

1. **cargo-audit 0.22.x 无漏洞时静默成功**：`presenter.rs` 只在 `report.vulnerabilities.found == true` 时打印 `N vulnerabilities found` 结语（对应 `commands/audit.rs` 退出码 1）；无漏洞时**不打印任何「通过」结语**、直接 `exit(0)`。判定判据 = **退出码**：`0` = 通过（无漏洞）、`1` = 发现漏洞、`2` = 运行出错。
2. **审计确认方式**：PowerShell 下 `cargo audit; Write-Output "exit=$LASTEXITCODE"` 看退出码；**不能以「有没有结语输出」判断成败**——旧版本才有 `No vulnerabilities found` 类收尾，0.22.x「无洞就闭嘴」。

## 当前结论

- 质量门禁中 audit 的通过判据 = **退出码 0**，而非输出结语。命令无报错跑完 + `exit 0` 即过。

## allowed warnings 语义与现状归档（2026-09-01）

### 语义（cargo-audit 源码实证）

- `allowed warnings` **不是白名单**：无需任何 audit.toml 配置（仓库从未配过），是 presenter 的固定措辞——检出的 warning 二分为「命中 `deny` 列表（denied）」与「未命中（allowed）」；默认 `deny = []`，故所有 warning 天然 allowed。
- 退出码规则：真实漏洞（error 级 advisory）→ 1；命中 deny 的 warning → 1；仅 unmaintained/unsound/notice/yanked 等 allowed warning → **0**（门禁通过，与 D-12「退出码非 0 即失败」吻合）。可用 `--deny warnings/unmaintained/unsound` 收紧，非必要不启用（见下）。
- advisory `ignore = [...]`（完全不出现在输出）与 allowed（已报告不失败）是两回事，勿混淆。

### mdor 现状（14 条 allowed warnings 全景）

全部经 `cargo tree -i <crate> --target all` 追源确认，来自 `mdor-app → dioxus 0.7.10 → dioxus-desktop` 桌面 GUI 依赖链，mdor 自身代码零直接依赖：

| Advisory | 内容 | 引入链 |
|---|---|---|
| RUSTSEC-2024-0411~0420（9 条 unmaintained） | gtk-rs GTK3 绑定停维（gtk/gdk/atk 及 -sys、gtk3-macros） | wry/tao → webkit2gtk → GTK3 |
| RUSTSEC-2024-0429（unsound） | glib `VariantStrIter` 迭代器 soundness | 同 GTK 链 |
| RUSTSEC-2025-0057（unmaintained） | fxhash 停维 | wry → kuchikiki → selectors |
| RUSTSEC-2024-0436（unmaintained） | paste 停维 | dioxus-desktop → image → pulp/rav1e |
| RUSTSEC-2024-0370（unmaintained） | proc-macro-error 停维 | glib-macros / gtk3-macros |
| RUSTSEC-2026-0097（unsound） | rand 0.7 特定用法 unsound | selectors 构建依赖 phf_generator |

### 不处理的理由（四层）

1. **性质非漏洞**：unmaintained = 不再维护 ≠ 可利用缺陷；两条 unsound 的触发条件（直接用 `VariantStrIter` / 自定义 logger 下调 `rand::rng()`）mdor 代码碰不到。
2. **平台错位**：GTK 链仅 Linux 桌面编译（Windows 走 WebView2、Android M6 走系统 WebView），发布目标不含 Linux 桌面。
3. **无修复手段**：全是上游债务（wry/tao 未迁 GTK4、image 链挂 paste），钉版/改代码消除不了，只能等 dioxus/wry 升级——不可行动的告警，暂不 `--deny` 也不 ignore（保留可见性，升级后自然消减）。
4. **门禁语义吻合**：默认即「真漏洞挡 CI、信息类告警可见不失败」，无需额外配置。
