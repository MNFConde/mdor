---
type: project_topic
status: active
summary: "Cargo 配置 `.cargo/config.toml` 的查找层级、`[env]` 的 relative/force 语义，以及 mdor 采用「提交相对路径 config + gitignored 本地覆盖」注入 Android 工具链环境；坑：linker 的 OS 相关性、版本号不进提交文件、dx/Gradle 不读 cargo config"
tags: [mdor, cargo, config, android, toolchain, rust]
contains: [lesson, procedure, decision]
created: "2026-08-20"
updated: "2026-08-21"
related: [env.md, windows-msvc-toolchain.md, android-cross-compile-rust.md, cargo-workspace-resolver.md]
authoring_mode: ai_generated
---
# Cargo 配置 `.cargo/config.toml`（相对路径 + 提交模式）

## 背景

mdor 的 Android 工具链（SDK/NDK/JDK）采用「便携 + 会话变量」隔离方案（见 `doc/env.md` §2.6）。cargo 侧的环境注入不依赖系统环境变量，而是通过仓库内 `.cargo/config.toml` 完成——用**相对路径**（机器无关）与**无 force**（全局优先）两条性质，使该配置可以安全提交进 git。完整操作步骤见 `doc/env.md`；本节只留机制与坑。

## 当前结论

- **查找层级**：cargo 从当前目录逐级向上找 `.cargo/config.toml`，最后合并 `$CARGO_HOME/config.toml`（用户级）；项目级优先于用户级。`.toml` 后缀为推荐形式（1.39+），旧 `config` 无后缀仍兼容。
- **`[env]` 注入范围**：给 cargo 及其子进程（build script、rustc、`cargo run`、以及 rust-analyzer 内嵌的 cargo）设置环境变量。
- **`relative = true`**：值相对**含 config.toml 的 `.cargo` 目录之父目录**（即仓库根）解析，注入的环境变量为解析后的**绝对路径**。例：`<repo>/.cargo/config.toml` 中 `ANDROID_HOME = { value = "dev/android", relative = true }` → `<repo>/dev/android`。
- **无 `force`（默认）**：已存在于进程环境中的变量**不被覆盖** → 全局已装依赖的机器上本配置零介入（惰性）；CI 已设变量时同样不打架 → **提交安全**。
- **`include`**：`include = [{ path = "config.local.toml", optional = true }]` 加载额外配置，路径相对包含它的 config 所在目录（即 `.cargo/`）；`optional` 缺失时静默跳过。合并规则：先加载 include 文件，再叠加本文件自身值（本文件胜）。
- **mdor 提交模式**：机器无关的 `[env]`（仅 `ANDROID_HOME`，版本无关）提交；**版本化 NDK 路径与 OS 相关的 linker 放 gitignored 的 `.cargo/config.local.toml`**（提交 `.example` 模板）。
- **dx/Gradle 不读 cargo config**：dx 的 Gradle 侧需要 `JAVA_HOME`/`ANDROID_HOME` 等进程环境变量 → 走会话级 `dev/dev-env.ps1`（全局优先、便携兜底），与 cargo 配置互补。

## 教训

1. **linker 是 OS 相关，勿提交**：`[target.aarch64-linux-android].linker` 在 Windows 是 NDK 的 `.cmd` 包装（`aarch64-linux-android30-clang.cmd`，API 30 对齐项目 min_sdk）。提交后 Linux 克隆/CI 的完整链接（`dx build` release 级）会引用不存在的 `.cmd` → 只放 gitignored 本地覆盖；`cargo check` 不链接所以不受影响。
2. **版本号不进提交文件（L2 单源）**：版本号事实源 = `doc/env.md` §1。NDK 的版本化路径（`ndk/29.0.14206865`）进 gitignored `config.local.toml`，提交版 config 只保留版本无关的 `ANDROID_HOME`。升级 NDK 时只需改本地覆盖 + env.md，不碰提交文件。
3. **`[env]` 只服务 cargo 子进程**：不能靠它喂 dx/Gradle；会话变量脚本才是 dx 路径的注入点。
4. **rust-analyzer 继承环境**：rust-analyzer 读 `.cargo/config.toml` 且继承启动它的进程环境 → Android target 分析（`cargo check --target aarch64-linux-android`）在 M6 后无需「从开发终端启动 IDE」。M0–M5 纯桌面/core 分析不需要这些变量。
5. **`relative=true` 基准是 `.cargo` 父目录**：是仓库根，不是 `.cargo/` 自身——写 `dev/android` 而非 `../dev/android`。
6. **提交 `[env]` 在 dev/ 未创建时指向不存在路径**：`relative=true` 只在变量缺失时注入，且指向路径存在与否 cargo 不校验——M6 前惰性无害；一旦全局变量存在则根本不注入（全局优先）。
