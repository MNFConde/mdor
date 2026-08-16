---
type: project_topic
status: active
summary: "Windows 下运行 Project Cairn shell 脚本的前提与调用姿势（Git Bash 登录 shell + jq + /c/ 路径）"
tags: [cairn, windows, scripts, git-bash]
contains: [lesson, procedure]
created: "2026-08-16"
updated: "2026-08-16"
related: []
authoring_mode: ai_generated
---
# Windows 下运行 Cairn 脚本

## 背景

初始化 Cairn 时需运行 `obsidian-preflight.sh` 预检（只读脚本，用 bash 编写）。开发机为 Windows（PowerShell 5.1），bash 有两个来源：系统 `WindowsApps\bash.exe`（WSL 启动器）与 Git for Windows（Scoop 安装，`D:\Software\Scoop\apps\git\current\usr\bin\bash.exe`）。

## Lessons

- **系统自带 `bash.exe` 是 WSL 启动器，不是 bash**：Windows 反斜杠路径会被吞（`C:\Users\...` 变成 `C:Users...`），直接跑 Windows 路径脚本报 "No such file or directory"；且未配置 WSL 发行版时会走 WSL 环境。
- **Git Bash 非登录调用时核心工具不在 PATH**：直接 `bash script.sh` 时 `cut`/`grep` 找不到（PATH 被 Windows PATH 覆盖，不含 MSYS `/usr/bin`）；必须用登录 shell `bash -lc '...'`（`-l` 加载 profile 补全 PATH）。
- **bash -lc 里的 Windows 路径仍会被吞**：脚本路径须写成 MSYS 形式 `/c/Users/Admin/.agents/...`（或正斜杠）。
- **预检依赖 `jq`**：解析 obsidian CLI 输出必需；本机未预装需先 `scoop install jq`。
- **Obsidian 侧前提**：`obsidian` CLI 在 PATH、桌面 app 在运行、Settings → General → Command line interface 已开启、目标 vault 已注册（预检脚本会逐项诊断，输出结构化 JSON）。

## Practice Guide

已验证的调用姿势（2026-08-16，`status: ok`）：

```powershell
& "D:\Software\Scoop\apps\git\current\usr\bin\bash.exe" -lc 'bash /c/Users/Admin/.agents/skills/project-cairn/scripts/obsidian-preflight.sh --vault "SecondBrain" --target "Cairn" --index "Cairn/INDEX.md"'
```

要点：用 Git Bash 的 `bash.exe`；`-lc` 补 PATH；`-c` 内先 `bash <脚本>` 并以 `/c/...` 形式传脚本路径；脚本参数照常传。
