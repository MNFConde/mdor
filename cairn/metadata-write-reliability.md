---
type: project_topic
status: active
summary: "元数据写入可靠性：JSON 而非 SQLite、原子写 + fsync 按文件类型分层、提交点设计、serde_json 安全对照结论"
tags: [mdor, serde-json, atomic-write, fsync, metadata, reliability]
contains: [decision, lesson, procedure, experience]
created: "2026-08-16"
updated: "2026-08-21"
related: [decisions.md, project.md]
authoring_mode: ai_generated
---
# 元数据写入可靠性（JSON）

## 背景

书架与阅读进度元数据（`library.json` / `progress.json`）是「几十本书量级 < 100KB、按 book_id 简单读写、单进程单写者」的访问形态。可靠性方案在 JSON 选型基础上由原子写 + fsync 分层 + 提交点 + 读入 guard 四层兜底。完整论证见 `decisions.md` D-02/D-03 与 `doc/project.md` §6.7/§6.8。

## 教训

1. **JSON 不用 SQLite 的前提是量级判断**：< 100KB、无关系查询 → SQLite 引入的 C 依赖 / Android 交叉编译复杂度 / schema 迁移不划算。JSON 文本足够。
2. **原子写消除「半写态」但不管断电**：写 `*.tmp` + 同目录 `rename` 覆盖（Android/Linux 上原子），要么旧文件要么新文件。但 rename 不保证「断电后新文件已在磁盘」——那部分由 fsync 兜底。
3. **fsync 决策维度是「文件类型」不是「平台」**：fsync 成本 ∝ 写频率、收益 ∝ 状态价值。平台差异（Android 闪存慢）只是放大器。按文件类型分层 → 两端同一套代码、core 平台无关、不出现 `cfg(target_os)` 分支。
4. **serde_json 内置 128 层递归限制**（防深嵌套栈溢出 DoS）、无多态反序列化、内存安全由语言保证——Java/C 生态的 JSON 漏洞（Jackson-core、Parsson、Fastjson2 AutoType、cJSON）对 Rust/serde **结构性不适用**。选型时逐项对照过（D-02）。
5. **`read_json_capped()` 补齐唯一同源弱点**：serde_json 默认无文档大小上限（各主流解析器共性），其危害前提是「解析攻击者可控的网络 JSON」——本项目只解析应用自生成的本地元数据；仍以 1MB 读入 guard 做纵深防御。

## 当前结论

- **D-02 元数据 = JSON 文件而非 SQLite**，可靠性由原子写（D-03）+ 提交点 + 读入 guard 保证。
- **D-03 原子写 + fsync 分层**：`write_json_atomic(path, data, durability)`，`durability` = `Fsync` / `RenameOnly`，调用方按文件类型传，不按平台分支：
  - `library.json`（低频高价值）→ **Fsync**
  - `progress.json`（高频低价值）→ **RenameOnly**
  - `.mdor/versions/<sha>.json`（低频、可重写）→ RenameOnly
- **提交点设计**：`add_book`/更新多步中断 → `library.json` **最后写** = 提交点：中断后书架无此书/仍为旧版本；孤儿 `books/<id>/` 目录启动时清理。
- **孤儿目录清理（M1 定稿，2026-08-27）**：`books/<id>/` 存在但 library.json 无记录 = 孤儿（提交点中断残留），BookStore 启动**同步**扫描删除。不开后台线程——add_book 时序「先建目录 → 后写 library.json」，后台删除与建目录之间存在 TOCTOU，删前复查只能缩小窗口关不死；同步成本微秒~毫秒级（几十本书量级）。升级门：将来启动被大孤儿目录拖慢时改「同步扫描收集 + 后台删除 + 删前复查」。
- **读入 guard**：读取统一走 `read_json_capped(path, MAX_META_BYTES)`（默认 1MB），读文件前按字节数上限拦截，关死「超大文档耗尽 CPU/内存」DoS。

## 实践指南

- `write_json_atomic` 与 `read_json_capped` 封装为 `store` 内工具函数（`store/util.rs`），core 平台无关，两端复用。
- 详情见 `decisions.md` D-02/D-03 与 `doc/project.md` §6.7/§6.8。
