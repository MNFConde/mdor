# AGENTS.md

mdor —— script/ 目录约定（由根 AGENTS.md「script/ 目录约定」节迁移而来，本文件只约束 script/ 内的脚本组织与触发方式）。

## script/ 目录约定

- 需要持久化、本地运行的脚本一律放 `script/`，由 uv 管理环境（根 pyproject.toml，共享单环境）
- 简单脚本：单文件 `{name}.py`，本身即入口
- 复杂脚本：主体放 `{name}/`（作为包，含 `__init__.py`），入口 `{name}.py` 只做薄封装（import 子包 + 触发）
- 统一触发：`uv run --directory script {name}.py`
- 当前共享单环境；若日后多个复杂脚本依赖互相干扰，将冲突脚本拆成独立 uv 项目隔离