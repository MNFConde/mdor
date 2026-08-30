# mdor M0 环境搭建文档

> 移动端 mdBook 离线阅读器 · Android · Rust + Dioxus
> 对齐 `project.md` §10（M0 里程碑）与 §12（构建配置）
> 平台：Windows（PowerShell 5.1+）· 本机 2026-08 实测

---

## 1. 环境总览与版本矩阵

> 「阶段」列：**M0** = 桌面开发当前安装；**M6** = Android 打包前再装（§2.4/§2.5 标注「M6 暂缓」）。

| 组件 | 版本 | 用途 | 安装位置 | 阶段 |
|---|---|---|---|---|
| Rust | 1.97.1（`rust-toolchain.toml` 钉版） | 主语言 | Scoop rustup 管理 | M0 |
| MSVC 工具链 | `stable-x86_64-pc-windows-msvc` | Windows 本机构建/link | Scoop rustup 管理 | M0 |
| Android targets | `aarch64-linux-android` / `x86_64-linux-android` | Android 交叉编译（arm64-v8a / x86_64） | rustup target | M6 |
| **VS 2026 Build Tools** | MSVC **v14.50**（LTS，钉版）+ Win11 SDK 10.0.26100 | `link.exe` / Windows 本机链接 | `D:\VS\BuildTools` | M0 |
| JDK | Temurin 21（LTS，Adoptium zip 便携） | dx 的 Gradle 侧 | `<仓库>\dev\jdk`（便携；Scoop 兜底） | M6 |
| Android SDK | cmdline-tools **14742923** + platform-tools + `platforms;android-36` + build-tools | Android 构建基座 | `<仓库>\dev\android` | M6 |
| Android NDK | **r29**（`29.0.14206865`） | C/C++ 交叉编译（gix/ring 等） | `<仓库>\dev\android\ndk` | M6 |
| dioxus-cli | 0.7.10（cargo install，`--locked --version` 钉版） | `dx serve/build` | `~/.cargo/bin` | M0 |
| WebView2 | 随系统 | `dx serve --platform desktop` 渲染 | 预装（Win11） | M0 |

### 版本对齐说明

