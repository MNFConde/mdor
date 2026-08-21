---
type: project_topic
status: active
summary: "本地资源分发通道定案：为什么 http 而非 file://、URL 不带版本号、重写发生在渲染时、样式 include_bytes 内嵌"
tags: [mdor, webview, tiny-http, resource, dangerous-inner-html, dioxus]
contains: [decision, lesson, procedure]
created: "2026-08-16"
updated: "2026-08-21"
related: [decisions.md, diff.md, project.md]
authoring_mode: ai_generated
---
# 本地资源分发通道

## 背景

阅读页是注入 HTML，子资源（图片/CSS/JS）原本指向远端站点，离线时须从本地返回字节。选型经历了自定义 scheme → 本地 http 的收敛。完整论证见 `decisions.md` D-04/D-05/D-06 与 `doc/diff.md` §2。

## 教训

1. **为什么 http 而非 file://（两层叠加，不是单一原因）**：
   - 注入 HTML 的相对 URL 先按宿主文档 base 解析（release 为 `dioxus://`、桌面 dev 为 dev server 地址），无人回答 → 无论走什么方案都必须先重写为绝对地址（与「能否访问文件」无关）。
   - Chromium 子资源安全模型封死 `file://`：非 file 源的文档（正是 mdor 的宿主文档）加载 `file://` 子资源被系统性拦截（"Not allowed to load local resource"）。Android 沙箱禁读任意文件路径 + 运行时数据目录不在打包 assets 内 → 从根上不可行。
   - `http://127.0.0.1:PORT` 是两端唯一无条件放行的「合法网络访问」子资源通道。
2. **自定义 scheme `mdor-book://` 两端不对称**：WebView2 的 `AddWebResourceRequestedFilter` 可用但 API 异步、有怪癖；Android `shouldInterceptRequest` 对自定义 scheme 历来不可靠（导航可拦、资源加载不可靠）——故降级为备选。
3. **URL 里的 version 是「声称值」不是「保证值」**：单工作区 checkout 设计下一旦有竞态，URL 声称的版本与实际内容不符，排查时反被误导。缓存问题用响应头根治比 URL 带 version 更可靠。故 URL 不带版本号。
4. **重写必须发生在渲染时、不是存储时**：端口是动态的（`bind("127.0.0.1:0")` 只有运行时才确定）；保上游原样（场景 1 克隆的上游仓库不能污染、版本间 diff 不能被重写产物污染）；切版本零成本（重写规则与版本无关）。存储层永远是上游原样内容，URL 一个字符都不动。
5. **重写与服务两端必须共享同一套规范化逻辑**（`resources.rs` 唯一事实来源），防 `../` 穿越与编码错位——否则「重写出的 URL 服务器读不到」。

## 当前结论

- **D-04 本地资源分发**：两端统一本地 `tiny_http` 服务器（进程归 app 层）+ `http://127.0.0.1:PORT` 绝对 URL（`/books/<id>/<path>`，**不带版本号**）；服务器统一回 `Cache-Control: no-store` 根治缓存；绑 127.0.0.1 防外访问 + cleartext 白名单仅放行 127.0.0.1（Android 9+ 默认禁明文 http）。`mdor-book://` 降级为备选。
- **D-05 渲染形态**：`dangerous_inner_html` 注入（不用 iframe、不用 `<base>`）。副作用：不执行 `<script>` → 搜索/导航原生 JS 失效，由 Dioxus 自绘 TOC/导航替代（预期行为）。
- **D-06 静态资源分流**：App UI 资源走 Dioxus `[asset]` 打包；阅读页样式 **`include_bytes!` 内嵌 + 渲染时内联注入**（不经本地 http 服务器）；改主题 = 重新发布二进制；主题热更新走备选「首启复制落盘」（需 app 层兼容层抹平平台差异，后续整理）。

## 实践指南

- 服务器归 `mdor-app`：起停 + 动态端口 + no-store + 白名单校验（经 core `resources.rs`）；`render/resources.rs` 是 core 纯映射（URL↔规范化路径，无 socket），`html_extract.rs` 负责重写。
- M6 打包配置项：cleartext 白名单、INTERNET 权限、`min_sdk_version = 30`、`tiny_http` 独立线程池。
- 详情见 `decisions.md` D-04/D-05/D-06、`doc/diff.md` §2、`doc/project.md` §6.5/§10.1。
