---
type: project_topic
status: active
summary: "跨平台 UI 框架横向对比背景知识：Tauri/Electron/Flutter/自绘 Rust 四路线相对 mdor 约束（Windows+Android、内容本身是 HTML、纯 Rust core）的评估与核心判断；选型论证规范源在 doc/decisions.md D-15"
tags: [mdor, dioxus, tauri, electron, flutter, ui-framework, cross-platform]
contains: [decision, lesson]
created: "2026-08-23"
updated: "2026-09-02"
related: [webview-host-differences.md, decisions.md, local-resource-channel.md]
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

## M3 屏幕实现实践（Dioxus 0.7，切片4）

Dioxus 0.7 `mdor-app` 四屏接线落地时可复用的模式与坑（2026-09-02 切片4 实证）：

1. **屏幕间路由用手写 `GlobalSignal<AppRoute>` 枚举**，不引入 `dioxus-router`（D-17）：平面跳转、无 URL 栏、无参数化路径时手写最省；`AppRoute::Library/AddBook/Reader` + 全局信号 `NAV.with_mut(|n| *n = ...)` 即可切屏。
2. **页内锚点跳转经 `use_window()` → `webview.evaluate_script(js)`**：`use_window` 从 `dioxus::desktop`（re-export，非 `dioxus_desktop` crate）取 `DesktopContext`，其 `pub webview: WebView` 可跑 JS（`document.getElementById(id).scrollIntoView()`）。注意 `use_window()` 是 hook，只能在组件/effect 内调用，不能放进裸函数——把它拆成独立子组件（如 `AnchorItem`）即可。
3. **滚动比例用 `onscroll` 的 `ScrollData`**：滚动容器上挂 `onscroll`，`e.scroll_height() / e.client_height() / e.scroll_top()` 算 `scroll_top/(scroll_height-client_height)`，clamp 到 0~1；存进度走 `AppService::save_progress`（同步门面，非命令）。
4. **`dangerous_inner_html: html`** 注入正文（Dioxus 保留 `dangerous_inner_html` 提醒 XSS 面；本仓库文本来自本地镜像书，白名单已由 core 校验）。
5. **rsx 模板 `for` 体内不能写裸 `let` 语句**：`for a in &v { let x = a.clone(); ... }` 报 `expected identifier`；需在进 rsx 前预物化 owned 数据（如 `Vec<(id,title,url,version)>`），或把每项抽成独立子组件（`AnchorItem`/`TocNode`）让闭包各自捕获。
6. **`match` 分发给不同屏的类型一致化**：每个 `#[component]` 生成独立的 `Xxx_completions::Component` 类型，`match` 臂直接 `XxxScreen {}` 类型不匹配；让 `App` 返回 `Element`，每个 `match` 臂 `rsx! { XxxScreen {} }`（或调用组件函数 `XxxScreen()`）归一为 `Element`，或用外层包裹。

## 实践指南

- 选型论证 / 被否决方案的完整理由：[doc/decisions.md D-15](../doc/decisions.md#d-15-ui-框架选型)
- Dioxus 0.7 四屏接线模式与坑（GlobalSignal 路由 / use_window eval / onscroll 滚动比例 / rsx for 裸 let / match 归一 Element）：见上「M3 屏幕实现实践」
- WebView 宿主差异的具体表现与对冲：[webview-host-differences.md](webview-host-differences.md)
- 若将来重估选型（如 Flutter 移动生态吸引力上升），先读 D-15 的否决理由清单再评估，避免重新踩已论证过的坑
