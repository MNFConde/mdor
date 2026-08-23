---
type: project_topic
status: active
summary: "跨平台 UI 框架横向对比背景知识：Tauri/Electron/Flutter/自绘 Rust 四路线相对 mdor 约束（Windows+Android、内容本身是 HTML、纯 Rust core）的评估与核心判断；选型论证规范源在 doc/decisions.md D-15"
tags: [mdor, dioxus, tauri, electron, flutter, ui-framework, cross-platform]
contains: [decision]
created: "2026-08-23"
updated: "2026-08-23"
related: [webview-host-differences.md, decisions.md]
authoring_mode: ai_generated
---
# 跨平台 UI 框架横向对比

## 背景

mdor 的 UI 选型约束有三条：① 目标双端 = Windows（开发端）+ Android（一级支持），不做 iOS/Linux；② 书籍内容本身就是 HTML（mdBook 产物 / 渲染好的静态站），阅读页核心诉求是富 HTML 排版显示；③ 架构基石要求业务逻辑收敛在纯 Rust `mdor-core`（桌面直测）。规范决策与完整论证单源在 [D-15](../doc/decisions.md#d-15-ui-框架选型)，本篇只留横向对比的背景知识与判断框架，不复制论证全文。

## 当前结论

**四路线 × mdor 约束评估**：

| 路线 | 代表 | 引擎一致性 | 对 mdor 的关键错位点 |
|---|---|---|---|
| WebView 容器（Rust 单语言） | **Dioxus**【当前】 | 受系统内核碎片化影响 | 无——HTML 内容顺势、双端覆盖、无语言边界 |
| WebView 容器（JS 前端 + Rust 后端） | Tauri | 同样受影响（底层同为 wry） | 引擎差异直接砸在自家 JS 代码 + IPC 序列化边界 |
| 自带 Chromium | Electron | 零碎片化（最强卖点） | 无 Android 官方支持 = 目标平台不覆盖；体积；Node 栈推翻 core 架构 |
| 自绘引擎 | Flutter / egui / iced / Slint | 与系统内核无关（自己画） | 无浏览器排版能力，富 HTML 书页需自解；Flutter 保 Rust core 需 dart:ffi 三层桥接 |

三个可复用的判断：

1. **「webview 适配难受」不是 Tauri 特有问题**：所有基于系统 WebView 的框架共享同一份内核碎片化（Tauri 底层 wry 与 Dioxus 桌面端同源）；Tauri 社区声音大是因为其最痛点 Linux WebKitGTK 内核代差最大——mdor 双端皆 Chromium 天然绕开这一端。
2. **Electron 的成熟来自「自带整个 Chromium」**：零碎片化的代价是每 App 背一份完整浏览器；对 mdor 致命的不是体积而是 Android 缺位。
3. **内容形态决定渲染路线**：内容本身是 HTML 时，「WebView 当显示器」是顺势而非妥协——真正需要自绘引擎的场景是内容非 HTML 或需要极致一致性的重交互应用。

## 实践指南

- 选型论证 / 被否决方案的完整理由：[doc/decisions.md D-15](../doc/decisions.md#d-15-ui-框架选型)
- WebView 宿主差异的具体表现与对冲：[webview-host-differences.md](webview-host-differences.md)
- 若将来重估选型（如 Flutter 移动生态吸引力上升），先读 D-15 的否决理由清单再评估，避免重新踩已论证过的坑
