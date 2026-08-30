# script/ 脚本索引

本目录存放所有需要持久化、本地运行的脚本，由 uv 管理共享环境（根 `pyproject.toml`，单环境）。简单脚本为单文件、本身即入口；复杂脚本主体放 `script/{name}/`（作为包），入口 `script/{name}.py` 只做薄封装。统一触发：`uv run --directory script {name}.py`

| 脚本 | 作用 | 用法 |
|---|---|---|
| [check-links.py](check-links.py) | 校验 doc/ 与 cairn/ 各 Markdown 的跨文件与站内锚点引用一致性（标题 1–6 级） | `uv run --directory script check-links.py` |
| [check-markers.py](check-markers.py) | 校验 doc/ 方案状态标记（行内简记 + 块记）结构与类型↔标签映射 | `uv run --directory script check-markers.py` |
| [check-commit-msg.py](check-commit-msg.py) | 校验 git 提交信息格式（Conventional Commits），由 `.githooks/commit-msg` 调用 | `uv run --directory script check-commit-msg.py <提交信息文件>` |
| [shot-analyze.py](shot-analyze.py) | GUI 截图像素分析：窗口定位（xwininfo/手动 crop）+ 主色统计 + 文本行段检测 + 渲染判定 | `uv run --directory script shot-analyze.py <截图.png> [--crop x,y,w,h] [--window 类名] [--json]` |

## check-links.py

- **作用**：扫描 doc/ 与 cairn/ 顶层全部 `*.md`（不递归，自然排除 `archive_doc_v*`），对 `](…md#anchor)` 跨文件链接与 `](#anchor)` 站内链接，逐一与目标文件标题（`#` 1–6 级）生成的 GitHub slug 比对；跨文件目标按源文件所在目录的相对路径解析（如 cairn/ 内指 doc/ 用 `../doc/x.md`）；跳过 fenced code block 与行内反引号代码
- **用法**：`uv run --directory script check-links.py [--doc-root <doc目录>] [--cairn-root <cairn目录>]`（默认分别为仓库根下 `doc/`、`cairn/`）
- **退出码**：0 = 全部锚点一致；1 = 存在不匹配（逐条列出 MISMATCH 行）

## check-markers.py

- **作用**：对 `project.md / decisions.md / env.md / diff.md` 校验方案状态标记两格式——**行内简记**必须为 `[【标签】 方案标题](#锚点)` 链接（裸标签报错）；**块记**要求块上方有独立 `#` 小标题作锚点、callout 类型↔标签映射正确（当前→IMPORTANT、备选→NOTE、已否决→CAUTION、已替换→WARNING）、备选/已否决/已替换块首行带 `触发：` / `原因：` 前缀且后接 `> ` 空行、当前块标题行后直接 `> ` 空行
- **用法**：`uv run --directory script check-markers.py [--doc-root <doc目录>]`（默认 doc 目录为仓库根下 `doc/`）
- **退出码**：0 = 全部通过；1 = 存在违规（逐条列出 VIOLATION 行）

## check-commit-msg.py

- **作用**：校验提交信息首行 `类型(范围): 主题` 结构与 10 种类型白名单、主题规则、正文行 ≤72 字符、`BREAKING CHANGE:` / `Closes #` footer 格式；豁免 `Merge` / `Revert` / `fixup!` / `squash!` 与 `#` 注释行。规则全文见 `.agents/rules/commit.md`
- **用法**：`uv run --directory script check-commit-msg.py <提交信息文件>`（git 提交时由 commit-msg 钩子自动调用）
- **退出码**：0 = 格式通过；1 = 存在违规（逐条列出）；2 = 参数错误

## shot-analyze.py

- **作用**：GUI 截图取证的像素分析（agent 不支持读图，本脚本是「截图落盘 → 像素分析 → 用户肉眼终审」流水线的中坚）。管线：`locate_window`（xwininfo 按窗口类名定位内容区，滤 mutter 装饰壳）→ `crop_to_region`（手动 crop > 自动 > 整屏）→ `color_stats`（主色 + 非主色占比）→ `row_segments`（文本行段：行方差 + 对比密度 + x 跨度三重判据，标定值来自 2026-08-30 M1 书架验收会话）→ `verdict`（rendered / blank / black / unclear）
- **用法**：`uv run --directory script shot-analyze.py <截图.png> [--crop x,y,w,h] [--window 类名] [--row-thresh N] [--min-density N] [--min-span N] [--json]`；`--json` 供 agent 程序化消费。VM 无 uv 时系统 `python3` 直跑亦可（依赖 Pillow：`sudo apt install python3-pil`）
- **退出码**：0 = 分析完成（blank/black 是正常分析结论非错误）；1 = 文件不存在/格式错；2 = 缺 Pillow；3 = 参数错误
- **扩展约定**（三次法则，见 script/AGENTS.md）：新分析需求一律扩展本脚本（新函数 + argparse 参数 + 本文档登记用法），禁止另写临时分析脚本。扩展候选：OCR（tesseract）、双截图 diff、主题自适应阈值

## 临时探针台账（三次法则登记处）

一次性诊断探针在此登记（规则见 [script/AGENTS.md](AGENTS.md)「临时脚本三次法则」）；同一探针跨会话累计 3 次必须固化为正式脚本。写探针前先查本表。

| 日期 | 探针 | 用途 | 次数 | 状态 |
|---|---|---|---|---|
| 2026-08-30 | 会话内像素分析（PIL 主色统计 + 行方差文本行段，多份内联脚本） | M1 书架视觉验收截图判定 | 3（同会话） | ✔ 已固化 [shot-analyze.py](shot-analyze.py)（本规则来历） |
| 2026-08-30 | ctypes EGL 探针（`eglGetDisplay`+`eglInitialize`，/tmp/opencode/eglprobe*.py） | WebKit EGL 崩溃根因定位（glvnd vendor 平台探测） | 1 | 活跃——再遇 GUI/GL 初始化排障即固化 |
