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
| 加密 provider | **`ring`**（轻量，免 cmake/perl） | 同左；`ring` 需 NDK clang 参与构建（NDK 自带，正常）；如需换 `aws-lc-rs` 见 §1.9 |

**推荐统一方案**：两平台都用 `reqwest`（`default-features = false` + `rustls-tls`）+ `rustls-platform-verifier`，gix 开 `http-client-reqwest`。加密 provider 用 **`ring`**（轻量、构建简单、APK 体积小）；将来如需 FIPS / 更广算法面，按 §1.9 一行切换 `aws-lc-rs`。一份 Cargo 配置、一套信任逻辑，Windows/Android 行为一致；仅当"想要极致简单、放弃企业 CA"时才退回 webpki-roots。

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
- **"纯 Rust"需打折**：加密 provider 定为 **`ring`**（Rust + 手写汇编，非纯 Rust，无 FIPS）；原默认 `aws-lc-rs`（BoringSSL 的 Rust 包装，**含 C 代码**，有 FIPS）为可替换项（§1.9）。`ring` 的维护者在 2025 年宣布过"不再维护"（后续恢复），维护风险由 §1.9 可插拔机制兜底。不影响正确性——rustls 协议逻辑（握手/校验/证书链）为纯 Rust，C/汇编部分仅在底层加解密原语。真正的纯 Rust provider（RustCrypto/graviola）偏新，不必为此选它。
- **Android 交叉编译**：`ring` 需 NDK clang（NDK 自带，正常），同时规避了 `aws-lc-rs` 需 cmake/perl 参与构建的依赖——是构建期工具要求而非稳定性问题。

**对 mdor 的定性**：本项目为**客户端**、只访问知名站点（github.com 等），不涉及服务端自定义证书 / 客户端证书 / 双向 TLS 等复杂面，属 rustls 最成熟的使用面，风险很低；配合 `project.md` §12.1 的 `cargo audit` 持续跟踪即可。维持 §1.6 推荐方案，无需因"稳定性"疑虑改动。

**版本管理说明（backport 与选版）**：

- **backport 含义**：把新主线（未来 0.24）上的修复 cherry-pick 回 0.23 分支再发补丁版，使 0.23 用户**无需升级大版本即可拿到安全修复**，且 API 不变。等价于一个 LTS 稳定线（功能在新主线开发，修复持续反哺 0.23）。
- **选版 = 最新 0.23.x**：rustls 是 reqwest 的传递依赖，具体小版本由 Cargo 解析 + `Cargo.lock` 锁定（当前 0.23.43），不需要手动挑；`cargo update` 保持在 0.23 系列即可（reqwest 自身会约束范围）。
- **版本对齐（防双版本）**：`rustls-platform-verifier` 直接依赖 rustls，需在根 `[workspace.dependencies]` 把 rustls 钉成与 reqwest 传递进来的同一 0.23.x 版本（对应 `project.md` §12.1"依赖版本统一"）。

### 1.9 加密 provider 可插拔性（选 ring，随时可换）

**现状**：加密 provider 定为 **`ring`**（轻量、免 cmake/perl、APK 体积小）。但这不是写死的——rustls 的 **`CryptoProvider` 抽象**让 provider 可插拔，`ring` / `aws-lc-rs` 只是它的两个现成实现，切换成本很低。

**机制**：进程内通过 `install_default()` 装一个默认 provider，所有 TLS 都走它：

```rust
// mdor-app 启动最早处（main / AndroidMain）
rustls::crypto::ring::default_provider().install_default()          // 现选 ring
// rustls::crypto::aws_lc_rs::default_provider().install_default()  // 将来可换 aws-lc-rs
```

**切换成本 = Cargo feature 一行 + 重新打包**：

- 构建期选一个：`reqwest` 用 `default-features = false` + 对应 feature（`rustls-tls-ring` 或 `rustls-tls-aws-lc-rs`），换 provider = 改 `Cargo.toml` 一行 + 重发版。
- **不做运行时双 provider 注入**（`rustls-no-provider` 方案要把两个 provider 都编进包，体积翻倍）——mdor 是打包分发的客户端，换 provider 场景就是"发现问题 → 发版修复"，构建期切换足够，无需体积代价。

**两处 TLS 同时生效**：mdor 的 TLS 只经 reqwest 一层（§1.5），gix 复用同一个 reqwest → 换 provider **一处切换、两处全生效**，无需改 gix。

**注意点**：

- **reqwest 0.13 起不代选 provider**：必须自己 `install_default()`，且要在 `main()` / `AndroidMain` **最早处**调用（对应 §2.4"main 双入口"），否则 `Client::new()` 直接 panic（0.12 无此要求）。
- **aws-lc-rs 独有项**：FIPS、post-quantum 算法；切到 ring 会丢，mdor 用不到，无影响。
- **版本对齐照旧**：provider 换不换，`rustls-platform-verifier` 都在，仍按 §1.8 钉同一 0.23.x。

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

**决策（2026-08-09）：两端统一走本地 HTTP 服务器分发，自定义 scheme 降级为后续可选功能。**

- **背景**：阅读页是注入 HTML，其子资源（图片/CSS/JS）原本指向远端站点，离线时必须从本地磁盘返回字节；自定义 scheme 就是为此设计的一种"Rust 能回答的 URL"。直接 `file://` 不可行——Android WebView 沙箱禁读任意文件路径，且书籍数据运行期动态增长，不可能打包进 APK `assets/`。
- **结论**：首版 Windows 与 Android **统一用本地 `tiny_http` 服务器**，以 `http://127.0.0.1:PORT/...` 绝对 URL 分发阅读页子资源；Windows 不再用 WebView2 scheme handler。`mdor-book://` 自定义 scheme **降级为后续用户可选**（设置里的"资源通道"选项），前提是 `render/resources.rs` 保持"URL→文件路径"映射抽象、重写层可插拔，将来切换无需改 core。
- **依据**：自定义 scheme 的子资源拦截在两端不对称——WebView2 `AddWebResourceRequestedFilter` 可用但 API 异步、有怪癖；Android `shouldInterceptRequest` 对自定义 scheme 历来不可靠（导航可拦、img/css/script 子资源不稳定）。http 是两平台 WebView **原生最可靠**的子资源通道；两端 URL 形态一致，`html_extract.rs` 重写逻辑无平台分叉，core 保持平台无关。代价（端口管理、绑 127.0.0.1、cleartext 配置）均为可控工程问题。

#### 为什么是 http 而非 file://（正则重写为何不选）

**常见疑问**：注入 HTML 是否因为"WebView 访问不了本地文件"才被迫走 http？如果文件可以访问，能否用正则按规则把 URL 转成本地文件地址？