- **MSVC 版本与 VS 解耦**：VS 2026 中 MSVC 组件 ID 带独立版本号。本项目**钉 v14.50**（`VC.14.50.18.0.x86.x64`），与 `rust-toolchain.toml` 固定 1.97.1 的一致性策略一致；不随 VS 更新自动跳变。
- **14.50 是 LTS 非最新**：14.50 随 VS 2026 18.0（2025-11）首发并被指定为长期支持版（支持至 2028-11）；当前最新 GA 是 14.51（2026-05，VS 18.6 默认，标准 9 个月支持）。钉 14.50 = 最长支持窗口 + 稳定性，非追最新。
- **Rust ≥ 1.93 才能识别 VS 2026**：`find-msvc-tools 0.1.5` 于 Rust 1.93 合入；本机 1.97.1 满足。
- **MSVC 仅约束 host 目标**：MSVC（link.exe）只服务 `x86_64-pc-windows-msvc` 桌面目标；Android 交叉编译走 NDK 自带 clang/lld，与 MSVC 解耦，同一 rustup 工具链下 MSVC host 与 android targets 可并存。
- **NDK 选 r29**：dioxus 官方移动端文档与社区案例均覆盖 r28/r29；r28（`28.2.13676358`）可作回退。
- **cmdline-tools 目录结构**：必须为 `cmdline-tools/latest/bin`，否则 `sdkmanager` 不可用。
- **便携与全局优先**：M6 各项全部 zip 便携装 `<仓库>\dev\`（gitignored），环境注入走 `dev-env.ps1`（会话级）+ `.cargo/config.toml`（cargo 级，无 force = 已有变量优先）——全局已装依赖的机器零介入，直接开发（机制见 [§2.6](#26-环境注入机制cargo-配置与-dev-envps1)）。
- **版本号落点约定**：本矩阵为版本号唯一事实源（[project.md §12.3](project.md#123-ci-与发布github-actions)）；安装命令内联的具体号（§2.1/§2.3）为命令参数、与矩阵同一事实，升级两处同步改（约定见 [AGENTS.md](AGENTS.md#版本号事实落点约定)）。

### 版本钉版边界

- **不钉版本**（均无独立版本事实，不入本矩阵）：

  | 组件 | 原因 |
  |---|---|
  | cargo-audit / cargo-outdated / cargo-edit | 独立 dev 工具，结果由实时 RustSec/crates.io 数据驱动，不参与构建、无配对 |
  | WebView2 | 随系统（Win11 预装），非项目可控，走系统更新 |
  | rustc / clippy / rustfmt（rustup 组件） | 已被 `rust-toolchain.toml` 1.97.1 自动锁定 |
  | platform-tools / adb（SDK 子件） | 跟 SDK 管理，无独立版本事实 |
  | 「MSVC 工具链」矩阵行 | 实际版本由 `rust-toolchain.toml` 间接钉死（1.97.1-msvc） |

- **通用规则**：钉版本 = 影响构建/运行行为，或与项目库配对（dx↔dioxus）；否则不钉。按此，上表「需要钉版本」的环境依赖（Rust / MSVC / SDK / NDK / JDK / dx）初版版本已全部确认。
- **项目 crates 依赖**（serde / thiserror / tokio / reqwest / scraper / pulldown-cmark / gix 等）：版本**不在本文档确认**，按 [project.md §12.1](project.md#121-关键设计决策) 于 M1 建 workspace 时在根 `[workspace.dependencies]` 取当时稳定版钉下；唯一例外 **dioxus 库跟随 dioxus-cli**——dx 0.7.10 ↔ dioxus 0.7.x 配对升级（§4.3）。
- **`--locked` vs `--version`**：「不钉版本」只指不钉工具自身版本号（`--version`）；依赖图可复现（`--locked`）对所有 `cargo install` 一律启用，两者独立、互不替代。

### 开发环境拓扑

> 三端分工的论证与资源预算账见 [decisions.md D-16](decisions.md#d-16-开发环境三端架构)；此处只落事实。VirtualBox 自定义目录安装坑见 [cairn/vbox-windows-install.md](../cairn/vbox-windows-install.md)。

| 端 | 角色 | 事实 |
|---|---|---|
| 宿主机 Windows 原生 | Windows 桌面端构建+验证；安卓模拟器宿主 | VirtualBox **7.2.16** r174877 @ `D:\VirtualBox` + Extension Pack；Android Studio **待装**（走 Scoop）；SDK → `D:\Software\Android\Sdk`、AVD 目录由 `ANDROID_AVD_HOME=D:\Software\Android\avd` 指定（路径已定，未落地） |
| nixos-wsl | 日常主力 Linux 环境（Remote-WSL/SSH；Android 交叉编译；产物 `adb connect` 推宿主机模拟器） | WSL2 默认发行版已就绪；开发依赖由仓库 `flake.nix` 单源管理（2026-08-25 落地：Rust 工具链单源自 `rust-toolchain.toml`、dx 0.7.10 由 shellHook 钉版、质量门禁工具、dioxus 桌面 Linux 库；**Android SDK/NDK/JDK 的声明式锁定 = M6 待办**，见 [decisions.md D-16](decisions.md#d-16-开发环境三端架构)；不走 §1 矩阵的 `<仓库>\dev\` 便携树——那是宿主机 M6 方案）；环境侧个人工具（skills-manager CLI / Starship）见 [cairn/nix-env-tooling.md](../cairn/nix-env-tooling.md)；`.wslconfig` 上限 4 核 / 6G |
| ubuntu-dev VM（备用） | 仅两个触发场景：dioxus Linux 桌面 GUI 调试、真机 USB 直通 | Ubuntu **24.04.4** Desktop（无人值守安装），4 核 / 6G 内存 / 80G 动态 VDI / EFI / USB3.0(xHCI)；配置与磁盘在 `D:\Software\VM_Resource\ubuntu-dev\`；环境同样走 `flake.nix`；环境准备操作手册见 [cairn/ubuntu-vm-setup.md](../cairn/ubuntu-vm-setup.md)。**实测网络（2026-08-30）**：仅 NIC1=NAT（无 NIC2 Host-Only，[【已替换】 NIC2=Host-Only 原规划](#nic2host-only-原规划与实测修正)），SSH = 宿主端口转发 → VM `10.0.2.15:22`（宿主侧 `127.0.0.1:2222`） |

- 备用化最小配置（首次触发使用场景前补齐）：Guest Additions、openssh-server、git、Nix multi-user、装完快照。
- ISO/extpack 等安装介质统一放 `D:\Software\VM_Resource\`。

#### NIC2=Host-Only 原规划与实测修正

> [!WARNING] 【已替换】 VM 网络原规划 NIC1=NAT + NIC2=Host-Only
>
> 原因：2026-08-30 首次 GUI 调试触发时实测，该 VM 实例只有 enp0s3（NAT），从未配置第二块网卡；SSH 一直走 NAT 端口转发（宿主 127.0.0.1:2222 → VM 10.0.2.15:22）。
>
> mdor 各里程碑（M2 镜像/M6 adb/USB 直通）均不依赖 Host-Only——NAT 转发已满足 SSH + scp + agent 会话需求。固定 IP 诉求（宿主机其他服务直访 VM）出现时再补 NIC2，届时回改本表与 cairn/ubuntu-vm-setup.md §0.2。

---

## 2. 安装步骤（按依赖顺序）

**两阶段安装总览：**

- **M0（桌面开发，当前安装）**：VS Build Tools（§2.1）→ rust-toolchain.toml（§2.2）→ dioxus-cli（§2.3）
- **M6（Android 打包前再装，暂缓）**：JDK 21（§2.4）→ Android SDK + NDK（§2.5）；另需补回 android targets 与生成 `.cargo/config.local.toml`（见 §7 过渡清单）

### 2.1 VS 2026 Build Tools（MSVC v14.50 钉版）

> 作用：为 Windows 本机 MSVC 目标提供 `link.exe`。**必须先装，否则 `cargo build` 报 link 错误。**

**两种装法（选一）：**

**A. 已验证：workload 安装 + 装完卸载 Latest（2026-08 本机实测）**

```powershell
Invoke-WebRequest https://aka.ms/vs/stable/vs_buildtools.exe -OutFile "$env:TEMP\vs_buildtools.exe"
& "$env:TEMP\vs_buildtools.exe" --quiet --wait --norestart --nocache `
  --path install="D:\VS\BuildTools" `
  --path cache="D:\VS\cache" `
  --path shared="D:\VS\shared" `
  --add Microsoft.VisualStudio.Workload.VCTools `
  --add Microsoft.VisualStudio.Component.VC.14.50.18.0.x86.x64 `
  --add Microsoft.VisualStudio.Component.Windows11SDK.26100 `
  --add Microsoft.VisualStudio.Component.VC.CMake.Project `
  --add Microsoft.Component.VC.Runtime.UCRTSDK
