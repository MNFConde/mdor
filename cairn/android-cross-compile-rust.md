---
type: project_topic
status: active
summary: "Android 交叉编译的约束与 TLS 选型：依赖纯 Rust 是硬约束、native-tls 灾难、rustls + platform-verifier + ring 定案；工具链坑（Zig 不采纳、dx/dioxus 同步升、dx 链接代理 bug、min_sdk=30、cmdline-tools 结构）"
tags: [mdor, android, rustls, tls, cross-compile, reqwest, gix]
contains: [lesson, decision, procedure, experience]
created: "2026-08-16"
updated: "2026-08-21"
related: [diff.md, decisions.md, env.md]
authoring_mode: ai_generated
---
# Android 交叉编译与 TLS 选型（含工具链坑）

## 背景

mdor 目标平台是 Android，但开发在 Windows 桌面进行，core 需在两平台行为一致。Android 无 OpenSSL、交叉编译 C 依赖痛苦、WebView 宿主差异大——这些决定了「依赖纯 Rust」成为硬约束。完整背景见 `doc/diff.md` §1/§6；决策记录见 D-11。

## 教训

1. **Android 系统无 OpenSSL 库**：`native-tls` 在 Android 上走 OpenSSL，需用 NDK 把 OpenSSL C 代码交叉编译成 arm64（`openssl-sys` 找不到库 / 版本不匹配 / 构建脚本报错，著名坑）。这是技术栈「Android 无 OpenSSL 依赖」的根源。
2. **rustls 不自动知道系统信任库**：根证书必须显式喂给。不喂 → 能编能跑但**所有 HTTPS 请求报 `certificate verify failed`**，下载功能全废。
3. **gix 的 HTTP 后端是额外差异点**：默认 curl 后端要编 curl 的 C 代码；选 `http-client-reqwest` 复用 reqwest → TLS 栈完全统一、无需额外 C 依赖。
4. **reqwest 0.13 起不代选 provider**：必须自己在 `main()` / `AndroidMain` **最早处** `install_default()`，否则 `Client::new()` 直接 panic（0.12 无此要求）。
5. **任何新增 C 依赖都会重新引入 Android 交叉编译差异点**——引入前须评估（`doc/diff.md` §6.4）。
6. **根证书三选一的现实坑**：公司电脑常装内网 CA（`mitmproxy` 抓包、VPN 网关证书），只打包 webpki-roots 时 Windows 访问内网文档站报 `certificate verify failed`；`rustls-native-certs` 运行时读操作系统信任库，但 **Android 无实现**，天然只解决 Windows 端。

## 工具链坑（环境侧，`doc/env.md` §6/§8）

1. **Zig 不能替代 NDK（评估 2026-08-09，不采纳）**：Android 侧仅能替换编译器、仍需 NDK 的 bionic sysroot/platform libs（**NDK 省不掉**），且偏离 dioxus/dx 官方 Gradle 管线；host 侧 `zig cc -target x86_64-windows-msvc` 为非标组合，依赖 find-msvc-tools 的 build.rs 会失败。结论：维持 MSVC host + M6 补 NDK 官方路径。
2. **dx 与 dioxus 库版本必须一起升**：dioxus 库改 `Cargo.toml`，dx 执行 `cargo install dioxus-cli --locked`，然后 `dx doctor` 校验。大版本（0.7 → 0.8）等官方迁移指南、先升库再升 dx、二者版本须匹配、`dx serve --platform desktop` 冒烟。升级顺序：发版说明 → 升库 → 升 dx → `dx doctor` → 桌面冒烟 → 回归（`cargo test` + `cargo audit` + `dx doctor`）。
3. **旧版 dx 的 Windows 链接器代理 bug**：`dx build --android` 链接报乱码/参数过长 → 升级 dioxus-cli ≥ 0.7.1（PR #4126 已修）。
4. **Android 启动崩溃 `NoSuchMethodError getCurrentWindowMetrics`**：真机 API < 30 → 在 `Dioxus.toml` 设 `min_sdk_version = 30`。
5. **cmdline-tools 目录结构必须 `cmdline-tools/latest/bin`**：否则 `sdkmanager` 不可用（解压后需 rename 为 `latest`，`doc/env.md` §2.5）。

## 当前结论

- **统一 TLS 栈（D-11）**：两平台统一 `reqwest`（`default-features = false` + `rustls-tls`）+ **`rustls-platform-verifier`**（Android 走 JNI 调系统证书验证，Windows 退回 SChannel——唯一同时解决两端、认用户/系统 CA 的方案）；gix 开 `http-client-reqwest` 复用同一 TLS 栈。
- **加密 provider = `ring`**（轻量、免 cmake/perl、APK 体积小）；经 rustls `CryptoProvider` 抽象可插拔，切换 = Cargo feature 一行 + 重新打包，**不做运行时双 provider 注入**（体积翻倍）。备选 `aws-lc-rs`（触发：需 FIPS / 更广算法面）。
- **rustls 稳定性评估**：足够稳定可用于生产（2016 年诞生、0.23 长期系列 + backport = 等效 LTS、独立安全审计 + OpenSSF 徽章、Prossimo 主导、Let's Encrypt 计划替换 OpenSSL）。注意点：0.x 按 semver minor 可破坏 API；「纯 Rust」需打折（ring 含手写汇编、无 FIPS）。mdor 为客户端、只访问知名站点，属最成熟使用面。
- **版本对齐**：`rustls-platform-verifier` 与 reqwest 传递进来的 rustls 需在根 `[workspace.dependencies]` 钉同一 0.23.x（防双版本）。

## 实践指南

- 选型朝「纯 Rust」靠：serde_json（非 SQLite）、gix（非 git2）、rustls（非 openssl），把 C 依赖排除在外。
- 详情见 `doc/diff.md` §1.6/§1.8/§1.9/§6.3 与 D-11；工具链（NDK r29、JDK 21、android targets）安装见 `doc/env.md` §1/§2/§7；工具链坑与故障排查见 `doc/env.md` §6/§8，M0→M6 过渡清单见 §7。