**答案：不是单一原因，是两层叠加。** 正则重写机制本身完全可行（`html_extract.rs` 一直在做），但**重写的目的 scheme 不能是 `file://`**：

1. **注入 HTML 的相对 URL 先按宿主文档 base 解析**：阅读页是注入到 App 文档（release 为 `dioxus://`、桌面 dev 为 dev server 地址）里的，相对 URL（如 `chapter.png`）解析后指向宿主文档源，无人回答 → 无论走什么方案都必须先重写为绝对地址（这一层与"能否访问文件"无关）。
2. **Chromium 子资源安全模型封死 `file://`**：非 file 源的文档（正是 mdor 的宿主文档）加载 `file://` 子资源会被系统性拦截（"Not allowed to load local resource"）。两端各自再加一道墙：
   - Windows：WebView2 能读磁盘，但宿主文档是 `dioxus://` 自定义协议而非 file 源 → 仍被拦截；
   - Android：沙箱禁读任意路径 + 运行时数据目录不在打包 assets 内，`file://` 从根上不可行。
3. **`http://127.0.0.1:PORT` 是两端唯一无条件放行的"合法网络访问"子资源通道**：回环 http 被 Chromium 视为正常网络访问，不依赖文档源、不依赖打包时机。看似多绕一层网络，实际比 `file://` 更简单——`file://` 的 URL 编码（Windows 盘符/反斜杠、Android 运行时 `/data/...` 路径）在两端各有各的坑，反而导致重写逻辑分平台。

**结论**：走 http 不是因为"文件读不了"这一个原因，而是"Android 真读不了运行时数据目录" + "即使能读，file:// 子资源从非 file 文档源发起在两端都被安全模型拦截"两层叠加。正则/规则重写照常做，只是目的地址统一选 http。

#### 版本管理与重写时机

**重写发生在渲染时，不是更新/commit 时。** 存储层（git 对象库/工作区）永远是**上游原样内容**，URL 一个字符都不动：

```
更新时（存）:  fetch 内容 → 原样 commit → 打版本 tag      ← 内容零改动
渲染时（读）:  读章节 → 抽取 → 按规则重写 URL → 注入      ← 内存中临时改
```

三条理由，缺一不可：

1. **端口是动态的，更新时根本不知道**：服务器用 `bind("127.0.0.1:0")` 动态端口，只有运行时才确定。URL 里要带端口，就决定了重写**不可能**发生在无端口概念的更新/commit 阶段。
2. **保上游原样**：场景 1 克隆的上游仓库必须保持原状（不能污染克隆历史）；场景 2 自建链的意义就是"忠实镜像"。若更新时就重写并 commit，版本间 diff 会被重写产物污染，版本比较/回滚全部失真。
3. **切版本零成本**：重写规则与版本无关，只是把"当前工作区文件路径"转成 URL。切版本 = 换工作区内容 = 重新渲染一遍即可，无需任何存储级操作（与 project.md §11"切版本与 webview 协调"风险项正交，那边由"先加载到内存再切换 / 切换后 reload"处理）。

#### 重写规则与一一对应

**规则不是一条全局正则，而是"按来源定义的解析函数"**——因为"原始 URL → 工作区文件路径"需要上下文：

| 来源 | 原始 URL 形态 | 需要的上下文 |
|---|---|---|
| StaticSiteSource（镜像站） | 相对 `img/a.png` 或绝对 `https://host/book/img/a.png` | 章节自身路径（解相对）+ 镜像 base URL（解绝对） |
| GitHubSource（md 仓库） | markdown 相对路径 `img/a.png` | 章节自身路径（解相对） |

分两步、方向相反、共用同一套规范化逻辑：

```
重写方向（渲染时，html_extract.rs）:
  原始 URL
    ├─ 相对 → 按章节所在目录 resolve（等价浏览器 base 解析）
    └─ 绝对 → 按镜像 base 剥离出书内路径
    → 规范化路径（URL decode、消 ./ 与 ../、限制在书根内）
    → http://127.0.0.1:PORT/books/<id>/<path>

服务方向（服务器，resources.rs 同规则）:
  URL 路径 /books/<id>/<path>
    └─ 校验规范化后仍在书根内（防 ../ 穿越）
    └─ 定位工作区文件，读字节返回
```

**关键**：重写与服务两个方向必须**共享同一套规范化逻辑**（这正是 `resources.rs` 承担"唯一事实来源"的原因），保证对称，不产生"重写出的 URL 服务器读不到"的错位。

**一一对应成立在"规范化路径 ↔ URL"层，而不是"原始 URL ↔ URL"层**：

| 情形 | 是否破坏一一对应 | 说明 |
|---|---|---|
| 同一资源两种拼写（`./img/a.png` 与 `img/a.png`） | 不破坏 | 规范化为同一路径 → 同一 URL，本就是一个文件 |
| 两个章节都有 `img/a.png` | 不破坏 | 解析带章节目录上下文 → `ch1/img/a.png` ≠ `ch2/img/a.png`，天然区分 |
| 查询串 `a.png?v=2` | 不破坏 | 服务器忽略 query，同文件 |
| 锚点 `#anchor` | 不破坏 | fragment 到不了服务器，重写时剥离 |

真正要防的不是"一对多"，而是三类风险：

1. **`..` 目录穿越**：规范化层吃掉并拒绝（服务器侧白名单校验）；
2. **URL 编码对称性**：中文/非 ASCII 文件名（书里常见）两端必须统一 percent-encode/decode，否则重写编码与服务解码错位 → 404；
3. **大小写平台差异**：Windows NTFS 大小写不敏感、Android ext4 敏感（§4.3），URL 路径按原始内容编码，Windows 命中、Android 可能 404——是平台差异问题而非映射问题，靠 fixtures 规避同名不同大小写 + 必要时查实际文件。

**诚实边界**：映射只在"本地确实存在的资源"上一一对应。GitHub 里 markdown 引用绝对 CDN 图片等未镜像资源时，重写规则**不命中**（fall-through 保留原样或标记缺失）——是部分映射，范围外不承诺。

**URL 不带版本号（2026-08-09 决策）**：URL 形态为 `http://127.0.0.1:PORT/books/<id>/<path>`，不含 `<version>`。理由：

