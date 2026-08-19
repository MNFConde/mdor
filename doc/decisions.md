# mdor 决策记录（ADR）

> 记录影响架构的关键选型与讨论：**只回答"为什么这么选"**——系统"是什么"见 [project.md](project.md)，平台差异见 [diff.md](diff.md)。
> 每条独立编号 `D-xx`，必填：状态 / 日期 / 规范位置（反向链接）/ 背景 / 决策 / 依据 / 影响。
> 新增决策登记规则见 [README.md](README.md#decisionsmd-登记规则)。

---

## 决策总览

| 编号 | 决策 | 状态 | 规范位置 |
|---|---|---|---|
| [D-01](#d-01-gix-存储基座) | 存储基座 = gix（每书一个 git 仓库 + 私有 tag 统一版本） | 已决策 | [project.md §7.4](project.md#74-存储基座为何-v1-起就用-gix) |
| [D-02](#d-02-json-元数据而非-sqlite) | 元数据 = JSON 文件而非 SQLite（含 serde_json 安全对照） | 已决策 | [project.md §6.7](project.md#67-元数据写入可靠性json不用-sqlite) |
| [D-03](#d-03-原子写与-fsync-分层) | 原子写 + fsync 按文件类型分层（Fsync / RenameOnly） | 已决策 | [diff.md §7.2](diff.md#72-mdor-的取舍已决策-2026-08-09按文件类型分层) |
| [D-04](#d-04-本地资源分发) | 本地资源分发 = tiny_http + `http://127.0.0.1:PORT`（URL 不带版本号） | 已决策 | [diff.md §2.3](diff.md#23-逐维度对比)、[project.md §6.5](project.md#65-renderservice) |
| [D-05](#d-05-渲染形态) | 渲染形态 = `dangerous_inner_html` 注入（不用 iframe / `<base>`） | 已决策 | [diff.md §2.4](diff.md#24-对-mdor-的落地影响) |
| [D-06](#d-06-静态资源分流) | App 静态资源分流；阅读页样式内嵌定案（方案 1）；主题热更新走方案 2+兼容层（后续） | 已决策 | [diff.md §2.3](diff.md#23-逐维度对比) |
| [D-07](#d-07-薄门面与命令化) | 薄门面 `AppService` + 按需命令化，不引入全局中介者 | 已决策 | [project.md §6.9](project.md#69-服务编排薄门面-按需命令化) |
| [D-08](#d-08-变更检测) | 变更检测 = 原始字节 hash（autocrlf=false 前提）+ gix diff 展示 | 已决策 | [diff.md §4.5](diff.md#45-gix-三坑的配置规避机制梳理与待定讨论记录-2026-08-09) |
| [D-09](#d-09-gix-三坑配置规避) | gix 三坑配置规避（repo-local + config_overrides + 大小写冲突处理） | 待 M1 实测 | [diff.md §4.5](diff.md#45-gix-三坑的配置规避机制梳理与待定讨论记录-2026-08-09) |
| [D-10](#d-10-资源读取通道) | 资源读取通道可插拔：工作区直读默认，blob 直接读可选（互斥） | 已决策 | [project.md §12.1](project.md#121-关键设计决策) |
| [D-11](#d-11-tls-与加密选型) | TLS：rustls + platform-verifier + ring，gix 用 reqwest 后端 | 已决策 | [diff.md §1.6](diff.md#16-端到端对比与推荐方案) |
| [D-12](#d-12-依赖与安全审计) | tokio + 依赖版本统一 + `cargo audit`（不引入 cargo deny） | 已决策 | [project.md §12.1](project.md#121-关键设计决策) |
| [D-13](#d-13-数据目录注入) | 数据目录注入；Android `getFilesDir()` / Windows exe 同目录（便携式） | 已决策 | [project.md §9](project.md#9-存储布局) |

---

## D-01 gix 存储基座

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 已决策 | 2026-08-09 | [project.md §7.4](project.md#74-存储基座为何-v1-起就用-gix)、[§12.1](project.md#121-关键设计决策) |

**背景**：书籍需要版本历史、多版本阅读、数据同步（均为后续里程碑功能），但存储基座一旦选定就应指向长期正确的那个——版本 tag 随每次抓取/更新**自然积累**，将来只开放功能，不回头改造存储层。

**决策**：v1 起即以 gix（纯 Rust 的 git 实现）作为存储基座，每书一个 git 仓库，私有 tag `refs/mdor/versions/<seq>` 记录版本，`HEAD` = 当前指针。场景 1（GitHub）clone/fetch 上游保留历史、只加 tag 不 commit；场景 2（静态站）每次抓取自建一个 commit 并打 tag；内容树 hash 未变化时跳过空提交。

**依据**：

- commit 承载内容快照、commit 图承载历史关系、tag 承载"版本"语义、ref（HEAD）= 当前指针、对象库内容寻址 = 去重与校验——版本管理需要的每一件事都被 git 封装好；自己用"目录 + index.json"实现等于重写一个简化且不完整的 git
- 历史版本读取 = 按需 checkout 目标 commit 到单一工作区，无空间累加
- 未来数据同步直接复用 git 协议（fetch/push、partial clone 按需懒加载 blob）

### 被否定的替代方案
> [!CAUTION] 【已否决】 被否定的替代方案
> 原因：详见下列各项
> 
> - **全量目录快照 + COW 硬链接 + index.json 版本链**：需手写版本关系/去重/原子性，无同步能力
> - **git2/libgit2**：C 依赖，Android 交叉编译麻烦
> - **blob 直接读**（作为历史读取主路径）：不必要，单一工作区 checkout 已满足

**影响 / 诚实成本**：gix 依赖树大 → 按需裁剪 feature、每书独立小仓库控制对象库规模；写路径由"复制目录"变"一次 commit"（场景 1 只 fetch+tag）；GC/清理初期"删版本"= 仅删 tag，shallow 截断 + gc 延后为设置项（详见 [§7.4](project.md#74-存储基座为何-v1-起就用-gix) 成本表）。

## D-02 JSON 元数据而非 SQLite

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 已决策 | 2026-08-09 | [project.md §6.7](project.md#67-元数据写入可靠性json不用-sqlite) |

**背景**：几十本书量级的元数据总量 < 100KB，访问形态为按 `book_id` 的简单读写、单进程单写者（串行化即可），无关系查询需求。SQLite 引入 C 依赖 / Android 交叉编译复杂度 / schema 迁移。

**决策**：`library.json` + `progress.json` + `.mdor/versions/<sha>.json`，JSON 文本存储；可靠性由原子写（[D-03](#d-03-原子写与-fsync-分层)）+ 提交点（`library.json` 最后写）+ 读入 guard 保证。

**依据（serde_json 安全选型）**：选型时逐项对照了 Java/C 生态近期公开漏洞，结论均**结构性不适用**：

| 漏洞 | 属于 | 漏洞本质 | 对 `serde_json` 的适用性 |
|---|---|---|---|
| CVE-2026-18401（Jackson-core 异步解析器数字长度绕过） | Java | 非阻塞/异步解析路径漏掉 `maxNumberLength` 校验 → 无限分配 + O(n²) 大数解析 → DoS | 无此类"异步流式解析"API；本项目只用 `from_str` 整块解析自有小文件，不适用 |
| CVE-2026-29062（Jackson-core 嵌套深度限制绕过） | Java | `DataInput`/`Reader` 路径漏掉 `maxNestingDepth` 校验 → 栈溢出 DoS | **已内置防护**：`from_str` 路径默认 128 层递归限制（serde_json PR #163），无已知绕过通告；仅有的 serde#3023 边缘情况（`IgnoredAny` 处理程序构造的 10 万层 `Value`）在本项目"解析自有 <100KB 元数据"的流程中不可达 |
| CVE-2026-9563（Eclipse Parsson 无文档大小上限） | Java | 无默认 max 文档大小 → 超大文档耗尽 CPU/内存 | serde_json 默认同样无大小上限（各主流解析器共性），其危害前提是"解析攻击者可控的网络 JSON"——本项目解析的是**应用自生成的本地元数据**；另以 `read_json_capped()` 1MB 读入 guard 补齐纵深防御 |
| QVD-2026-45876（Fastjson2 反序列化 RCE） | Java | `@type` AutoType 哈希碰撞绕过白名单 + `jar:` URL 类加载 → RCE | **结构性不可能**：Rust/serde 无多态反序列化——JSON 无 `@type` 机制、不从 JSON 加载类、无反身构造副作用，`Deserialize` 全为编译期实现 |
| USN-7973-1（cJSON 多个内存安全漏洞） | C | OOB 读/写、大数 DoS | 内存安全由 Rust 语言保证（`serde_json` unsafe 极少、无 OOB）；大数解析进 `f64`/`i64`/`u64`，无放大性分配 |

**影响**：core 平台无关；仅解析应用自生成的本地元数据，不解析攻击者可控网络 JSON；依赖层面的"无已知未修复漏洞"由 `cargo audit`（[D-12](#d-12-依赖与安全审计)）持续验证。

## D-03 原子写与 fsync 分层

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 已决策 | 2026-08-09 | [diff.md §7.2](diff.md#72-mdor-的取舍已决策-2026-08-09按文件类型分层)、[project.md §6.7](project.md#67-元数据写入可靠性json不用-sqlite) |

**背景**：原子写（tmp + rename）已保证"要么旧文件要么新文件，无半写态"——文件损坏风险已被 rename 消除。**fsync 只决定一件事：断电/崩溃时，最近一次保存能否活下来**。

**决策**：`write_json_atomic(path, data, durability)`，`durability` = `Fsync` / `RenameOnly`，调用方按文件类型传，不按平台分支：

- `library.json`（低频高价值）→ **Fsync**
- `progress.json`（高频低价值）→ **RenameOnly**
- `.mdor/versions/<sha>.json`（低频、可重写）→ RenameOnly

**依据**：fsync 的成本 ∝ 写频率、收益 ∝ 状态价值。对低频高价值的 `library.json` 做 fsync，一次几 ms、几乎无感，换来书架状态断电保全；对高频低价值的 `progress.json` 做 fsync，每次保存都等磁盘、收益只是"进度多回退几秒"。平台差异（Android 闪存慢）只是**放大器**，真正决定该不该 fsync 的是文件类型——故决策维度取"文件类型"而非"平台"。

**影响**：两端同一套代码，core 平台无关，不出现 `cfg(target_os)` 分支。

### 被否定的替代方案
> [!CAUTION] 【已否决】 覆盖前保留 `.bak` 备份
> 原因：原子写（`*.tmp` + `rename`）已消除半写态，文档化故障场景（写一半被杀/断电）下不会读到坏文件；断电持久性由 fsync 分层覆盖（本 ADR）；每次保存多一次全量副本，与 `progress.json` 高频 RenameOnly 取舍冲突
> 
> 仅"绕过原子写的代码路径"才需要它，属代码纪律问题而非可靠性机制。曾列于 `project.md` §6.7「启动读到坏文件」行，2026-08-16 删除并记此否决。

## D-04 本地资源分发

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 已决策 | 2026-08-09 | [diff.md §2.3](diff.md#23-逐维度对比)、[project.md §6.5](project.md#65-renderservice) |

**背景**：阅读页是注入 HTML，其子资源（图片/CSS/JS）原本指向远端站点，离线时必须从本地磁盘返回字节；自定义 scheme（`mdor-book://`）两端不对称——WebView2 的 `AddWebResourceRequestedFilter` 可用但 API 异步、有怪癖，Android `shouldInterceptRequest` 对自定义 scheme 历来不可靠。直接 `file://` 不可行：Android WebView 沙箱禁读任意文件路径，且书籍数据运行期动态增长、无法打包进 APK `assets/`。

**决策（2026-08-09）**：

### 统一本地 HTTP 服务器分发
> [!IMPORTANT] 【当前】 统一本地 HTTP 服务器分发
> 
> 首版 Windows 与 Android **统一用本地 `tiny_http` 服务器**，以 `http://127.0.0.1:PORT` 绝对 URL 分发阅读页子资源；URL 形态 `http://127.0.0.1:PORT/books/<id>/<path>`，**不带版本号**；缓存由服务器统一回 `Cache-Control: no-store` 根治。

### mdor-book 自定义 scheme
> [!NOTE] 【备选】 mdor-book:// 自定义 scheme
> 触发：用户启用"资源通道"设置项
> 
> 自定义 scheme 降级为后续可选。两端不对称：WebView2 的 `AddWebResourceRequestedFilter` 可用但 API 异步、有怪癖；Android `shouldInterceptRequest` 对自定义 scheme 历来不可靠。

**为什么是 http 而非 file://（两层叠加，不是单一原因）**：

1. **注入 HTML 的相对 URL 先按宿主文档 base 解析**：阅读页注入到 App 文档（release 为 `dioxus://`、桌面 dev 为 dev server 地址）里，相对 URL（如 `chapter.png`）解析后指向宿主文档源，无人回答 → 无论走什么方案都必须先重写为绝对地址（与"能否访问文件"无关）。
2. **Chromium 子资源安全模型封死 `file://`**：非 file 源的文档（正是 mdor 的宿主文档）加载 `file://` 子资源会被系统性拦截（"Not allowed to load local resource"）。两端各加一道墙：Windows WebView2 能读磁盘，但宿主文档是 `dioxus://` 非 file 源 → 仍被拦；Android 沙箱禁读任意路径 + 运行时数据目录不在打包 assets 内 → 从根上不可行。
3. **`http://127.0.0.1:PORT` 是两端唯一无条件放行的"合法网络访问"子资源通道**：回环 http 被 Chromium 视为正常网络访问，不依赖文档源、不依赖打包时机；`file://` 的 URL 编码（Windows 盘符/反斜杠、Android 运行时 `/data/...`）在两端各有各的坑，反而导致重写逻辑分平台。

**重写时机（渲染时，不是存储时）**：存储层（git 对象库/工作区）永远是**上游原样内容**，URL 一个字符都不动；重写在渲染时内存中按规则进行。理由：

1. **端口是动态的**：服务器用 `bind("127.0.0.1:0")` 动态端口，只有运行时才确定 → 重写不可能发生在无端口概念的更新/commit 阶段。
2. **保上游原样**：场景 1 克隆的上游仓库必须保持原状；若更新时就重写并 commit，版本间 diff 会被重写产物污染，版本比较/回滚全部失真。
3. **切版本零成本**：重写规则与版本无关，只是把"当前工作区文件路径"转成 URL；切版本 = 换工作区内容 = 重新渲染一遍即可。

**URL 不带版本号（2026-08-09 决策）**：URL 形态不含 `<version>`。理由：

- URL 里的 version 是"声称值"不是"保证值"——单工作区 checkout 设计下，一旦有竞态，URL 声称的版本与实际内容不符，排查时反而被误导；
- 调试场景不依赖它（DevTools/日志排查时版本号由 App 状态与日志承载）；
- 缓存问题用响应头根治（统一 `Cache-Control: no-store`），比 URL 带 version 更可靠；
- 若将来改为"按版本从 git blob 直接服务"（[D-10](#d-10-资源读取通道)），version 成为寻址必需，届时加回 `<version>` 段是纯增量，现在不预支。

**影响 / 代价**：端口管理、绑 127.0.0.1 防外访问、cleartext 白名单（[diff.md §5.1](diff.md#51-背景知识http-的明文与-app-网络策略)）；服务器进程归 `mdor-app`（起停 + 端口注入 `AppService`），`render/resources.rs` 保持"URL→文件路径"映射抽象（core 纯函数、无 socket）、重写层可插拔，将来切换无需改 core；重写与服务两个方向必须**共享同一套规范化逻辑**（`resources.rs` 为唯一事实来源），防 `../` 穿越与编码错位。

## D-05 渲染形态

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 已决策 | 2026-08-09 | [diff.md §2.4](diff.md#24-对-mdor-的落地影响)、[project.md §6.5](project.md#65-renderservice) |

**背景**：章节 HTML 注入 WebView 的方式选择。[【已否决】 iframe 方案](#iframe-注入方案) 虽 HTML 保真度高（脚本/锚点/样式隔离），但 App 页面（`dioxus://`）与 `http://127.0.0.1` **跨源**，Dioxus 读不到 iframe 内部滚动 → 破坏统一滚动进度跟踪，需 postMessage 桥或全页同源化，复杂度不划算。

### iframe 注入方案
> [!CAUTION] 【已否决】 iframe 注入方案
> 原因：跨源破坏统一滚动进度跟踪，需 postMessage 桥或全页同源化，复杂度不划算
> 
> iframe 虽 HTML 保真度高（脚本/锚点/样式隔离），但 App 页面（`dioxus://`）与 `http://127.0.0.1` **跨源**，Dioxus 读不到 iframe 内部滚动 → 破坏统一滚动进度跟踪；需 postMessage 桥或全页同源化。

**决策**：

### dangerous_inner_html 注入
> [!IMPORTANT] 【当前】 dangerous_inner_html 注入
> 
> 章节 HTML 用 `dangerous_inner_html` 注入，资源链接在 `html_extract.rs` 统一重写为**绝对 http URL**；不用 `<base>` 标签、不用 iframe。

**依据**：innerHTML 与"Dioxus 自绘导航/TOC + `scroll_ratio` 节流存进度 + 不依赖 JS"的设计基石契合。

**影响**：`dangerous_inner_html` 不执行 `<script>` → 搜索/导航原生 JS 失效，由 Dioxus 自绘 TOC/导航替代（预期行为，见 [project.md §11](project.md#11-风险与待定项) 风险项）。

## D-06 静态资源分流

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 已决策 | 2026-08-13 | [diff.md §2.3「静态资源加载」](diff.md#23-逐维度对比) |

**背景**：App 有两类静态资源、被不同页面消费：① App 自身 UI（书架/设置/抽屉）的 CSS/字体/图标；② 阅读页要用的样式（书籍渲染 CSS、代码高亮主题）。前者由 Dioxus 渲染，后者是注入 HTML，WebView 加载时其 `<link>` 引用必须可访问。

**决策（2026-08-09，已由下方 2026-08-13 决策覆盖）**：App UI 资源走 Dioxus `[asset]` 打包（`asset!()` 宏两平台自动适配，Android 进 APK assets，无平台差异）；阅读页样式**原拟**"与书籍资源同通道、统一经本地 http 分发"（从 `assets/` 样式目录映射，或随书存储分发）——该通道分发仅对备选方案 2 成立，v1 不采用。

**决策（2026-08-13，覆盖上方 2026-08-09 决策）**：

### 方案 1 include_bytes 内嵌
> [!IMPORTANT] 【当前】 方案 1：include_bytes! 内嵌
> 
> 样式体积小（几 KB），直接编进发布二进制，渲染时随章节 HTML **内联**注入、**不经本地 http 服务器**；改主题 = 重新发布二进制；两端零分叉、无启动流程。

### 方案 2 首启复制落盘
> [!NOTE] 【备选】 方案 2：首启复制落盘
> 触发：主题热更新（需样式可下载/替换）
> 
> 首启/更新时落盘 `getFilesDir()`（Windows 为普通文件）替代内嵌。Android 上"随 App 分发"的样式打进的是 APK（zip），Rust 无法像读普通文件那样读 APK 内部，需 JNI AssetManager 读出后写入私有目录。方案 2 / 主题热更新的平台差异（Android JNI AssetManager 读 APK、路径注入自运行时 `getFilesDir()`；Windows 读普通文件）**收敛在 app 层兼容层**，向 core 提供统一"样式资源提供者"接口，**不放平台差异进 core**——v1 内联无需该兼容层，仅备选方案 2 需要；**兼容层需抹平的具体平台差异清单后续整理**（见 [project.md §11](project.md#11-风险与待定项)）。

**影响**：`Dioxus.toml [asset]` 只服务于 App UI，不承担阅读页样式；两端 `<link>` 引用 URL 形态统一，core 无分叉。

## D-07 薄门面与命令化

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 已决策 | 2026-08-09 | [project.md §6.9](project.md#69-服务编排薄门面-按需命令化) |

**背景**：避免"UI 直接认识多个服务、一次操作多次跨层调用"。架构为分层树状（UI→服务→核心叶子），服务间几乎无互调。

**决策**：

- **薄门面（Facade，v1 即生效）**：UI 层只依赖 `AppService` 一个入口，单次用户操作 = 单次门面调用（`add_book(url)` 合并 detect + add；`open_reading(book_id)` 合并"读位置 + 渲染章节 + 保存进度"）。UI 依赖数从 O(服务数) 降到 O(1)。
- **命令化（Command，按需而非全局）**：短、无网络、无并发、无进度需求 → 普通函数（删除书籍、progress.json 读写）；长 + 网络 + 可中断 + 需进度/串行 → 命令对象（`UpdateBookCommand`），经命令队列串行执行（一次一条，落实[D-02](#d-02-json-元数据而非-sqlite)单写者），`progress()` 暴露阶段供 UI 订阅，命令携带执行阶段支持中断续做。
- **v1 边界**：仅"更新书籍"（SD-3）命令化；"添加书籍"先拆命名函数（`check_latest` / `fetch_to_temp` / `commit_and_tag` / `migrate_and_save`），出现并发/进度需求再升级。

**为什么不引入全局中介者**：架构为分层树状，`UpdateService`/`PositionService` 本就是各自流程的中介者；再引入全局 hub 会令所有模块反向依赖一个中央对象，流程变隐式、难以测试与定位——是净负收益。

**依据**：门面让业务编排不泄漏进 Dioxus 屏幕（`mdor-app` 只做"拿到 HTML 注入 + 交互"）；命令边界单一（"完成一次更新"），注入假适配器即可单测。

**影响**：服务层模块（`UpdateService`/`PositionService`）保留为各自流程的中介者，不引入全局 hub。

## D-08 变更检测

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 已决策 | 2026-08-09 | [diff.md §4.5.7](diff.md#45-gix-三坑的配置规避机制梳理与待定讨论记录-2026-08-09) |

**背景**：更新时判断"内容变没变"。gix/git status 的 stat 快路径（`lstat()` 的 size/mtime 对比）被 mdor 全量重写流击穿——每次 fetch 全量重写工作区 → 每个文件 mtime 都是新的 → 与 index stat 快照必不匹配 → 永远落慢路径（全量读盘 + 逐文件 hash）；且 status 依赖写盘时序与 stat 缓存边缘情况。

**决策**：

### 检测层 原始字节 hash
> [!IMPORTANT] 【当前】 检测层 = 原始字节 hash
> 
> 下载字节 hash vs 上个 commit blob hash，前提是 [D-09](#d-09-gix-三坑配置规避) 已强制 `core.autocrlf=false`；**展示层**（版本比较界面给人看的文本 diff）用 **gix diff**（树对象级，与过滤器无关）。分工：hash 回答"内容变没变"，gix diff 回答"变了什么"，两者不是竞争关系。

**依据（权衡对照）**：

| 维度 | 原始 hash + autocrlf=false | gix status（容忍任意 autocrlf） |
|---|---|---|
| 无变化时 I/O | 内存比 hash，磁盘零接触 | 先全量写盘 → 再全量读回 hash |
| 新/删文件发现 | 抓取清单天然已知，无需枚举 | 依赖工作区目录遍历 |
| 跨平台字节身份 | 两端 blob 字节一致，可同步/去重/互验 | 源为 CRLF 时 Windows blob=LF、Android blob=CRLF，同步失效 |
| 版本 diff 语义 | 字节级（换行变化也算） | 内容级（忽略仅换行变化） |
| 实现复杂度 | 一个 hash + 比较 | gix status API、写盘时序、stat 缓存边缘情况 |

### gix status 检测层
> [!CAUTION] 【已否决】 gix status 检测层
> 原因：接受字节分叉，跨平台同步存储即不可用，与 mdor 字节保真原则冲突
> 
> 其唯一的独有优势是"检测层对任意配置免疫"，但前提是接受字节分叉——一旦接受，跨平台同步存储即不可用，与 mdor 字节保真原则冲突，故否决。

**假差异精确链条**：autocrlf=true 时 commit 的 clean 过滤器把 blob 归一化为 LF，而下载/工作区是原始字节（源为 CRLF 时）→ 在"未归一化的原始字节"与"已归一化的 blob"上比 hash 必然不等。**只有源为 CRLF 才触发**；autocrlf=false 使 下载≡blob≡磁盘，原始 hash 退化为恒等比较。这正是"不赌源恰好是 LF、必须强制 false"的原因。

**影响**：autocrlf=false 下内容保留源站换行风格，但解析层**无需手写归一化**——`pulldown-cmark`（CommonMark：`\n`/`\r\n`/`\r` 均为行结束）与 `scraper`（HTML5：tokenize 阶段 CRLF→LF）已按各自规范处理；`str::lines()` 原生兼容 `\r\n`。底线仅是"不假设上游是 LF"（不手写 `split('\n')`、正则跨行用 `\r?\n`）。跨平台同步存储：blob 字节两端一致，下载一次可跨端复用/去重/校验。

## D-09 gix 三坑配置规避

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 待 M1 实测 | 2026-08-09 | [diff.md §4.5](diff.md#45-gix-三坑的配置规避机制梳理与待定讨论记录-2026-08-09) |

**背景**：§4.3 三个 Windows 特有坑（长路径 / 大小写 / autocrlf）需在 gix 配置侧规避，但 gix 是库而非 CLI、没有 `git config` 命令，且三坑性质不同不能一刀切。**关键风险——全局约定是毒药**：gix 会读到用户机器全局配置（Git for Windows 常写 system 级 `core.autocrlf=true`，实锤案例 helix #6467），mdor 要求工作区字节 = 上游字节，任何 CRLF 转换都破坏它，且与 Android 行为不一致——不能靠"用户改全局配置"这类约定，必须由 mdor 主动在更高优先级压掉。

**决策（推荐方向，待 M1 实测后敲定）**：A + B 叠加收敛到**两个配置施加点**，另有大小写冲突处理定案：

1. **snapshot.rs 的 clone/init 路径**：成功后、checkout 前执行 `apply_windows_safety_config()`，写 repo-local：`core.autocrlf=false`（必须）、`core.longpaths=true`（防御 + git CLI 互操作）；`core.ignorecase` 交给 gix 探测（Windows 上确认自动为 true）。
2. **AppService 统一仓库打开入口**：`config_overrides` 兜底 `core.autocrlf=false`，保证进程内行为确定。
3. **ignorecase 物理冲突不在配置层解决（定案 2026-08-09）**：fetch/clone 后对 tree 做大小写冲突检测，对象层恒两条目，分两层处理——**同 blob**（同一内容）归一为一个资源；**异 blob**（内容真不同）双渲染+标注（默认）/ 报错（整书级，两平台一致）。**Windows 退化**：NTFS 物理只能落一个文件 → "单渲染+标注"；跨平台真双渲染绑定 [D-10](#d-10-资源读取通道) blob 直接读能力（v1 默认不引入）。

**三坑性质（为什么不能一刀切）**：

| 坑 | gix 真实行为 | 配置能否解决 |
|---|---|---|
| autocrlf | gix 默认遵循配置（含 system/global），会真做 LF↔CRLF 转换 | **必须**显式压为 `false`，有明确手段 |
| ignorecase | clone/init 时经 `create::Options::fs_capabilities` 探测文件系统并写入 git-config（NTFS 上大概率自动 `core.ignorecase=true`） | 只能让索引比较按大小写不敏感；**救不了**物理冲突 |
| longpaths | 260 限制是 Win32 API 限制而非 NTFS；gix 走 Rust `std::fs`（宽字符 API + 超长路径自动 `\\?\`） | 大概率**不需要**；设它仅为 git CLI 互操作 / 防御 |

**待 M1 实测项**：gix 在 Windows clone 是否自动写 `core.ignorecase=true`；checkout 超 260 路径是否无碍；模拟 Git for Windows system autocrlf=true 时压成 false 后 checkout 不再转换；碰撞路径 checkout 实际行为（告警 / 静默覆盖）；tree 级大小写冲突检测在 fixtures 验证；同 blob / 异 blob 判定（读两路径 blob oid 是否相等）。

**影响**：autocrlf=false 使"磁盘字节 ≡ blob 字节 ≡ 两端字节"，把 gix 当字节透明存储用。

## D-10 资源读取通道

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 已决策 | 2026-08-09 | [project.md §12.1](project.md#121-关键设计决策)、[diff.md §2.3](diff.md#23-逐维度对比) |

**背景**：资源服务器从哪读字节有两个候选——工作区文件（默认）或 git 对象库 blob。

**决策**：

### 工作区直读
> [!IMPORTANT] 【当前】 默认通道 = 工作区直读
> 
> `render/resources.rs` 的 URL→文件路径映射，本地 `tiny_http` 服务器（进程归 app 层）读磁盘字节。**现状：v1 不引入 blob 直接读，保持工作区直读。**

### blob 直接读
> [!NOTE] 【备选】 blob 直接读
> 触发：跨平台真双渲染 / M5 免 checkout 服务任意版本 / 数据同步懒加载
> 
> URL→blob oid，从 git 对象库读字节，为可选能力，与工作区直读**互斥、不并行**——要么这个要么那个，做成插件化通道，经设置选项二选一。

**收益（blob 直接读）**：① 大小写碰撞边界跨平台一致（Windows 真双渲染，见 [D-09](#d-09-gix-三坑配置规避)）；② 服务字节 = 对象库权威字节，无工作区竞态；③ 免 checkout 服务任意版本（M5 铺路，并可加回 [D-04](#d-04-本地资源分发) 的 `<version>` 寻址）；④ 与数据同步懒加载 blob 复用同一能力；⑤ 渲染不受 Windows checkout 的路径长度/大小写坑影响。

**代价**：对象库读取慢于文件读（大内容需缓存）；服务器引入对象访问（gix 本就是 core 依赖）。

**接口可行性（gix 现成）**：读 blob 是 gix 一等公民——`repo.find_blob(oid)` → `Blob { data: Vec<u8> }`；路径→oid 用 `repo.tree(id).traverse()`（或 `find_tree_entry_by_path`）；pack 解压/delta 链解析已由 gix 封装。实现复杂度集中在**映射与缓存**：`resources.rs` 的 URL→规范化路径逻辑不变，只多一步"路径→oid"，缓存用 gix buffer 复用 API；服务器改为持有 gix 对象访问即可，通道接口不变——互斥二选一正是可插拔预留点。

## D-11 TLS 与加密选型

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 已决策 | 2026-08-09 | [diff.md §1.6](diff.md#16-端到端对比与推荐方案)、[§1.8](diff.md#18-rustls-稳定性评估选型依据)、[§1.9](diff.md#19-加密-provider-可插拔性选-ring随时可换) |

**背景**：mdor 有两处走 HTTPS（reqwest：镜像 HTML / GitHub API 探测；gix：git clone/fetch）。Android 系统无 OpenSSL 库，`native-tls` 在 Android 上需用 NDK 把 OpenSSL C 代码交叉编译成 arm64 目标（`openssl-sys` 找不到库/版本不匹配/构建脚本报错，著名坑）——这是 [project.md §1.2](project.md#12-技术栈)"Android 无 OpenSSL 依赖"的原因。

**决策**：

### 统一 TLS 栈 reqwest + rustls
> [!IMPORTANT] 【当前】 统一 TLS 栈：reqwest + rustls
> 
> 两平台统一 `reqwest`（`default-features = false` + `rustls-tls`）+ **rustls-platform-verifier**（Android 走 JNI 调系统证书验证，Windows 退回 SChannel）；gix 开 `http-client-reqwest`（复用同一 TLS 栈，`curl` 后端需编 curl 的 C 代码）。

### 加密 provider ring
> [!IMPORTANT] 【当前】 加密 provider = ring
> 
> 轻量、免 cmake/perl、APK 体积小，经 rustls `CryptoProvider` 抽象可随时换 [【备选】 aws-lc-rs](#加密-provider-aws-lc-rs)。

### 加密 provider aws-lc-rs
> [!NOTE] 【备选】 加密 provider = aws-lc-rs
> 触发：需 FIPS / 更广算法面
> 
> 切换 = Cargo feature 一行 + 重新打包；不做运行时双 provider 注入。

**根证书三选一**：

### 根证书 platform-verifier
> [!IMPORTANT] 【当前】 根证书 = rustls-platform-verifier
> 
> 唯一同时解决两端，认用户/系统 CA。公司电脑常装内网 CA（`mitmproxy` 抓包、VPN 网关证书），只打包 webpki-roots 时 Windows 访问内网文档站会报 `certificate verify failed`——故选 platform-verifier。

### 根证书 webpki-roots
> [!NOTE] 【备选】 根证书 = webpki-roots
> 触发：想要极致简单、放弃企业 CA
> 
> 编译进包，离线，忽略企业 CA。

### 根证书 native-certs
> [!CAUTION] 【已否决】 根证书 = rustls-native-certs
> 原因：运行时读操作系统信任库，Android 无实现
> 
> 运行时读操作系统信任库（Windows 完美），但 Android 无实现，天然只解决 Windows 端。

**rustls 稳定性评估**：2016 年诞生、活跃维护（当前 0.23.x 系列，backport 修复 = 等效 LTS 稳定线）；独立安全审计 + OpenSSF 徽章；Prossimo（ISRG）主导，Google/AWS/Flyio 资助；Let's Encrypt（服务数亿网站的 CA）计划用 rustls 替换 OpenSSL。注意点：版本号 0.x 按 semver minor 可破坏 API（维护策略是"0.23 长期系列 + backport"，社区当稳定版用）；"纯 Rust"需打折（ring 含手写汇编、非纯 Rust、无 FIPS；协议逻辑为纯 Rust）。

**影响**：reqwest 0.13 起必须在 `main()` / `AndroidMain` **最早处** `install_default()`（否则 `Client::new()` 直接 panic）；`ring` 需 NDK clang 参与构建（NDK 自带，正常）；`rustls-platform-verifier` 与 reqwest 传递进来的 rustls 需在根 `[workspace.dependencies]` 钉同一 0.23.x（防双版本）。

## D-12 依赖与安全审计

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 已决策 | 2026-08-09 | [project.md §12.1](project.md#121-关键设计决策) |

**决策**：

- 异步运行时：core 用 tokio；reqwest 配 rustls（[D-11](#d-11-tls-与加密选型)）
- 依赖版本统一：reqwest / scraper / pulldown-cmark / serde / gix 等在根 `[workspace.dependencies]` 钉一次
- 依赖安全审计 = **`cargo audit`（零配置）**：本地定期或 CI 跑，对照 RustSec Advisory Database（RUSTSEC），漏洞存在时退出码非 0；保证"无已知未修复漏洞"可持续验证而非一次性判断
- **不引入 `cargo deny`**：许可证合规（licenses）对离线阅读器非刚需、来源检查（sources）冗余（依赖全来自 crates.io 且 `Cargo.toml` 自持）

**依据（补充 2026-08-19）**：根钉版对 rustls（[D-11](#d-11-tls-与加密选型)，防双版本）与 dioxus（跟随 dx 配对，[env.md §4.3](env.md#43-框架-dioxus-dx必须同步)）是**硬约束**——即使推翻本决策改 per-member，也绕不开这两者的根钉版，只会变成「大部分 per-member + 个别钉根」的混合态，更混乱。`cargo add` 无 workspace 支持（#11527 / #16797），钉根 = 手动维护根表 + member `{ workspace = true }` 引用，操作细节见 [env.md §4.1](env.md#41-rust-依赖crates-最常见)「版本约束落根工作流」。

**影响**：CI 的 `core-quality` job 挂 `cargo audit`（[project.md §12.3](project.md#123-ci-与发布github-actions)）；APK 体积敏感时用 cargo 自带 `cargo tree -d` 按需排查重复版本（multiple-versions），无需整套 deny。

## D-13 数据目录注入

| 状态 | 日期 | 规范位置 |
|---|---|---|
| 已决策 | 2026-08-09 | [project.md §9](project.md#9-存储布局)、[§12.1](project.md#121-关键设计决策)、[diff.md §3](diff.md#3-数据目录与存储路径app-层注入) |

**背景**：两端对"应用数据能放哪"约束本质不同——Windows 原生进程不设限（位置是**产品选择**），Android 沙箱强制（只有 `getFilesDir()` 与外部存储两块）。

**决策**：`BookStore::new(base_dir)` 接收路径，core 平台无关；`mdor-app` 启动时按平台解析数据根（cfg 分支）——Android 走 JNI `getFilesDir()`（应用私有目录，免权限、随卸载删除）；Windows 走 `std::env::current_exe()` 的 **exe 同目录** `data/bookstore/`（便携式：不存在则 `create_dir_all`，**目录不可写直接报错，不回退**到系统用户目录）。

**依据**：差异收敛在"数据根"一层，根之下 `data/bookstore/` 是产品自定义结构、与平台无关，core 只依赖这一层；Windows 选 exe 同目录不污染 `%APPDATA%`/系统目录，数据随 exe 走、整目录可拷贝迁移。

**影响**：Android 卸载即丢（离线书可重下，可接受）；JNI 路径只能在运行时获取（`main()` 阶段拿不到）。

---

*本文件为决策记录，随实现推进持续更新；新增决策先读 [README.md](README.md#decisionsmd-登记规则)。*
