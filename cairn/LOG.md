# Project Cairn 日志

本文件按时间倒序记录实质进展——最新条目在最上、紧跟本行。每条保持精简——摘要 + 指针；结论沉淀进 `cairn/<主题>.md`。
## 2026-08-30 · D-09 gix Windows 侧六项实测闭环 + 大小写碰撞静默覆盖实证 + 检测/标注排期定案

- M1 遗留「D-09 gix Windows 侧实测」在 Windows 宿主闭环（NTFS + Git for Windows system autocrlf=true 毒药环境）：六项全过，回归测试钉于 `store/snapshot.rs` `#[cfg(windows)]` 模块（commit 905666b），`cargo test -p mdor-core` 40 绿（34+6），门禁 fmt/clippy/audit 全绿。ubuntu CI 编译期自动剔除。
- 关键实证：① 裸 init 探针证实 gix 读 system 级配置（helix #6467 同款毒药环境本机即有），mdor 双施加点压成 false + CRLF 字节端到端保真；② ignorecase 自动探测实证（init 写 `core.ignorecase=true`）；③ ~350 字符长路径 checkout 无碍；④ **大小写碰撞 checkout 静默覆盖**（无告警，树序后者胜出、前者字节丢失）——gix 不提供知情信号，tree 级检测前置（D-09 定案 3）是实现「标注」的唯一可行位置。
- 排期定案：定案 3 拆三件——tree 级检测 + SnapshotMeta 碰撞字段落库归 **M2**（M4 复用、报错选项 M2 接线）；归一 + 标注渲染归 **M3**（Windows 单渲染=工作区直读 D-10 自然退化，标注不依赖 blob 直接读）。plan.todo M2 加检测任务。
- 沉淀：[gix-windows-pitfalls.md](gix-windows-pitfalls.md) 开放问题节重写为实测结论 + 排期注记；decisions.md D-09 状态→已实测 + 实测结论六项 + 排期定案；diff.md §4.4/§4.5 收敛；project.md §11 补排期指针；ROADMAP 当前焦点 40 绿 + 开放问题清理。
- 待办：plan.todo「CI core-quality 实跑验证」随 feature/m1-core → master 合并执行（ff-only，D-14 直推保线性），Actions core-quality 绿后勾选。
- 门禁：fmt/clippy/test 40 绿/audit exit 0 + check-links.py 通过。
## 2026-08-30 · M1 书架视觉验收闭环（ubuntu-dev VM 首次 GUI 调试）+ WebKit EGL 崩溃根因沉淀

- M1 遗留 @critical「书架弹窗视觉确认」在 ubuntu-dev VM 闭环：SSH 带 X11 env 起 `cargo run`（Xwayland）→ seed 2 本书渲染（像素分析 8 文本行段 = 1 标题 + 2 条目组）→ `gnome-screenshot` 取证 → 用户 VirtualBox 肉眼复核通过。质量门禁 VM 实跑全绿（fmt/clippy/test 34/audit exit 0）。
- 核心坑：VirtualBox VMSVGA 模拟 VMware（vmwgfx）× nix mesa ≥25 已移除该 DRI 驱动 → WebKit GPU 进程 EGL 初始化崩（`EGL_BAD_PARAMETER`）→ WebProcess 反复自杀黑窗。环境变量绕不过，解法 = llvmpipe 软渲染 + `__EGL_VENDOR_LIBRARY_FILENAMES` 指到 `nixpkgs#mesa`（devShell 的 mesa-libgbm 无头子集缺 libEGL_mesa）。变量矩阵试错多轮，最终 strace openat 一击定位（glvnd 读系统 vendor json 找不到 libEGL_mesa）。
- 网络事实更正：VM 从未配置 NIC2 Host-Only，SSH 实际走 NAT 端口转发（10.0.2.15:22）——env.md §1 补【已替换】块、ubuntu-vm-setup.md §0.2 加更正注。
- 沉淀：[ubuntu-vm-setup.md](ubuntu-vm-setup.md) 新增「GUI 调试操作要点」（完整环境变量组合 / 成功判据=WebKitWebProcess 存活 / strace-ctypes-strings 诊断三件套）；env.md §6 排查表补 EGL 坑 + gnome-screenshot AccessDenied 坑；plan.todo 勾阶段3。
- 补充沉淀（同日第二轮）：① nix-daemon 持久代理 × clash 停运 = `nix build` 全断的误诊陷阱（curl 通 + nix 报 connect 127.0.0.1 失败即查 daemon env）→ [nix-mirror-proxy.md](nix-mirror-proxy.md) 代理架构节；② agent 不支持读图 → 截图像素分析固化为 [script/shot-analyze.py](../script/shot-analyze.py)（三 fixture 冒烟全过）+ **临时脚本三次法则**成文（script/AGENTS.md 规则节 + 根 AGENTS.md 指针 + scripts.md 临时探针台账）。
- 阶段4 全部闭环：env-ready 快照已打（26-08-30 用户侧）。~~VM 环境块阶段2/3 仍待做~~【更正 26-08-30 晚】阶段2/3 已全部闭环：阶段3 实测早已完成（opencode 1.18.21 + auth 2 credentials + 5 skills deploy + devShell 继承式启动）；阶段2 补装 Starship 1.26.0 后闭环（skills-manager GUI+CLI 1.34.2 早已在 profile）；GUI 冒烟一次即失败（bwrap uid map 被 Ubuntu apparmor userns 限制拒绝，未到 WebKit 层，不重试）。plan.todo VM 环境块全勾；环境侧细节见 [ubuntu-vm-setup.md](ubuntu-vm-setup.md)。
- 门禁：VM 内 fmt/clippy/test/audit 全绿 + check-links.py 通过。
## 2026-08-30 · 环境侧 /etc/nixos home-manager 接线两坑修复（users 传路径 + extraSpecialArgs 包裹 inputs）

