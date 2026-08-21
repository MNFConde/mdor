---
type: project_topic
status: active
summary: "服务编排：薄门面 AppService（UI 依赖 O(1)）+ 按需命令化（命令队列串行 = 单写者）；为何不引全局中介者"
tags: [mdor, architecture, facade, command, orchestration]
contains: [decision, lesson]
created: "2026-08-16"
updated: "2026-08-21"
related: [decisions.md, project.md]
authoring_mode: ai_generated
---
# 服务编排：薄门面 + 按需命令化

## 背景

mdor 架构为分层树状（UI→服务→核心叶子），服务间几乎无互调。为避免"UI 直接认识多个服务、一次操作多次跨层调用"，采用薄门面 + 按需命令化。决策记录见 D-07；规范见 `doc/project.md` §6.9。

## 教训

1. **薄门面把 UI 依赖从 O(服务数) 降到 O(1)**：UI 层只依赖 `AppService` 一个入口，单次用户操作 = 单次门面调用（`add_book` 合并 detect+add；`open_reading` 合并"读位置 + 渲染章节 + 保存进度"）；业务编排不泄漏进 Dioxus 屏幕（`mdor-app` 只做"拿到 HTML 注入 + 交互"）。
2. **命令化按需而非全局套壳**：短、无网络、无并发、无进度需求 → 普通函数（删除书籍、progress.json 读写）；长 + 网络 + 可中断 + 需进度/串行 → 命令对象（`UpdateBookCommand`），经命令队列**串行执行**（一次一条 = 天然落实 D-02 单写者），`progress()` 暴露当前阶段供 UI 订阅，命令携带执行阶段支持中断续做。
3. **为何不引全局中介者（可复用内核）**：架构是分层树状，`UpdateService`/`PositionService` 本就是各自流程的中介者；再引入全局 hub 会令所有模块**反向依赖**一个中央对象、流程变隐式、难以测试与定位——净负收益。
4. **命令边界单一 → 可测**：命令只回答"完成一次更新"，注入假适配器即可单测（httpmock）。

## 当前结论

- **D-07 已决策**：薄门面 v1 即生效；命令化 v1 仅"更新书籍"（SD-3），"添加书籍"先拆命名函数（`check_latest` / `fetch_to_temp` / `commit_and_tag` / `migrate_and_save`），出现并发/进度需求再升级。
- 服务层模块保留为各自流程的中介者，不引入全局 hub。

## 实践指南

- 详情见 `doc/project.md` §6.9 与 D-07。
