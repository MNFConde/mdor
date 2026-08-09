# mdor M0 环境搭建文档

> 移动端 mdBook 离线阅读器 · Android · Rust + Dioxus
> 对齐 `project.md` §10（M0 里程碑）与 §12（构建配置）
> 平台：Windows（PowerShell 5.1+）· 本机 2026-08 实测

---

## 1. 环境总览与版本矩阵

| 组件 | 版本 | 用途 | 安装位置 |
|---|---|---|---|
| Rust | 1.97.1（`rust-toolchain.toml` 钉版） | 主语言 | Scoop rustup 管理 |
| MSVC 工具链 | `stable-x86_64-pc-windows-msvc` | Windows 本机构建/link | Scoop rustup 管理 |
| Android targets | `aarch64-linux-android` / `x86_64-linux-android` | Android 交叉编译（arm64-v8a / x86_64） | rustup target |
| **VS 2026 Build Tools** | MSVC **v14.50**（钉版）+ Win11 SDK 10.0.26100 | `link.exe` / Windows 本机链接 | `D:\VS\BuildTools` |
| JDK | Microsoft OpenJDK 21 | dx 的 Gradle 侧 | `C:\Program Files\Microsoft\jdk-21...` |
| Android SDK | cmdline-tools **14742923** + platform-tools + `platforms;android-36` + build-tools | Android 构建基座 | `D:\Android\Sdk` |
| Android NDK | **r29**（`29.0.14206865`） | C/C++ 交叉编译（gix/ring 等） | `D:\Android\Sdk\ndk` |
| dioxus-cli | 0.7.x（cargo install） | `dx serve/build` | `~/.cargo/bin` |
| WebView2 | 随系统 | `dx serve --platform desktop` 渲染 | 预装（Win11） |

### 版本对齐说明

- **MSVC 版本与 VS 解耦**：VS 2026 中 MSVC 组件 ID 带独立版本号。本项目**钉 v14.50**（`VC.14.50.18.0.x86.x64`），与 `rust-toolchain.toml` 固定 1.97.1 的一致性策略一致；不随 VS 更新自动跳变。
- **Rust ≥ 1.93 才能识别 VS 2026**：`find-msvc-tools 0.1.5` 于 Rust 1.93 合入；本机 1.97.1 满足。
- **NDK 选 r29**：dioxus 官方移动端文档与社区案例均覆盖 r28/r29；r28（`28.2.13676358`）可作回退。
- **cmdline-tools 目录结构**：必须为 `cmdline-tools/latest/bin`，否则 `sdkmanager` 不可用。

---

## 2. 安装步骤（按依赖顺序）

### 2.1 VS 2026 Build Tools（MSVC v14.50 钉版）

> 作用：为 Windows 本机 MSVC 目标提供 `link.exe`。**必须先装，否则 `cargo build` 报 link 错误。**

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

**两个必知的坑：**

1. **`Windows11SDK.26100` 不可省**：漏装 SDK 组件时，即使手动能找到 `link.exe`，rustc 仍会报「找不到 link.exe」（2026-01 社区真实案例）。`link.exe` 在，缺的是 SDK。
2. **`VC.Tools.x86.x64`（Latest 指针）是 Recommended 非 Required**：只 `--add` 工作负载不会装上编译器。本命令显式钉了 `VC.14.50.18.0.x86.x64`，若改回 Latest 需显式加 `--add Microsoft.VisualStudio.Component.VC.Tools.x86.x64`。

**验证：**

```powershell
rustup default stable-x86_64-pc-windows-msvc   # 切换默认工具链为 MSVC
rustup run stable-x86_64-pc-windows-msvc rustc -vV   # host 应显示 x86_64-pc-windows-msvc
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
targets = [
    "aarch64-linux-android",
    "x86_64-linux-android",
]
profile = "minimal"
```

- `channel` 固定 1.97.1，进入仓库目录即自动使用（需先 `rustup default stable-x86_64-pc-windows-msvc` 保证 host 为 MSVC）。
- `targets` 仅列 arm64-v8a / x86_64（对应 §12 规划）；armv7/i686 不需要。
- `profile = "minimal"` 减省 rustup 组件占用。

