---
type: project_topic
status: active
summary: "ubuntu-dev VM（Ubuntu 24.04.4，备用环境，D-16）从裸机到 M0 桌面可用的环境准备操作手册：阶段0 最小配置（Guest Additions / openssh-server / git / Nix multi-user / SSH key clone / 快照）→ 阶段1 devShell + direnv → 阶段2 个人工具（skills-manager GUI+CLI / Starship）→ 阶段3 opencode → 阶段4 验收 + 快照。网络加速（国内镜像 / 代理链路 / 工具链预置）回指 nix-mirror-proxy.md；skills-manager 版本/哈希事实源引 nix-env-tooling.md"
tags: [mdor, ubuntu, vm, nix, devshell, direnv, skills-manager, starship, opencode, setup, procedure]
contains: [procedure]
created: "2026-08-28"
updated: "2026-08-30"
related: [env.md, decisions.md, nix-env-tooling.md, nix-mirror-proxy.md]
authoring_mode: ai_generated
---
# ubuntu-dev VM 环境准备操作手册

## 背景

三端架构（[decisions.md D-16](../doc/decisions.md#d-16-开发环境三端架构)）下，ubuntu-dev VM（Ubuntu **24.04.4** Desktop，4 核/6G/80G，网络实测 = 仅 NIC1 NAT（原规划 NIC2=Host-Only 未配置，更正见 §0.2），配置与磁盘见 [env.md §1](../doc/env.md#开发环境拓扑)）是备用环境，仅两个触发场景：dioxus Linux 桌面 GUI 调试、真机 USB 直通。本手册记录从裸机到 **M0 桌面可用** 的完整操作流程（2026-08-28 实测落地）；环境侧加速手段（国内镜像 / 代理 / 工具链预置）见 [nix-mirror-proxy.md](nix-mirror-proxy.md)。

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

**0.2 openssh-server（宿主机 Remote-SSH）**

> 更正（2026-08-30）：原写 Host-Only NIC2 供 SSH，实测该 VM 实例从未配置 NIC2，SSH 实际一直走 **NAT 端口转发**（宿主 `127.0.0.1:2222` → VM `10.0.2.15:22`）；env.md §1 已改记 NAT 事实。固定 IP 诉求出现时再补 NIC2 并回改两处。

```bash
sudo apt install -y openssh-server
sudo systemctl enable --now ssh
ip a   # 记下 VM IP（NAT 内网 10.0.2.15；宿主经端口转发连接）
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

**2.3 安装实测补充（26-08-30）**：

- Starship 1.26.0 装机正常（USTC 镜像秒下）；`~/.config/starship.toml` 按 nix-env-tooling.md attrset **忠实转译**（time 模块只写 format 不加 disabled = false——该模块默认禁用，VM 与 WSL 机行为一致，`[%H:%M]` 不显示属预期）。
- bashrc hook 在**系统 bash 5.2**（GNOME Terminal）下干净生效；nix devShell 的 **bash 5.3** 把 `complete`/`progcomp` 挪成 loadable builtin，sourcing Ubuntu 默认 bashrc 的 bash-completion 会报 `complete: command not found`——与 starship 无关的既有条件，不影响真实终端。

**2.4 skills-manager GUI 启动方式与冒烟记录（26-08-30，一次尝试即失败，不重试）**

调用方式（mdor-app 同款 Xwayland 组合，需 devShell 已加载以继承 nix mesa 路径）：

```bash
# 前置（同 cairn/ubuntu-vm-setup.md「GUI 调试操作要点」）
export GDK_BACKEND=x11 DISPLAY=:0 XAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.*
export WEBKIT_DISABLE_COMPOSITING_MODE=1 GSK_RENDERER=cairo LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe
export __EGL_VENDOR_LIBRARY_FILENAMES=$(nix eval --raw nixpkgs#mesa.out)/share/glvnd/egl_vendor.d/50_mesa.json
export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
skills-manager
```

冒烟结果：**未到 WebKit 层即死**——`bwrap: setting up uid map: Permission denied`。根因：appimageTools wrapType2 用 bubblewrap 沙箱起 AppImage，Ubuntu 24.04 默认 `apparmor_restrict_unprivileged_userns=1` 拒绝非特权进程建 uid map。CLI（官方二进制，无沙箱）不受影响、正常可用；GUI 日后若要用，方向 = 关 userns 限制（`sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0`）或换官方 AppImage 直接跑（`--appimage-extract-and-run`），本文档只记现象不展开。

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

## GUI 调试操作要点（2026-08-30 M1 书架视觉验收实测）

### WebKit EGL 崩溃（核心坑）

**症状链**：dioxus/WebKitGTK 应用启动 → `Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...` → WebKitWebProcess 反复自杀 → 窗口黑屏或空白（UI 主进程存活，仅 web 内容进程死）。

**根因**：VirtualBox 显卡默认 **VMSVGA** 控制器模拟 VMware 硬件（`lspci` vendor `0x15ad`，内核走 `vmwgfx`），而 nix mesa ≥25 已移除 vmwgfx 用户态 DRI 驱动 → WebKit GPU 进程经 GBM 打开 renderD128 找不到匹配驱动 → EGL 初始化崩溃。**任何 WebKit 环境变量组合都无法绕过**（`WEBKIT_DISABLE_DMABUF_RENDERER` 在 2.52 仍存在但只影响渲染器选择，不解决 EGL display 创建失败）。

**解法**（devShell 内启动前注入，全部必要）：

```bash
export WEBKIT_DISABLE_COMPOSITING_MODE=1        # 关 WebKit 合成（合成路径依赖 GL）
export GSK_RENDERER=cairo                        # GTK 渲染器走 cairo
export LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe   # mesa 全软渲染
export __EGL_VENDOR_LIBRARY_FILENAMES="$(nix build nixpkgs#mesa --print-out-paths --no-link)/share/glvnd/egl_vendor.d/50_mesa.json"  # glvnd 指到 nix mesa vendor（devShell 默认只有 mesa-libgbm 无头子集，缺 libEGL_mesa）
export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1  # VM 内可接受；沙箱下 /nix 路径注入常失败
```

**SSH 会话跑 GUI 应用**：须注入 `GDK_BACKEND=x11 DISPLAY=:0 XAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.*`（GNOME Wayland 会话经 Xwayland；auth 文件名随机，`ls` 取）。纯 Wayland 后端 + SSH 实测同样崩在 EGL（X11 路径才验证通过）。

**验证成功判据**：`ps -e` 出现 **WebKitWebProcess 且稳定存活**（>10s）；失败时它 3s 内消失只剩 WebKitNetworkProcess。窗口渲染像素分析用 [script/shot-analyze.py](../script/shot-analyze.py)（亮色主题下内容区应 verdict=rendered：白底 + 文本行段，书架 2 本书 = 1 标题行 + 2×条目组共 8 行段）。

### 截图取证

- `gnome-screenshot` 须 `sudo apt install`（GNOME 41+ 的 org.gnome.Shell.Screenshot D-Bus 对非授权调用方 AccessDenied，gdbus 直调不可行）；SSH 会话带 `XDG_RUNTIME_DIR=/run/user/1000` 即可，无需 DISPLAY。
- 系统自带 `xwd` 只能抓 Xwayland 客户端窗口（`xwd -id <window>`），非整屏替代品。
- **agent 不能读图**（Read PNG 表面成功、媒体附件实际报错）→ 取证流水线 = gnome-screenshot 落盘 → [script/shot-analyze.py](../script/shot-analyze.py) 像素分析（窗口定位/主色/文本行段/verdict，`--json` 供 agent 消费）→ 用户肉眼终审。
- 新分析需求（OCR / 双图 diff / 主题阈值等）**先扩展 shot-analyze.py 再执行**，禁止临时另写分析脚本——三次法则见 [script/AGENTS.md](../script/AGENTS.md)。

### 环境诊断手法沉淀

- `strace -f -e trace=openat` 抓 WebProcess 文件访问：本次定位到 glvnd 读系统 `/usr/share/glvnd/egl_vendor.d/50_mesa.json` → 找不到 `libEGL_mesa.so.0`（nix RPATH 链内无此库）→ 一击实锤根因。
- ctypes 探针（`eglGetDisplay` + `eglInitialize`）测 EGL 平台：注意测试进程的 glibc 必须与目标库匹配（系统 glibc 进程加载 nix mesa 报 `GLIBC_ABI_GNU2_TLS not found`，不代表 nix 链接的应用内会失败）。
- 变量名不要靠记忆猜：`strings libwebkit2gtk-4.1.so.0 | grep '^WEBKIT_[A-Z_]+$'` 直接枚举该构建支持的全部开关。
