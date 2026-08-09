# 跨平台依赖差异（Windows 桌面 / Android）

> 本文记录 mdor 在 Windows（桌面开发、测试）与 Android（目标）两端依赖点不一致的地方。
> 依据 `doc/project.md`，随实现推进持续更新。

---

## 1. HTTP/TLS 栈（影响 core）

### 1.1 整体认知：HTTP 客户端由什么组成

`reqwest` 是高层封装，底下分三层：

```
你的代码 (reqwest::Client)
    │
    ▼
hyper          ← HTTP 协议本身（纯 Rust）：HTTP/1.1、HTTP/2、重定向
    │
    ▼
TLS 后端        ← 加解密层（HTTPS 的 S），本节焦点
    │
    ▼
TCP + 证书验证   ← 信任"服务器是谁"的系统机制
```

HTTP 协议部分（hyper）两平台完全一样、纯 Rust、无差异；**差异全在最后两层——TLS 实现和根证书来源**。

### 1.2 TLS 的作用与"信任锚"

TLS 做两件事：加密（防抓包）+ 验证身份（防中间人）。验证依赖**信任锚**——一份**根证书（Root CA）列表**。设备收到服务器证书链，回溯到某个根证书，若在信任列表内则可信。

| 平台 | 信任列表存在哪 | Rust 程序能直接读到吗 |
|---|---|---|
| Windows | 系统证书库（Crypto API） | 能 |
| Android | Java KeyStore（Java 层私有） | **不能直接读** |

**这是全部差异的根源**：Windows 把信任库暴露给原生程序，Android 没有——必须用别的方式弄到信任列表。

### 1.3 reqwest 的两种 TLS 后端

**native-tls（Windows 默认）**：不是 TLS 实现，是"各家系统自带 TLS 的薄包装"：

| 系统 | native-tls 实际调用 |
|---|---|
| Windows | SChannel（系统自带） |
| macOS | Security Framework |
| Linux / Android | **OpenSSL**（第三方 C 库） |

- Windows 上免费：系统自带、读系统证书库、企业内网自签 CA 也认。
- **Android 上是灾难**：Android 系统无 OpenSSL 库，需用 NDK 把 OpenSSL C 代码交叉编译成 arm64 目标（`openssl-sys` 找不到库 / 版本不匹配 / 构建脚本报错，著名坑）。这是 `project.md` §1.2"Android 无 OpenSSL 依赖"的原因。

**rustls（纯 Rust 实现）**：不用 SChannel、不用 OpenSSL，逻辑全是 Rust 代码。交叉编译 Android 只需 Rust 工具链 + NDK C 编译器，**无需编译第三方 C 库**；跨平台行为一致。

- 代价：它不自动知道系统信任库，**根证书必须显式喂给**——引出下一节。

### 1.4 根证书喂给 rustls：三选一

| 方案 | 机制 | Windows 效果 | Android 效果 |
|---|---|---|---|
| **`webpki-roots`** | 把 Mozilla 根证书列表**编译进 APK/EXE** | 可用，但忽略手动安装的企业 CA | 可用，且**最简单** |
| **`rustls-native-certs`** | 运行时读**操作系统**信任库 | 完美，企业 CA 也认 | **不可用**（Android 读不到） |
| **`rustls-platform-verifier`** | JNI 调 Android `X509TrustManager`；Windows 调 SChannel | 完美 | 可用，且认用户/系统 CA |

- **webpki-roots**：硬编码根证书列表随包走，**离线**——Windows Update 更新了系统根证书它不知道，要等发新版 App；用户手动信任的证书（公司内网 CA、抓包代理）一概不认。
- **rustls-native-certs**：用 Windows API 拷系统证书库喂给 rustls，Windows 端完美；**Android 无实现**，天然只解决 Windows 端。
- **rustls-platform-verifier**：Android 走 JNI 调系统证书验证（与 Chrome 同一套信任逻辑），Windows 退回 SChannel。**唯一同时解决两端的方案**。
  - 现实坑：公司电脑常装内网 CA（`mitmproxy` 抓包、VPN 网关证书）。只打包 webpki-roots 时 Windows 访问内网文档站会报 `certificate verify failed`；platform-verifier 不会。webpki-roots 胜在零 JNI、构建简单。

### 1.5 mdor 的两处 HTTPS + gix 第二层

mdor 有**两处**走 HTTPS：

1. **reqwest**：StaticSiteSource 镜像 HTML、GitHub API 探测
2. **gix**：GitHubSource 的 `git clone/fetch`（git-over-HTTPS）

gix 的 HTTP 传输后端是**额外的平台差异点**：