```

> 18.6+ 的 C++ workload 会默认带上 Latest 编译器（当前 14.51），装完与 14.50 并存。卸载 Latest：

```powershell
& "$env:TEMP\vs_buildtools.exe" modify --installPath "D:\VS\BuildTools" `
  --remove Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
  --quiet --wait --norestart
```

> 卸载后清理残留（原因见坑 3）：

```powershell
Remove-Item "D:\VS\BuildTools\VC\Tools\MSVC\14.51.36231" -Recurse -Force
Set-Content "D:\VS\BuildTools\VC\Auxiliary\Build\Microsoft.VCToolsVersion.v145.default.txt" "14.50.35717"
```

**B. 理论：不用 workload、纯组件（推荐全新安装，从源头避免 Latest）**

```powershell
Invoke-WebRequest https://aka.ms/vs/stable/vs_buildtools.exe -OutFile "$env:TEMP\vs_buildtools.exe"
& "$env:TEMP\vs_buildtools.exe" --quiet --wait --norestart --nocache `
  --path install="D:\VS\BuildTools" `
  --path cache="D:\VS\cache" `
  --path shared="D:\VS\shared" `
  --add Microsoft.VisualStudio.Component.VC.14.50.18.0.x86.x64 `
  --add Microsoft.VisualStudio.Component.Windows11SDK.26100 `
  --add Microsoft.VisualStudio.Component.VC.CMake.Project `
  --add Microsoft.Component.VC.Runtime.UCRTSDK
