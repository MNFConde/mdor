# doc/ 文档地图

mdor 项目文档索引。所有文档为规划稿，随实现推进持续更新。

| 文档 | 职责 | 何时读 |
|---|---|---|
| [project.md](project.md) | 架构总纲：目标、技术栈、分层、数据模型、核心模块、版本控制、存储布局、里程碑、CI | 一切起点 |
| [decisions.md](decisions.md) | 决策记录（ADR）：关键选型的背景 / 决策 / 依据 / 影响 | 遇到"为什么这么选"时查阅 |
| [diff.md](diff.md) | 跨平台差异：Windows 桌面 vs Android 的依赖点差异与对策 | 涉及平台差异 / 移植时 |
| [env.md](env.md) | 环境搭建：M0 桌面 / M6 Android 的安装、验收、排障、依赖升级 | 搭环境 / 升级工具链时 |
| [mdor.c4](mdor.c4) | C4 组件图源（LikeC4 DSL） | 修改架构组件关系时 |
| [script/scripts.md](../script/scripts.md) | script/ 脚本索引：各脚本用途与用法 | 使用/新增 script/ 下脚本时 |

**推荐阅读顺序**（新加入者）：README → project → diff → env；decisions 按需查阅。

## 引用规范

- 全文档引用一律用 **markdown 链接**，不用裸编号：
  - 跨文件：`[文本](文件名.md#标题锚点)`
  - 同文件：`[文本](#标题锚点)`
- 锚点 = 标题转小写，去除 `（）、·、/、.` 等标点，空格 → `-`，中文保留（GitHub slug 规则）。例如 `## 6.8 解析器安全对照（serde_json 选型依据）` → `#68-解析器安全对照serde_json-选型依据`
- 被抽取的决策统一收口到 [decisions.md](decisions.md)，规范文档只留摘要 + 链接
- 锚点一致性检查：`uv run --directory script check-links.py`（扫描四篇文档的跨文件 + 站内锚点，有不匹配即返回非零退出码）

## decisions.md 登记规则

- 每条 ADR 独立编号 `D-xx`，编号连续递增；标题不含 `、（）+` 等标点（保证锚点干净）
- 每条必填：**状态**（已决策 / 待定）/ **日期** / **规范位置**（反向链接）/ **背景** / **决策** / **依据** / **影响**
- 规范文档里凡涉及该决策处，用链接指向对应 D 编号，不重复粘贴论证
- 讨论未决项（如 M1 实测后才能敲定的）标"待定"，并列出验证项

## 再生成

C4 组件图：`likec4 gen mermaid doc -o <输出目录>`（源为 [mdor.c4](mdor.c4)）