| gix HTTP 后端 | 底层 | Android 交叉编译 |
|---|---|---|
| `curl`（gix 默认） | `curl-sys`（C 库） | 要编 curl 的 C 代码，麻烦 |
| `ureq` | 纯 Rust（rustls） | 顺利 |
| **`reqwest`** | 复用我们的 reqwest | 顺利，且 **TLS 栈与第一层完全统一** |

### 1.6 端到端对比与推荐方案

| 环节 | Windows（桌面开发） | Android（目标） |
|---|---|---|
| reqwest TLS 后端 | rustls（与 Android 一致） | rustls（必须，无 OpenSSL） |
| 根证书 | `rustls-native-certs` 读系统库，或 platform-verifier | webpki-roots 打包，或 platform-verifier(JNI) |
| gix HTTP 传输 | reqwest 后端 | reqwest 后端（避免 curl C 依赖） |
| 加密 provider | 默认 `aws-lc-rs`（构建需 cmake/perl）或 `ring` | 同左；`ring` 需 NDK clang 参与构建（NDK 自带，正常） |

**推荐统一方案**：两平台都用 `reqwest`（`default-features = false` + `rustls-tls`）+ `rustls-platform-verifier`，gix 开 `http-client-reqwest`。一份 Cargo 配置、一套信任逻辑，Windows/Android 行为一致；仅当"想要极致简单、放弃企业 CA"时才退回 webpki-roots。

### 1.7 选错的后果

- 默认 reqwest（native-tls）build Android → 交叉编译 OpenSSL 失败或出诡异构建错误
- rustls 却不喂根证书 → 能编能跑，但**所有 HTTPS 请求报 `certificate verify failed`**，下载功能全废
- gix 用 curl 后端 → Android 交叉编译需单独处理 `curl-sys`，且与上面 TLS 策略各管各的
- 只打包 webpki-roots 的 Windows 端 → 公司内网文档站 / 抓包代理环境访问失败，难排查

### 1.8 rustls 稳定性评估（选型依据）

**结论：足够稳定，可放心用于生产（含 Android 客户端场景）。**

| 维度 | 现状 |
|---|---|
| 成熟度 | 2016 年诞生，活跃维护；当前 0.23.x 系列（0.23.43），发布频繁、对 0.23 分支 backport 修复 |
| 审计 | 有过独立安全审计（`rustls/audit/TLS-01-report.pdf`），OpenSSF Best Practices 徽章 |
| 资金 | Prossimo（ISRG）主导，Google / AWS / Flyio 等资助——Rust 内存安全基建旗舰项目 |
| 生产采用 | **Let's Encrypt**（服务数亿网站的 CA）计划用 rustls 替换 OpenSSL；curl、Apache httpd `mod_tls`、Firefox 等在用 |
| 性能 | 与 OpenSSL/BoringSSL 对比：收发吞吐相当甚至略优，resume 握手明显更快；全握手略慢（非瓶颈） |

注意点：

- **版本号 0.x 而非 1.0**：按 semver 0.x 的 minor 版本可破坏 API；实际维护策略是"0.23 长期系列 + backport"，社区当作稳定版用。mdor 仅经 reqwest 间接使用，几乎不直接碰其 API，风险可忽略。
- **"纯 Rust"需打折**：默认加密 provider 是 `aws-lc-rs`（BoringSSL 的 Rust 包装，**含 C 代码**）；`ring` 的维护者在 2025 年宣布过"不再维护"（后续恢复）。不影响正确性——rustls 协议逻辑（握手/校验/证书链）为纯 Rust，C 部分仅在底层加解密原语，且有 AWS 背书、支持 FIPS。真正的纯 Rust provider（RustCrypto/graviola）偏新，不必为此选它。
- **Android 交叉编译**：`aws-lc-rs` 需 cmake/perl 参与构建，`ring` 需 NDK clang——均能正常编，是构建期工具要求而非稳定性问题。

**对 mdor 的定性**：本项目为**客户端**、只访问知名站点（github.com 等），不涉及服务端自定义证书 / 客户端证书 / 双向 TLS 等复杂面，属 rustls 最成熟的使用面，风险很低；配合 `project.md` §12.1 的 `cargo audit` 持续跟踪即可。维持 §1.6 推荐方案，无需因"稳定性"疑虑改动。

**版本管理说明（backport 与选版）**：