- **URL 里的 version 是"声称值"不是"保证值"**：服务器只从当前工作区读文件（单工作区 checkout 设计）。渲染旧版本 = 先 checkout 再渲染，URL 声称 v2 时内容确实 v2——但这只在"checkout 与渲染严格串行、渲染期间不切工作区"时成立；一旦有竞态，URL 声称的版本与实际内容不符，排查时反而被误导；
- **调试场景不依赖它**：DevTools Network 面板 / 服务器访问日志 / 截图排查，版本号由 App 状态与日志承载（渲染时本来就知道 version），无需编码进 URL；
- **缓存问题用响应头根治**：切版本后同路径资源吃旧缓存，靠本地服务器统一回 `Cache-Control: no-store` 解决，比 URL 带 version 更可靠；
- **未来可加回**：若将来改为"按版本从 git blob 直接服务"（不 checkout 工作区），version 成为寻址必需，届时加回 `<version>` 段是纯增量，现在不预支。

#### 本地 http 方案的 Android 限制与对策

Android 上跑本地 http 服务器不是没有限制，逐项列清并给对策（多数为配置/工程问题，非死路）：

| 限制 | 说明 | 对策 |
|---|---|---|
| cleartext 禁止 | Android 9+ 默认禁明文 http，连 `http://127.0.0.1` 也被拦 | network security config 白名单**仅放行 127.0.0.1**（§5.1） |
| INTERNET 权限 | 无该权限 bind socket 直接失败 | manifest 声明；mdor 因 reqwest 下载本就必加 |
| 服务器线程 | 不能跑在 UI/主线程 | `tiny_http` 独立线程池，主线程不受阻 |
| 端口冲突 | 固定端口可能被其他 App 占用 | `bind("127.0.0.1:0")` 动态端口；端口须先于渲染确定并写入重写 URL |
| 进程被杀 | 退后台/低内存被回收，服务器随进程消失 | 阅读页打开期间保持前台；不依赖常驻 Service |
| 目录穿越 | 绑 127.0.0.1 不等于安全 | URL 路径白名单/规范化校验，防 `../` 读任意文件 |
| 跨源（CORS） | App 页面 origin（dioxus:// 等）与 `http://127.0.0.1` 不同源 | 仅子资源加载无碍；若涉及 iframe/读写需 CORS 头 |
| 开发期端口占用 | 桌面 `dx serve` 开发服务器占端口 | 仅开发期注意；发布版无影响 |

#### 静态资源加载（CSS/字体/图标）

| | Windows | Android |
|---|---|---|
| 资源来源 | 直接读磁盘文件 | 打包进 APK `assets/`，用 `file:///android_asset/...` 或 `content://` |
| 文件访问 | WebView2 能读本地文件 | WebView 默认**禁读任意文件路径**（安全沙箱） |

→ **决策（2026-08-09）：App 静态资源分两类处理**——
- **背景**：App 有两类静态资源、被不同页面消费：① App 自身 UI（书架/设置/抽屉）的 CSS/字体/图标；② 阅读页要用的样式（书籍渲染 CSS、代码高亮主题）。前者由 Dioxus 渲染，后者是注入 HTML，WebView 加载时其 `<link>` 引用必须可访问。
- **结论**：App UI 资源走 Dioxus `[asset]` 打包（`asset!()` 宏两平台自动适配，Android 进 APK assets，无平台差异）；**阅读页样式与书籍资源同通道**，统一经本地 http 分发（从 `assets/` 样式目录映射，或随书存储分发）。
- **依据**：阅读页是注入 HTML，样式引用必须能被 WebView 访问；与书籍资源同通道才能保证两端 `<link>` 引用 URL 形态统一、core 无分叉。影响 `Dioxus.toml [asset]` 配置：`[asset]` 只服务于 App UI，不承担阅读页样式。

**实现分支（M3 待验证，2026-08-09 记录）**：本地 http 服务器跑在 Rust 侧、读的是磁盘文件，但 Android 上"随 App 分发"的样式打进的是 APK（zip），Rust 无法像读普通文件那样读 APK 内部（需 JNI AssetManager）。阅读页样式要能被服务器分发，两条路二选一：
- **内嵌进二进制**：样式体积小（几 KB），用 `include_bytes!`/`include_str!` 内嵌，服务器从内存直接吐（最省事，倾向此路）；
- **首启复制**：启动时把样式复制到 `getFilesDir()`，服务器照常读磁盘。

M3 实现时敲定。

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
2. **资源协议层（已决策 2026-08-09）**：`render/resources.rs` 维护"本地 URL → 文件路径"映射（core 复用、跨平台一致）；**接入点在 app 侧统一为本地 `tiny_http` 服务器**，Windows 不再用 WebView2 scheme handler。URL 形态两端一致：`http://127.0.0.1:PORT/books/<id>/<path>`（**不带版本号**，理由见 §2.3「重写规则与一一对应」末尾；缓存由服务器 `Cache-Control: no-store` 头根治）。
3. **main 双入口**：桌面 `main()` + Android `AndroidMain`，启动时解析数据目录（对应 §3）。
4. **线程纪律**：所有 UI 更新最终发生在主线程；tokio 后台任务回 UI 需显式切换。
5. **渲染形态（已决策）**：章节 HTML 用 `dangerous_inner_html` 注入，资源链接在 `html_extract.rs` 统一重写为**绝对 http URL**（不用 `<base>` 标签、不用 iframe）。依据：innerHTML 与"Dioxus 自绘导航/TOC + `scroll_ratio` 节流存进度 + 不依赖 JS"的设计基石契合；iframe 虽 HTML 保真度高（脚本/锚点/样式隔离），但 App 页面与 `http://127.0.0.1` 跨源，Dioxus 读不到 iframe 内部滚动 → 破坏统一滚动进度跟踪，需 postMessage 桥或全页同源化，复杂度不划算。

## 3. 数据目录与存储路径（app 层注入）

**背景知识：为什么两端数据根不同**

桌面（Windows）与 Android 对"应用数据能放哪"的约束本质不同：

- **Windows：原生进程、系统不设限 → 位置是产品选择。** 桌面程序与登录用户同权限，写 C 盘任意目录、用户目录、exe 同目录都合法。所以选"便携式 exe 同目录"是**产品决策**而非系统要求：不污染 `%APPDATA%`/系统目录，数据随 exe 走、整个目录可拷贝迁移；约束只在权限——目录不可写则直接报错。
- **Android：沙箱强制 → 位置由系统指定，应用只有取舍。** 每个应用默认只可访问两块目录：
  - 内部私有目录 `getFilesDir()`（`/data/data/<包名>/files`）：免权限、只有本应用可读、随卸载删除；
  - 外部存储（`/sdcard/`、`getExternalFilesDir()`）：需运行时权限、用户可插拔/清理、其他应用可读。
  - mdor 选内部目录：免权限申请、数据与应用同生命周期；代价是路径只能运行时经 JNI 从 Android 框架获取（`main()` 阶段拿不到），且卸载即丢（离线书可重下，可接受）。

