---
type: project_topic
status: active
summary: "mdor 日志基座（tracing）决策：M1 切片4；core 只打点不碰配置；桌面 tracing-subscriber fmt + RUST_LOG；不引分布式全家桶(留 OTel 门)；M6 appender+logcat + 发布裁剪"
tags: [mdor, logging, tracing, rust]
contains: [decision]
created: "2026-08-26"
updated: "2026-08-26"
related: []
authoring_mode: ai_generated
---
# mdor 日志基座（tracing）

## 决策

- M1 新增「切片4 日志基座（tracing）」（plan.todo 已收录，原切片4/5 顺延为 5/6）：workspace 钉版 `tracing` + `tracing-subscriber`。
- mdor 是单机离线阅读器，只需**最简单的应用内日志**——三支柱里只有 Log 一种，连日志服务端都不需要；与分布式遥测方案（见 [telemetry-schema-registry-design.md](telemetry-schema-registry-design.md)）对照见下方「明确不做」。

## 方案

- `mdor-core`：挂 tracing **门面**——业务只打点、不碰配置。
- `mdor-app` 桌面：`tracing-subscriber` fmt 层输出控制台/滚动文件，`RUST_LOG` 控级别。
- 埋点：AppService 门面调用打 info（add_book / open_reading / 命令开始·结束·耗时），错误路径 warn/error + 结构化字段（book_id、version）。

## 明确不做

- 远端汇聚 / 指标 / 分布式追踪 / 上报（无此需求）。
- 留门：`tracing` 数据模型对齐 OpenTelemetry，将来若要匿名错误上报接 OTel 语义即可，无需重构。

## M6（不在本期）

- `tracing-appender` 滚写到 app 数据目录文件（配合数据清除范围）；调试 adb logcat。
- 发布构建 feature gate 裁剪 trace 层。