- **backport 含义**：把新主线（未来 0.24）上的修复 cherry-pick 回 0.23 分支再发补丁版，使 0.23 用户**无需升级大版本即可拿到安全修复**，且 API 不变。等价于一个 LTS 稳定线（功能在新主线开发，修复持续反哺 0.23）。
- **选版 = 最新 0.23.x**：rustls 是 reqwest 的传递依赖，具体小版本由 Cargo 解析 + `Cargo.lock` 锁定（当前 0.23.43），不需要手动挑；`cargo update` 保持在 0.23 系列即可（reqwest 自身会约束范围）。
- **版本对齐（防双版本）**：`rustls-platform-verifier` 直接依赖 rustls，需在根 `[workspace.dependencies]` 把 rustls 钉成与 reqwest 传递进来的同一 0.23.x 版本（对应 `project.md` §12.1"依赖版本统一"）。

## 2. WebView 宿主（影响 app 层，差异最大）

### 2.1 背景知识：Dioxus 渲染的是什么

Dioxus 是 React 风格 UI 框架。RSX 编译为虚拟 DOM，默认渲染器是 **HTML/CSS**——应用本质是**藏在浏览器内核（WebView）里的网页**：

```
RSX（组件树）→ 虚拟 DOM → HTML/CSS/JS → 注入 WebView → 屏幕
```

Rust 与网页通过桥接双向通信。**"WebView 宿主差异"的本质**：同一个 Rust 应用，Windows 用微软的 WebView2 内核，Android 用安卓的 System WebView，两者对接方式完全不同。

### 2.2 底层三件套

| 库 | 作用 | Windows 端 | Android 端 |
|---|---|---|---|
| **winit** | 创建窗口 + 事件循环 | 调 Win32 API 开窗口 | 基本不直接参与（Android 无"创建窗口"） |
| **wry** | 在窗口里塞浏览器内核 | 装 WebView2 | 装 Android System WebView |
| **android_activity** | Rust 与 Android 对接（JNI） | 不存在（无 Java 层） | `Activity` 生命周期经 JNI 调进 Rust |

关键认知：

- **Windows**：程序从 `main()` 主动启动，自己开窗口、自己跑循环。
- **Android**：**没有 `main()` 入口**。系统启动一个 `Activity`，在固定时间点调用（`onCreate → onStart → onResume → ... → onDestroy`），`android_activity` 把生命周期事件转发给 Rust。代码是"被系统叫醒"，不是"主动跑起来"。

### 2.3 逐维度对比

#### 入口与进程模型

| | Windows | Android |
|---|---|---|
| 入口 | `main()` 主动启动 | `AndroidMain`（系统创建 Activity 触发） |
| 主线程 | winit 事件循环跑在主线程 | 系统"UI 线程"（Looper） |
| 渲染进程 | WebView2 独立 Chromium 子进程（无需管理） | 系统 WebView 独立渲染进程（无需管理） |

→ `mdor-app/src/main.rs` 必须有双入口路径（`cfg` 区分）；Android 上进程存活/被杀（低内存）与桌面完全不同。

#### WebView 运行时可得性

| | Windows | Android |
|---|---|---|
| 内核 | WebView2（Edge Chromium），**可能没装**，需检测/引导安装 | System WebView，**系统自带**（可商店更新） |
| 版本 | 跟随 Edge Runtime | 跟随系统 WebView |

→ Windows 开发机必须装 WebView2 Runtime（Win11 自带，Win10 需装）；Android 注意 minSdk。

#### 线程模型（最易踩坑）

| | Windows | Android |
|---|---|---|
| UI 操作线程 | winit 主线程；后台更新 UI 需经事件循环投递 | **必须 Android UI 线程**，后台碰 UI 直接崩/抛异常 |
| 桥接 | wry 封装好的 JS ↔ Rust | JNI；创建 WebView 必须在主线程 |

→ tokio 下载完成后刷新 UI：Windows 随便投递即可，Android **必须显式切回主线程**。

#### 自定义协议 `mdor-book://`（doc §11 风险项，差异核心）

背景：阅读器加载本地章节 HTML/图片，网页内 `<img src="...">` 默认走 `http://`。自定义协议 = 当 WebView 请求 `mdor-book://xxx` 时，引擎回调 Rust 代码从本地磁盘返回字节——不经过网络、不受文件访问限制。

| | Windows (WebView2) | Android (System WebView) |
|---|---|---|
| 自定义 scheme | `AddWebResourceRequestedFilter` 注册，可用但 API 异步、有怪癖 | **支持很差**：导航可用 `shouldOverrideUrlLoading` 拦，但资源加载（img/css/script）走 `shouldInterceptRequest`，自定义 scheme 历来不可靠 |
| 替代方案 | 勉强可用 | **建议不用**，改 `http://127.0.0.1:port/...` |

→ 这就是 `tiny_http` 备选的原因：Android 上起本地 HTTP 服务，用 `http://127.0.0.1:PORT` 加载资源。两平台原生支持 http，行为统一，绕开 scheme 兼容问题；代价是本地端口管理、需绑 127.0.0.1 防外访问。

