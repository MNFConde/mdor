---
type: project_topic
status: active
summary: "用 MCP（jdocmunch）做 doc→cairn 提炼的实证与约定：索引含 archive 副本稀释、表格/论证内容召回弱、形式取信陷阱、必须补手工核读"
tags: [cairn, mcp, jdocmunch, retrieval, doc-audit]
contains: [lesson, experience, procedure]
created: "2026-08-16"
updated: "2026-08-21"
related: [env.md, decisions.md, diff.md]
authoring_mode: ai_generated
---
# MCP 检索 doc→cairn 的实证与约定

## 背景

doc→cairn 知识提炼任务，两次执行对照：直接全文阅读（96303 tokens）vs MCP 索引检索（125837 tokens，约 +30%）。MCP 用 jdocmunch 索引 `local/mdor` 覆盖整个仓库（521 sections / 26 docs，**含 `archive_doc_v*/` 副本**）。两次合并后的完整方案是本次沉淀的来源；检索约定已写入根 `AGENTS.md`「检索约定（MCP）」节。

## 教训

1. **索引含 archive 副本会稀释覆盖判断**：`archive_doc_v*/` 是当前 doc 的更早状态（内容 ⊆ 当前 doc），检索会在旧副本上打转，可能把"旧版已覆盖"误判为"当前已覆盖"。用 MCP 检索 doc→cairn 时须**显式排除 `archive_doc_v*/`**。
2. **MCP 对表格型/散点式/长篇论证内容召回弱**：env.md 的故障排查表（§6）、日期式更新记录（§8）、decisions.md 的长篇 rationale——BM25+embedding 偏向标题与角色分类，恰恰漏掉 Cairn 最该沉淀的"坑 + 决策摘要"。实测：MCP 漏掉 Zig 评估、旧版 dx 链接代理 bug、`NoSuchMethodError`→`min_sdk`、cmdline-tools 目录结构等**全部工具链坑**，并把 Dioxus/dx 同步升错标到 env.md §4（实际在 §2.3/§4.3）。
3. **形式取信陷阱**：MCP 输出的章节引用 / A-B 分级 / 结构化证据会放大"可信"错觉，但精确的形式 ≠ 正确的召回。本次即有第三方会话被形式误导、建议"采纳 MCP 版"——照做会造成实际知识损失。判断必须以可核实的证据为准，不以其输出形式为准。
4. **MCP 单用 ≠ 完整检索**：MCP 适合作**覆盖矩阵 + 排序**（判断哪些已沉淀、优先级），召回必须补**手工核读**（env.md / decisions.md / project.md 全文）。两次结果合并才是完整答案；MCP 的成本约 +30%。

## 当前结论

- 检索约定（已入根 `AGENTS.md`）：先确认索引新鲜度；显式排除 `archive_doc_v*/`；对 env.md / decisions.md / project.md 补手工核读；MCP 结果只作覆盖矩阵与排序，**不替代全文召回**。

## 实践指南

- doc→cairn 提炼流程：① 用 MCP 跑覆盖矩阵（排除 archive、确认索引新鲜）；② 手工核读 env.md / decisions.md / project.md 的表格与散点记录；③ 两次结果合并后定沉淀范围。