**验证：**

```powershell
rustup show   # 显示 active toolchain = 1.97.1-x86_64-pc-windows-msvc，且两个 android target 已安装
```

### 2.3 JDK 21（dx 的 Gradle 侧需要）

```powershell
winget install --id Microsoft.OpenJDK.21 --source winget --accept-package-agreements --accept-source-agreements
[Environment]::SetEnvironmentVariable('JAVA_HOME', 'C:\Program Files\Microsoft\jdk-21.0.x.x-hotspot', 'User')
```

> `JAVA_HOME` 路径以实际安装版本为准（`C:\Program Files\Microsoft\jdk-*`）。设 User 级即可，无需管理员。

**验证（新开终端）：**

```powershell
java -version   # 输出 openjdk 21.x
echo $env:JAVA_HOME
```

### 2.4 Android SDK + NDK

> 根目录统一放 `D:\Android\Sdk`，与其余大件（VS、JSB）同盘。

```powershell
$sdkRoot = 'D:\Android\Sdk'
New-Item -ItemType Directory -Path "$sdkRoot\cmdline-tools" -Force | Out-Null

# 1) 下载并解压 cmdline-tools（版本号 14742923 为 2026-03 最新）
$zip = "$env:TEMP\cmdline-tools.zip"
Invoke-WebRequest https://dl.google.com/android/repository/commandlinetools-win-14742923_latest.zip -OutFile $zip
Expand-Archive $zip -DestinationPath "$sdkRoot\cmdline-tools" -Force
# 目录结构必须为 cmdline-tools\latest\bin
Rename-Item "$sdkRoot\cmdline-tools\cmdline-tools" "$sdkRoot\cmdline-tools\latest" -Force

# 2) 设置环境变量（User 级）
[Environment]::SetEnvironmentVariable('ANDROID_HOME', $sdkRoot, 'User')
[Environment]::SetEnvironmentVariable('NDK_HOME',  "$sdkRoot\ndk\29.0.14206865", 'User')
[Environment]::SetEnvironmentVariable('ANDROID_NDK_HOME', "$sdkRoot\ndk\29.0.14206865", 'User')
```

**PATH 追加**（User 级）：

```powershell
$sdkTools = "$sdkRoot\cmdline-tools\latest\bin"
$platformTools = "$sdkRoot\platform-tools"
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
[Environment]::SetEnvironmentVariable('Path', "$userPath;$sdkTools;$platformTools", 'User')
```

**安装 SDK 组件**（新开终端使环境变量生效）：

```powershell
sdkmanager "platform-tools" "platforms;android-36" "build-tools;36.0.0" "ndk;29.0.14206865"
# 接受 license：
# yes | sdkmanager --licenses
```

> 若 sdkmanager 报 Java 相关错误，确认 JAVA_HOME 已生效（2.3 节）。组件版本可用 `sdkmanager --list` 查询最新。

**验证：**

```powershell
adb --version                              # 平台工具可用
ls "$env:ANDROID_NDK_HOME\toolchains\llvm\prebuilt\windows-x86_64\bin\aarch64-linux-android24-clang.cmd"
```

### 2.5 dioxus-cli（dx）

```powershell
cargo install dioxus-cli --locked
```

> 首次安装会编译约数百个 crate，耗时较长。装完 `dx` 位于 `~/.cargo/bin`（Scoop 的 rustup 已在 PATH）。

**验证：**

```powershell
dx --version
dx doctor    # 体检：SDK/NDK/JDK/rust targets/WebView2 全绿
```

---

## 3. M0 验收

`project.md` §10 M0 验收标准：

```powershell
dx serve --platform desktop
```

跑通即表示：workspace 可编译、wry/WebView2 可用、MSVC 链接正常。桌面跑通后 Android 侧可试：

```powershell
dx build --platform android --target aarch64-linux-android
```

（`dx serve --platform android` 需要模拟器/真机，见 §5 体积选项。）

---

## 4. 依赖升级

