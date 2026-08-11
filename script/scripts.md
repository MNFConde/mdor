# script/ 脚本索引

本目录存放所有需要持久化、本地运行的脚本，由 uv 管理共享环境（根 `pyproject.toml`，单环境）。简单脚本为单文件、本身即入口；复杂脚本主体放 `script/{name}/`（作为包），入口 `script/{name}.py` 只做薄封装。统一触发：`uv run --directory script {name}.py`

| 脚本 | 作用 | 用法 |
|---|---|---|
| [check-links.py](check-links.py) | 校验 doc/ 各 Markdown 的跨文件与站内锚点引用一致性 | `uv run --directory script check-links.py` |

## check-links.py

- **作用**：对 `project.md / decisions.md / env.md / diff.md` 中 `](…md#anchor)` 跨文件链接与 `](#anchor)` 站内链接，逐一与目标文件标题生成的 GitHub slug 比对
- **用法**：`uv run --directory script check-links.py [--doc-root <doc目录>]`（默认 doc 目录为仓库根下 `doc/`）
- **退出码**：0 = 全部锚点一致；1 = 存在不匹配（逐条列出 MISMATCH 行）