```

> 依赖（MSBuild、VC Runtime 等）由安装器自动解析；不装 workload 便不会引入 Latest（`VC.Tools.x86.x64` 无组件依赖它）。**未实测**，装完须核对 `VC\Tools\MSVC` 下只有 14.50。

**三个必知的坑：**

1. **`Windows11SDK.26100` 不可省**：漏装 SDK 组件时，即使手动能找到 `link.exe`，rustc 仍会报「找不到 link.exe」（2026-01 社区真实案例）。`link.exe` 在，缺的是 SDK。
2. **18.6+ 的 C++ workload 默认带上 Latest 编译器**：VS 2026 18.0 时代「只 `--add` 工作负载不会装上编译器」，18.6 起默认装 Latest（当前 14.51），与钉版 14.50 并存。装法 A 已含卸载流程；全新安装建议直接装法 B。若想用 Latest 指针而非钉版，需显式 `--add Microsoft.VisualStudio.Component.VC.Tools.x86.x64`。
3. **卸载 Latest 后残留 stub 目录与 default 标记**：无论 UI 取消勾选还是 `--remove`，`VC\Tools\MSVC\14.51.xxx` 都会剩 2 个 props 残壳（无 `link.exe`），且 `VC\Auxiliary\Build\Microsoft.VCToolsVersion.v145.default.txt` 仍指向已删版本。需手动删除空壳目录、把 `v145.default.txt` 改为 `14.50.35717`；只剩 14.50 时 Rust 的 find-msvc-tools 会回退到目录扫描。MSBuild 读 `v145\Microsoft.VCToolsVersion.VC.14.50.18.0.props`，不受影响。**注意**：`Microsoft.VCRedistVersion.default.txt` 指向最新 redist（14.51）是刻意设计（redist 跨工具集共享、v14x ABI 兼容），不要改。

**验证：**

```powershell
rustup default stable-x86_64-pc-windows-msvc   # 切换默认工具链为 MSVC
rustup run stable-x86_64-pc-windows-msvc rustc -vV   # host 应显示 x86_64-pc-windows-msvc
Get-ChildItem "D:\VS\BuildTools\VC\Tools\MSVC"        # 应只有 14.50.35717
```

新建临时工程做一次真实链接验证：

```powershell
cargo new smoke --bin
Set-Location smoke
cargo build   # 成功生成 target\debug\smoke.exe 即通过
Set-Location ..
```

> 注：本机 Scoop 的 rustup 同时装了 GNU/MSVC 工具链，GNU 保留作为备选；默认切换为 MSVC 后（`rustup default stable-x86_64-pc-windows-msvc`），GNU 不再激活。

### 2.2 rust-toolchain.toml（仓库内钉版）

仓库根目录新建 `rust-toolchain.toml`（对齐 `project.md` §12）：

```toml
[toolchain]
channel = "1.97.1"
# M6：补回 "aarch64-linux-android"、"x86_64-linux-android"
profile = "minimal"
```

- `channel` 固定 1.97.1，进入仓库目录即自动使用（需先 `rustup default stable-x86_64-pc-windows-msvc` 保证 host 为 MSVC）。
- **M0 版不列 `targets`**：桌面开发（`dx serve --platform desktop`）只需 host 的 MSVC 目标，android targets 留到 M6 打包前再补（对应 §1 阶段列 / §12 规划；armv7/i686 不需要）。
- `profile = "minimal"` 减省 rustup 组件占用。

**验证：**

```powershell
rustup show   # 显示 active toolchain = 1.97.1-x86_64-pc-windows-msvc（M0 不含 android targets）
```

> M0 阶段 `dx doctor` 提示缺 android targets 属正常（见 §2.3）。

### 2.3 dioxus-cli（dx）

```powershell
cargo install dioxus-cli --locked --version 0.7.10
```

> `--version 0.7.10` 与 §1 版本矩阵一致（版本号落点约定见 [AGENTS.md](AGENTS.md#版本号事实落点约定)）；`--locked` 的含义与加/不加区别见 [§4.1 版本锁定机制](#版本锁定机制)；首次安装会编译约数百个 crate，耗时较长。装完 `dx` 位于 `~/.cargo/bin`（Scoop 的 rustup 已在 PATH）。

**验证：**

```powershell
dx --version   # 应显示 0.7.10（与 §1 版本矩阵一致）
dx doctor    # 体检：桌面项（MSVC/WebView2）应全绿；SDK/NDK/JDK/rust android targets 缺失属正常（M6 才启用，见 §7）
```

### 2.4 JDK 21（dx 的 Gradle 侧需要）【M6 暂缓】

> M0 桌面开发不需要（`dx serve --platform desktop` 不走 Gradle）；M6 打包前再装（见 §7 过渡清单）。
> 便携为主：Temurin 21 zip 解压到 `dev\jdk\`（gitignored，随 `dev/` 树走）；Scoop/既有 `JAVA_HOME` 作兜底（`dev-env.ps1` 已有变量优先）。

**主选：Temurin 21（Adoptium）zip 便携**

```powershell
$zip = "$env:TEMP\temurin21.zip"
Invoke-WebRequest https://api.adoptium.net/v3/binary/latest/21/ga/windows/x64/jdk/hotspot/normal/eclipse -OutFile $zip
Expand-Archive $zip -DestinationPath "dev\jdk" -Force
. .\dev\dev-env.ps1        # 会话变量：dev\jdk 存在则设置 JAVA_HOME
& "$env:JAVA_HOME\bin\java.exe" -version   # 输出 openjdk 21.x
```

**备选：Scoop 管理（兜底）**

```powershell
scoop bucket add java
scoop install temurin21-jdk
# JAVA_HOME 由 dev-env.ps1 保留既有值；或按需设 User 级变量
```

> 不写死机器路径：`dev-env.ps1` 运行时扫描 `dev\jdk\` 取已装版本；升级只需替换目录内容。

### 2.5 Android SDK + NDK【M6 暂缓】

> M0 桌面开发不需要；M6 打包前再装（见 §7 过渡清单）。
> 便携安装到仓库内 `dev\android\`（gitignored）；**不写任何持久环境变量**——注入走 [§2.6](#26-环境注入机制cargo-配置与-dev-envps1)（`dev-env.ps1` 会话级 + `.cargo/config.toml` cargo 级）。

```powershell
$sdkRoot = Join-Path $PWD 'dev\android'
New-Item -ItemType Directory -Path "$sdkRoot\cmdline-tools" -Force | Out-Null

# 1) 下载并解压 cmdline-tools（版本号 14742923 为 2026-03 最新）
$zip = "$env:TEMP\cmdline-tools.zip"
Invoke-WebRequest https://dl.google.com/android/repository/commandlinetools-win-14742923_latest.zip -OutFile $zip
Expand-Archive $zip -DestinationPath "$sdkRoot\cmdline-tools" -Force
# 目录结构必须为 cmdline-tools\latest\bin
Rename-Item "$sdkRoot\cmdline-tools\cmdline-tools" "$sdkRoot\cmdline-tools\latest" -Force