#### 静态资源加载（CSS/字体/图标）

| | Windows | Android |
|---|---|---|
| 资源来源 | 直接读磁盘文件 | 打包进 APK `assets/`，用 `file:///android_asset/...` 或 `content://` |
| 文件访问 | WebView2 能读本地文件 | WebView 默认**禁读任意文件路径**（安全沙箱） |

→ 书籍 CSS/代码高亮：桌面读文件即可，Android 走 assets 打包或统一经 `mdor-book://`/本地 http 分发。影响 `Dioxus.toml [asset]` 配置。

#### 内核版本与渲染一致性

两个都是 Chromium 但**版本不同**（WebView2 跟 Edge，Android 跟系统）。CSS/JS 有细微差异 → doc §6.5"内联书籍 CSS 保证样式一致"即为此对冲。

#### 开发调试体验

| | Windows | Android |
|---|---|---|
| DevTools | WebView2 自带调试器（F12 类） | `chrome://inspect` + adb 远程调试 |
| 热重载 | `dx serve` 直接热更 | 需连设备/模拟器，迭代慢 |

#### 构建工具链

| | Windows | Android |
|---|---|---|
| 命令 | `dx serve --platform desktop` | `dx serve --platform android` |
| 产物 | 直接跑 EXE | 生成 `mobile/` Gradle 工程 → APK |
| 额外配置 | 无 | manifest、minSdk、`android_activity` 接入 |

### 2.4 对 mdor 的落地影响

1. **依赖分层**：`mdor-core` 完全不碰 WebView（纯逻辑）；差异全在 `mdor-app`，用 `[target.'cfg(target_os = "android")'.dependencies]` 隔离。
2. **资源协议层**：`render/resources.rs` 的"协议→文件路径"映射逻辑在 core 复用，但**注册/拦截的接入点在 app 侧分平台实现**——Windows 用 WebView2 scheme handler，Android 用本地 http 服务器（或先走 http）。这是 M3/M6 需验证的最大不确定项（doc §11 风险行）。
3. **main 双入口**：桌面 `main()` + Android `AndroidMain`，启动时解析数据目录（对应 §3）。
4. **线程纪律**：所有 UI 更新最终发生在主线程；tokio 后台任务回 UI 需显式切换。

## 3. 数据目录与存储路径（app 层注入）

- `project.md` §9 / §12.1：Windows 走 `dirs` 用户数据目录；Android 走 JNI `getFilesDir()`。
- core 的 `BookStore::new(base_dir)` 平台无关，路径解析在 `mdor-app` 启动时 cfg 分支。

## 4. 文件系统语义（core 内需注意）

- 原子 `rename`：Windows(NTFS) 与 Android 均原子，`project.md` §6.7 已论证。
- **Windows 特有**：`MAX_PATH` 长路径、大小写不敏感 / 保留文件名 → gix 仓库在 Windows 上可能踩坑；**autocrlf 换行转换**需在 gix 侧处理（GitHub 源 mdBook 是 LF）。
- fixtures / 测试样例路径分隔符需用 `Path` 抽象，避免硬编码。

## 5. 安全 / 明文策略（app 层）

- Android 默认禁 `http://` cleartext，需 network security config（`project.md` §10 M6"cleartext 配置"）；Windows 无此限制。
- Android 应用私有目录免权限；Windows 用户目录同理，均无额外权限依赖。

## 6. 构建工具链（工程层）

- `project.md` §12：`rust-toolchain.toml` 固定 1.97.1 + android targets（arm64-v8a / x86_64）。
- Windows 本机用 MSVC 目标；Android 需 NDK + clang 交叉编译环境。
- **依赖"纯 Rust"是对 Android 的硬约束，对 Windows 则宽松**——这是选 serde_json（非 SQLite，§6.7）、gix（非 git2，§7.4）、rustls（非 openssl）的原因；任何新增 C 依赖都会引入 Android 交叉编译差异点。

## 7. 可靠性策略细节

- `project.md` §6.7：`fsync` 在 Android 上"可选"（性能），Windows rename 已足够 → 可做成平台感知策略。

## 8. 测试策略

- core 平台无关，Windows 桌面直接 `cargo test` + `httpmock`（§12.2）；Android 不能跑单测，集成测试验证依赖 Windows 环境。

---

## 汇总

平台差异集中在：

- **mdor-app**（WebView 宿主、数据目录、cleartext、JNI）；
- **core 少数点**（TLS 根证书、fsync 策略）；

需用 `cfg(target_os)` / target 条件依赖隔离；core 保持平台无关是 `project.md` §2 设计原则的约束。
