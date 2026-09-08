---
type: project_topic
status: active
summary: "NixOS-WSL 下 VS Code Remote 对接架构：宿主机（Windows）客户端负责下载/管理 server 与扩展（services.vscode-server 模块只打补丁、不管理扩展），因此自定义仓库扩展在「Windows 宿主 ↔ WSL server」拓扑下无法 Nix 声明式安装，只能走客户端 code --install-extension / GUI Install from VSIX 同步进 server；含 nix-ld vs nixos-vscode-server 两类声明式 enabler 对比与 Rebuild-gitlens vsix 安装 procedure"
tags: [nixos, wsl, vscode, vscode-server, remote, extension, gitlens, nix-ld, dev-env]
contains: [decision, lesson, procedure]
created: "2026-08-27"
updated: "2026-08-27"
related: [nix-env-tooling.md, decisions.md, env.md]
authoring_mode: ai_generated
---
# NixOS-WSL 下 VS Code Remote 对接（server + 扩展声明边界）

## 背景

三端架构（[decisions.md D-16](../doc/decisions.md#d-16-开发环境三端架构)）下，nixos-wsl 是日常主力 Linux 环境，配合**宿主机 Windows 的 VS Code** 通过 Remote-WSL 对接。`module/dev/vscode-server.nix` 启用了 `services.vscode-server.enable = true`（nix-community/nixos-vscode-server flake）。会话起因是「在 NixOS 上从指定 GitHub 仓库安装自定义 VS Code 扩展（aliveranme/Rebuild-gitlens，脱去订阅校验的重打包 GitLens）」，由此梳理出 VS Code Remote 在 NixOS 上的架构边界。

## 当前结论

1. **server 本体无法由 Nix 构建并声明安装**。VS Code Remote / Remote-WSL 架构由**宿主机客户端**负责 server 的下载、安装与管理（放在 WSL 的 `~/.vscode-server/`），且 server 版本**与客户端绑定**（客户端升级 server 跟着换）。生态里没有、也不可能有「把 server 打进 Nix store、让客户端对接」的成熟方案——客户端只认自己管理的路径。
2. **NixOS 侧能做的是「声明让客户端下载的 server 跑起来」**，两类实现（可并存，本机均已具备）：
   - `programs.nix-ld.enable = true`：提供 `/lib64/ld-linux-x86-64.so.2` shim，让非 NixOS 二进制直接可执行，**不 patch**。NixOS-WSL 官方推荐、更稳。
   - `services.vscode-server.enable = true`：systemd 监控 `~/.vscode-server`，把 server 自带 nodejs 换成 Nix 版 / 打 RPATH 补丁。
3. **扩展也无法在「Windows 宿主 ↔ WSL server」链路上 Nix 声明式安装**。因为 server 与扩展都由客户端推送到 server（远程端插件如 GitLens 装到客户端后自动部署进 server），`services.vscode-server` 模块**明确不含扩展管理口**。唯一的扩展声明式方案 `vscode-with-extensions` 属于「NixOS 自己当客户端」（如 NixOS 桌面直接开 vscode）的另一拓扑，与当前 Windows 宿主对接不相容。
4. 因此在这种拓扑下，自定义仓库扩展的**干净落点是客户端安装**，装完自动同步进 WSL server。

## 决策记录

- **决策**：不为此重构拓扑。保持「Windows 宿主 + services.vscode-server + nix-ld」现状；Rebuild-gitlens 走客户端安装。
- **否决**：改用 `vscode-with-extensions` 把扩展包成 Nix 声明式——server 是客户端自动装的、与 remote 会话不对接，装了也用不上，且牺牲 Windows GUI（D-16 三端拓扑的价值）。

## 实践指南

### 客户端安装自定义仓库扩展（以 Rebuild-gitlens 为例）

1. 取 vsix 直链与 sha256（复用 [nix-env-tooling.md 坑 6](nix-env-tooling.md) 的 Releases API 技巧）：
   ```bash
   curl -s https://api.github.com/repos/aliveranme/Rebuild-gitlens/releases/latest
   # 资产 digest 字段 = sha256 hex；browser_download_url 字段 = vsix 直链
   ```
   当前 v19.0.1：`https://github.com/aliveranme/Rebuild-gitlens/releases/download/v19.0.1/gitlens-19.0.1.vsix`，sha256 `22ba796ef7f9973e9d8d95fdf48ebe05797afc58e25ebec297b1003ddcae19d6`。国内访问需先 `setproxy`（代理 `172.25.64.1:7897`）。
2. 安装（二选一，均落在**客户端**，remote 会话自动部署到 WSL server）：
   - GUI：扩展面板 `...` → **Install from VSIX…** 选该文件。
   - CLI：`code --install-extension gitlens-19.0.1.vsix`（Windows 的 `code.cmd`，或在 WSL 里 `code` 也会转发到客户端）。
3. 重连 WSL 窗口验证扩展在 server 端生效。

## 教训

1. **别误以为「NixOS 装了 vscode-server 模块＝扩展也能声明式」**。`services.vscode-server` 的职责边界是「patch server 二进制」，**不含扩展装载**——装扩展的高层路径在客户端。动手前先看模块 `module.nix` 的 option 列表，别凭模块名猜能力。
2. **`vscode-with-extensions` 声明式 ≠ Remote 场景通用**。它只在「NixOS 即 VS Code 客户端」时有效；对「Windows 宿主 + WSL server」是死路。遇到「要 100% 声明式装扩展」需求，先分清当前到底是哪种拓扑，再决定是要留在客户端安装、还是整体切换 NixOS 本机 vscode。

## 开放问题

- 若日后确需「连扩展也 100% 声明式 + 可复现」，需评估切换「NixOS 本机 `vscode-with-extensions` + `vscode-extensions.ms-vscode-remote`」的整体架构变更（牺牲 Windows GUI），与 D-16 三端拓扑权衡后再定。当前无此需求，仅记录方向。