- 环境侧 flake（/home/morr/nixos-config/default）`nix build` 连续两错：① `home-manager.users.morr = import ./home.nix { inherit inputs; }` 手动调用模块函数只给 `inputs` → `function ... called without required argument 'config'`；② 改传路径 `./home.nix` 后 `home-manager.extraSpecialArgs = inputs;` 摊平 flake inputs（specialArgs key = self/nixpkgs/charmbracelet-nur…），home.nix 声明 `{ inputs, ... }` 解析不到 → infinite recursion（栈内 `argument `inputs` is not externally provided, so querying `_module.args` instead`）。
- 修复（官方最佳实践，home-manager manual NixOS module 节）：`home-manager.users.morr = ./home.nix;` + `home-manager.extraSpecialArgs = { inherit inputs; };`；仅改 flake.nix，home.nix 无它改（inputs 只在 imports 用）。
- 诊断法沉淀：`nix eval --impure --expr` 最小 nixosSystem + 假 inputs 复现（摊平复现 / 包裹通过）；`--show-trace` + 读 `lib/modules.nix` `applyModuleArgs` 定位 fallback；root 构建 `$HOME not owned by you` 与 dirty tree 警告无害可忽略。
- 沉淀：新建 [nixos-home-manager-flake.md](nixos-home-manager-flake.md)（`graduation_status: candidate`，跨项目可复用）。
- 门禁：`check-links.py` 通过。

## 2026-08-30 · D-16 VM 基座复核（维持 ubuntu+nix）+ ubuntu-dev 装 opencode

