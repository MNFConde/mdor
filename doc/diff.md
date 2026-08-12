# 跨平台依赖差异（Windows 桌面 / Android）

> 本文记录 mdor 在 Windows（桌面开发、测试）与 Android（目标）两端依赖点不一致的地方。
> 依据 [project.md](project.md)，随实现推进持续更新；关键决策记录见 [decisions.md](decisions.md)。

---

## 结论速览

平台差异集中在：

- **mdor-app**（WebView 宿主、数据目录、cleartext、JNI）；
- **core 少数点**（TLS 根证书、fsync 策略）；

需用 `cfg(target_os)` / target 条件依赖隔离；core 保持平台无关是 [project.md §2](project.md#2-设计原则) 设计原则的约束。

**已决策（详见 [decisions.md](decisions.md)）**：

- **资源分发通道**：两端统一本地 `tiny_http` 服务器 + `http://127.0.0.1:PORT` 绝对 URL（[D-04](decisions.md#d-04-本地资源分发)）；自定义 scheme 降级为后续可选
- **渲染形态**：`dangerous_inner_html` 注入 + 绝对 http URL 重写（不用 iframe、不用 `<base>`）（[D-05](decisions.md#d-05-渲染形态)）
- **App 静态资源**：UI 资源走 Dioxus `[asset]` 打包；阅读页样式随书籍资源同通道分发（[D-06](decisions.md#d-06-静态资源分流)）
- **其余**：gix 基座（[D-01](decisions.md#d-01-gix-存储基座)）、JSON 非 SQLite（[D-02](decisions.md#d-02-json-元数据而非-sqlite)）、fsync 分层（[D-03](decisions.md#d-03-原子写与-fsync-分层)）、TLS 选型（[D-11](decisions.md#d-11-tls-与加密选型)）、变更检测（[D-08](decisions.md#d-08-变更检测)）

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
- **Android 上是灾难**：Android 系统无 OpenSSL 库，需用 NDK 把 OpenSSL C 代码交叉编译成 arm64 目标（`openssl-sys` 找不到库 / 版本不匹配 / 构建脚本报错，著名坑）。这是 [project.md §1.2](project.md#12-技术栈)"Android 无 OpenSSL 依赖"的原因。

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
| 根证书 | `rustls-platform-verifier`（Windows 退回 SChannel，认系统/企业 CA） | `rustls-platform-verifier`（JNI 调系统 `X509TrustManager`，认用户/系统 CA） |
| gix HTTP 传输 | reqwest 后端 | reqwest 后端（避免 curl C 依赖） |
| 加密 provider | **`ring`**（轻量，免 cmake/perl） | 同左；`ring` 需 NDK clang 参与构建（NDK 自带，正常）；如需换 `aws-lc-rs` 见 §1.9 |

**统一方案（已决策，[D-11](decisions.md#d-11-tls-与加密选型)）**：两平台都用 `reqwest`（`default-features = false` + `rustls-tls`）+ `rustls-platform-verifier`，gix 开 `http-client-reqwest`。加密 provider 用 **`ring`**（轻量、构建简单、APK 体积小）；将来如需 FIPS / 更广算法面，按 §1.9 一行切换 `aws-lc-rs`。一份 Cargo 配置、一套信任逻辑，Windows/Android 行为一致；仅当"想要极致简单、放弃企业 CA"时才退回 webpki-roots。

### 1.7 选错的后果

- 默认 reqwest（native-tls）build Android → 交叉编译 OpenSSL 失败或出诡异构建错误
- rustls 却不喂根证书 → 能编能跑，但**所有 HTTPS 请求报 `certificate verify failed`**，下载功能全废
- gix 用 curl 后端 → Android 交叉编译需单独处理 `curl-sys`，且与上面 TLS 策略各管各的
- 只打包 webpki-roots 的 Windows 端 → 公司内网文档站 / 抓包代理环境访问失败，难排查

### 1.8 rustls 稳定性评估（选型依据）

**结论：足够稳定，可放心用于生产（含 Android 客户端场景）。** 2016 年诞生、活跃维护（0.23 长期系列 + backport 修复，等效 LTS 稳定线）；有过独立安全审计 + OpenSSF 徽章；Prossimo（ISRG）主导；Let's Encrypt 计划用 rustls 替换 OpenSSL。mdor 为**客户端**、只访问知名站点，属 rustls 最成熟的使用面，风险很低；配合 [project.md §12.1](project.md#121-关键设计决策) 的 `cargo audit` 持续跟踪即可。完整评估（成熟度/审计/资金/生产采用/性能 + 0.x 版本策略 + "纯 Rust 需打折" + 版本管理/backport 说明）见 [D-11 TLS 与加密选型](decisions.md#d-11-tls-与加密选型)。

### 1.9 加密 provider 可插拔性（选 ring，随时可换）

加密 provider 定为 **`ring`**（轻量、免 cmake/perl、APK 体积小），经 rustls `CryptoProvider` 抽象可插拔——切换 = Cargo feature 一行 + 重新打包，**不做运行时双 provider 注入**（体积翻倍）。`main()` / `AndroidMain` **最早处** `install_default()`（reqwest 0.13 起必须，否则 `Client::new()` 直接 panic）。两处 TLS 只经 reqwest 一层（§1.5），换 provider 一处切换、两处全生效。机制与注意点见 [D-11 TLS 与加密选型](decisions.md#d-11-tls-与加密选型)。

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

#### 自定义协议 `mdor-book://`（[project.md §11](project.md#11-风险与待定项) 风险项，差异核心）

背景：阅读器加载本地章节 HTML/图片，网页内 `<img src="...">` 默认走 `http://`。自定义协议 = 当 WebView 请求 `mdor-book://xxx` 时，引擎回调 Rust 代码从本地磁盘返回字节——不经过网络、不受文件访问限制。

| | Windows (WebView2) | Android (System WebView) |
|---|---|---|
| 自定义 scheme | `AddWebResourceRequestedFilter` 注册，可用但 API 异步、有怪癖 | **支持很差**：导航可用 `shouldOverrideUrlLoading` 拦，但资源加载（img/css/script）走 `shouldInterceptRequest`，自定义 scheme 历来不可靠 |
| 替代方案 | 勉强可用 | **建议不用**，改 `http://127.0.0.1:port/...` |

→ 这就是 `tiny_http` 备选的原因：Android 上起本地 HTTP 服务，用 `http://127.0.0.1:PORT` 加载资源。两平台原生支持 http，行为统一，绕开 scheme 兼容问题；代价是本地端口管理、需绑 127.0.0.1 防外访问。

**决策（2026-08-09）**：两端统一走本地 HTTP 服务器分发，自定义 scheme 降级为后续可选功能——背景/结论/依据见 [D-04 本地资源分发](decisions.md#d-04-本地资源分发)。

#### 为什么是 http 而非 file://（正则重写为何不选）

**常见疑问**：注入 HTML 是否因为"WebView 访问不了本地文件"才被迫走 http？如果文件可以访问，能否用正则按规则把 URL 转成本地文件地址？

**答案：不是单一原因，是两层叠加**：① 注入 HTML 的相对 URL 先按宿主文档 base 解析（release 为 `dioxus://`、桌面 dev 为 dev server 地址），无人回答 → 必须重写为绝对地址；② Chromium 子资源安全模型封死非 file 源文档加载 `file://` 子资源，两端各再加一道墙。完整论证见 [D-04 本地资源分发](decisions.md#d-04-本地资源分发)。

#### 版本管理与重写时机

**重写发生在渲染时，不是更新/commit 时。** 存储层（git 对象库/工作区）永远是**上游原样内容**，URL 一个字符都不动：

```
更新时（存）:  fetch 内容 → 原样 commit → 打版本 tag      ← 内容零改动
渲染时（读）:  读章节 → 抽取 → 按规则重写 URL → 注入      ← 内存中临时改
```

三条理由，缺一不可：

1. **端口是动态的，更新时根本不知道**：服务器用 `bind("127.0.0.1:0")` 动态端口，只有运行时才确定。URL 里要带端口，就决定了重写**不可能**发生在无端口概念的更新/commit 阶段。
2. **保上游原样**：场景 1 克隆的上游仓库必须保持原状（不能污染克隆历史）；场景 2 自建链的意义就是"忠实镜像"。若更新时就重写并 commit，版本间 diff 会被重写产物污染，版本比较/回滚全部失真。
3. **切版本零成本**：重写规则与版本无关，只是把"当前工作区文件路径"转成 URL。切版本 = 换工作区内容 = 重新渲染一遍即可，无需任何存储级操作（与 [project.md §11](project.md#11-风险与待定项)"切版本与 webview 协调"风险项正交，那边由"先加载到内存再切换 / 切换后 reload"处理）。

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

**URL 不带版本号（2026-08-09 决策）**：URL 形态为 `http://127.0.0.1:PORT/books/<id>/<path>`，不含 `<version>`——URL 里的 version 是"声称值"不是"保证值"（单工作区 checkout 设计下有竞态风险）、调试场景不依赖它、缓存问题用 `Cache-Control: no-store` 头根治、将来按 blob 直接服务（[D-10](decisions.md#d-10-资源读取通道)）时可加回。完整论证见 [D-04 本地资源分发](decisions.md#d-04-本地资源分发)。

#### 本地 http 方案的 Android 限制与对策

Android 上跑本地 http 服务器不是没有限制，逐项列清并给对策（多数为配置/工程问题，非死路）：

| 限制 | 说明 | 对策 |
|---|---|---|
| cleartext 禁止 | Android 9+ 默认禁明文 http，连 `http://127.0.0.1` 也被拦 | network security config 白名单**仅放行 127.0.0.1**（[§5.1](#51-背景知识http-的明文与-app-网络策略)） |
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

→ **决策（2026-08-09）**：App 静态资源分两类处理——App UI 资源走 Dioxus `[asset]` 打包；**阅读页样式与书籍资源同通道**统一经本地 http 分发。背景/结论/依据见 [D-06 静态资源分流](decisions.md#d-06-静态资源分流)。

**决策（2026-08-13）**：阅读页样式 v1 定为 **`include_bytes!` 内嵌**——样式随二进制编译进包、服务器从内存直接吐，两端零分叉、无启动流程（依据与取舍见 [D-06 静态资源分流](decisions.md#d-06-静态资源分流)）。**将来若需主题热更新**（样式可下载/替换），再实现方案 2——首启/更新时把样式落盘到 `getFilesDir()`（Windows 为普通文件）替代内嵌；该扩展要求"样式资源提供者"收敛在 **app 层兼容层**（Android 读 APK `assets/` 需 JNI AssetManager、路径注入自运行时的 `getFilesDir()`；Windows 读普通文件），向 core 暴露统一接口，**不放平台差异进 core**。**兼容层需抹平的具体平台差异清单后续整理**（见 [project.md §11](project.md#11-风险与待定项)）。

#### 内核版本与渲染一致性

两个都是 Chromium 但**版本不同**（WebView2 跟 Edge，Android 跟系统）。CSS/JS 有细微差异 → [project.md §6.5](project.md#65-renderservice)"内联书籍 CSS 保证样式一致"即为此对冲。

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
2. **资源协议层（已决策 2026-08-09）**：`render/resources.rs` 维护"本地 URL → 文件路径"映射（core 复用、跨平台一致）；**接入点在 app 侧统一为本地 `tiny_http` 服务器**，Windows 不再用 WebView2 scheme handler。URL 形态两端一致：`http://127.0.0.1:PORT/books/<id>/<path>`（**不带版本号**，理由见 [D-04](decisions.md#d-04-本地资源分发)；缓存由服务器 `Cache-Control: no-store` 头根治）。
3. **main 双入口**：桌面 `main()` + Android `AndroidMain`，启动时解析数据目录（对应 §3）。
4. **线程纪律**：所有 UI 更新最终发生在主线程；tokio 后台任务回 UI 需显式切换。
5. **渲染形态（已决策）**：章节 HTML 用 `dangerous_inner_html` 注入，资源链接在 `html_extract.rs` 统一重写为**绝对 http URL**（不用 `<base>` 标签、不用 iframe）。依据与取舍见 [D-05 渲染形态](decisions.md#d-05-渲染形态)。

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
- core 的 `BookStore::new(base_dir)` 平台无关，只认 `bookstore/` 这一层；数据根解析在 `mdor-app` 启动时 cfg 分支完成（对应 [project.md §9](project.md#9-存储布局) / [§12.1](project.md#121-关键设计决策) / [D-13](decisions.md#d-13-数据目录注入)）。

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

→ **两端一致**，core 的 `write_json_atomic`（[project.md §6.7](project.md#67-元数据写入可靠性json不用-sqlite)）可以放心用。注意：rename 保证"无脏文件"，但不保证"断电后新文件已在磁盘上"——那部分由 fsync 兜底，见 §7。

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
2. gix 仓库在 Windows 上的坑：长路径、大小写、autocrlf——主要在 checkout/clone 时触发，需在 gix 配置侧规避（机制梳理与候选方向见 [§4.5](#45-gix-三坑的配置规避机制梳理与待定讨论记录-2026-08-09) / [D-09](decisions.md#d-09-gix-三坑配置规避)，策略待 M1 实测后敲定）。
3. fixtures / 测试样例路径用 `Path` 抽象，避免硬编码分隔符。

### 4.5 gix 三坑的配置规避：机制梳理与待定（讨论记录 2026-08-09）

**现状**：§4.3 三个 Windows 特有坑（长路径 / 大小写 / autocrlf）的规避列分别写"gix 侧注意 / gix 侧关闭·固定换行"，§4.4 落地影响 2 说"需在 gix 配置侧规避"，但**具体策略未定**（施加层级、机制、时点均未明确），因为 gix 是库而非 CLI、且三坑性质不同不能一刀切。

**决策摘要**（完整机制梳理与讨论见 [D-09 gix 三坑配置规避](decisions.md#d-09-gix-三坑配置规避)；变更检测定案见 [D-08 变更检测](decisions.md#d-08-变更检测)）：

- **配置机制**：git 配置分层 `system < global < local < worktree < env`；gix 提供 `Repository::config_mut()`（落盘 local）、`gix::open::Options::config_overrides`（纯内存）、`gix::config::tree`（类型化 key）三个程序化入口；checkout 行为由 checkout options 驱动（gix-filter 做 CRLF 转换）。**关键风险——全局约定是毒药**：gix 会读到 Git for Windows 的 system 级 `core.autocrlf=true`（实锤 helix #6467），必须由 mdor 在更高优先级压掉。
- **三坑性质**：autocrlf 必须显式压 false（有明确手段）；ignorecase 只能让索引比较按大小写不敏感，**救不了**物理冲突；longpaths 大概率不需要（gix 走 Rust `std::fs` 自动长路径）。
- **推荐方向（待 M1 实测）**：A + B 叠加——snapshot.rs clone/init 后写 repo-local（`core.autocrlf=false` 必须、`core.longpaths=true` 防御）+ AppService 统一打开入口 `config_overrides` 兜底。
- **大小写冲突定案**：tree 级检测，同 blob 归一 / 异 blob 双渲染+标注（默认）/ 报错；Windows 退化为"单渲染+标注"；跨平台真双渲染绑定 blob 直接读能力（[D-10](decisions.md#d-10-资源读取通道)，v1 不引入）。
- **autocrlf 深究**：git 平时不"误判"靠对称过滤器（smudge/clean 都归一化到 LF）；mdor 消费**工作区原始字节** + 两平台配置注定不同 → 把隐形的换行转换变成持续可见分歧；`core.autocrlf=false` 使 磁盘≡blob≡两端。
- **变更检测定案**：检测层用原始字节 hash（前提 autocrlf=false），展示层用 gix diff；不用 gix status（stat 快路径被全量重写流击穿）。
- **待 M1 实测项**：见 [D-09](decisions.md#d-09-gix-三坑配置规避)。

**未决状态**：本节为讨论记录；M1 实测后更新 [§4.3](#43-windows-特有坑android-没有) / [§4.4](#44-对-mdor-的落地影响) 敲定策略。

## 5. 安全 / 明文策略（app 层）

### 5.1 背景知识：HTTP 的"明文"与 App 网络策略

`http://`（非 https）传输的内容是**明文**，可被中间人抓包、篡改。网页里写死 `http://` 链接本身没问题，但 App/WebView 能不能发起明文请求，由系统策略决定：

| 平台 | 明文 http 策略 | 配置位置 | 影响面 |
|---|---|---|---|
| Windows | 无此限制，`http://` 随便用 | 无 | 无 |
| Android | **9+ 默认全局禁 http**，连 `http://127.0.0.1` 也被拦 | APK 内 **network security config**（`cleartextTrafficPermitted`） | 本地 http 服务器分发资源需先放开白名单 |

- Android 上要放开：在 network security config 里声明 `cleartextTrafficPermitted="true"` 并**限定域名/网段**（如仅 127.0.0.1），而不是全局放开。
- 这正是 §2 "Android 用 `http://127.0.0.1:port` 本地服务器分发资源"方案能成立的前提——**需先配置 cleartext 白名单**（[project.md §10](project.md#10-里程碑) M6"cleartext 配置"）。

### 5.2 文件访问与权限模型

| 平台 | 自有目录 | 是否需权限 | 目录之外 |
|---|---|---|---|
| Windows | 用户目录 / exe 同目录 | 免权限 | 其他目录依赖账号权限 |
| Android | 应用私有目录（`getFilesDir()`） | 免权限 | 外部存储需运行时权限 |

mdor 只用自有目录 → **两端均无额外权限依赖**。

### 5.3 对 mdor 的落地影响

- M6 打包时配置 network security config（[project.md §10](project.md#10-里程碑)）。
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

1. 选型全朝"纯 Rust"靠：serde_json（非 SQLite，[project.md §6.7](project.md#67-元数据写入可靠性json不用-sqlite)）、gix（非 git2，[project.md §7.4](project.md#74-存储基座为何-v1-起就用-gix)）、rustls（非 openssl）——把 C 依赖排除在外。
2. **任何新增 C 依赖都会重新引入 Android 交叉编译差异点**——引入前须评估。
3. `rust-toolchain.toml` 固定 1.97.1（M0 阶段不装 android targets，M6 打包前补回 arm64-v8a / x86_64，见 [env.md §7](env.md#7-m0-到-m6-过渡清单补回-android-侧)）。

## 7. 可靠性策略细节

### 7.1 背景知识：文件写入后，数据真的"落盘"了吗

为了性能，操作系统不会每次写入都立刻写进磁盘——先放进**内存页缓存**，稍后异步才落盘：

```
程序 write → 页缓存（内存，快） →（异步 / fsync）→ 磁盘（慢，持久）
```

- 断电/崩溃时，缓存里没落盘的数据会丢 → 文件丢或坏。
- **fsync = 强制立即落盘**的系统调用：保数据，但每次都要等磁盘物理写入（手机闪存上更明显）。

### 7.2 mdor 的取舍（已决策 2026-08-09：按文件类型分层）

**决策**：按**文件类型**分层决定是否 fsync，不按平台（平台差异只是放大器，决策维度取"文件类型"）。完整论证（原子写已消除半写态、fsync 的成本 ∝ 写频率、收益 ∝ 状态价值）见 [D-03 原子写与 fsync 分层](decisions.md#d-03-原子写与-fsync-分层)。

| 文件 | 写频率 | 断电丢了的代价 | 决策 |
|---|---|---|---|
| `progress.json` | **高**（滚动节流、切章都写） | 低——进度回退，重滚即可（与"离线书可重下"同类可接受损失） | **仅 rename**（跳 fsync） |
| `library.json` | **低**（仅 add/remove/update 写） | 较高——书架状态回退（一致性由提交点设计兜底） | **fsync**（一次几 ms，成本可忽略） |
| `.mdor/versions/<sha>.json` | 低（每版本一次），可重写 | 低 | 仅 rename（或 fsync，无感） |

**实现**：`write_json_atomic(path, data, durability)` 增加 `Durability` 参数（`Fsync` / `RenameOnly`），**调用方按文件类型传**——不按 `cfg(target_os)` 分支，两端同一套代码，core 保持平台无关（严格实现时 Linux/Android 上 rename 后还应 fsync 父目录才完整，mdor 场景不必较真，文件级 fsync 足够）。

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

HTTP 测试桩：起一个假服务器，按预设返回响应。`StaticSiteSource` / `GitHubSource` 的测试用它模拟真实站点——**不依赖真实网络、可重复、可断网跑**（[project.md §10](project.md#10-里程碑) M2/M4 行）。

### 8.3 对 mdor 的落地影响

- core 桌面直测（`cargo test -p mdor-core`）；
- app 层的平台问题（WebView 宿主、cleartext、数据目录）靠 M6 真机集成验证（实施清单见 [project.md §10.1](project.md#101-m6-真机验证清单)）。