**为何最终结构仍对称**：差异只收敛在"数据根"一层（Windows=exe 同目录，Android=`getFilesDir()`），根之下的 `data/bookstore/` 是产品自定义结构、与平台无关，core 只依赖这一层 → 差异落在 `mdor-app` 启动 cfg 分支，core 保持平台无关。

- 两端存储结构**对称**：数据根下统一为 `data/` 分层（书籍数据在 `data/bookstore/`），仅"数据根"因平台而异。
- Windows（桌面）：数据根 = **exe 同目录**（`std::env::current_exe()`），`data/bookstore/` 不存在则 `create_dir_all`，**目录不可写直接报错**，不写 `%APPDATA%`、不回退到系统用户目录（便携式）。
- Android：数据根 = JNI `getFilesDir()`（应用私有目录），同样 `data/bookstore/` 分层。
- core 的 `BookStore::new(base_dir)` 平台无关，只认 `bookstore/` 这一层；数据根解析在 `mdor-app` 启动时 cfg 分支完成（对应 `project.md` §9 / §12.1）。

## 4. 文件系统语义（core 内需注意）

### 4.1 背景知识：为什么"同一份 Rust 代码"在两台设备上行为不同

mdor 的 `BookStore` 要读写文件，但"读写文件"不是简单地把字节放进磁盘——最终都落到操作系统的**文件系统**上执行。文件系统决定：路径怎么解析、文件名允许哪些字符、改名是不是原子、大小写敏不敏感、换行怎么处理。

```
Rust 代码（Path / fs 操作）
    │
    ▼
系统调用（read / write / rename / fsync）
    │
    ▼
文件系统实现（NTFS / ext4 / F2FS）
    │
    ▼
磁盘
```

- **Windows** 用 **NTFS**：大小写不敏感、有 260 字符路径上限、有保留文件名。
- **Android 手机**用 **ext4 / F2FS**（闪存优化）：大小写敏感、无路径上限、无保留文件名。

规则不同，同一段 Rust 代码的表现就不同——这就是 core 内必须注意差异的根源。好在**大多数语义一致**，只有少数点是 Windows 特有坑。

### 4.2 原子写与 rename（为什么 core 敢放心用）

程序写完一个文件，不能直接覆盖正式文件——万一写一半进程被杀/断电，文件就变成"一半旧一半新"的脏数据，下次读取无法解析（对 `progress.json` 这类关键元数据是灾难）。

业界解法是**原子写**：

1. 先写到 `xxx.tmp`（临时文件）；
2. 全部写完再 `rename` 覆盖成正式名。

`rename` 在**同一个文件系统内**是原子操作：要么看到完整的旧文件，要么看到完整的新文件，**不存在中间态**。

| 平台 | 文件系统 | 同目录 rename 原子性 |
|---|---|---|
| Windows | NTFS | 原子 |
| Android | ext4 / F2FS | 原子 |

→ **两端一致**，core 的 `write_json_atomic`（`project.md` §6.7）可以放心用。注意：rename 保证"无脏文件"，但不保证"断电后新文件已在磁盘上"——那部分由 fsync 兜底，见 §7。

### 4.3 Windows 特有坑（Android 没有）