# 2) 注入会话变量（先装 JDK §2.4，sdkmanager 依赖 JAVA_HOME）
. .\dev\dev-env.ps1
```

**安装 SDK 组件：**

```powershell
sdkmanager "platform-tools" "platforms;android-36" "build-tools;36.0.0" "ndk;29.0.14206865"
# 接受 license：
# yes | sdkmanager --licenses
```

> 若 sdkmanager 报 Java 相关错误，确认 `dev-env.ps1` 已生效（§2.4）。组件版本可用 `sdkmanager --list` 查询最新。

**验证：**

```powershell
adb --version                              # 平台工具可用
ls "$env:ANDROID_NDK_HOME\toolchains\llvm\prebuilt\windows-x86_64\bin\aarch64-linux-android30-clang.cmd"
```

> **卸载**：删 `dev\android` 即回收全部（无持久变量/注册残留）。

### 2.6 环境注入机制（cargo 配置与 dev-env.ps1）

> Android 工具链不写系统环境变量；cargo 与 dx 两条注入路径互补，均「全局优先、便携兜底」——全局已装依赖时零介入，直接开发。

**路径 A — cargo 级：`.cargo/config.toml`（已提交，机器无关）**

- `[env] ANDROID_HOME = { value = "dev/android", relative = true }`：相对仓库根解析为绝对路径，注入 cargo 及子进程（build script / rustc / rust-analyzer）。
- **无 `force`**：已存在的环境变量不被覆盖 → 全局依赖机 / CI 惰性；`dev/` 未创建时指向不存在路径亦无害。
- 版本化 NDK 路径与 Windows linker 在 gitignored `.cargo/config.local.toml`（`Copy-Item` 自 `.example` 生成，见 §7 第 4 步）。

**路径 B — dx/Gradle 级：`dev/dev-env.ps1`（已提交）**

- dx 的 Gradle 侧不读 cargo config，需进程环境变量：`dev-env.ps1` 按 `dev\` 存在与否决定注入或保留已有变量（`JAVA_HOME` / `ANDROID_HOME` / NDK 扫描已装版本）。
- 用法：`. .\dev\dev-env.ps1` 只在当前会话生效，关终端即无痕迹。

---

## 3. M0 验收

`project.md` §10 M0 验收标准：

```powershell
cd crates/mdor-app   # Dioxus.toml 所在处；dx serve 从当前目录查找配置（0.7 无 --project/-p 选项）
dx serve --platform desktop
```

跑通即表示：workspace 可编译、wry/WebView2 可用、MSVC 链接正常。Android 侧（`dx build` / `dx serve --platform android`）延后至 M6，见 §5 与 §7 过渡清单。

---

## 4. 依赖升级

> 原则：**版本约束只钉在根 `[workspace.dependencies]` 一处**（`project.md` §12.1）；升级后必须 `cargo test` + `cargo audit` 验证。

### 4.1 Rust 依赖（crates）— 最常见

**常用工具（按需安装）：**

```powershell
cargo install cargo-outdated --locked   # 查看依赖新版本：cargo outdated
cargo install cargo-edit --locked       # 升 Cargo.toml 版本约束：cargo upgrade
cargo install cargo-audit --locked      # 漏洞扫描：cargo audit（§12.1 已定为必跑）
```

**升级流程：**

```powershell
cargo update                # 只升 Cargo.lock 内小版本，尊重 Cargo.toml 约束
cargo update -p gix         # 单升某个 crate
cargo upgrade --dry-run     # 预览 Cargo.toml 约束可升到多少
cargo test                  # 全量回归
cargo audit                 # 漏洞扫描，退出码非 0 即存在已知漏洞
```

- **小版本（patch/minor）**：`cargo update` 自动跟进即可；`rustls` 这类随 reqwest 走的传递依赖也只需 `cargo update`，不需手动挑版本（`diff.md` §1.6）。
- **大版本（破坏性）**：需改根 `[workspace.dependencies]` 钉版 + 人工核对 breaking changes（CHANGELOG / migration guide），逐 crate 验证。
- **重复版本排查**：APK 体积敏感时 `cargo tree -d` 查 multiple-versions，按需处理。

#### 版本约束落根工作流（workspace 继承）

member 依赖一律 `{ workspace = true }` 引用根表，**版本号只落根 `[workspace.dependencies]` 一处**（§4 原则；[D-12](decisions.md#d-12-依赖与安全审计)）。

- **`cargo add` 不能直接钉根表**：无 `--workspace` 支持（rust-lang/cargo #11527 / #16797，2026-03 仍未实现），只能 `cargo add dep -p <member>`（workspace 根为 virtual manifest 时**不带 `-p` 会直接报错** `could not determine which package to modify`，不会写任何文件），且会把具体版本号直接写进该 member 的 `[dependencies]`——版本会散到各 member，违背钉根约定。故 `cargo add` 只用来**探测当前稳定版号**，版本落根靠手动写根表。
- **推荐流程**：
  1. `cargo add <dep> -p <member>` 探测稳定版号（只取版本信息；member 那行随后改回 `{ workspace = true }`）；
  2. 手动把 `dep = "<版本>"` 写进根 `[workspace.dependencies]`；
  3. member 侧改为 `dep = { workspace = true }`（member 需要专属 features 时写 `dep = { workspace = true, features = [...] }`）。
- **workspace 继承的两个 Cargo 语义约束**：根 `[workspace.dependencies]` 不能声明 `optional`；`features` 是 additive（member 侧可追加，但 member 侧不能重写 `version`/`registry`/`path` 等键）。
- **virtual workspace 须显式设 `resolver`**：根 `[workspace]` 写 `resolver = "3"`（edition 2024 默认、Rust 1.84+）；resolver 为全局项、member 里写无效——普通包由 `edition` 推断，virtual workspace 无 `[package]` 无法推断，不写直接报错。

#### 版本锁定机制

`--locked` 的含义与机制，分 `cargo install`（全局工具）与项目构建两类：

- **含义**：`--locked` = 要求依赖解析结果与 lockfile **完全一致**，不一致直接报错终止，不静默换版本。
- **`cargo install --locked`（全局工具）**：发布包自带发布者提交的 `Cargo.lock`（`cargo publish` 时打进包）。不加 `--locked`：Cargo 忽略包内 lockfile、**重新解析**取当时最新兼容版；加 `--locked`：强制用包内 lockfile 的精确版本。工具自身版本用 `--version <号>` 钉、依赖图用 `--locked` 钉，两者结合 = 跨环境可复现（对齐 §2.3 `dx` 0.7.10）。
- **`cargo build --locked`（项目）**：用**仓库提交**的 `Cargo.lock`（应用项目须提交 lock）；别处 clone 后 `--locked` 构建即拉取完全相同的依赖版本。

| | 不加 `--locked` | 加 `--locked` |
|---|---|---|
| 依赖版本 | 重新解析，取当时最新兼容版 | 用 lockfile 精确版本 |
| 与 lock 不一致 | 静默按新解析 | 报错终止 |
| 可复现性 | 否（不同时间/环境可能不同） | 是 |

> 两套 lockfile 是独立机制，勿混：工具 install 用**包自带** lockfile，项目构建用**仓库提交**的 `Cargo.lock`。

### 4.2 工具链 / 环境 — 低频率

| 对象 | 操作 | 注意 |
|---|---|---|
| **rustc 1.97.1** | 改 `rust-toolchain.toml` 的 `channel` → `rustup update` | 项目用 edition 2024；升大版本前查各依赖 MSRV |
| **VS 2026 / MSVC v14.50（钉版）** | 无需主动升，VS 自动更新不影响构建 | 要升 MSVC 时改 §2.1 组件 ID 与 §1 矩阵 |
| **Android SDK/NDK** | `sdkmanager --list` 查新版，按需升 | NDK 大版本影响 ring/gix 交叉编译，谨慎；先查 [Revision History](https://developer.android.com/ndk/downloads/revision_history) |
| **JDK** | 替换 `dev\jdk` 目录内容 / `scoop update temurin21-jdk` | 与 dx 的 Gradle 侧兼容性验证 |

### 4.3 框架 — Dioxus / dx（必须同步）

- **dx 与 dioxus 库版本必须一起升**：dioxus 库改 `Cargo.toml`，dx 执行 `cargo install dioxus-cli --locked`，然后 `dx doctor` 校验。
- **大版本（0.7 → 0.8 等）**：等官方迁移指南，先升 dioxus 库再升 dx，二者版本须匹配；`dx serve --platform desktop` 冒烟。
- 升级顺序：**先看发版说明 → 升库 → 升 dx → dx doctor → 桌面冒烟 → 按 §4.1 回归**。

### 4.4 升级后必跑清单

```powershell
cargo test        # 全量测试
cargo audit       # 漏洞扫描
dx doctor         # 工具链体检（框架/环境升级后）
```

---

## 5. 体积与精简选项

工具链占用按两阶段估算：

**M0（桌面开发，当前安装）≈ 7–10 GB：**

| 组件 | 占用 |
|---|---|
| VS 2026 Build Tools（v14.50 + SDK） | 6–8 GB |
| dioxus-cli + cargo 依赖 | 0.5–1 GB |
| **M0 小计** | **≈ 7–10 GB** |

**M6（Android 打包，追加安装）≈ 5–7 GB：**

| 组件 | 占用 |
|---|---|
| Android NDK r29 | 4–6 GB（最大单件） |
| Android SDK 其余（cmdline-tools + platform-tools + platform/build-tools） | 0.5–0.8 GB |
| JDK 21 | ~0.3 GB |
| **M6 追加小计** | **≈ 5–7 GB** |

精简建议（按需取舍，标注适用阶段）：

1. **（M0）只装桌面所需**：按 §2 两阶段执行——M0 只装 VS Build Tools + dx；JDK/SDK/NDK 留到 M6 打包前再装，省 5–7 GB。
2. **（M6）ABI 按场景取舍**：**本地模拟器 debug 编 arm64-v8a + x86_64**（双 ABI，仍单 APK：模拟器跑 x86_64、真机跑 arm64）；**release / 纯真机调试只编 arm64-v8a**（单 ABI 最省）。`rustup target add` 两个 target 都装（对齐 §7 toml 补回），release 构建只用 arm64。
3. **（M6）不装模拟器**：开发期直接连真机（`dx serve --platform android --device`），省 emulator + 系统镜像 2.5–3.5 GB。
4. **（M0）VS 缓存目录清理**：`D:\VS\cache` 可在安装后删除（`--nocache` 已抑制本轮缓存）。
5. **（M6）位置与回收**：M6 各项装 `<仓库>\dev\`（gitignored）；删 `dev\` 树即全部回收（SDK/NDK/JDK ≈ 5–7 GB），无持久变量/注册残留（见 [§2.6](#26-环境注入机制cargo-配置与-dev-envps1)）。

---

## 6. 故障排查速查

| 症状 | 原因 | 处理 |
|---|---|---|
| `cargo build` 报找不到 link.exe / MSVC 链接失败 | VS Build Tools 未装 / MSVC 工具链未激活 | 重跑 §2.1；`rustup show` 确认 host 为 msvc |
| `link.exe` 找得到但 rustc 仍报错 | 缺 Windows 11 SDK 组件 | 确认 `Windows11SDK.26100` 已 `--add`（§2.1） |
| `sdkmanager` 无法启动 | JAVA_HOME 未生效 | `. .\dev\dev-env.ps1` 注入会话变量；确认 §2.4 |
| `. .\dev\dev-env.ps1` 报「此系统上禁止运行脚本」 | PowerShell 执行策略限制 | `Set-ExecutionPolicy RemoteSigned -Scope CurrentUser`（User 级）后重试 |
| `cargo check --target aarch64-linux-android` 报找不到 NDK / 链接错误 | 本地 `.cargo/config.local.toml` 未生成或版本不符 | `Copy-Item` 自 `.example` 生成并核对版本（§7 第 4 步）；或确认全局 `ANDROID_NDK_HOME` |
| `dx build --android` 链接报乱码/参数过长 | 旧版 dx 的 Windows 链接器代理 bug | 升级 dioxus-cli ≥ 0.7.1（PR #4126 已修） |
| `dx doctor` 提示缺 android target / SDK / NDK / JDK | **M0 阶段属正常**（Android 侧 M6 才启用） | M0 不必处理；M6 时按 §7 过渡清单补装并改回 `rust-toolchain.toml` |
| Android 启动崩溃 `NoSuchMethodError getCurrentWindowMetrics` | 真机 API < 30 | 在 `Dioxus.toml` 设 `min_sdk_version = 30` |
| WSLg 桌面跑 dioxus 报 `libEGL`/`MESA-ZINK failed to choose pdev` 警告 | WSLg 无 GPU 直通，走软件渲染回退（zink） | 属噪声非错误，窗口正常渲染；渲染验证用日志（`RUST_LOG=mdor_core=debug` 见 store 读取）或宿主肉眼 |
| WSLg 截图失败（grim 报合成器不支持 wlr-screencopy / imagemagick `x:` 无 X11 delegate / 缺 xwd·scrot） | WSLg 合成器未实现屏幕捕获协议、nixpkgs imagemagick 未编 X11 delegate | 视觉验证改 ubuntu-dev VM / Windows 宿主；或接受日志证据（渲染时读取 library.json） |
| VM 桌面跑 dioxus 报 `Could not create default EGL display: EGL_BAD_PARAMETER`，WebKitWebProcess 反复自杀（黑窗/空白窗） | VirtualBox 显卡 VMSVGA 模拟 VMware（`vmwgfx`），nix mesa ≥25 已移除该 DRI 驱动 → WebKit GPU 进程 GBM 平台 EGL 初始化崩溃 | 启动前注入（devShell 内）：`WEBKIT_DISABLE_COMPOSITING_MODE=1 GSK_RENDERER=cairo LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe __EGL_VENDOR_LIBRARY_FILENAMES=<nix-mesa>/share/glvnd/egl_vendor.d/50_mesa.json WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1`；详见 [cairn/ubuntu-vm-setup.md §GUI 调试](../cairn/ubuntu-vm-setup.md) |
| VM 内 `gnome-screenshot` 报 AccessDenied（D-Bus org.gnome.Shell.Screenshot） | GNOME 41+ 截图接口仅放行授权调用方 | `sudo apt install gnome-screenshot`（mutter 白名单工具），`gnome-screenshot -f <path>`；SSH 会话须带 `XDG_RUNTIME_DIR=/run/user/1000` |

---

## 7. M0 到 M6 过渡清单（补回 Android 侧）

M0 验收通过、进入 M6 Android 打包前，按序补齐（便携方案，全部落 `<仓库>\dev\`，不写持久变量）：

```powershell
# 1) rust-toolchain.toml 补回 android targets（去掉 M0 注释，恢复 targets 数组）
#    targets = ["aarch64-linux-android", "x86_64-linux-android"]
rustup target add aarch64-linux-android x86_64-linux-android

