---
type: project_topic
status: active
summary: "Nix 环境侧外围工具配置（/etc/nixos 环境 flake，与 mdor 项目 flake.nix 分工）：skills-manager CLI 在 NixOS 声明式打包（官方二进制 + patchelf，锁 v1.34.2；Linux 无自更新→声明式锁版；patchelf 漏 xz 致 liblzma.so.5 not found），GUI 仅装 ubuntu-dev 桌面（AppImage wrapType2，定义见文内脚本）；Starship 双行提示符（home-manager programs.starship 跨机复用）"
tags: [mdor, nix, nixos, flake, home-manager, skills-manager, starship, tooling, patchelf, appimage]
contains: [lesson, procedure, experience]
created: "2026-08-26"
updated: "2026-08-26"
related: [decisions.md, env.md]
authoring_mode: ai_generated
graduation_status: candidate
---
# Nix 环境外围工具配置（skills-manager / Starship）

## 背景

三端架构（[decisions.md D-16](../doc/decisions.md#d-16-开发环境三端架构)）下，nixos-wsl 是日常主力 Linux 环境、ubuntu-dev VM 备用，两者都跑 Nix。工程存在**两个职责不同的 flake**：

- **mdor 项目 `flake.nix`**（仓库内）：只装 mdor 项目构建/门禁依赖（Rust 单源、dx、webkit 库、cargo-audit 等），喂 `nix develop`。
- **环境侧 `/etc/nixos` flake**（`/home/morr/nixos-config/default`，独立 git 仓库）：系统级 + home-manager 级个人工具。

skills-manager（管理 `~/.skills-manager` 中央库 + 全部 agent 全局 skills 目录的跨项目工具）与 Starship 提示符均属**跨项目个人工具链**，与 mdor 构建无关，故归环境侧 `home.packages`，不污染项目 flake。

## 方案与决策

- **归属原则**：跨项目个人工具（GUI + 与 OS 关系不大的 CLI）→ 环境侧 `home.nix` 的 `home.packages`；项目构建/门禁依赖 → 项目 `flake.nix`。两者不互塞。
- **skills-manager 打包**（nixpkgs 无此包，官方只发 release 资产）：
  - **GUI** = 官方 Tauri AppImage → `appimageTools.wrapType2 { pname; version; src; }`（新版 API 必须 `pname`+`version`，`name` 会报「extract called without required argument version」；wrapType2 解包后运行、无需 FUSE）。**仅 ubuntu-dev 桌面安装**——nixos-wsl 不装：WSLg 实测能显示到宿主机，但 100MB AppImage 构建耗时（>10min）且 webkit 在 WSLg 下偶发渲染问题（D-16），CLI 对命令行主力已够用。
  - **CLI** = 官方独立 Linux x64 二进制 → `stdenv.mkDerivation` + `fetchurl` + `patchelf --set-interpreter "$(cat $NIX_CC/nix-support/dynamic-linker)"` + `--set-rpath`（NixOS 无 `/lib64/ld-linux`，必须重指 interpreter 与 rpath）。**两机都装**。
  - **锁版**：Linux 无 in-app 自更新（官方仅 macOS/Windows），声明式锁版本+sha256 正好弥补；升级 = 改 URL/sha256 后 rebuild。版本/哈希事实源 = `/etc/nixos/module/dev/skills-manager.nix`（CLI）与本文 ubuntu 脚本（GUI），文档不复制重复哈希。
  - NixOS 侧经 `nixpkgs.overlays` 只暴露 `pkgs.skills-manager-cli`（overlay 写法与 flake.nix 现有 opencode overlay 一致），home.packages 引用；GUI 定义在 ubuntu 脚本小节（见实践指南），不并入 NixOS overlay。
- **Starship 提示符**：home-manager `programs.starship`（enable + enableBashIntegration），settings 声明式；双行布局 = format 里 `$line_break` 把「环境状态行」与「目录+git+`$` 行」分开。ubuntu-dev 复用同一 settings（见实践指南）。
- **flake 新文件必须 git add**：NixOS flake 目录是 git 仓库，新建 `.nix` 未跟踪时 `nix flake check`/rebuild 报「Path '...' is not tracked by Git」，须 `git add`（仅 add，不必 commit）。

## 坑

1. **`appimageTools.wrapType2` 旧参数报错**：传 `name` 而非 `pname`+`version` → `error: function 'extract' called without required argument 'version'`（nixos-unstable 新版 API）。改 `pname`/`version` 即过。
2. **CLI patchelf 漏 `liblzma`**：CLI 动态链接 `liblzma.so.5`（xz），rpath 只给 glibc/openssl/zlib 时 `ldd` 报 `liblzma.so.5 not found`、运行即失败。补 `final.xz` 到 makeLibraryPath。**验证法**：构建后 `ldd <store>/bin/skills-manager-cli | grep "not found"` 应为 0，再跑 `--version`。
3. **`nix build`/`nix eval` 无法用 `#nixosConfigurations...config...` 深层 attrpath**：`.#` 简写会走 `packages.<system>` 命名空间，含 `.config` 的深层路径报「flake does not provide attribute」。**验证单个自定义包**：`sudo nixos-rebuild dry-run`（实例化新 drv）→ `ls -t /nix/store/*<包名>*.drv | head -1` → `nix-store --realise <drv>`。
4. **`nixos-rebuild dry-run` 只做 dry 计划不实际构建**：产物不落 store（只有 `.drv`），要实构建验证用第 3 条的 `nix-store --realise`。
5. **AppImage wrapType2 构建慢**（100MB 解包 + fakeroot，本机 >10 分钟）：非配置问题，耐心等或直接交给 rebuild。
6. **官方 release 锁版技巧**：GitHub Releases API（`api.github.com/repos/<owner>/<repo>/releases/latest`）每资产的 `digest` 字段就是 **sha256 hex**（且附 `browser_download_url` 直链）——抓 API 拿 URL + hex，再 `nix hash convert --hash-algo sha256 --to sri <hex>` 转 Nix SRI（如 `sha256-ytqRv0OUfnYri0elSzFFnEwCr/31mp0uefPs0TLkETI=`），无需本地下载算哈希。适用于任何「官方 release 资产 + Nix 锁版」场景。
7. **WSLg 可用性三查**（判断 GUI 能否显示到宿主机）：① `/mnt/wslg/` 目录存在（socket/runtime）；② 环境变量 `DISPLAY`、`WAYLAND_DISPLAY`、`XDG_RUNTIME_DIR` 已注入；③ NixOS-WSL 模块 `wsl.enable = true`。三者齐备即 WSLg 正常、X11+Wayland 双协议通，GUI 应用（含 Tauri/webkitgtk）会显示到 Windows 桌面。

## 实践指南

### NixOS 落地（本机 /etc/nixos，已执行）

1. 新建 `module/dev/skills-manager.nix`：`nixpkgs.overlays` 暴露 `skills-manager-cli`（mkDerivation + patchelf，rpath 含 glibc/openssl/zlib/**xz**）；**GUI 不在本机定义**（ubuntu 脚本小节持有）。
2. `flake.nix` modules 追加 `./module/dev/skills-manager.nix`。
3. `home.nix`：`home.packages` 追加 `skills-manager-cli`（GUI 行注释保留备恢复）；新增 `programs.starship`（settings 见下）。
4. `git -C /etc/nixos add module/dev/skills-manager.nix`（新文件必须 add）。
5. 验证：`nix flake check`（求值）→ `sudo nixos-rebuild dry-run`（计划）→ 按上面坑 3 实构建 CLI 验证 ldd/运行。
6. `sudo nixos-rebuild switch` 生效；验证 `skills-manager-cli repo status`（输出 `~/.skills-manager` 路径 JSON）、提示符变为双行。

Starship settings（跨机复用同一份）：

```nix
programs.starship = {
  enable = true;
  enableBashIntegration = true;
  settings = {
    add_newline = true;
    format = "$username$hostname$python$rust$time$status$cmd_duration$line_break$directory$git_branch$git_status$character";
    cmd_duration.min_time = 0;
    status.disabled = false;
    time.format = "[%H:%M]";
    username.show_always = false;
    hostname.disabled = false;
    python.disabled = false;
    rust.disabled = false;
    git_status.disabled = false;
  };
};
```

### ubuntu-dev VM 复用（粘贴即用）

**GUI + CLI 都在此机安装**（nixos-wsl 只装 CLI，见 NixOS 落地）。URL/sha256 与 NixOS 侧 `module/dev/skills-manager.nix` 的 CLI 同步维护（GUI 哈希以本脚本为准）。在 VM 内建文件 `~/skills-manager-pkgs.nix`：

```nix
# ~/skills-manager-pkgs.nix —— 提供 skillsManagerPkgs attrset
# 版本/哈希以 /etc/nixos/module/dev/skills-manager.nix 为准（此处为拷贝）
{ pkgs }:
let
  gui = pkgs.appimageTools.wrapType2 {
    pname = "skills-manager";
    version = "1.34.2";
    src = pkgs.fetchurl {
      url = "https://github.com/xingkongliang/skills-manager/releases/download/v1.34.2/skills-manager_1.34.2_amd64.AppImage";
      sha256 = "sha256-679vJPKySWcHPYZE9vD3deja83wW19PRgRyNnosrHxU=";
    };
  };
  cli = pkgs.stdenv.mkDerivation {
    pname = "skills-manager-cli";
    version = "1.34.2";
    src = pkgs.fetchurl {
      url = "https://github.com/xingkongliang/skills-manager/releases/download/v1.34.2/skills-manager-cli-Linux-x64";
      sha256 = "sha256-ytqRv0OUfnYri0elSzFFnEwCr/31mp0uefPs0TLkETI=";
    };
    dontUnpack = true;
    installPhase = ''
      install -Dm755 $src $out/bin/skills-manager-cli
    '';
    # Ubuntu 有系统 glibc/ld，无需 patchelf（NixOS 才需要）
  };
in { inherit gui cli; }
```

安装（二选一）：

```bash
# 方式 A：nix profile（声明式弱，升级靠重装）
nix profile install --impure --expr 'let p = import ./skills-manager-pkgs.nix { pkgs = import <nixpkgs> {}; }; in p.gui'
nix profile install --impure --expr 'let p = import ./skills-manager-pkgs.nix { pkgs = import <nixpkgs> {}; }; in p.cli'

# 方式 B：并入 VM 现有 flake（推荐，若 VM 已用 home-manager/flake）
# 在 VM 的 flake/home 里把 gui、cli 加入 packages，并加 programs.starship（settings 同上）
```

GUI 只在 ubuntu-dev（真 GUI 环境）跑 Tauri/webkit 稳定；nixos-wsl 若日后需要 GUI，WSLg 实测能显示到宿主机（偶发 webkit 渲染小毛病，D-16 已记录），把 home.nix 里注释的 `skills-manager` 行恢复并按坑 5 接受构建耗时即可。

### skills-manager CLI 使用速查

首次运行自动初始化 `~/.skills-manager` 库；`skills-manager-cli` 由 home-manager 注入 PATH。

```bash
# 状态 / 基础
skills-manager-cli repo status                    # 库路径/数据库状态（JSON）
skills-manager-cli skills list                    # 列出中央库 skills
skills-manager-cli --json skills list             # JSON 输出（脚本/agent 友好）
# 安装（默认只进库，不部署到 agent）
skills-manager-cli skills install ./my-skill                    # 本地目录
skills-manager-cli skills install https://github.com/foo/bar.git # git URL
skills-manager-cli skills install user/repo@skill-name          # skills.sh marketplace
skills-manager-cli skills search react --limit 5                # 搜 marketplace
# 部署到 agent（关键动作）
skills-manager-cli skills deploy <ref> --agent claude_code --agent codex
skills-manager-cli skills status <ref>
skills-manager-cli skills undeploy <ref> --agent codex          # --dry-run 预览
skills-manager-cli skills adopt ~/.claude/skills                # 采纳已有 skills
# 预置组 presets / 更新 / git 备份仓库
skills-manager-cli presets list / create / deploy ...
skills-manager-cli skills update --all / check --all
skills-manager-cli git status / pull / commit -m "..."
```

- 所有命令支持 `--help`；安全操作可先 `--dry-run`；CLI 与 GUI 共用同一库与锁，可并存。
- 官方仓库自带 agent 用 skill（`skills/manage-skills`），装进 agent 后 agent 可自助管理 skills。
- 详细 CLI 命令以官方 README 为准：<https://github.com/xingkongliang/skills-manager#cli>。