| 坑 | 现象 | 例子 | 规避 |
|---|---|---|---|
| `MAX_PATH`（260 字符路径上限） | 深层路径操作失败 | mdBook 深层章节 `book/src/very/long/.../chapter.md` 一长串很容易超限 | `\\?\` 长路径前缀，或规避超深目录 |
| 大小写不敏感 | NTFS 默认不区分大小写，但 git 区分 | 仓库里同时有 `Foo.md` 和 `foo.md`，Windows checkout 冲突/丢文件 | gix 侧注意；fixtures 避免同名不同大小写 |
| 保留文件名 | 不能用作文件名 | `CON`、`PRN`、`AUX`、`NUL` 等设备名 | 命名避开 |
| autocrlf 换行 | git 默认按 `core.autocrlf` 把 LF↔CRLF 互转 | GitHub 源 mdBook 是 LF，转换后章节内容与上游不一致 | gix 侧关闭/固定换行 |

**路径分隔符**：Windows 用 `\`，Android 用 `/`。硬编码拼接必出错 → 必须用 `Path`/`PathBuf`（Rust 自动适配当前平台），代码里不出现分隔符字面量。

### 4.4 对 mdor 的落地影响

1. 原子写统一封装为 `write_json_atomic`（`store/util.rs`），core 内复用，两端一致。
2. gix 仓库在 Windows 上的坑：长路径、大小写、autocrlf——主要在 checkout/clone 时触发，需在 gix 配置侧规避（机制梳理与候选方向见 §4.5，策略待 M1 实测后敲定）。
3. fixtures / 测试样例路径用 `Path` 抽象，避免硬编码分隔符。

### 4.5 gix 三坑的配置规避：机制梳理与待定（讨论记录 2026-08-09）

**现状**：§4.3 三个 Windows 特有坑（长路径 / 大小写 / autocrlf）的规避列分别写"gix 侧注意 / gix 侧关闭·固定换行"，§4.4 落地影响 2 说"需在 gix 配置侧规避"，但**具体策略未定**：施加到哪个层级、用什么机制、何时施加，均未明确。

**为什么悬而未决**：gix 是库而非 CLI，没有 `git config` 命令——"在 gix 侧配置"有多种机制可选（写 repo 配置 / 打开时传覆盖 / 改 checkout 选项），作用域与持久性各不相同；且三个坑性质不同，不能一刀切。

#### 4.5.1 gix 配置机制（背景）

git 配置按优先级分层：`system < global（用户）< local（.git/config）< worktree < env`，gix 照常读取 system 与 global（含 Windows 上 Git for Windows 写入的配置）。gix 提供三个程序化入口：

| 入口 | 作用域 | 持久性 |
|---|---|---|
| `Repository::config_mut()` → `SnapshotMut`（drop 时自动 commit） | 当前仓库 | 落盘 `.git/config`（local 级） |
| `gix::open::Options::config_overrides` | 本次打开的所有操作 | 纯内存、不落盘 |
| `gix::config::tree` | 类型化已知 key（如 `Core::AUTO_CRLF`） | — |

checkout 行为（CRLF 转换、属性）由 checkout options 驱动，checkout options 从 config 构建（`gix::config::checkout_options`）；checkout 流水线经 `gix-filter` 做 CRLF 转换。

**关键风险——全局约定是毒药**：gix 会读到用户机器全局配置，其中常有 Git for Windows 的 system 级 `core.autocrlf=true`（实锤案例：helix #6467，gix 能解析到 Git for Windows 的 system 配置并用于 autocrlf）。mdor 要求工作区字节 = 上游字节，任何 CRLF 转换都破坏它，且与 Android 行为不一致。故不能靠"用户改全局配置"这类约定，必须由 mdor 主动在更高优先级压掉。

#### 4.5.2 三坑性质不同，不能一刀切

| 坑 | gix 真实行为 | 配置能否解决 |
|---|---|---|
| autocrlf | gix 默认遵循配置（含 system/global），会真做 LF↔CRLF 转换 | **必须**显式压为 `false`，有明确手段 |
| ignorecase | clone/init 时经 `create::Options::fs_capabilities` 探测文件系统并写入相应 git-config 字段（NTFS 上大概率自动写 `core.ignorecase=true`） | 只能让索引比较按大小写不敏感；**救不了**"上游同时含 `Foo.md` 与 `foo.md`"的物理冲突（NTFS 存不下两个） |
| longpaths | 260 限制是 Win32 API 限制而非 NTFS；Git for Windows 用旧 API 才需 `core.longpaths`；gix 走 Rust `std::fs`（内部宽字符 API + 超长路径自动 `\\?\`） | gix 大概率**不需要**；设它仅为 git CLI 互操作 / 防御 |

#### 4.5.3 autocrlf 深究：git 为何平时不"误判"，mdor 为何例外（讨论补充 2026-08-09）

**日常直觉**：Windows 开发每天 clone/pull 从不需关注 autocrlf，git 也从不"误认为本地与上游不一致"——这个直觉是对的，普通单机使用确实如此。

**为什么 git 不误判（对称过滤器）**：

```
检出 smudge:  blob(LF)   ──>  工作区(CRLF)
入库 clean:   工作区(CRLF) ──>  blob(LF)
```

- 索引/对象库永远存 LF；`status`/`diff` 比较的是"clean 之后的工作区 vs blob"——**两边先归一化到 LF 再比**，永远干净。
- 与"上游"的比较发生在 blob（LF）层，与本地配置无关。
- 前提：**配置稳定 + 所有比较都过过滤器**。此时 autocrlf 完全隐形。

**打破对称的三种触发条件**（才会出现"全变了 / 假差异"）：

| 触发 | 现象 | mdor 是否可能 |
|---|---|---|
| 配置在两次操作间变化（改 autocrlf / 加 `.gitattributes` `text=auto`） | 整库文本"全变"，直到重新 normalize | 仓库在两台配置不同的机器间打开，或机器中途装 Git for Windows |
| 仓库里已存在 CRLF 字节 | autocrlf 只转新写入内容，**不影响已在仓库里的内容**；这些文件永远与 LF 预期不符 | 上游恰好含 CRLF 文件（少见） |
| 绕过过滤器直接比原始字节（`--no-convert`、外部 diff 工具、程序读磁盘字节与 blob 对） | 磁盘 CRLF vs blob LF，必然不等 → 假差异 | **正是 mdor 渲染/哈希/测试的消费方式** |

**mdor 逐场景定位**：

| 场景 | 是否产生版本噪音 | 说明 |
|---|---|---|
| 场景 1（clone 上游 + 只读） | **不会** | 变更检测用上游 git **tree 对象 hash**（§7.2"内容树 hash"），天然与本地 autocrlf 配置无关；autocrlf 只影响工作区字节本身（保真 / 双端不一致 / 渲染细节） |
| 场景 2（fetch 镜像 + 写工作区 + commit） | **有条件触发** | 若 mdor 用"下载原始字节 hash vs 上个 commit blob（已 clean）"做无变化跳过，且源为 CRLF、autocrlf=true → 每次判"变了" → 空提交 / 版本链翻动。autocrlf=false 时磁盘字节≡blob 字节，歧义不成立（检测方式定案见 §4.5.7） |
| 跨机器 / 配置漂移 | 触发（条件 1） | 同仓库在无 git 机器（LF）与 Git for Windows（CRLF）间横跳 → 整库翻转 |

**结论（修正早前表述）**：准确说法不是"git 会误判本地与上游不一致"，而是——mdor **消费工作区原始字节**（渲染 / 正则 / 哈希 / 测试）而非过过滤器的归一化字节，且两平台**配置注定不同**（Windows 可能被宿主 git 安装影响、Android 永远无配置），把 git 设计里隐形的"换行转换"变成持续可见的分歧。`core.autocrlf=false` 的作用是让"磁盘字节 ≡ blob 字节 ≡ 两端字节"，把 gix 当字节透明存储用。

#### 4.5.4 候选方向

| 方向 | 机制 | 施加时点 | 优点 | 缺点 |
|---|---|---|---|---|
| **A 写 repo-local config** | `config_mut()` set，drop 自动 commit | clone/init 后、首次 checkout 前（snapshot.rs） | 持久、不受用户全局配置影响、对 git CLI 等任何工具生效 | 多一步写入；已有仓库需"打开时校正"；ignorecase 物理冲突救不了 |
| **B `config_overrides`**（安全版"全局约定"） | `gix::open::Options::config_overrides` | 每次打开仓库（AppService 统一入口） | 一处定义、零落盘、无需迁移、覆盖优先级高 | 仅本进程 gix 生效；对 create 期 ignorecase 探测无帮助 |
| **C 关 checkout filter** | 显式关 smudge/filter，或 `.git/info/attributes` 写 `* -text` | checkout 时 | 不落盘、确定性 | 需每次显式设置；单独用不稳妥 |

#### 4.5.5 推荐方向（待 M1 实测后敲定）

**A + B 叠加，收敛到两个施加点**：

1. **snapshot.rs 的 clone/init 路径**：成功后、checkout 前执行 `apply_windows_safety_config()`，写 repo-local：`core.autocrlf=false`（必须）、`core.longpaths=true`（防御 + git CLI 互操作）；`core.ignorecase` 交给 gix 探测（Windows 上确认自动为 true）。
2. **AppService 统一仓库打开入口**：`config_overrides` 兜底 `core.autocrlf=false`，保证进程内行为确定。
3. **ignorecase 物理冲突不在配置层解决（定案 2026-08-09）**：fetch/clone 后对 tree 做大小写冲突检测（同目录下仅大小写不同的路径），对象层恒两条目，分两层处理：
   - **同 blob**（两路径同一内容）：归一为一个资源，无冲突；
   - **异 blob**（内容真不同）：两选项——① **双渲染+标注**（默认）：能渲染的都渲染、碰撞节点标注；② **报错**：检测到即拒绝该书（整书级，两平台一致）。
   - **Windows 退化**：NTFS 物理只能落一个文件，双渲染在 Windows 退化为"单渲染+标注"（渲染存在的那个 + 标注提示另一冲突版本存在但不可显示）；Android 两文件都在则真双渲染。跨平台真双渲染绑定可选"blob 直接读"能力（project.md §12.1，互斥、经选项切换，v1 默认不引入）。
   - fixtures 测试涵盖：同 blob 组 / 异 blob 双文件组 / 异 blob Windows checkout 行为组。
4. **longpaths 措辞**：主缓解 = gix / 纯 Rust 自动长路径；§4.3"或规避超深目录"保留为兜底。
5. **变更检测方式（定案见 §4.5.7）**：检测层用原始字节 hash（autocrlf=false 下恒等）；版本比较展示用 gix diff。

#### 4.5.6 待 M1 实测验证

- gix 在 Windows clone 时是否自动写 `core.ignorecase=true`（fs probing）；
- gix 在 Windows 上 checkout 超 260 路径是否无碍（超长 fixture 实测）；
- 模拟 Git for Windows system `core.autocrlf=true` 时，确认压成 false 后 checkout 不再转换。
- gix 在 Windows 上碰撞路径 checkout 的实际行为（告警 / 静默覆盖），确认"单渲染+标注"退化可容忍；
- tree 级大小写冲突检测（平台无关，对象层比对 oid）在 M1 fixtures 验证；
- 同 blob / 异 blob 判定（读两路径 blob oid 是否相等）。

**未决状态**：本节为讨论记录；M1 实测后更新 §4.3 / §4.4 敲定策略。

#### 4.5.7 变更检测方式定案：原始字节 hash（讨论记录 2026-08-09）

**决策**：检测层用**原始字节 hash**（`下载字节 hash vs 上个 commit blob hash`），前提是 §4.5.5 已强制 `core.autocrlf=false`；**展示层**（版本比较界面给人看的文本 diff）用 **gix diff**（树对象级，与过滤器无关）。分工：hash 回答"内容变没变"，gix diff 回答"变了什么"，两者不是竞争关系。

**为什么不用 git/gix status 做检测**——先拆它判定"变没变"的两层机制：

- **工作区遍历**：把 index 里的 tracked 清单与磁盘目录递归枚举对齐，用于发现新增/删除文件。
- **stat + 内容对比**：对每个 index 条目，先 `lstat()` 拿当前 stat（size/mtime/ctime/inode），与 index 里存的 stat 快照比——吻合则快路径判"没变"（**不读文件内容**）；不同则慢路径读内容算 blob hash、与 index 里的 hash 裁决。stat 是优化门卫，内容对比是兜底裁决（`touch` 只改 mtime 的场景靠它纠偏）。另有 racy git 边界：文件刚写入且 mtime 与快照同秒时，git 保守走内容对比。

**mdor 全量重写流恰好击穿 stat 快路径**：若用 status 做检测，必须"先写盘再比"，而每次 fetch 全量重写工作区 → 每个文件 mtime 都是新的 → 与 index stat 快照必不匹配 → **永远落慢路径**：全量读盘 + 逐文件 hash。stat 缓存是为"编辑器改一两个文件"的增量场景设计的，mdor 恰恰是它的反例。

**权衡对照表**：

| 维度 | 原始 hash + autocrlf=false | gix status（容忍任意 autocrlf） |
|---|---|---|
| 无变化时 I/O | 内存比 hash，磁盘零接触 | 先全量写盘 → 再全量读回 hash |
| 新/删文件发现 | 抓取清单天然已知，无需枚举 | 依赖工作区目录遍历 |
| 跨平台字节身份 | 两端 blob 字节一致，可同步/去重/互验 | 源为 CRLF 时 Windows blob=LF、Android blob=CRLF，同步失效 |
| 渲染/正则/测试读盘 | 磁盘≡blob≡两端 | Windows 上是 CRLF，所有消费者需对换行免疫 |
| 配置漂移（Git for Windows 全局配置） | 靠 repo-local 强制压掉 | 检测层免疫，但其它消费者仍暴露字节分叉 |
| 版本 diff 语义 | 字节级（换行变化也算） | 内容级（忽略仅换行变化） |
| 实现复杂度 | 一个 hash + 比较 | gix status API、写盘时序、stat 缓存边缘情况 |

gix status 唯一的独有优势是"检测层对任意配置免疫"，但前提是接受字节分叉——一旦接受，跨平台同步存储即不可用，与 mdor 字节保真原则冲突，故否决。

**假差异精确链条（与 §4.5.3 呼应）**：autocrlf=true 时 commit 的 clean 过滤器把 blob 归一化为 LF，而下载/工作区是原始字节（源为 CRLF 时），mdor 在"未归一化的原始字节"与"已归一化的 blob"上比 hash → 必然不等。**只有源为 CRLF 才触发**（LF 源过 clean 不变，两边相等）。autocrlf=false 使 下载≡blob≡磁盘，原始 hash 退化为恒等比较。这正是"不赌源恰好是 LF、必须强制 false"的原因。

**新增约束（跟随本决策）**：autocrlf=false 下内容保留源站换行风格，但解析层**无需手写归一化**——`pulldown-cmark`（CommonMark：`\n`/`\r\n`/`\r` 均为行结束）与 `scraper`（HTML5：tokenize 阶段将 CRLF 归一为 LF）已按各自规范处理；`str::lines()` 原生兼容 `\r\n`。底线仅是"不假设上游是 LF"：不手写 `split('\n')`、正则跨行用 `\r?\n`。渲染端同理：Chromium 把所有行终止符归一为 LF，两端无需处理。

**跨平台同步存储**：若书店数据在 Windows/Android 间共享（拷贝/云同步/跨端校验），autocrlf=false 使 blob 字节两端一致，下载一次可跨端复用、去重、校验；任何自动换行转换都会破坏这份字节身份。

## 5. 安全 / 明文策略（app 层）

### 5.1 背景知识：HTTP 的"明文"与 App 网络策略

`http://`（非 https）传输的内容是**明文**，可被中间人抓包、篡改。网页里写死 `http://` 链接本身没问题，但 App/WebView 能不能发起明文请求，由系统策略决定：