# 2) 装 JDK 21（§2.4）：Temurin 21 zip → dev\jdk\（Scoop 兜底）

# 3) 装 Android SDK + NDK（§2.5，全落 dev\android\）
. .\dev\dev-env.ps1    # 会话变量（ANDROID_HOME/ANDROID_NDK_HOME/JAVA_HOME/PATH）
sdkmanager "platform-tools" "platforms;android-36" "build-tools;36.0.0" "ndk;29.0.14206865"
yes | sdkmanager --licenses

# 4) 生成本地 .cargo 覆盖（版本化 NDK 路径 + Windows linker，gitignored）
Copy-Item .cargo\config.local.toml.example .cargo\config.local.toml
# 编辑核对版本号与 env.md §1 一致；非 Windows 或全局已有 linker 的机器可删 [target.*] 段

# 5) 体检全绿
dx doctor    # SDK/NDK/JDK/rust targets 不再缺项
cargo check --target aarch64-linux-android -p mdor-app   # 交叉冒烟（含 ring 的 NDK clang 构建）
```

> 完成即回到 §3 的 Android 侧验证（`dx build --platform android --target aarch64-linux-android`）。
> 卸载：删 `dev\` 树 + `rustup target remove`，无持久变量/注册残留（见 [§2.6](#26-环境注入机制cargo-配置与-dev-envps1)）。

---

## 8. 记录

- 2026-08-25：`flake.nix` 落地（D-16「环境复现单源」）——Rust 工具链改用 `rust-bin.fromRustupToolchainFile` 复用 `rust-toolchain.toml` 单一事实源（钉 1.97.1，M6 补 targets 后自动生效）；devShell 补齐 dioxus 桌面 Linux 构建库（webkitgtk_4_1/gtk3/libsoup_3/gdk-pixbuf，服务 ubuntu-dev VM 的 GUI 调试场景）、质量门禁工具（cargo-audit/outdated/edit）；dx 0.7.10 由 shellHook `cargo install --locked` 钉版精确对齐 §1 矩阵。Android SDK/NDK/JDK 的 flake 声明式锁定定为 **M6 待办**（声明即安装、M0 用不到，且 rust targets 亦 M6 才补）。

- 2026-08-24：§1 新增「开发环境拓扑」小节——三端架构（宿主机原生 / nixos-wsl 主力 / ubuntu-dev VM 备用）落地事实：VirtualBox 7.2.16 @ `D:\VirtualBox`、ubuntu-dev（24.04.4 无人值守装好）配置、Android Studio/SDK/AVD 的 D 盘路径；决策留痕 [decisions.md D-16](decisions.md#d-16-开发环境三端架构)。

- 2026-08-20：定稿便携 Android 工具链方案——SDK/NDK/JDK 全部 zip 便携装 `<仓库>\dev\`（gitignored）；环境注入改会话级 `dev-env.ps1` + cargo 级 `.cargo/config.toml`（相对路径、无 force、已提交），版本化 NDK 与 Windows linker 走 gitignored `config.local.toml`（模板 `.example`）；JDK 定 Temurin 21；「全局优先、便携兜底」，全局依赖机零介入。相关改动：§1 矩阵路径、§2.4、§2.5、新增 §2.6、§4.2、§5、§6、§7。
- 2026-08-09：初始化本文档；本机环境核对（Rust 1.97.1 / MSVC 工具链已装未激活 / dx、JDK、Android SDK、NDK 待装 / VS Build Tools 待装）。新增 §4 依赖升级策略（Rust 依赖 / 工具链 / Dioxus 框架 / 升级后必跑清单）。
- 2026-08-09：MSVC 钉版实操——18.6+ workload 默认带 Latest（14.51）致与 14.50 并存，卸载 Latest 后清理 stub 目录 + `v145.default.txt` 残留；smoke 构建（1.97.1-msvc）验证链接走 14.50；默认工具链切至 `stable-x86_64-pc-windows-msvc`。§2.1 增补装法 B（纯组件，理论）。
- 2026-08-09：环境按 M0/M6 两阶段拆分——§1 矩阵加阶段列；§2 顶部加安装总览并重排（§2.3 dx / §2.4 JDK / §2.5 SDK+NDK，后两者标「M6 暂缓」）；§2.2 改 M0 版 toml（去 android targets）；§3 验收删 Android 试用段并保留 M6 指引；§5 拆两阶段体积表；§6 更新 dx doctor 行；新增 §7 过渡清单。
- 2026-08-09：已评估「Zig 替代 C 编译工具链」——Android 侧仅能替换编译器、仍需 NDK 的 bionic sysroot/platform libs（省不掉 NDK），且偏离 dioxus/dx 官方 Gradle 管线；host 侧 `zig cc -target x86_64-windows-msvc` 为非标组合，依赖 find-msvc-tools 的 build.rs 会失败。结论：不采纳，维持 MSVC host + M6 补 NDK 官方路径。同日本文档补充：§1「MSVC 仅约束 host」解耦说明；§5 精简建议第 2 条改 ABI 场景取舍（本地模拟器 debug 双 ABI / release 单 arm64）。
- 组件版本以官方渠道为准，升级前先查 `sdkmanager --list` 与 [NDK Revision History](https://developer.android.com/ndk/downloads/revision_history)。
