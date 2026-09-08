---
type: project_topic
status: active
summary: "书源扩展长期形态：声明式 manifest（scoop 式，数据）+ 既有 SourceAdapter 编译期插件（行为）；v1 落地前仅执行防返工约束，v1 全量落地后重评"
tags: [mdor, architecture, source-adapter, manifest, scoop]
contains: [decision, experience, open_question]
created: "2026-09-03"
updated: "2026-09-03"
related: [decisions.md D-18, project.md §4.3, plan.todo M4 锚块]
authoring_mode: ai_generated
contributors:
  - MNF_Condé
---
# 书源声明式 manifest 扩展路线

## 背景

用户设想输入来源扩展为更多方式（mdBook、博客网站、电子书站如 ddia），长期形态类比 scoop 包管理器：书源（站点结构、镜像边界、TOC/正文选择器、构建/渲染管线选择）用 JSON/TOML 等声明式格式承载，不同输入源的获取与构建以插件形式接入。

### Origin quote

> 「我觉得输入可以扩展成为 scoop 管理软件的方式，通过 json 或者 toml 或者其它符合功能要求的格式来承载输入源，然后通过不同的构建比如 mdbook 或者这个电子书（ddia）的输入，来构建界面阅读；不同的输入源，构建方式通过前述文件做定义，然后输入处理，构建等则通过插件形式来便于扩展」
>
> —— MNF_Condé（用户），2026-09-03，讨论 mdor 输入来源扩展方向

> 注：讨论中触发时机经用户修正为「等当前所有阶段做完、有第一个完整版本后再考虑是否上，之前只做一些零打碎敲的优化方便后续上这个功能」——以修正后为准。

## 当前结论

- **方向采纳（D-18，2026-09-03 已决策）**：书源扩展长期形态 = 声明式 manifest（数据）+ 既有编译期 `SourceAdapter` 插件通道（行为）。manifest 未来只是 detect/fetch 前的新入口：反序列化为来源参数 struct，适配器/落库/渲染管线全复用；新增来源 = 新增适配器模块 + 一份 manifest 描述。
- **时机**：v1（M0–M7）落地前不实现 manifest 任何部分；**v1 全量落地后重评是否引入**。M3–M7 期间仅执行防返工约束。
- **防返工约束（M4 随手做，本身是分层纪律、有无 manifest 都成立）**：
  1. 「URL 推导出的参数」与适配器行为分离——参数进独立 struct（未来 manifest 直接反序列化为同一 struct）；
  2. SUMMARY 解析做独立纯函数模块（manifest 的 TOC 描述与 GitHub 路线可共用）。
- **已否决**：运行时动态插件加载（WASM/dylib）、无 manifest 的任意站自动识别（Readability 式）、立即立项。

## 经验

1. **难度分层评估**（本提案拆解）：manifest 参数化（小，serde 双格式近乎免费）< 新构建管线插件（中，M4 本身）< 任意站自动识别（大，被否决）< 运行时动态插件（大，被否决）。提案真正的增量只是「数据描述层」，行为插件通道现有架构已具备。
2. **scoop manifest 字段与现有能力的隐式对应**：`remote_version` ≈ autoupdate；D-08 原始字节 hash ≈ hash 校验——项目已站在 manifest 一半的位置上，缺的只是「站点结构描述 + 构建管线选择」，故等待成本为零。
3. **单适配器阶段设计 schema = 猜需求**：schema 该长什么样由真实来源多样性教出来（GitHub 暴露 SUMMARY 路径/markdown 选项，ddia 类 Oink 主题站暴露选择器差异）；提前定 schema 反成兼容负担。
4. **manifest 核心收益是书源共享（bucket 式分发）**，单人使用阶段该收益不存在——这是「等 v1」而非「现在做」的判据主轴。
5. **提案验证法**：用具体站点（ddia.vonng.com 静态站 + Vonng/ddia 仓库）走一遍两条既有路线，检验出真实缺口仅「TOC sidebar/正文选择器硬编码」——说明提案与现有架构正交，不需改已落地设计。

## 开放问题

- manifest schema 字段集（触发后由真实来源多样性教出：GitHub 特有字段、非标站选择器等）
- 格式选型（TOML vs JSON vs 其他）——serde 双格式成本低，届时定
- 书源分发形态（scoop bucket 式社区仓库 vs 内置示例源）——核心收益项，v1 后重评时一并定
- 选择器参数化（TOC sidebar / `<main>` 正文选择器外置）——manifest 前置拼图之一，有意不排期（ddia 类非标站真要接入时再做，属 v1 后自然增强）

## 实践指南

- 重评时以 [D-18](../doc/decisions.md#d-18-书源声明式-manifest-扩展路线) 为基线逐条核验触发条件（来源多样性需求是否真实存在）。
- M4 实施时防返工约束两条随切片执行，不单独立项；见 plan.todo M4 锚块「既定约束」。

### 零碎项清单（v1 落地前可零打碎敲带上）

完整清单（约束性 + 顺手性）；执行落点 = plan.todo M4 锚块「零碎项」：

1. **来源参数 struct 化**（约束①，M4）：「URL 推导出的参数」与适配器行为分离，参数进独立 struct——未来 manifest 直接反序列化为同一 struct。
2. **SUMMARY 解析独立纯函数模块**（约束②，M4）：输入输出纯数据，不掺网络/落库——manifest 的 TOC 描述与 GitHub 路线共用。
3. **选择器集中定义**（顺手，M4——下一个触碰解析层的里程碑）：TOC sidebar（`ol.chapter`/`ol.section`，`source/static_site.rs` 的 build_toc 硬编码）与正文（`main#content`，`render/html_extract.rs` 硬编码）选择器收拢为单一常量模块。注意：这是「内部单源收拢」≠「运行时参数化」（参数化仍留触发后），本身是可维护性改进，有无 manifest 都值得做。
4. **M4 收尾知识回写**（顺手，M4 收尾）：GitHub 路径暴露的 manifest schema 线索（SUMMARY 路径、markdown 渲染选项等特有字段）回写本文「开放问题」节，为 v1 后重评攒证据。

**边界（无需动作）**：

- M3 剩余切片（切片5 自适应布局 / 切片6 碰撞标注渲染）：不碰来源与选择器层，无事可带。
- M5 版本功能 / M6 Android 打包 / M7 CI 发布：与来源层零交集，无事可带。
- **已满足、无需动作**：适配器注册插件模式（`SourceRegistry` 编译期注册）、`SourceKind` serde snake_case（manifest 可按字符串引用 kind）、`Book` serde 序列化、`MirrorOptions::from_entry` 构造模式（约束①的现成样板）、D-08 检测函数 M4 复用（D-08 排期已含）。