| 平台 | 明文 http 策略 | 配置位置 | 影响面 |
|---|---|---|---|
| Windows | 无此限制，`http://` 随便用 | 无 | 无 |
| Android | **9+ 默认全局禁 http**，连 `http://127.0.0.1` 也被拦 | APK 内 **network security config**（`cleartextTrafficPermitted`） | 本地 http 服务器分发资源需先放开白名单 |

- Android 上要放开：在 network security config 里声明 `cleartextTrafficPermitted="true"` 并**限定域名/网段**（如仅 127.0.0.1），而不是全局放开。
- 这正是 §2 "Android 用 `http://127.0.0.1:port` 本地服务器分发资源"方案能成立的前提——**需先配置 cleartext 白名单**（`project.md` §10 M6"cleartext 配置"）。

### 5.2 文件访问与权限模型

| 平台 | 自有目录 | 是否需权限 | 目录之外 |
|---|---|---|---|
| Windows | 用户目录 / exe 同目录 | 免权限 | 其他目录依赖账号权限 |
| Android | 应用私有目录（`getFilesDir()`） | 免权限 | 外部存储需运行时权限 |

mdor 只用自有目录 → **两端均无额外权限依赖**。

### 5.3 对 mdor 的落地影响

- M6 打包时配置 network security config（`project.md` §10）。
- 无额外权限申请：Android 用 `getFilesDir()` 私有目录、Windows 用 exe 同目录，都免权限。