- 复核「备用 VM 是否换 NixOS 以一步迁移 nixos-wsl 声明式配置」：dev 层 `flake.nix` 本就 WSL/VM 两侧单源零差别；ubuntu+nix 双源（apt 底座 + Nix 工具）风险由「遮蔽即钉版」（devShell 内 PATH 置顶 store + PKG_CONFIG_PATH + rpath）兜底；不换两主因 = ① NixOS 上 VirtualBox Guest Additions 兼容风险（GA 是 GUI 调试/USB 直通前提）② ubuntu-dev 已装好没必要重来。
- 决策：维持 ubuntu+nix；D-16 补【已否决】[NixOS VM 复用作 VM 基座](../doc/decisions.md#nixos-vm-复用作-vm-基座)块 + 状态表加「已复核 2026-08-30」行。
- ubuntu-dev 阶段 3：`nix profile install nixpkgs#opencode` 已装 **1.18.21**；`install` 为 `add` 的 deprecated 别名（warning 正常）、`nixpkgs#opencode` 按 attrpath 定顶层包不与 `haskellPackages.opencode`/`cc-switch` 混淆、opencode 无 apt 源——坑注沉淀于 [ubuntu-vm-setup.md](ubuntu-vm-setup.md) 阶段 3。
- 待办：auth login / skills deploy / 阶段 4 验收 + 快照，随 VM 环境收尾一并更新 plan.todo。
- 门禁：check-links.py 通过。

## 2026-08-29 · PS 5.1 stdin 管道中文损坏坑补充 + commit 消息修复

- 提交 docs 批次时，`@'…'@ | git commit -F -` 在 PS 5.1 下把提交信息中文字符全写成字面 `?`（0x3F）——根因是管道写原生命词 stdin 用 `$OutputEncoding`（默认 ASCII），与已知 stdout GBK 解码坑（powershell-encoding 坑7）同族反向；`cmd /c` 直读原始字节仍 `?` 证实字节级损坏、非显示伪影。
- 修复：`git commit --amend -F <UTF-8 无 BOM 文件>`（未推送，安全 amend），commit afcec32 中文完好。
- 沉淀：[powershell-encoding.md](powershell-encoding.md) 补坑8 + summary/updated 同步；对策 = 中文喂原生命词用 `-F <UTF-8文件>` 或先 `$OutputEncoding=UTF8`。
- 门禁：check-links.py 通过。

## 2026-08-28 · ubuntu-dev VM 环境准备（M0 桌面）+ Nix 国内镜像/代理链路沉淀

- 会话：从裸 Ubuntu 24.04.4 搭起备用环境（D-16 三端）到 M0 桌面可用——阶段0 最小配置（Guest Additions / openssh-server(Host-Only) / git / Nix multi-user+flakes / SSH clone / 快照）→ 阶段1 `nix develop` + direnv+nix-direnv（`direnvrc` source + `direnv allow`，`/.direnv` 已 gitignore）→ 阶段2 skills-manager GUI+CLI + Starship → 阶段3 opencode（`nixpkgs#opencode` + 继承式启动）→ 阶段4 验收 + 快照。
- 慢速根因链：国际直连 `static.rust-lang.org` 仅 ~50–80KB/s 为真实瓶颈（关宿主机 TUN 后真直连更慢=证实）→ 对策 = 绕开而非调代理：国内镜像矩阵实测（nix 缓存 USTC/TUNA/SJTU；rust-static **USTC VM 直连 403→弃 / rsproxy 可用 / SJTU 200 / TUNA·BFSU 404**；crates rsproxy sparse）+ 工具链 fixed-output 预置 `nix-store --add-fixed`。
- 代理架构：nix **client 吃 shell 代理、nix-daemon 不吃**（须 `systemctl edit nix-daemon` 注入 8 行环境，大小写都配）；`sudo -E` 对 nix 无效；clash `external-controller`(9090, clashctl ui 打印) ≠ `mixed-port`(7890)；`clashctl off` 不清当前 shell 导出变量（假直连）；宿主机 TUN + VM clash = 双代理链。
- 沉淀：新建 [ubuntu-vm-setup.md](ubuntu-vm-setup.md)（操作手册, procedure）与 [nix-mirror-proxy.md](nix-mirror-proxy.md)（通用镜像/代理专题, lesson/experience/decision）；[env.md §1](../doc/env.md#开发环境拓扑) ubuntu-dev 行补操作手册指针。
- 执行状态（本机）：阶段0/1 已闭环（nix flake check ✅ / cargo test 34 绿 / dx 0.7.10），阶段2-4 待做；进度权威记录 = `plan.todo`「ubuntu-dev VM 环境」块。
- 门禁：check-links.py 通过。

## 2026-08-27 · M1 全量落地（六切片）+ gix 存储基座实测

- 切片1 书架持久化（error.rs thiserror + `write_json_atomic`/`read_json_capped` 1MB + LibraryStore 提交点）；切片2 阅读进度（ReadingPosition + ProgressStore RenameOnly）；切片3 gix 基座（BookRepo init/自建 commit/版本 tag/checkout 硬切换 + D-09 双施加点 + blob 跨版本去重 + CRLF 字节保真）；切片4 tracing（core 打点门面 + app fmt/RUST_LOG）；切片5 服务接线（SourceAdapter/Registry + PositionMigrator/PathMigrator + AppService 薄门面 + Command 串行队列 + BookStore 孤儿同步清理 + 书架渲染真实数据）；切片6 ci.yml core-quality。
- 验证：`cargo test -p mdor-core` 34 绿、fmt/clippy/audit(exit 0) 全绿；WSLg 桌面日志证明书架渲染读取 library.json（609 字节 seed 无错误）。视觉截图受限：WSLg 合成器不支持 wlr-screencopy（grim）、nixpkgs imagemagick 无 X11 delegate。
- gix 0.87 实测：checkout 无高层 API，须直依赖 gix-worktree-state；D-09 结论回写 decisions/diff/cairn（Linux 通过、Windows 五项遗留需宿主机）。
- 决策：孤儿清理同步执行（防 add_book 建目录→登记 TOCTOU，升级门写入 store/mod.rs）；AppService 注入用 OnceLock（Dioxus GlobalSignal 因 UnsyncStorage 非 Sync 不适用）。
- 待办：CI 实跑（推送 master）、M2 StaticSiteSource。门禁：check-links.py 通过。

## 2026-08-27 · nixos-wsl 工具链单源化 + direnv 自动加载 devShell

- 早前中断的 rustup 下载遗留残缺 1.97.1 工具链（Missing manifest）；系统 rustup 1.29.0 代理在 `/run/current-system/sw/bin`。定位为非 flake 问题（rust-bin 原子构建不可能残缺）→ 环境治理：`/etc/nixos/module/dev/rust.nix` 移除 rustup+cargo（保留 rust-analyzer）+ 仓库根 `.envrc`（`use flake`，nix-direnv）+ `direnv allow` → cd 进仓库自动加载 devShell。
- 关键认知：opencode 的 bash 工具是非交互 shell，direnv 不生效；agent 要吃到 flake 工具链须在已加载 devShell 的交互终端里启动 opencode（继承式）。shell.args 方案不成立——当前 opencode schema `shell` 仅字符串；且 home-manager `.bashrc` 有 `[[ $- == *i* ]] || return`，`-l` 非交互登录被早退拦掉 direnv hook。
- 残余：`~/.rustup` 已清理；Windows 宿主 scoop rustup-msvc 不受影响（M0 桌面）。

## 2026-08-27 · NixOS-WSL vscode-server 架构边界 + 自定义仓库扩展安装复核

- 会话：梳理「在 NixOS 从指定仓库装 VS Code 扩展（aliveranme/Rebuild-gitlens，脱订阅校验的重打包 GitLens）」，核定 `services.vscode-server` 模块（nixos-vscode-server）**不管理扩展**、只 patch 客户端自动下载的 server 二进制；server 本体由宿主机客户端下载管理、与客户端绑定，无法 Nix 声明。
- 结论：Remote-WSL 拓扑下扩展只能走**客户端安装**（`code --install-extension` / GUI Install from VSIX）自动同步进 WSL server；`vscode-with-extensions` 属「NixOS 当客户端」拓扑，与 Windows 宿主对接不相容。
- 沉淀：新建 [nixos-wsl-vscode-server.md](nixos-wsl-vscode-server.md)——架构（nix-ld vs services.vscode-server 两类声明式 enabler、扩展声明边界）+ Rebuild-gitlens vsix 安装 procedure（Releases API digest→sha256 技巧复用自 nix-env-tooling 坑 6）+ 教训（模块职责边界凭 option 核实、`vscode-with-extensions` 非 Remote 通用）。
- 门禁：`check-links.py` 通过。

## 2026-08-26 · 遥测链路设计 + 模块边界内核 + mdor 日志基座沉淀

- 会话产出（跨项目遥测设计讨论）：多语言（Python×2 / JS 浏览器 / Java）分布式高频遥测链路完整设计归档 [telemetry-schema-registry-design.md](telemetry-schema-registry-design.md)——形态 A（定义层 schema 注册表 + 运行时本地分发）、统一 WebSocket 信封（WS1/WS2 为连接腿标签、per-payload v/ts/seq、WS1 zstd / WS2 deflate、wss）、JSON Schema vs Protobuf+buf 双线工具链（含 Avro 备选）、落地路径；含教训「10ms 连续值=指标非日志」。
- 可复用内核 [module-boundary-contract-design.md](module-boundary-contract-design.md)：纵切模块×横切数据链路×共享底座；契约=定义/触点=绑定；私有 vs 共享类型；改类型「四处辐射」；形态 A；对 AI/vibe-coding 冲突规避的价值；并入静态/运行时架构项目化一句。
- mdor 侧决策 [mdor-logging-base.md](mdor-logging-base.md)：M1 切片4 日志基座（tracing）——core 只打点、桌面 RUST_LOG、不引分布式全家桶（留 OTel 门）、M6 appender+logcat（对应 plan.todo）。
- 命名教训：WS1/WS2 连接腿标签撞名 `ws://` 协议被误当"WebSocket 版本"——文档给连接/组件起标签应避免与既有技术名词撞名（见 telemetry-schema-registry-design.md 约束表）。
- 外部资料参考：三支柱/工具链背景单开 [Reference/observability-toolchain-basics.md](Reference/observability-toolchain-basics.md)（外部资料定位，只增不改）。
- 门禁：check-links.py 通过。

## 2026-08-26 · skills-manager CLI 备份同步机制探索 + 会话知识沉淀

- 会话过程：`git pull` 报 unrelated histories——本机 `git init` 独立空骨架历史 vs 远端（Windows 机）真实备份无共同祖先；本地无数据故 `fetch origin` + `reset --hard origin/main` 对齐远端历史；随后 opencode 识别不到 skill——根因是「git 同步库 ≠ deploy 到 agent 全局目录」两步机制，`skills deploy --agent opencode` 后生效；`presets add-skill` membership 存 SQLite `scenario_skills`（多对多），只改 DB 不部署。
- 沉淀：新建 [skills-manager-cli.md](skills-manager-cli.md)——备份同步机制（中央库=普通 git 仓库、DB 不入 git 从文件重建、快照 tag `sm-v*`、冲突永不覆盖进 pending_conflicts、勿 raw `--mirror/--all` push 污染 refs/skills-manager/*）+ Q1 远端拉取（`git clone <URL>`，避免 init 撞坑）/ Q2 本地同步（commit→push、pull 行级合并、versions/restore）+ 常用操作速查 + 开放问题（冲突裁决 CLI 缺失、preset membership metadata 往返待实证）。
- 门禁：`check-links.py` 通过。

## 2026-08-26 · mdor 项目 flake 落地 + Nix 环境侧外围工具（skills-manager / Starship）

- **项目侧 flake**（`flake.nix`，D-16 环境复现单源落地）：Rust 工具链改 `rust-bin.fromRustupToolchainFile ./rust-toolchain.toml` 单源钉 1.97.1（`profile=minimal` 不含 rustfmt/clippy，须 `.override` extensions 补 rustfmt/clippy/rust-src/rust-analyzer，否则 treefmt 初始化失败）；dx 0.7.10 由 shellHook `cargo install --locked` 钉版（幂等判断须用 `$HOME/.cargo/bin/dx` 绝对路径，非交互 bash PATH 无 `~/.cargo/bin`）；devShell 补 cargo-audit/outdated/edit + dioxus 桌面库（webkitgtk_4_1/gtk3/libsoup_3/gdk-pixbuf，实链接还缺 `libxdo`→补 `xdotool`）。`nix flake check` / `nix fmt --fail-on-change` / 门禁 fmt+clippy+test / `cargo audit`(exit 0) 全绿；`flake.lock` 生成。D-16 与 [env.md §1 拓扑](../doc/env.md#开发环境拓扑) 注明 Android SDK/NDK/JDK 的 flake 声明式锁定 = M6 待办（Nix 声明即实例化、M0 用不到）。
- **环境侧 `/etc/nixos`**：新增 `module/dev/skills-manager.nix`（overlay 暴露 `skills-manager` GUI = AppImage wrapType2 + `skills-manager-cli` = 官方二进制 + patchelf，锁 v1.34.2；Linux 无 in-app 自更新→声明式锁版）+ `home.nix` 装两包 + `programs.starship`（双行全信息、默认配色）。CLI 已实构建验证（ldd 0 not-found、`repo status` 通）；GUI 构建慢（100MB AppImage 解包）未本地构建，交 rebuild。
- **坑**：wrapType2 新版须 `pname`+`version`（`name` 报 extract 缺 version）；CLI patchelf 漏 `xz` 致 `liblzma.so.5 not found`；flake 新文件须 `git add` 否则「not tracked」；`nix build/eval '.#nixosConfigurations…config…'` 深层 attrpath 不可用，验证单包用 `dry-run` 取新 drv + `nix-store --realise`；`dry-run` 只 dry 计划不实构建。
- 沉淀：新建 [nix-env-tooling.md](nix-env-tooling.md)（项目/环境双 flake 分工 + skills-manager 打包方案与坑 + Starship 配置 + ubuntu-dev VM 粘贴即用脚本）与 [nix-project-flake.md](nix-project-flake.md)（项目侧 flake：Rust 单源 fromRustupToolchainFile / minimal profile 缺 rustfmt-clippy / libxdo→xdotool / dx shellHook PATH 幂等，`graduation_status: candidate`）。
- 毕业候选（标记 `candidate`，未执行 provider write）：Nix 声明式打包与工具链经验（patchelf 外部二进制 / AppImage wrapType2 / fromRustupToolchainFile 坑，见 nix-env-tooling.md + nix-project-flake.md）——跨项目可复用。`.cairn/config.yaml` 已配 `provider: obsidian`；实际毕业前需 obsidian-preflight + INDEX 查重 + 人工确认（本次仅标记候选）。
- 更正（同日）：skills-manager **GUI 从 nixos-wsl 撤除**——WSLg 实测可显示到宿主机（`/mnt/wslg` + `DISPLAY=:0` + `WAYLAND_DISPLAY=wayland-0` 均在），但 100MB AppImage 构建耗时（>10min）且 webkit 在 WSLg 下偶发渲染问题（D-16）；`module/dev/skills-manager.nix` 只保留 `skills-manager-cli`（patchelf 版），`home.nix` 注释 `skills-manager` 行备恢复；GUI 定义与安装移交 ubuntu-dev（见 nix-env-tooling.md ubuntu 脚本）。
- 门禁：`check-links.py` 通过。

## 2026-08-24 · 开发环境三端架构定稿（D-16）+ VirtualBox 安装坑沉淀

- 会话起于 VirtualBox「invalid installation directory」安装报错：哈希比对证明镜像站文件完好后定位为 7.0.14+ 目录安全校验；官方 icacls Deny 配方反噬管理员（`Authenticated Users` 在任何管理员令牌内，Deny 优先 → MSI 1303），改 `/inheritance:r` + 仅授 RX 解决。VirtualBox 7.2.16 落地 `D:\VirtualBox`。
- 架构决策 [decisions.md D-16](../doc/decisions.md#d-16-开发环境三端架构)：宿主机原生管桌面构建+安卓模拟器；**nixos-wsl 为日常主力**（Remote-WSL/SSH + 交叉编译，产物 adb connect 推宿主机模拟器）；ubuntu-dev VM（24.04.4 已装）备用，仅 dioxus 桌面调试与真机 USB 直通时启用；环境复现单源 = 仓库 `flake.nix`，WSL/VM 共用。VM 内跑 AVD 因嵌套虚拟化被否决。
- 沉淀：新建 [vbox-windows-install.md](vbox-windows-install.md)（坑链 + 正确 icacls 配方 + `%TEMP%\MSI*.LOG` 排查法）；[env.md §1](../doc/env.md#1-环境总览与版本矩阵) 新增「开发环境拓扑」小节落事实。
- 待办：flake.nix 编写、Android Studio(scoop)/SDK/AVD 迁 D 盘、VM 备用化最小配置——均未执行。

## 2026-08-23 · opencode 会话备份两段式编码坑（UTF-16 容器 + GBK 管道损坏）

- opencode 会话备份导入两连败：先「Unrecognized token '�'」（PS 5.1 `>` 落盘 UTF-16 LE，教训 6）；转码修好容器后仍报错——根因是导出时 PS 管道把 opencode 的 UTF-8 stdout 按 GBK 解码，中文固化为 652 个 PUA 字符 + `?` 替换，**内容导出即损坏、不可逆**。
- 修复：`cmd /c "opencode export <id> > file"` 直通原始字节重导出——JSON 解析通过、4904 汉字完好，`opencode import` 成功恢复会话 ses_fd0e80a5。
- 沉淀：[powershell-encoding.md](powershell-encoding.md) 教训区新增第 6 条（UTF-16 LE 写侧）与第 7 条（管道捕获损坏）+ 当前结论补「UTF-16 → UTF-8 转码」「cmd 直通原始字节」两姿势 + 实践指南补导出三件套断言；summary 同步。

## 2026-08-23 · UI 框架选型论证留痕（D-15）+ webview 差异专题扩充

- 会话讨论「Dioxus 是否抹平 webview 差异」引出选型论证补记：新建 ADR [decisions.md D-15](../doc/decisions.md#d-15-ui-框架选型)——Dioxus 选型留痕 + 四路线否决理由（Tauri/Electron/Flutter/自绘 Rust）；project.md §1.2 补反向链接。
- 核心定性：wry/Dioxus 只抹 API 封装层不抹引擎差异；样式碎片化可对冲（内联 CSS），**性能随系统 WebView 版本浮动不可对冲**，只能架构性压小依赖面。
- 沉淀：[webview-host-differences.md](webview-host-differences.md) 教训区新增第 7 条（API 抹平边界 + 性能碎片化）；新建 [ui-framework-selection.md](ui-framework-selection.md)（四路线横向对比背景知识，论证细节链接回 D-15 不复制）。
- 工具链：check-links.py 扫描范围扩至 cairn/ 顶层（跨文件目标改按源文件目录相对路径解析，负例冒烟通过）——修复「cairn→doc 链接不被门禁覆盖」盲区；LOG 两处根相对链接（`doc/…`）统一为源目录相对（`../doc/…`）；diff.md §2.1 补 D-15 导航指针。
- 门禁：`check-links.py` 通过（245 anchors，含 cairn 侧新覆盖 6 处）。

## 2026-08-21 · 小节标题统一为中文（zh-glossary）+ 毕业候选盘点

- 14 篇知识专题文档的英文小节标题按 zh-glossary 固定词统一：Lessons→教训、Current Conclusions→当前结论、Practice Guide→实践指南、Open Questions→开放问题；文件名保持英文 slug（skill 规则：`language: zh` 只约束正文与标题，不改文件名）。
- 过程坑：PowerShell 5.1 `Get-Content -Raw` 对无 BOM UTF-8 按 ANSI/GBK 误读，回写造成字节级损坏；从 `experiment/cairn-track` 分支（cairn/ 已跟踪、版本仅差一晚）按 blob 字节恢复后以 .NET UTF-8 无 BOM I/O 重做。已沉淀 [powershell-encoding.md](powershell-encoding.md)。
- 毕业候选盘点完成（未写入知识库）：Tier A 四篇（cargo-audit-behavior / windows-scripts / metadata-write-reliability / cargo-config-toml）、Tier B 六篇（gix-windows-pitfalls 仅已验证部分 / windows-msvc-toolchain / android-cross-compile-rust / local-resource-channel / mcp-doc-retrieval / dioxus-cli-0.7-config）；Tier C 四篇不标记。毕业时需重跑 obsidian-preflight + INDEX 查重。
- 更正：恢复源实为 master 自身历史——cairn/ 与 `.cairn/config.yaml` 均已被 master 跟踪（自 `68398e2` 起），并非依赖 `experiment/cairn-track` 分支；该分支与 master 完全同步、无独有内容（保留不删）。

## 2026-08-20 · manifest 收敛 workspace 层 + D-14 单人仓库协作决策定稿

- `build(workspace)`：根加 `[workspace.package]`（version/edition/rust-version=1.97.1 统一）+ `[workspace.lints]`（unsafe_code=deny）；两成员 `version/edition/rust-version` 改 `workspace = true` 继承 + `publish = false`。门禁（fmt/clippy/test）全绿。
- `docs`：单人维护 + 偶尔外部贡献的协作模式定稿——维护者直推 master 保线性、外部 PR 一律 squash 合入；决策留痕 [decisions.md D-14](../doc/decisions.md#d-14-单人仓库协作与外部贡献流程)，操作规范单源落仓库根 `CONTRIBUTING.md`（协作流程属约定不入 doc/，仅留痕不展开），AGENTS.md 补协作模式指针。
- 门禁：`check-links.py` 通过（231 anchors）。

## 2026-08-20 · 便携 Android 工具链方案定稿（dev/ + 提交相对路径 .cargo 配置）

- 评估 Podman6（仅 Linux VM、无 Windows 容器计划，GH #27842）与 Hyper-V Windows VM（过重）后，定「本机便携隔离」：Android SDK/NDK/JDK 全部 zip 便携装 `<repo>\dev\`（gitignore，M6 执行）；不写任何持久环境变量。
- 提交层落地：`.cargo/config.toml`（`relative=true` 的 `[env] ANDROID_HOME` + `include` 本地覆盖，无 force = 全局优先）、`.cargo/config.local.toml.example`（版本化 NDK + Windows linker 模板）、`dev/dev-env.ps1`（全局优先、便携兜底，dx/Gradle 用）、`.gitignore` 加 `/dev/*` + `!dev-env.ps1` + `config.local.toml`。
- 归档 `cairn/cargo-config-toml.md`：config 层级、`[env]` relative/force 语义、提交模式与坑（linker OS 相关、版本号不进提交文件、dx/Gradle 不读 cargo config）。
- JDK 定 Temurin 21（zip → `dev\jdk`），`dev-env.ps1` 保留 Scoop/既有 `JAVA_HOME` 兜底。
- 文档同步：env.md §1/§2.4/§2.5/新增 §2.6/§5/§6/§7/§8、project.md §12 文件树、根 AGENTS.md 一行。
- 门禁：`check-links.py` 通过。

## 2026-08-19 · M1 workspace 骨架落地 + 书架弹窗验收

- M1 workspace 重构落地：根 Cargo.toml 补 `[workspace.dependencies]`（serde 1.0.229 / serde_json 1.0.151 / thiserror 2.0.20 / dioxus 0.7.10，`cargo info` 验证均为当时稳定版）；删根 src/ hello world；建 `crates/mdor-core`（lib 空壳）+ `crates/mdor-app`（bin 骨架，中文书架占位）。
- 验收：门禁全绿（test / fmt / clippy / audit exit=0）；`dx build --platform desktop` 产物 exe 弹窗（标题 Dioxus App，MainWindowHandle 非零）——合并勾掉 plan.todo 14 + 20/21/22/48/49。
- 实证沉淀 `cairn/dioxus-cli-0.7-config.md`：Dioxus.toml 0.7 用 name + default_platform（非 0.6 app_id）、dx serve/check 无 --project 须 cd 进 member、dx build 产物在 `target/dx/<crate>/debug/windows/app`、name 不映射窗口标题。
- 过程实证：开工时仓库即处 workspace 中间态（Cargo.toml members 已改、`crates/` 未建，不可构建）；agent shell 无法驻留常驻 GUI 进程，弹窗验收改用 dx build + exe；PowerShell 下 dx 的红色 NativeCommandError 为 stderr 合并噪音，以 `$LASTEXITCODE` 判结果。
- audit 注记：dioxus-desktop 引入 14 条 unmaintained/unsound 级 advisory（gtk 系 Linux 侧 + rand 0.7.3 等传递依赖），退出码 0 不阻塞（D-12 判据）。
- 文档同步：plan.todo 勾选；根 AGENTS.md 状态/命令/单测/钉版四处更新；env.md §3 补工作目录；ROADMAP M0 勾掉、当前焦点转 M1。
- 门禁：`check-links.py` 通过。

## 2026-08-19 · AGENTS.md 状态修正 + resolver 纳入与归档

- AGENTS.md：android targets 状态修正（toolchain 已带 rust-std-{aarch64,x86_64}-linux-android，JDK/SDK/NDK 未装）；质量门禁补单 crate 骨架期命令（`cargo test`，workspace 落地后切 `-p mdor-core`）。
- resolver 纳入：plan.todo M1 workspace 重构补 `resolver = "3"`；env.md §4.1 补 virtual workspace 须显式设 resolver（全局项、member 写无效）。
- 归档 `cairn/cargo-workspace-resolver.md`：resolver 1/2/3 版本差异、virtual workspace 无 edition 必须显式的原因、mdor 取值建议（Cargo book 整理）。
- 门禁：`check-links.py` 通过。

## 2026-08-19 · 质量门禁首跑 + audit 静默成功实证 + 版本落根工作流定稿

- 质量门禁首跑四门全绿（fmt --check / clippy -D warnings / cargo test / cargo audit `exit=0`）；plan.todo 勾选，符号规范补 `@done(yy-mm-dd HH:MM)` 标注格式。
- 实证：cargo-audit 0.22.2 无漏洞时静默成功、不打印结语，**退出码 0 才是判据**（presenter.rs 仅在 found 时打输出）——沉淀 `cairn/cargo-audit-behavior.md`。
- env.md §4.1「版本约束落根工作流」：定稿 `cargo add` 不能钉根表（#11527 / #16797）、3 步流程、workspace 根不带 `-p` 直接报错、两个 Cargo 语义约束；decisions D-12 补依据（rustls/dioxus 根钉版为硬约束，推翻也绕不开）。
- 门禁：`check-links.py` 通过。

## 2026-08-18 · doc/ L2 扩「确定信息单源」+ README 补抽象层级说明

- doc/AGENTS.md L2 新增「确定信息同样单源」bullet：具体易变的确定信息（依赖版本、当前方案）只一处落值、别处链接引用防漏改；层级关系链接 README。
- README「文档间引用关系」补抽象层级说明（规范 > 论证 > 差异 > 操作；上层稳定、下层易变）——四层模型唯一源在 README，L2 不复制。
- 审查确认：decisions 为论证层（非纯具体）、diff 含背景知识，维持既有四层模型不重构。
- 门禁：`check-links.py` 通过（222 anchors）。

## 2026-08-18 · 全量 cargo install 统一 --locked + CI 补 dx 钉版

- env.md §4.1 三命令（cargo-outdated / cargo-edit / cargo-audit）补 `--locked`；根 AGENTS.md 门禁行的 cargo-audit 安装同步补；project.md §12.3 CI 缓存行 dx 安装命令改链接 env.md §2.3、不内联版本号（版本号单源 env.md §1，避免升级漏改）。
- env.md §1「版本钉版边界」加澄清条：`--locked`（依赖图可复现）≠ `--version`（钉工具自身版本），「不钉版本」只指后者，`--locked` 对全部 cargo install 一律启用。
- 核查：全仓库 27 处 cargo install，archive_doc_v* 存档不改，其余已全部带 `--locked`。
- 门禁：`check-links.py` 通过（219 anchors）。

## 2026-08-18 · doc/ 版本钉版边界：不钉清单 + 通用规则 + crates 钉版时机

- env.md §1 新增「版本钉版边界」：不钉版本清单（cargo-audit/outdated/edit、WebView2、随 toolchain 自动锁定项）；通用规则（钉 = 影响构建/运行行为或与项目库配对，否则不钉）；crates 依赖版本不在文档确认、M1 建 workspace 时于根 workspace.dependencies 钉下，dioxus 库跟随 dx（0.7.10 ↔ 0.7.x）。
- 门禁：`check-links.py` 通过（219 anchors）。

## 2026-08-18 · doc/ 版本号落点约定 + dx 钉 0.7.10 + --locked 机制说明

- 新增约定（doc/AGENTS.md「版本号事实落点约定」）：版本号事实源 = env.md §1 矩阵；安装命令内联具体号是命令参数、与矩阵同一事实；升级两处同步改。
- env.md：dx 钉 0.7.10（§1 矩阵 + §2.3 `cargo install dioxus-cli --locked --version 0.7.10`）；§4.1 新增「版本锁定机制」（--locked 含义 / 加不加区别 / 与项目 Cargo.lock 区分）。
- 门禁：`check-links.py` 通过（218 anchors）。

## 2026-08-16 · doc/ A 类单源化 + 引用方向原则

- 版本钉版单源化到 `env.md` §1：project §12.3 CI 表改「钉版」+ 指针、§12 文件树注释指针化、diff §6.4 指针化。
- fsync 分层映射单源化到 project §6.7：diff §7.2 删完整表，保留平台结论 + 链接。
- 版本清理策略单源化到 §7.4：§11「版本历史的存储占用」行改链接。
- doc/AGENTS.md「L2 单详述源」追加引用方向原则（单一事实源 + 反向链接 + 互引不算环）。
- 门禁：`check-links.py` 通过（211 anchors）。

## 2026-08-16 · doc/ 决策状态单表化 + §11 补孤儿清理行

- project.md §12.1「决策摘要表」删除（decisions 决策总览的复述，违反 L2 单详述源），改为一行指针链接 `decisions.md#决策总览`；ADR 状态单一事实源收敛到 decisions。
- project.md §11 新增「孤儿 `books/<id>/` 目录清理」行（非 ADR 的实现待定项，只进 §11 不进 decisions）。
- 门禁：`check-links.py` 通过（207 anchors）。

## 2026-08-16 · doc/ 一致性审计与修正

- 双轨检索（MCP 覆盖矩阵 + 全文核读）审查 doc/（排除 `archive_doc_v*/`）：结构自洽、无存档信息丢失（v1 diff §9 WebView 清单全量迁入 §10.1 且 B7 已闭环）。
- 修正：project.md §12.3「无跨平台交叉编译需求」→「无引入 Zig 的 C 交叉编译需求」；§9 `site/` 注释「webview 直接读文件」→「经本地 http 服务读取」；§6.9 httpmock 引用 §12.2 → §10 M2/M4；hash 术语对齐 D-08（§7.3/§7.2/§4.1/§5）；env.md §2.5 示例 android24 → android30。
- 决策：`.bak` 备份机制否决（原子写已消除半写态），删除 project.md §6.7 对应行，论证记入 decisions.md D-03、diff.md §7.2 留短块链接。
- 门禁：`check-links.py` 通过（218 anchors）。

## 2026-08-16 · doc→cairn 第二批沉淀（4 新 2 扩）+ Cairn 规则拆分

- 新建专题：[webview-host-differences.md](webview-host-differences.md)（diff §2.1–2.4 宿主/线程/内核/DevTools，纯落空）、[data-directory-platform.md](data-directory-platform.md)（diff §3 + D-13）、[service-orchestration.md](service-orchestration.md)（D-07：薄门面 + 命令串行 = 单写者、为何不引全局中介者）、[mcp-doc-retrieval.md](mcp-doc-retrieval.md)（MCP 检索实证：archive 稀释/表格召回弱/形式取信陷阱）。
- 扩充：[gix-windows-pitfalls.md](gix-windows-pitfalls.md)（+§4.3 保留设备名/Path 分隔符/fixtures 大小写）、[android-cross-compile-rust.md](android-cross-compile-rust.md)（+Zig 评估不采纳、dx/dioxus 同步升、旧版 dx 链接 bug、min_sdk=30、cmdline-tools 结构）。
- 规则层：Cairn 规则集中到新 `cairn/AGENTS.md`（根 `AGENTS.md` 瘦身至 ≤60 行 + 新增「检索约定（MCP）」节）；三次模板偏离固化为项目级约束（不用 Claude Code / cairn 私有不分发 / 规则集中子文件）。
- 判定：D-01/D-12/D-10 偏规范留 doc 不沉淀；测试策略已进 AGENTS.md 不重复。

## 2026-08-16 · 从 doc/ 提炼知识，新建 5 篇专题文档

- 对比 `doc/` 及 `archive_doc_v1/`、`archive_doc_v2/`：存档是同一批文档的更早状态，内容 ⊆ 当前 doc/，无被丢弃知识（v1 diff.md 详细评估已收口进 decisions.md D-11）。
- 按「经验/坑为主 + 决策摘要」约定新建专题（规范细节一律链接回 doc/）：[windows-msvc-toolchain.md](windows-msvc-toolchain.md)、[gix-windows-pitfalls.md](gix-windows-pitfalls.md)、[android-cross-compile-rust.md](android-cross-compile-rust.md)、[local-resource-channel.md](local-resource-channel.md)、[metadata-write-reliability.md](metadata-write-reliability.md)。
- 涉及 D-08/D-09/D-11/D-04/D-05/D-06/D-02/D-03 与 env.md §2.1、diff.md §1/§2/§4/§6、project.md §6.5/§6.7/§6.8。

## 2026-08-16 · 实验分支：放开 cairn 分发

- 建分支 `experiment/cairn-track`（commit 3dffd8e）：`.gitignore` 移除 cairn/ 与 .cairn/ 忽略规则，`git_policy` 翻转为 `track`，cairn 知识层与配置随仓库分发。
- master 保持 cairn 私有策略不变；实验评估后决定合并或丢弃该分支。
- 详情：见 `.cairn/config.yaml`。

## 2026-08-16 · Cairn 调整：守卫声明 + `.cairn/` 忽略 + CLAUDE.md 删除 + 坑沉淀

- AGENTS.md：Cairn 引言加守卫声明（无 skill 或无 `cairn/` 时 Cairn 规则不适用），并删除 CLAUDE.md 相关引用。
- `.cairn/` 与 `cairn/` 一并忽略：Cairn 层完全私有不分发。对 Cairn「永不忽略 `.cairn/config.yaml`」默认的刻意偏离，取舍：配置仅 5 行、无密，本地文件仍在磁盘，功能不受影响。
- `CLAUDE.md` 删除：不用 Claude Code（对 Cairn 模板的偏离，AGENTS.md 引用已同步清理）。
- 坑沉淀：Windows 下运行 Cairn 脚本的调用姿势 → [windows-scripts.md](windows-scripts.md)。
- 详情：见 `AGENTS.md` 与 `.gitignore`。

## 2026-08-16 · Project Cairn 初始化

- 初始化 Project Cairn 结构（语言 zh、git_policy=ignore、migration_mode=inventory_only）。
- 毕业目标：Obsidian（vault: SecondBrain → Cairn/INDEX.md），预检通过。
- doc/ 保持架构规范不动，cairn/ 专注经验沉淀，链接互补（doc 规范 ↔ cairn 经验）。
- 详情：见 `AGENTS.md` 与 `.cairn/config.yaml`。
