---
type: project_topic
status: active
summary: "WebView 宿主双端差异：入口模型（main vs AndroidMain 被生命周期叫醒）、线程纪律（UI 必须主线程/tokio 显式切回）、运行时可得性、内核版本不一致与内联 CSS 对冲、DevTools/热更差异"
tags: [mdor, webview, winit, wry, android-activity, threading, dioxus]
contains: [lesson, decision, open_question]
created: "2026-08-16"
updated: "2026-08-16"
related: [diff.md, project.md, decisions.md]
authoring_mode: ai_generated
---
# WebView 宿主双端差异

## 背景

同一个 Rust 应用，Windows 用微软的 WebView2 内核、Android 用安卓的 System WebView，两者的对接方式完全不同——不只是"浏览器内核不同"，而是"进程怎么起、UI 在哪条线程、资源怎么喂"都不同。底层三件套：`winit`（创建窗口 + 事件循环，Android 基本不直接参与）、`wry`（在窗口里塞浏览器内核）、`android_activity`（Android `Activity` 生命周期经 JNI 调进 Rust）。完整对比见 `doc/diff.md` §2。

## Lessons

1. **Android 没有 `main()`，是被 Activity 生命周期"叫醒"的**：入口是 `AndroidMain`（系统创建 `Activity` 触发），代码是"被系统叫醒"而非"主动跑起来"（`onCreate → onStart → onResume → … → onDestroy`）；`mdor-app/src/main.rs` 必须有双入口路径（cfg 分支区分）；进程存活/被杀（低内存回收）行为与桌面完全不同。
2. **线程模型最易踩坑**：Android 上后台线程碰 UI 直接崩/抛异常，**创建 WebView 必须在主线程**；tokio 下载完成刷新 UI——Windows 随便投递即可，Android **必须显式切回主线程**（`doc/project.md` §11 风险项同源）。
3. **WebView 运行时可得性**：Windows 的 WebView2（Edge Chromium）**可能没装**，需检测/引导安装（Win11 自带、Win10 需装 WebView2 Runtime）；Android 的 System WebView **系统自带**（可商店更新），只需注意 minSdk。
4. **双端内核版本不一致 → 内联 CSS 对冲**：都是 Chromium 但版本不同（WebView2 跟 Edge、Android 跟系统），CSS/JS 有细微差异——RenderService「内联书籍 CSS 保证样式一致」即为此对冲（`doc/project.md` §6.5）。
5. **DevTools/热更差异**：Windows 是 WebView2 自带调试器（F12 类）+ `dx serve` 直接热更；Android 是 `chrome://inspect` + adb 远程调试、需连设备/模拟器、迭代慢。
6. **差异全在 mdor-app，core 不碰 WebView**：依赖分层用 `[target.'cfg(target_os = "android")'.dependencies]` 隔离；资源/渲染协议差异已由 D-04/D-05/D-06 定案（见 `local-resource-channel.md`）。

## Current Conclusions

- **双入口 + 线程纪律（diff.md §2.4）**：桌面 `main()` + Android `AndroidMain`，启动时解析数据目录；所有 UI 更新最终发生在主线程，tokio 后台任务回 UI 需显式切换。
- **渲染/资源协议层**：`render/resources.rs` 维护"本地 URL → 文件路径"映射（core 纯函数、无 socket），本地 `tiny_http` 服务器归 app 层——见 `local-resource-channel.md`。

## Open Questions

- A3 桌面 WebView2 与 Android System WebView 渲染不一致 → 同一书页双端截图对比（字体/图片/代码高亮），待 M6 真机验证（`doc/project.md` §11）。
- D1 触摸滚动性能待 M6 真机（`doc/project.md` §11）。

## Practice Guide

- 详情见 `doc/diff.md` §2.1–2.4、`doc/project.md` §11（风险 A1/A3、D1）；相关决策 D-04/D-05/D-06（见 `local-resource-channel.md`）。
