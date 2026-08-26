---
type: reference
status: active
summary: "可观测性/遥测外部资料参考：三支柱(logs/metrics/traces)、工具链四层(采集/存储/查询/告警)、常见栈黑话、OTel、多语言 codegen 工具速查、日志vs指标判定、WebSocket 压缩与 wss 背景"
tags: [observability, telemetry, logging, toolchain, reference]
contains: [reference]
created: "2026-08-26"
updated: "2026-08-26"
authoring_mode: ai_generated
---
# 可观测性与遥测工具链（外部资料参考）

> **定位**：外部资料参考（教材/社区共识整理），**非 mdor 或任何项目的当前真相**；只增不改。项目化教训与决策见对应 topic（[telemetry-schema-registry-design.md](../telemetry-schema-registry-design.md) / [module-boundary-contract-design.md](../module-boundary-contract-design.md)）。

## 三支柱（Logs / Metrics / Traces）

| 形态 | 样子 | 回答 | 典型 |
|---|---|---|---|
| Log 日志 | 离散事件，一行一条 | 发生了什么 | 报错、状态变化 |
| Metric 指标 | 连续数值，时间序列 | 值多少、趋势 | CPU、温度、延迟 |
| Trace 追踪 | 一次操作的调用树 | 慢在哪、错在哪环节 | 跨模块请求 |

判定：**离散事件 → 日志；连续数值 → 指标；跨模块一次操作 → 追踪**。10ms 连续采样 = 指标非日志。

## 工具链四层

采集代理（Vector / Fluent Bit / Promtail / OTel Collector）→ 传输队列（Kafka/NATS，量大削峰才上）→ 存储索引（日志：Loki/OpenSearch；指标：VictoriaMetrics/ClickHouse/Prometheus；追踪：Jaeger/Tempo）→ 查询可视化（Grafana）+ 告警（Grafana Alerting / Alertmanager）。

- 指标必须进时序库（列式压缩 + 时间窗口聚合 + 降采样），别进日志库。
- 实时性边界：端到端 1~3s = 社区"近实时"标准；毫秒级只能模块本地。

## 常见栈黑话

- ELK/EFK：Elasticsearch + Logstash/Fluent Bit + Kibana（日志）
- Prometheus 栈：Prometheus + Grafana（指标）
- Loki 栈：Promtail + Loki + Grafana（日志轻量）
- Grafana 全家桶：Loki + VictoriaMetrics + Tempo + Grafana（新项目默认）
- OpenTelemetry（OTel）：行业统一数据模型/传输标准；新系统按 OTel 语义打点，接任何工具不绑死

## 多语言 codegen 工具速查

| 语言 | JSON Schema | Protobuf |
|---|---|---|
| Python | datamodel-code-generator → Pydantic v2 | protobuf（buf generate） |
| JS/TS | json-schema-to-typescript + ajv | ts-proto / protobuf-es（浏览器纯 TS 可跑） |
| Java | jsonschema2pojo + networknt validator | protobuf-java |

## WebSocket 压缩与安全背景

- WebSocket 协议主流仅一个标准：RFC 6455（握手 version 13），无"WebSocket 2"版本之分。
- 压缩按"端点能力"选，不按"版本"选：permessage-deflate（RFC 7692，浏览器原生、跨消息有状态字典）vs 应用层 zstd（压缩比/速度更好，但浏览器无原生 zstd 需 wasm）。
- ws:// vs wss://：wss = TLS；浏览器安全页只允许 wss；代价 = 证书 + TLS 终止 + 首连握手 +1 RTT。