## 6. 构建工具链（工程层）

### 6.1 背景知识：从源码到能跑的 App，经历了什么

Rust 源码要变成可执行文件，走：**编译 → 汇编 → 链接**。Rust 用 **target** 描述"编出来的产物给什么平台跑"。

```
源代码（.rs）
    │
    ▼
编译器（rustc，按 target 生成目标平台机器码）
    │
    ▼
汇编 → 链接器（把依赖的库拼进最终二进制）
    │
    ▼
可执行文件（Windows: exe / Android: APK 内的 so）
```

关键概念：**host vs target**。

- **host**：在哪台机器上编译（我们的开发机 = Windows）。
- **target**：编出来给谁跑（Windows 桌面 / Android arm64 手机）。

host ≠ target 时叫**交叉编译**：在 Windows 上编出 Android 能跑的 arm64 二进制。

### 6.2 两端工具链对比

| | Windows（桌面） | Android（目标） |
|---|---|---|
| target | `x86_64-pc-windows-msvc` | `aarch64-linux-android` / `x86_64-linux-android` |
| 编译器/链接器 | **MSVC**（微软 C/C++ 工具链） | **NDK + clang**（Android 官方 C/C++ 工具链 + bionic libc） |
| 编译方式 | 本机编本机跑，直接 | **交叉编译**，需要 NDK |
| 复杂度 | 低 | 高 |

Windows 本机用 MSVC 目标：编译器、链接器都是现成的，`cargo build` 直接出 exe。Android 需要在 Windows 上产出 arm64 机器码 = 交叉编译：`rustc` 自己只能出 Rust 侧代码，涉及 C 的部分（见 §6.3）需要 NDK 的 clang 交叉编译器 + bionic（Android 的 C 标准库）。

### 6.3 C 依赖为什么是 Android 交叉编译的痛

Rust 依赖若含 C 库（如 `openssl-sys`、`curl-sys`），交叉编译时要**把整套 C 代码也用 NDK 编一遍**：

| 情况 | 体验 |
|---|---|
| 编 Windows 本机（host） | 容易：MSVC 现成 |
| 编 Android（交叉编译 C 依赖） | 常见"找不到库 / 版本不匹配 / 构建脚本报错"（呼应 §1.3 OpenSSL 灾难） |

**纯 Rust 依赖没有 C 代码** → 交叉编译只需 Rust 工具链 + NDK clang，平滑很多。

一句话总结：**"依赖纯 Rust"是对 Android 的硬约束，对 Windows 则宽松**。

### 6.4 对 mdor 的落地影响

1. 选型全朝"纯 Rust"靠：serde_json（非 SQLite，§6.7）、gix（非 git2，§7.4）、rustls（非 openssl）——把 C 依赖排除在外。
2. **任何新增 C 依赖都会重新引入 Android 交叉编译差异点**——引入前须评估。
3. `rust-toolchain.toml` 固定 1.97.1（M0 阶段不装 android targets，M6 打包前补回 arm64-v8a / x86_64，见 `env.md` §7）。

## 7. 可靠性策略细节

### 7.1 背景知识：文件写入后，数据真的"落盘"了吗

为了性能，操作系统不会每次写入都立刻写进磁盘——先放进**内存页缓存**，稍后异步才落盘：

```
程序 write → 页缓存（内存，快） →（异步 / fsync）→ 磁盘（慢，持久）
```

- 断电/崩溃时，缓存里没落盘的数据会丢 → 文件丢或坏。
- **fsync = 强制立即落盘**的系统调用：保数据，但每次都要等磁盘物理写入（手机闪存上更明显）。

### 7.2 mdor 的取舍（已决策 2026-08-09：按文件类型分层）

**背景：先厘清取舍对象。** 原子写（§4.2 的 tmp + rename）已保证"要么旧文件要么新文件，无半写态"——文件损坏风险已被 rename 消除。**fsync 只决定一件事：断电/崩溃时，最近一次保存能否活下来**：

```
write → 页缓存（内存，快） →（异步 / fsync）→ 磁盘（慢，持久）
```

- 做 fsync：断电后最后一次保存也保住；代价是每次保存都等闪存物理落盘（几十~几百 ms）。
- 不做 fsync：快；断电丢"最后一次保存"，回退到上次成功落盘的状态——但**无脏文件、无损坏**（rename 保证）。

**关键变量是"写频率 × 状态价值"，不是平台：**

| 文件 | 写频率 | 断电丢了的代价 | 决策 |
|---|---|---|---|
| `progress.json` | **高**（滚动节流、切章都写） | 低——进度回退，重滚即可（与"离线书可重下"同类可接受损失） | **仅 rename**（跳 fsync） |
| `library.json` | **低**（仅 add/remove/update 写） | 较高——书架状态回退（一致性由提交点设计兜底） | **fsync**（一次几 ms，成本可忽略） |
| `.mdor/versions/<sha>.json` | 低（每版本一次），可重写 | 低 | 仅 rename（或 fsync，无感） |

- **依据**：fsync 的成本 ∝ 写频率，收益 ∝ 状态价值。对低频高价值的 `library.json` 做 fsync，一次几 ms、几乎无感，换来书架状态断电保全；对高频低价值的 `progress.json` 做 fsync，每次保存都等磁盘、收益只是"进度多回退几秒"。平台差异（Android 闪存慢）只是**放大器**，真正决定该不该 fsync 的是文件类型——故决策维度取"文件类型"而非"平台"。
- **实现**：`write_json_atomic(path, data, durability)` 增加 `Durability` 参数（`Fsync` / `RenameOnly`），**调用方按文件类型传**——不按 `cfg(target_os)` 分支，两端同一套代码，core 保持平台无关（严格实现时 Linux/Android 上 rename 后还应 fsync 父目录才完整，mdor 场景不必较真，文件级 fsync 足够）。

