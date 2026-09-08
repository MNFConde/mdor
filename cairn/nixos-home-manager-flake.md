---
type: project_topic
status: active
summary: "环境侧 /etc/nixos flake（/home/morr/nixos-config/default）home-manager 接线踩坑与最佳实践：home-manager.users.<name> 应传模块路径 ./home.nix（勿 import 手动调用，缺 config 报错）；home-manager.extraSpecialArgs 传 flake inputs 须包裹为 { inherit inputs; }（摊平会把各 input 名变 specialArgs key，home.nix 声明 inputs 参数时解析不到 → 无限递归）；官方 manual NixOS module 节即此写法；含最小复现诊断法与 root 构建无害警告备忘"
tags: [nix, nixos, home-manager, flake, nixos-wsl, tooling]
contains: [decision, lesson, procedure]
created: "2026-08-30"
updated: "2026-08-30"
related: [nix-env-tooling.md, nix-project-flake.md]
authoring_mode: ai_generated
graduation_status: candidate
---
# NixOS home-manager flake 接线（users 传路径 + extraSpecialArgs 包裹 inputs）

## 背景

环境侧 `/etc/nixos` flake（`/home/morr/nixos-config/default`，独立 git 仓库，与 mdor 项目 flake 分工见 [nix-env-tooling.md](nix-env-tooling.md)）在 home-manager 接线时连续踩了两个构建错误。修好后对照 [home-manager 官方手册 NixOS module 节](https://nix-community.github.io/home-manager/nix-flakes/nixos.html) 确认为社区标准写法。

home.nix 是一个 **home-manager 模块函数**（签名 `{ config, pkgs, inputs, ... }:`），并在 `imports` 里用了 `inputs.charmbracelet-nur.homeModules.crush`。

## 方案与决策

- **`home-manager.users.<name>` 传模块路径**：`home-manager.users.morr = ./home.nix;`。让 home-manager 框架在求值时注入 `config`/`pkgs`/`inputs` 等模块参数；**不要** `import ./home.nix { inherit inputs; }` 手动调用。
- **flakes inputs 进 home-manager 用包裹写法**：`home-manager.extraSpecialArgs = { inherit inputs; };`。把整个 inputs attrset 以**名为 `inputs` 的特殊参数**传给 home-manager 各模块。
- 官方 manual（NixOS module 节）原文即：`home-manager.extraSpecialArgs = { inherit inputs; };` + `home-manager.users.jdoe = ./home.nix;`，并注明「让完整 inputs attrset 可被模块以 `{ inputs, ... }:` 形式声明取用」。

## 坑

1. **`import ./home.nix { inherit inputs; }` 手动调用模块函数** → `error: function 'anonymous lambda' called without required argument 'config'`。home.nix 是函数，被立即调用时只给了 `inputs` 一个参数，缺 `config`/`pkgs`。正确做法是传路径让框架调用。
2. **`home-manager.extraSpecialArgs = inputs;`（摊平写法）+ home.nix 声明 `{ inputs, ... }`** → `error: infinite recursion encountered`，栈内提示：`noting that argument `inputs` is not externally provided, so querying `_module.args` instead, requiring `config``。
   - 机制：`extraSpecialArgs = inputs` 把 flake inputs **摊平展开**成 specialArgs 的各个 key（`self`/`nixpkgs`/`home-manager`/`charmbracelet-nur`…），**并不存在名为 `inputs` 的参数**。home.nix 索要 `inputs` 时 `args.inputs` 缺失 → 模块系统（`lib/modules.nix` 的 `applyModuleArgs`）回退查 `config._module.args` → 算 `config` 又得加载 home.nix 本身 → 无限递归。
   - 修复只动 `flake.nix` 一处（`{ inherit inputs; }`）；home.nix 无需其它改动（`inputs` 仅在其 `imports` 行使用）。

## 经验（诊断法）

- **最小复现**：用 `nix eval --impure --expr` 构造最小 `nixpkgs.lib.nixosSystem`（含 `home-manager.nixosModules.home-manager` + 假 inputs + 一个取 `inputs` 的 home.nix 内联函数）——摊平写法复现同样的无限递归、包裹写法通过，即可锁定根因，不必在真实大配置里猜。
- **读栈定位**：`--show-trace` 全栈 + 读 nixpkgs `lib/modules.nix` 的 `applyModuleArgs`（`args.${name} or config._module.args.${name}`）理解 fallback 触发点；错误文案 `not externally provided` 即表示 specialArgs 缺该参数名。
- **root 构建无害警告**：`warning: $HOME ('/home/morr') is not owned by you`（以 root 跑 build 而 `$HOME` 指向普通用户目录，WSL 常见）与 `Git tree ... is dirty`（有未提交改动）均可忽略，与本次报错无关。

## 实践指南

```nix
# flake.nix（环境侧 /etc/nixos，核心三行）
home-manager.useGlobalPkgs = true;
home-manager.useUserPackages = true;
home-manager.users.morr = ./home.nix;                    # 传路径，勿 import 手动调用
home-manager.extraSpecialArgs = { inherit inputs; };     # 包裹命名，勿直接 = inputs
```

验证（改完先快速求值再 rebuild）：

```bash
nix --extra-experimental-features 'nix-command flakes' eval \
  '/home/morr/nixos-config/default#nixosConfigurations.nixos.config.home-manager.users.morr.home.username'
# 输出 "morr" 即通过
sudo nixos-rebuild switch --flake /home/morr/nixos-config/default
```
