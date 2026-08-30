---
type: project_topic
status: active
summary: "ubuntu-dev VM（Ubuntu 24.04.4，备用环境，D-16）从裸机到 M0 桌面可用的环境准备操作手册：阶段0 最小配置（Guest Additions / openssh-server / git / Nix multi-user / SSH key clone / 快照）→ 阶段1 devShell + direnv → 阶段2 个人工具（skills-manager GUI+CLI / Starship）→ 阶段3 opencode → 阶段4 验收 + 快照。网络加速（国内镜像 / 代理链路 / 工具链预置）回指 nix-mirror-proxy.md；skills-manager 版本/哈希事实源引 nix-env-tooling.md"
tags: [mdor, ubuntu, vm, nix, devshell, direnv, skills-manager, starship, opencode, setup, procedure]
contains: [procedure]
created: "2026-08-28"
updated: "2026-08-28"
related: [env.md, decisions.md, nix-env-tooling.md, nix-mirror-proxy.md]
authoring_mode: ai_generated
---
# ubuntu-dev VM 环境准备操作手册

## 背景

三端架构（[decisions.md D-16](../doc/decisions.md#d-16-开发环境三端架构)）下，ubuntu-dev VM（Ubuntu **24.04.4** Desktop，4 核/6G/80G，NIC1=NAT + NIC2=Host-Only，配置与磁盘见 [env.md §1](../doc/env.md#开发环境拓扑)）是备用环境，仅两个触发场景：dioxus Linux 桌面 GUI 调试、真机 USB 直通。本手册记录从裸机到 **M0 桌面可用** 的完整操作流程（2026-08-28 实测落地）；环境侧加速手段（国内镜像 / 代理 / 工具链预置）见 [nix-mirror-proxy.md](nix-mirror-proxy.md)。

> 范围 = M0（桌面开发）。Android SDK/NDK/JDK 的 flake 声明式锁定为 M6 待办（D-16），本手册不涉及。
>
> 本机执行进度状态以 `plan.todo`「ubuntu-dev VM 环境」块为准；本文档只给操作流程与坑。

## 阶段 0：备用化最小配置（env.md §1 清单）

**0.1 Guest Additions**
```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential dkms linux-headers-$(uname -r)
# VirtualBox 菜单: Devices → Insert Guest Additions CD image…
mkdir -p /media/cdrom
sudo mount /dev/cdrom /media/cdrom
sudo /media/cdrom/VBoxLinuxAdditions.run
# 重启后验证: lsmod | grep vboxguest
```

**0.2 openssh-server（Host-Only NIC2 供宿主机 Remote-SSH）**
```bash
sudo apt install -y openssh-server
sudo systemctl enable --now ssh
ip a   # 记下 enp0s8（Host-Only）的 IP
```

**0.3 git**
```bash
sudo apt install -y git
git config --global user.name "名字"
git config --global user.email "邮箱"
```

**0.4 Nix multi-user（daemon 模式）+ flake 实验特性**
```bash
sh <(curl -L https://nixos.org/nix/install) --daemon
# 重开终端（或 source /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh）
nix --version
sudo tee -a /etc/nix/nix.conf > /dev/null <<'EOF'
experimental-features = nix-command flakes
EOF
sudo systemctl restart nix-daemon
```
> 镜像 substituters 与代理注入见 [nix-mirror-proxy.md](nix-mirror-proxy.md)（在 nix develop 前配好可省大量等待）。

**0.5 SSH key + 克隆仓库**
```bash
ssh-keygen -t ed25519 -C "ubuntu-dev"
cat ~/.ssh/id_ed25519.pub   # 加到 GitHub → Settings → SSH keys
ssh -T git@github.com       # "You've successfully authenticated" 即通过
git clone git@github.com:MNFConde/mdor.git ~/mdor
```

**0.6 快照**：VirtualBox → Take Snapshot，命名 `base-minimal`。

## 阶段 1：devShell + direnv

**1.1 首次进 devShell**（Rust 1.97.1 单源 `rust-toolchain.toml`；dx 0.7.10 shellHook 钉版；dioxus 桌面库；门禁工具）
```bash
cd ~/mdor
nix develop   # 首次下载 + dx 全量编译约数分钟，二次幂等
# 验证：
nix flake check
cargo test
dx --version   # dioxus 0.7.10
```
> 首次若受国际链路下载卡顿，先做 [nix-mirror-proxy.md](nix-mirror-proxy.md) 的镜像/预置再重跑。

**1.2 direnv + nix-direnv**（进目录自动加载 devShell）
```bash
nix profile install nixpkgs#direnv nixpkgs#nix-direnv
echo 'eval "$(direnv hook bash)"' >> ~/.bashrc
mkdir -p ~/.config/direnv
echo 'source "$HOME/.nix-profile/share/nix-direnv/direnvrc"' > ~/.config/direnv/direnvrc
# 重开终端后：
cd ~/mdor
direnv allow   # 生成 /.direnv 缓存（仓库 .gitignore 已排除，勿提交）
```

## 阶段 2：个人工具（skills-manager GUI+CLI / Starship）

**2.1 skills-manager**：完整打包脚本与安装命令见 [nix-env-tooling.md](nix-env-tooling.md#ubuntu-dev-vm-复用粘贴即用)（版本/哈希事实源 = 该文档；此处不复制正文）。要点：GUI（AppImage wrapType2）+ CLI（官方二进制）都装；`nix profile install --impure --expr '… import <nixpkgs> …'`，若 `import <nixpkgs>` 报错（flake 安装无 channel）先 `nix-channel --add https://mirrors.ustc.edu.cn/nix-channels/nixpkgs-unstable nixpkgs && nix-channel --update`。

**2.2 skills-manager 初始化 + 同步 + 部署**（两步机制：同步库 ≠ 部署到 agent）
```bash
skills-manager-cli repo status
skills-manager-cli git pull          # 远端已有存档用 clone 而非 init（撞 unrelated histories，见 skills-manager-cli.md）
skills-manager-cli skills deploy --agent opencode
```

**2.3 Starship**（双行提示符，settings 跨机复用，TOML 版见 nix-env-tooling.md 的 nix attrset 等价写法）
```bash
nix profile install nixpkgs#starship
echo 'eval "$(starship init bash)"' >> ~/.bashrc
# ~/.config/starship.toml：format 用 $line_break 分行（目录+git+$ 一行、环境状态一行）
```

## 阶段 3：opencode

```bash
nix profile install nixpkgs#opencode   # 若 nix search 无该 attr，官方脚本兜底: curl -fsSL https://opencode.ai/install | bash
opencode auth login
```
**安装实测补充（26-08-30）**：

- `nix profile install` 是新版 Nix 中 `add` 的 deprecated 别名——`warning: 'install' is a deprecated alias for 'add'` 属正常、非错误，装成功即入 profile；新写法用 `nix profile add`。
- `nix search` 会命中多个同名包，按 **attrpath** 区分：`nixpkgs#opencode` 只解析顶层 `opencode`（1.18.21），**不会**装到 `haskellPackages.opencode` 或仅描述含 opencode 的 `cc-switch`；要装嵌套包须写全路径（如 `nixpkgs#haskellPackages.opencode`）。
- opencode **无 apt 源**；官方安装方法仅 curl / npm / pnpm / bun / brew（见 `opencode upgrade --help` 的 `--method` 枚举）。

**关键使用姿势**：opencode 的 bash 工具是非交互 shell，direnv 不生效——须在**已加载 devShell 的交互终端**里启动 opencode（进程环境继承），agent 才能吃到 flake 工具链（详见 [nix-project-flake.md](nix-project-flake.md) 坑 5）。skills 经 2.2 部署到 `~/.config/opencode/skills/`，重启会话生效。

## 阶段 4：验收 + 快照

```bash
cd ~/mdor
cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo audit
dx serve --platform desktop   # 弹出桌面窗口（VM 核心存在意义）
```
验收通过后 VirtualBox 打快照 `env-ready`。

## 常见坑速查

- `nix develop` 大件下载极慢 / 反复从 0 重下 → 国际链路瓶颈，按 [nix-mirror-proxy.md](nix-mirror-proxy.md) 镜像预置。
- VM 内 curl 国内镜像 403（宿主机同 URL 200）→ 多为残留代理变量或出口差异，见 [nix-mirror-proxy.md](nix-mirror-proxy.md) 坑。
- `dx serve` 无窗口 → 确认在 GUI 会话（Desktop）内运行。
- 进 `~/mdor` 未自动加载 devShell → 确认 direnv hook 与 `direnv allow` 已做（1.2）。