## 8. 测试策略

### 8.1 背景知识：测试在哪个平台上跑？

core 是**平台无关库**（纯 Rust、不碰 WebView/Android API）→ 它的行为不依赖操作系统差异。在 Windows 上 `cargo test` 跑的就是与 Android 上相同的逻辑，Android 无需单独验证 core。

**为什么 Android 不能直接跑单测**：

- Rust 单测编出的是 **native 二进制**，不能直接在手机/模拟器上直接跑；
- 要走模拟器/真机 + adb 部署，流程重、慢；
- `mdor-app` 层（WebView、JNI）在无设备环境根本跑不起来。

| 层 | 测试方式 | 平台 |
|---|---|---|
| core | `cargo test` + httpmock | Windows 桌面直测 |
| app | 集成/真机验证 | M6 模拟器/真机 |

### 8.2 httpmock 是什么

HTTP 测试桩：起一个假服务器，按预设返回响应。`StaticSiteSource` / `GitHubSource` 的测试用它模拟真实站点——**不依赖真实网络、可重复、可断网跑**（`project.md` §12.2）。

### 8.3 对 mdor 的落地影响

- core 桌面直测（`cargo test -p mdor-core`）；
- app 层的平台问题（WebView 宿主、cleartext、数据目录）靠 M6 真机集成验证。

---

## 汇总

平台差异集中在：

- **mdor-app**（WebView 宿主、数据目录、cleartext、JNI）；
- **core 少数点**（TLS 根证书、fsync 策略）；

需用 `cfg(target_os)` / target 条件依赖隔离；core 保持平台无关是 `project.md` §2 设计原则的约束。

**已决策（2026-08-09）**：

- **资源分发通道**：两端统一本地 `tiny_http` 服务器 + `http://127.0.0.1:PORT` 绝对 URL；自定义 `mdor-book://` 降级为后续用户可选（`resources.rs` 保持"URL→路径"映射可插拔）。
- **阅读页渲染形态**：`dangerous_inner_html` 注入 + 绝对 http URL 重写（不用 iframe、不用 `<base>`）。
- **App 静态资源**：UI 资源走 Dioxus `[asset]` 打包；阅读页样式随书籍资源同通道分发。

---

## 9. WebView 风险验证清单（M6）

> 维持 Dioxus WebView 方案的前提下，把剩余 WebView 相关风险归拢为 M6 真机验证与打包配置项（2026-08-09 决策）。多数已在前文给出方案，此处为实施清单。

### 9.1 宿主与内核

| # | 风险 | 依据 | 验证/落地 |
|---|---|---|---|
| A1 | 真机 API < 30 崩溃 `NoSuchMethodError getCurrentWindowMetrics` | env.md §6 | `Dioxus.toml` 设 `min_sdk_version = 30` |
| A2 | System WebView 版本碎片化 → 渲染差异 | §2.3「内核版本与渲染一致性」 | 内联书籍 CSS 对冲；真机对比不同 WebView 版本的书页排版 |
| A3 | 桌面 WebView2 与 Android System WebView 渲染不一致 | §2.3「内核版本与渲染一致性」 | 同一书页双端截图对比（字体/图片/代码高亮） |

### 9.2 本地资源通道（tiny_http）

| # | 风险 | 依据 | 验证/落地 |
|---|---|---|---|
| B1 | 服务器跑主线程卡 UI | §2.3「本地 http 方案的 Android 限制与对策」 | `tiny_http` 独立线程池，主线程不阻塞 |
| B2 | 端口冲突 | 同上 | `bind("127.0.0.1:0")` 动态端口，先于渲染确定并写入重写 URL |
| B3 | cleartext 明文被 Android 9+ 拦截 | §5.1 | network security config 白名单**仅放行 127.0.0.1**，不全局放开 |
| B4 | INTERNET 权限缺失 bind 失败 | §2.3 | manifest 声明（reqwest 本就需要） |
| B5 | 目录穿越 `../` 读任意文件 | §2.3「重写规则与一一对应」 | URL 规范化 + 书根内白名单校验 |
| B6 | 切版本后同路径资源吃旧缓存 | §2.3「URL 不带版本号」 | 服务器统一 `Cache-Control: no-store` |
| B7 | 阅读页样式分发方式未定 | §2.3「静态资源加载」 | **M3 敲定**：`include_bytes!` 内嵌 vs 首启复制，二选一 |
| B8 | 进程被杀，服务器随进程消失 | §2.3 | 阅读页前台期间依赖存在；不常驻 Service |

### 9.3 线程纪律

| # | 风险 | 依据 | 验证/落地 |
|---|---|---|---|
| C1 | tokio 后台任务直接碰 UI 崩溃 | §2.3「线程模型」 | 所有 UI 更新切回主线程；创建 WebView 必须在主线程 |
| C2 | Android 生命周期（被杀/恢复）与命令中断续做 | project.md §6.9 | `UpdateBookCommand` 携带阶段，重试跳过已完成步骤 |

### 9.4 真机交互

| # | 风险 | 依据 | 验证/落地 |
|---|---|---|---|
| D1 | 触摸滚动与渲染性能 | project.md §10 M6 | 真机长文档滚动帧率/流畅度实测 |
| D2 | Android 返回键 / 手势返回 | project.md §3.3 | 返回键优先级：阅读页 → 目录 → 书架 |
| D3 | 安全区（状态栏/导航栏避让） | project.md §3.3 | WebView 内容避让 + 底部导航适配 |
| D4 | 切版本与渲染协调 | project.md §11 | 先加载章节到内存再 checkout，或切换后 reload |

### 9.5 渲染形态

| # | 风险 | 依据 | 验证/落地 |
|---|---|---|---|
| E1 | `dangerous_inner_html` 不执行 `<script>` | project.md §11 | 由 Dioxus 自绘 TOC/导航替代（预期行为），验证书页无功能缺失 |
| E2 | 本地 http 与 App 页面跨源 | §2.3「本地 http 方案的 Android 限制与对策」 | 仅子资源加载，无 iframe/读写，无需 CORS 头 |
| E3 | DevTools 调试 | §2.3「开发调试体验」 | `chrome://inspect` + adb 远程调试 |

### 9.6 双端对齐

| # | 风险 | 依据 | 验证/落地 |
|---|---|---|---|
| F1 | 桌面与 Android 行为一致性 | §8.3 | core 桌面 `cargo test` 已覆盖；app 层差异 M6 真机回归 |

> 状态划分：**B7 为 M3 实现决策**（未定）；**A1 / B3 / B4 为 M6 打包配置项**；其余为 M6 真机验证项。
