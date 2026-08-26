---
type: project_topic
status: active
summary: "mdor 仓库 flake.nix（项目侧 devShell，D-16 环境复现单源）落地：Rust 工具链用 rust-bin.fromRustupToolchainFile 复用 rust-toolchain.toml 单源钉 1.97.1（profile=minimal 不含 rustfmt/clippy，须 .override extensions 补否则 treefmt 初始化失败）；dx 0.7.10 由 shellHook cargo install --locked 钉版（非交互 PATH 无 ~/.cargo/bin 须绝对路径幂等判断）；dioxus 桌面 Linux 库需 xdotool（链接缺 libxdo）；门禁工具 cargo-audit/outdated/edit；与 /etc/nixos 环境侧 flake 分工"
tags: [mdor, nix, flake, rust, toolchain, dev-shell, dioxus, treefmt]
contains: [lesson, decision, procedure]
created: "2026-08-26"
updated: "2026-08-26"
related: [decisions.md, env.md]
authoring_mode: ai_generated
graduation_status: candidate
---
# mdor 项目 flake.nix（项目侧 devShell）

## 背景

[decisions.md D-16](../doc/decisions.md#d-16-开发环境三端架构) 定「环境复现单源 = 仓库 `flake.nix`」。仓库内这份 flake 定位是**项目 devShell**（`nix develop` 喂 mdor 构建/门禁依赖），与 `/etc/nixos` 环境侧 flake（个人工具链，见 [nix-env-tooling.md](nix-env-tooling.md)）分工：跨项目个人工具不入项目 flake，项目构建依赖不入环境 flake。落地时 nixpkgs = nixos-unstable + rust-overlay + treefmt-nix。

## 方案与决策

- **Rust 工具链单源**：`pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml` —— channel/profile/targets 全部读仓库 toml（版本单一事实源 = `rust-toolchain.toml` 1.97.1），M6 补 android targets 后自动生效；不得改用 `stable.latest`（会漂移、双源冲突）。
- **extensions 补齐**：`.override { extensions = [ "rust-src" "rust-analyzer" "rustfmt" "clippy" ]; }` —— toml 的 `profile=minimal` 不含 rustfmt/clippy，而 treefmt 与质量门禁都要用。
- **dx 0.7.10 钉版**：shellHook `cargo install dioxus-cli --locked --version 0.7.10`（对齐 env.md §1 版本矩阵；nixpkgs 的 dioxus-cli 版本未必匹配），并 `export PATH="$HOME/.cargo/bin:$PATH"`。
- **质量门禁工具**（AGENTS.md 门禁）：nativeBuildInputs 加 cargo-audit / cargo-outdated / cargo-edit。
- **dioxus 桌面 Linux 构建库**：buildInputs = openssl + webkitgtk_4_1 / gtk3 / libsoup_3 / gdk-pixbuf / xdotool（服务 ubuntu-dev VM 的 `dx serve --platform desktop` 场景）。
- **formatter**：treefmt-nix（`programs.rustfmt.package = rustToolchain` + alejandra，`alejandra.toml` 钉 FourSpaces）。
- **Android 声明式锁定 = M6 待办**（D-16 注记）：Nix 声明即实例化，SDK/NDK 数 GB 且 M0 用不到，随 rust-toolchain.toml targets 一起 M6 补。

## 坑

1. **`fromRustupToolchainFile` 按 `profile=minimal` 不含 rustfmt/clippy**：产物 `rust-minimal-1.97.1` 只有 rustc/cargo/rust-std；treefmt 初始化报 `formatter command not found in PATH: .../rust-minimal-1.97.1/bin/rustfmt`。须 `.override { extensions = [...]; }` 补；extensions 是**追加**到 profile 组件之上，不覆盖 toml 的 profile。
2. **dioxus 桌面 Linux 链接缺 `-lxdo`**：wry/tray 栈的 libxdo crate 动态链 `libxdo`，rpath/链接缺该库报 `rust-lld: error: unable to find library -lxdo`。补 `pkgs.xdotool`（4.x 提供 `libxdo.so` + `libxdo.pc`）；注意旧版 nixpkgs 的 xdotool（3.x）可能不带 dev 文件，确认 store 内 4.x。
3. **shellHook 幂等判断不能 `command -v dx`**：非交互 `nix develop --command ...` 下 PATH 不含 `~/.cargo/bin`（非交互 bash 不读 .bashrc），`command -v dx` 恒失败导致每次进 shell 重跑 cargo install（虽被 cargo 自身幂等跳过，仍多耗几秒）。改用 `[ ! -x "$HOME/.cargo/bin/dx" ] || ! "$HOME/.cargo/bin/dx" --version 2>/dev/null | grep -q "0.7.10"` 判断 + `export PATH="$HOME/.cargo/bin:$PATH"`。
4. **首次 `nix develop` 触发 dx 全量编译**（约 3 分钟，数百 crate）：正常现象，二次进入幂等跳过；`dx --version` 输出 `dioxus 0.7.10 (...)`，grep `0.7.10` 可匹配。

## 实践指南

- 进入：`nix develop`（devShell 自动装 dx 0.7.10 并注入 PATH）。
- 验证：`nix flake check`（求值全绿）→ `nix fmt -- --fail-on-change`（treefmt 的检查参数是 `--fail-on-change` 不是 `--check`）。
- 门禁（在 flake 工具链下）：`cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test` → `cargo audit`（退出码非 0 即失败）。
- 升级流程：改 `rust-toolchain.toml` 的 channel/targets → rebuild 自动生效；dx 版本改 shellHook 的 `--version`，并同步 env.md §1 版本矩阵（版本号事实源 = env.md §1）。Rust 依赖版本仍只钉根 `[workspace.dependencies]` 一处，与 flake 无关。
- `flake.lock` 随仓库提交（D-16 锁版依据）；NixOS flake 目录（/etc/nixos）新增 `.nix` 文件须 `git add` 否则 Nix 报 not tracked（见 nix-env-tooling.md）。
