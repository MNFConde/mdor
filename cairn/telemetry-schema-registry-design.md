---
type: project_topic
status: active
summary: "多语言分布式高频遥测链路完整设计（参考）：定义层 schema 注册表(形态A)+运行时本地分发；统一 WebSocket 信封；JSON Schema vs Protobuf+buf 双线工具链；含『10ms 连续值=指标非日志』教训"
tags: [telemetry, architecture, websocket, schema-registry, protobuf, json-schema, polyglot]
contains: [decision, lesson, reference, open_question]
created: "2026-08-26"
updated: "2026-08-26"
related: [module-boundary-contract-design.md]
authoring_mode: ai_generated
---
# 多语言分布式高频遥测链路设计（参考）

## 背景与适用范围

本设计面向**另一个项目**（多语言分布式高频遥测系统），**非 mdor 本体架构**，归档于此作可复用参考。mdor 为单机离线阅读器，不需要其中的任何分布式组件；本设计与其对照的价值在于「何时用日志、何时用指标」的判定与模块边界原则（见 [module-boundary-contract-design.md](module-boundary-contract-design.md)）。

## 已确认约束

| 项 | 定值 |
|---|---|
| 阶段 | 研发期，线上格式可改 |
| 语言/模块 | Python（设备端）×1、Python（服务器）×1、JS（浏览器前端）×1、Java 后端入库 |
| 拓扑 | 设备 Python ↔ 服务器 Python ↔ 浏览器 JS；Java 后端 ←服务器→ DB |
| 链路 | 三跳均 WebSocket（WS1 = 设备↔服务器；WS2 = 服务器↔浏览器；**标签，非协议版本**） |
| 负载 | 10ms 采样、一项 10–20 值、多项；连续 + 实时 |
| 形态 | 定义层 schema 单一事实源 + 运行时本地分发（形态 A） |

## 教训：10ms 连续值是指标，不是日志

- 高频连续数值（10ms 一个值、一项多个值）= **时序指标（Metric）**，必须走指标/时序管道，不是日志（Log）。
- 当日志设计的后果：存储/索引爆炸、查询退化为全表扫描、实时性达不到；当指标存：列式压缩 + 时间窗口聚合 + 降采样，成本低一个量级。
- 判定准则：**离散事件 → 日志；连续数值 → 指标；跨模块一次操作 → 追踪**。三种都是带时间戳的数据点，形态决定存储与工具链（工具链背景见 [Reference/observability-toolchain-basics.md](Reference/observability-toolchain-basics.md)）。

## 核心设计原则（形态 A）

1. **定义层统一、运行时本地分发**：所有类型形状/版本/校验规则在 schema 单一事实源；各语言从它生成解析代码；运行时每个接收进程只解析自己订阅的 key。
2. **契约 = 定义，触点 = 绑定**：契约层定义"数据长什么样"（DTO/schema/版本）；触点是模块把自己的内部类型接入契约的适配点（一个契约两侧各一个触点）。
3. **私有 vs 共享**：私有类型（生产者/消费者仅同一条链路的两个端点）可直接改 + 两端同步；共享类型（跨多条链路）只能版本化演进 + 全消费者评审。
4. **改类型「四处辐射」**：契约定义 + 全部触点适配 + 链路中段转换 + 落盘/外部边界兼容，四处联动，不只是触点。
5. **版本化演进 + 未知跳过告警**：加字段不删字段、加变体不删变体；消费端对不认识的 key/版本 = 跳过 + 告警（四语种同一策略，写进生成代码）。
6. **时序自携带**：per-payload `v/ts/seq`；信封只搬运不推导时间；NTP 兜底时钟偏差；接收侧按 ts+seq 归并；重连 seq 跳变 = 断流检测。

## 统一信封协议

```
Envelope { producer, sent_at, seq, map<key, payload> }   // 有 A 无 B 天然表达
每个 payload = { v, ts, seq, 业务字段... }
```

- key 命名空间 `chain_x.metric`（防全局注册表碰撞）
- 三腿只差异在参数：WS1 1s 聚合 + zstd；WS2 近实时小批量 + permessage-deflate；浏览器→服务器低频命令
- 安全：直接上 **wss**（浏览器安全页强制；ws→wss 代价 = 证书 + TLS 终止 + 首连握手 +1 RTT）
- 连续/实时保证：1s 批量 + 环形缓冲丢最旧（**连续性 > 完整性**）+ 降采样阶梯（10ms→100ms）

## 两线工具链对比

| | A · JSON Schema | B · Protobuf + buf |
|---|---|---|
| 事实源 | `.schema.json` | `.proto`（buf workspace） |
| codegen | datamodel-code-generator / json-schema-to-typescript / jsonschema2pojo | `buf generate`（Python / ts-proto / Java） |
| 版本强制 | CI diff 脚本（自写硬规则：不删 required / 不改类型 / 只加性） | `buf breaking --against` 机器强制 |
| WS | text + permessage-deflate | binary + zstd（WS2 仍 deflate） |
| 运维增量 | 最低（无新 CLI、JSON 可读） | buf CLI + 生成产物策略 + 二进制解码工具 |

备选 C：**Avro + Confluent Schema Registry**——兼容模式自动校验，适合 Kafka/流式管道；JS 支持弱。决策门：格式可改 → B；必须 JSON → A；Kafka/重度流 → C。起步最省事：**git 仓库 + CI 兼容检查即最轻注册表**，不必先上注册表服务。

## 运维与可观测（B 比 A 多的）

- buf CLI 工具链（开发机 + CI + 生成环境，钉版）
- 生成代码产物策略（提交 vs 构建时生成）要显式定
- （可选）BSR 注册表服务——跨团队共享才引入
- **二进制协议的可观测性**：抓包/日志不可读，需解码工具（持续隐性成本）

## 落地路径

```
阶段0  盘点现状 key/字段/类型 + 第一版 schema + git 注册表 + CI 检查（JSON Schema 起步）
  →   A 线三语种 codegen 替换手写 extractor + 往返单测（含"有A无B"用例）
  →   WS 三跳端到端 + 压测（吞吐/批量/压缩/丢最旧连续性/重连 seq）
  →   决策门：定 A 长期 or 切 B（JSON Schema 语义迁移 .proto + 换生成器，重跑验证）
  →   Java 入库（默认 long-format 时序表 + 保留期降采样）
```

阶段 0 产物两线共用，切 B 不推翻设计——渐进重构方法论：**先盘点现状 → schema 正式化 → 小验证 → 决策门**。

## 开放问题

- buf 工具链接受度（决定决策门 A/B）
- Java 入库形态（待确认；默认 long-format 时序表）
- 浏览器端压缩默认 permessage-deflate（待确认）
