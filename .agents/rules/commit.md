# 提交信息格式规范（Conventional Commits）

> 作用范围：本仓库所有提交。安装钩子（一次性，本地配置不入库）：`git config core.hooksPath .githooks`

> 本文件为提交相关经验/坑的**必选登记处**：后续任何提交相关教训，必须在本文件
> 「六」节登记一份（可在 cairn/ 另存详情），不得只沉淀在 cairn/ 知识文档。

## 一、标准完整格式（多行详细版）

当改动逻辑较复杂时，建议使用多行信息：

```
<类型>(<可选范围>): <主题描述>（注意冒号后面有空格）

<可选的详细正文描述>
为什么这么改，改动了什么逻辑，解决了什么问题。

<可选的脚注>
比如关闭的 Issue 编号，或 BREAKING CHANGE 说明
```

## 二、核心类型（Type）速查表

提交时请根据实际改动选择以下标识之一：

| 类型 | 含义 | 对应版本号变化 |
|---|---|---|
| feat | 新增功能、新特性 | MINOR（次版本号 +1） |
| fix | 修复 Bug | PATCH（修订号 +1） |
| docs | 仅修改文档（如 README） | 不影响版本 |
| style | 代码格式调整（空格、缩进、分号，不影响逻辑） | 不影响版本 |
| refactor | 代码重构（既不是修 Bug 也不是加功能） | 不影响版本 |
| perf | 性能优化相关的改动 | PATCH |
| test | 增加或修改测试用例 | 不影响版本 |
| build | 影响构建系统或外部依赖（如 webpack、npm） | 不影响版本 |
| ci | 修改 CI/CD 配置文件（如 GitHub Actions） | 不影响版本 |
| chore | 杂项，不涉及 src 或 test（如 .gitignore 修改） | 不影响版本 |
| revert | 回滚之前的某次提交 | 不影响版本 |

## 三、编写规范建议（必须遵守）

**Subject（主题）行：**

- 必须以类型开头，紧跟冒号和空格
- 使用祈使句，现在时态（中文即"修复"而不是"修复了"）【语义层，钩子不校验】
- 首字母不要大写（英文时），且结尾不要加句号

**Body（正文）：**

- 用来解释"为什么改"和"怎么改的"，而不是只罗列代码变动【语义层，钩子不校验】
- 每行建议不超过 72 个字符，方便在终端阅读（钩子校验）

**Footer（脚注）：**

- 如果存在 Breaking Change（破坏性变更），必须在脚注写明 `BREAKING CHANGE: <描述>`，这会导致主版本号（MAJOR）+1
- 关闭 Issue：`Closes #123, #456`

## 四、钩子强制项 vs 语义人工项

**钩子强制（`.githooks/commit-msg`）**：`类型(范围): 主题` 结构、10 种类型白名单、冒号后空格、主题非空 / 结尾不加句号 / 英文首字母不大写、正文行 ≤72 字符、`BREAKING CHANGE:` 与 `Closes #` 格式、空提交信息拦截；自动豁免 `Merge` / `Revert` / `fixup!` / `squash!` 开头与 `#` 注释行

**语义人工项（钩子校验不了）**：类型选型是否准确（feat vs refactor vs chore）、祈使句/现在时、正文是否真在解释"为什么"、是否真算 BREAKING —— 靠提交者判断与 review

## 五、提交前自检清单

- [ ] 首行 `类型: 主题`，冒号后带空格
- [ ] 类型与改动性质匹配（见速查表）
- [ ] 主题为祈使句 / 现在时，结尾无句号
- [ ] 正文解释了"为什么改"，每行 ≤72 字符
- [ ] 破坏性变更已注明 `BREAKING CHANGE: <描述>`
- [ ] 关联 Issue 已用 `Closes #xxx` 标注
- [ ] 提交信息含中文时走 UTF-8 文件 `-F`（Windows/PS 5.1 勿用管道喂 git，见六）

## 六、提交相关经验与坑（必选登记）

> 提交相关教训必须在本节登记一份精简版（附 cairn/ 详情指针），供提交时一眼可见。

### 6.1 Windows / PowerShell 5.1 中文编码

- **坑**：Windows 宿主 PowerShell 5.1 下，`@'…'@ | git commit -F -` 管道把提交信息中文字符写成字面 `?`（0x3F）——管道写原生命词 stdin 用 `$OutputEncoding`（默认 ASCII），字节级损坏、非显示伪影（`cmd /c "git log -1 --format=%B"` 直读原始字节仍是 `?` 可证）。
- **对策**：提交信息含中文时，写 UTF-8 无 BOM 消息文件再 `git commit -F <file>`；或先 `$OutputEncoding=[Text.Encoding]::UTF8` 再管道。
- **校验**：`cmd /c "git log -1 --format=%B"` 不得含 `?`（显示层乱码 ≠ 损坏）。
- **详情**：cairn/powershell-encoding.md 坑8（2026-08-29 实证，commit afcec32 / d4dadea）。

## 七、钩子安装

一次性执行（本地配置，不入库，不自动推送）：

```
git config core.hooksPath .githooks
```