> 原则：**版本约束只钉在根 `[workspace.dependencies]` 一处**（`project.md` §12.1）；升级后必须 `cargo test` + `cargo audit` 验证。

### 4.1 Rust 依赖（crates）— 最常见

**常用工具（按需安装）：**

```powershell
cargo install cargo-outdated   # 查看依赖新版本：cargo outdated
cargo install cargo-edit       # 升 Cargo.toml 版本约束：cargo upgrade
cargo install cargo-audit      # 漏洞扫描：cargo audit（§12.1 已定为必跑）
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

### 4.2 工具链 / 环境 — 低频率

| 对象 | 操作 | 注意 |
|---|---|---|
| **rustc 1.97.1** | 改 `rust-toolchain.toml` 的 `channel` → `rustup update` | 项目用 edition 2024；升大版本前查各依赖 MSRV |
| **VS 2026 / MSVC v14.50（钉版）** | 无需主动升，VS 自动更新不影响构建 | 要升 MSVC 时改 §2.1 组件 ID 与 §1 矩阵 |
| **Android SDK/NDK/JDK** | `sdkmanager --list` 查新版，按需升 | NDK 大版本影响 ring/gix 交叉编译，谨慎；先查 [Revision History](https://developer.android.com/ndk/downloads/revision_history) |
| **JDK** | winget upgrade | 与 dx 的 Gradle 侧兼容性验证 |

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

M0 全量工具链占用估算：

| 组件 | 占用 |
|---|---|
| VS 2026 Build Tools（v14.50 + SDK） | 6–8 GB |
| Android NDK r29 | 4–6 GB（最大单件） |
| Android SDK 其余 | 0.5–0.8 GB |
| JDK 21 | ~0.3 GB |
| dioxus-cli + cargo 依赖 | 0.5–1 GB |
| **合计** | **≈ 12–16 GB** |

精简建议（按需取舍）：

1. **只做桌面开发可暂缓 NDK**：M0 验收只要 `dx serve --platform desktop`；NDK 延后到 M6 打包再装，省 4–6 GB。
2. **只装单 ABI**：`dx build --android --target aarch64-linux-android` 只编 arm64，不必装多余 build-tools 平台。
3. **不装模拟器**：开发期直接连真机（`dx serve --platform android --device`），省 emulator + 系统镜像 2.5–3.5 GB。
4. **VS 缓存目录清理**：`D:\VS\cache` 可在安装后删除（`--nocache` 已抑制本轮缓存）。

---

## 6. 故障排查速查

| 症状 | 原因 | 处理 |
|---|---|---|
| `cargo build` 报找不到 link.exe / MSVC 链接失败 | VS Build Tools 未装 / MSVC 工具链未激活 | 重跑 §2.1；`rustup show` 确认 host 为 msvc |
| `link.exe` 找得到但 rustc 仍报错 | 缺 Windows 11 SDK 组件 | 确认 `Windows11SDK.26100` 已 `--add`（§2.1） |
| `sdkmanager` 无法启动 | JAVA_HOME 未生效 | 新开终端；确认 §2.3 |
| `dx build --android` 链接报乱码/参数过长 | 旧版 dx 的 Windows 链接器代理 bug | 升级 dioxus-cli ≥ 0.7.1（PR #4126 已修） |
| `dx doctor` 提示缺 android target | rust-toolchain.toml 未生效 | 在仓库根目录运行；`rustup target list --installed` 核对 |
| Android 启动崩溃 `NoSuchMethodError getCurrentWindowMetrics` | 真机 API < 30 | 在 `Dioxus.toml` 设 `min_sdk_version = 30` |

---

## 7. 记录

- 2026-08-09：初始化本文档；本机环境核对（Rust 1.97.1 / MSVC 工具链已装未激活 / dx、JDK、Android SDK、NDK 待装 / VS Build Tools 待装）。新增 §4 依赖升级策略（Rust 依赖 / 工具链 / Dioxus 框架 / 升级后必跑清单）。
- 组件版本以官方渠道为准，升级前先查 `sdkmanager --list` 与 [NDK Revision History](https://developer.android.com/ndk/downloads/revision_history)。
