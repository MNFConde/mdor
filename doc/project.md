# mdor 项目架构文档

> 移动端 mdBook 离线阅读器 · Android · Rust + Dioxus
> 版本：0.1.0（规划稿）

---

## 1. 项目概述

### 1.1 目标

构建一个 **Android 移动端 mdBook 阅读器**，核心能力：

- 通过 **网址** 将远程文档下载到本地，离线阅读
- 支持两类输入来源：**GitHub 仓库（markdown 源文件）** 与 **托管静态站点（渲染好的 HTML）**
- 通过 **网址更新** 文档，并保留版本历史
- 记录 **阅读位置**（章节 + 滚动位置），支持书籍多版本阅读

### 1.2 技术栈

| 项 | 选择 | 说明 |
|---|---|---|
| 语言 | Rust (edition 2024) | 当前 1.97.1 |
| UI 框架 | Dioxus 0.7 | WebView 渲染，Android 一级支持 |
| 构建 | dioxus-cli (`dx`) | `dx serve --platform android` |
| HTTP | reqwest (rustls) | Android 无 OpenSSL 依赖 |
| HTML 解析 | scraper | 抽取 mdBook `<main>` 内容 |
| Markdown | pulldown-cmark | 渲染 GitHub markdown 源 |
| 序列化 | serde / serde_json | 元数据与进度存储；`serde_json` 内置 128 层递归深度限制（防深嵌套栈溢出 DoS）、无 RUSTSEC、Rust 内存安全（见 §6.7） |
| 存储 | 本地文件系统 + gix | JSON 元数据 + 每书一个 git 仓库 |
| 版本/去重 | gix | commit 图 + 版本 tag 承载版本关系，对象库内容寻址去重 |

### 1.3 非目标

- 不内嵌 mdBook 构建流程（不在设备上跑 `mdbook build`）
- 不做在线搜索 / 云同步（首版）
- 不做 iOS（Windows 平台无法构建）

---

## 2. 设计原则

1. **插件化输入**：所有文档来源（GitHub、静态站点、未来更多）通过统一的 `SourceAdapter` trait 接入，核心逻辑不感知来源差异。
2. **核心与 UI 解耦**：`core/` 为纯 Rust、平台无关，可在桌面直接 `cargo test`；`ui/` 为 Dioxus 界面。
3. **离线优先**：所有内容必须先落盘才可阅读，网络仅用于获取与更新。
4. **版本感知**：书籍内容按 git commit 管理，每次抓取/更新打一个版本 tag（指向 commit sha），版本关系由 commit 图承载；阅读位置绑定具体版本，可回放、可迁移。
5. **位置迁移可插拔**：版本间的阅读位置跳转策略做成插件；v1 默认实现为"路径映射"（更新追最新），后续可扩展其他策略（含多版本快照直连）。

---

## 3. 总体架构

### 3.1 架构分层

```
┌──────────────────────────────────────────────────────────────┐
│                      UI 层 (Dioxus RSX)                      │
│   书架 LibraryUI · 添加 AddBookUI · 阅读 ReaderUI · 版本历史 │
├──────────────────────────────────────────────────────────────┤
│                      服务层 (Rust)                           │
│   BookManager · SourceRegistry · UpdateService               │
│   PositionService · RenderService                            │
├──────────────────────────────────────────────────────────────┤
│                      核心/插件层 (Rust)                      │
│   StaticSiteSource · GitHubSource      (输入适配插件)        │
│   PathMigrator (v1 默认) · SnapshotMigrator (+ 预留)         │
│   Versioning · BookStore               (核心能力)            │
├──────────────────────────────────────────────────────────────┤
│                      设备存储 (本地文件系统 + gix)           │
│   library.json · progress.json                               │
│   books/<id>/  (git 仓库：.git 对象库 + 工作区 + 版本 tag)   │
└──────────────────────────────────────────────────────────────┘
```
### 3.2 C4 组件图

```mermaid
graph TB
  User(("用户"))
  subgraph Mdor["mdor Android 应用 (Dioxus + WebView)"]
    Mdor.BookManager["BookManager"]
    Mdor.SourceRegistry["SourceRegistry"]
    Mdor.UpdateService["UpdateService"]
    Mdor.PositionService["PositionService"]
    Mdor.RenderService["RenderService"]
    Mdor.StaticSite["StaticSiteSource"]
    Mdor.GithubSource["GitHubSource"]
    Mdor.SnapshotMigrator["SnapshotMigrator (M5 开放)"]
    Mdor.PathMigrator["PathMigrator (v1 默认)"]
    Mdor.Versioning["Versioning"]
    Mdor.BookStore["BookStore"]
  end
  RemoteHtml["托管静态站点"]
  Github["GitHub"]
  Fs[("设备本地存储")]

  User -. "浏览/阅读/操作" .-> Mdor.BookManager
  Mdor.BookManager -. "add_book(url): 探测" .-> Mdor.SourceRegistry
  Mdor.SourceRegistry -. "静态站点适配" .-> Mdor.StaticSite
  Mdor.SourceRegistry -. "GitHub 适配" .-> Mdor.GithubSource
  Mdor.BookManager -. "初始化仓库 / 打首个版本 tag" .-> Mdor.Versioning
  Mdor.BookManager -. "读写书籍元数据" .-> Mdor.BookStore
  Mdor.UpdateService -. "check_update / 拉取或提交" .-> Mdor.Versioning
  Mdor.UpdateService -. "更新后按路径映射位置" .-> Mdor.PathMigrator
  Mdor.PositionService -. "版本切换/迁移 (M5)" .-> Mdor.SnapshotMigrator
  Mdor.PositionService -. "读/写 progress.json" .-> Mdor.BookStore
  Mdor.Versioning -. "提交 commit / 打版本 tag / 更新 HEAD" .-> Mdor.BookStore
  Mdor.RenderService -. "读取章节内容" .-> Mdor.BookStore
  Mdor.StaticSite -. "HTTP 镜像 (reqwest)" .-> RemoteHtml
  Mdor.GithubSource -. "git clone/fetch 上游" .-> Github
  Mdor.BookStore -. "文件读写" .-> Fs
```

> 该图由 LikeC4 建模生成，源文件见 `doc/mdor.c4`（再生成：`likec4 gen mermaid doc -o <输出目录>`）。

---

## 4. 插件化输入架构

所有文档来源抽象为统一的 `SourceAdapter` trait，输入模块即为"插件"，通过 `SourceRegistry` 注册、按 URL 探测。

```rust
// core/source/mod.rs
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum SourceKind {
    StaticSite, // 托管静态站点：镜像 HTML
    GitHub,     // GitHub 仓库：拉取 markdown 源
}

#[async_trait]
pub trait SourceAdapter: Send + Sync {
    fn kind(&self) -> SourceKind;
    fn name(&self) -> &'static str;

    /// 探测：该适配器是否认识这个 URL
    fn detect(&self, url: &str) -> bool;

    /// 获取/刷新：从 URL 拉取内容写入 dest，返回书籍结构与版本标识
    async fn fetch(&self, url: &str, dest: &Path) -> Result<FetchResult>;

    /// 版本标识：返回当前远端版本字符串（GitHub: commit SHA；静态站: ETag/内容树 hash）
    async fn remote_version(&self, url: &str) -> Result<Option<String>>;
}

pub struct FetchResult {
    pub version_id: String,
    pub book: BookInfo,
    pub toc: Vec<TocEntry>,
}
```

### 4.1 内置适配器

| 适配器 | 输入 | 处理方式 |
|---|---|---|
| `StaticSiteSource` | `https://host/book/...` | 递归镜像同源页面/资源，保 mdBook 目录结构，抽取 `<main>`；抓取内容自建 git 链（场景 2） |
| `GitHubSource` | `https://github.com/user/repo[/tree/...]` | **git clone/fetch 上游仓库**（保留上游历史），解析 `src/SUMMARY.md` 建 TOC，工作区即上游文件，`pulldown-cmark` 渲染；拉取点打版本 tag（场景 1） |

### 4.2 添加书籍流程（序列图 SD-1）

```mermaid
sequenceDiagram
    autonumber
    actor U as 用户
    participant ADD as AddBookUI
    participant REG as SourceRegistry
    participant BM as BookManager
    participant AD as StaticSiteSource / GitHubSource
    participant RS as RenderService
    participant VER as Versioning
    participant BS as BookStore

    U->>ADD: 输入书籍 URL
    ADD->>REG: detect(url) 遍历已注册适配器
    REG-->>ADD: 返回匹配的 SourceAdapter
    ADD->>BM: add_book(url, kind)
    BM->>AD: fetch(url, 暂存目录)
    AD->>AD: 下载内容（镜像 HTML / clone 上游仓库）
    AD->>RS: 构建 TOC / 抽取章节内容
    RS-->>AD: BookInfo + Toc
    AD-->>BM: FetchResult { version_id, toc }
    BM->>VER: 初始化仓库（场景1 clone 上游 / 场景2 空仓库），生成 commit
    VER->>VER: 写入 .mdor/ 元数据（按 commit sha 索引 toc/meta）
    VER->>VER: 打首个版本 tag refs/mdor/versions/v1
    VER->>BS: 对象库落盘（内容寻址去重），更新 HEAD
    BM->>BS: 写入 library.json（current_version = commit sha）
    BM-->>ADD: 添加成功
    ADD-->>U: 书架出现新书籍
```

---

## 5. 数据模型

```rust
/// 书架上的书籍元数据（library.json 中每条）
#[derive(Serialize, Deserialize, Clone)]
pub struct Book {
    pub id: String,              // book_id，由来源 URL 派生
    pub source_kind: SourceKind,
    pub url: String,             // 原始网址（用于更新）
    pub title: String,
    pub current_version: String, // 当前版本（HEAD commit sha）
    pub added_at: i64,
    pub updated_at: i64,
}

/// 一次获取的内容快照（书籍的一个版本；由 tag refs/mdor/versions/<seq> 标记）
pub struct VersionSnapshot {
    pub version_id: String,      // 版本 tag 指向的 commit sha
    pub workdir: PathBuf,        // 仓库工作区（场景1: 上游文件；场景2: books/<id>/site/）
    pub toc: Vec<TocEntry>,      // 章节树（存于 .mdor/versions/<sha>.json，未被 git 跟踪）
    pub meta: SnapshotMeta,      // 获取时间、来源版本标识、内容树 hash
    // 版本标记 = 私有 tag（refs/mdor/versions/<seq>）；parent 由 commit 图派生，不单独存储
}

/// 阅读位置 —— 与具体版本绑定
#[derive(Serialize, Deserialize, Clone)]
pub struct ReadingPosition {
    pub book_id: String,
    pub version_id: String,      // 位置所属版本（commit sha；v1 更新时被 path 迁移改写）
    pub chapter_path: String,    // 章节路径（TOC 中的相对路径）
    pub heading_anchor: Option<String>, // 标题锚点（mdBook 输出自带）
    pub scroll_ratio: f32,       // 章节内滚动比例 0.0~1.0
    pub saved_at: i64,
}

/// 位置迁移结果
pub struct MigratedPosition {
    pub target_version: String,
    pub target_chapter: String,
    pub target_anchor: Option<String>,
    pub strategy: MigrateStrategy, // 见 §8
}
```

---

## 6. 核心模块

### 6.1 BookManager
书籍生命周期编排：`add_book`、`remove_book`、`update_book`。不感知来源差异，统一走 `SourceRegistry` + `Versioning`。

### 6.2 SourceRegistry
注册表 + 工厂：持有 `Vec<Box<dyn SourceAdapter>>`，`detect(url)` 依次询问。新增来源 = 新增一个实现 `SourceAdapter` 的插件模块并注册，核心与 UI 零改动。

### 6.3 UpdateService
更新编排（见 §7）。

### 6.4 PositionService
阅读位置读写（`progress.json`），以及调用 `PositionMigrator` 完成版本间位置迁移（见 §8）。

### 6.5 RenderService
统一渲染管线：
- `StaticSite` 路径：读取章节 HTML → `scraper` 抽取 `<main id="content">` → 重写资源链接为本地协议 `mdor-book://` → `dangerous_inner_html` 注入
- `GitHub` 路径：读取 `.md` → `pulldown-cmark` → HTML → 同一注入管线
- 内联书籍 CSS，保证代码高亮等样式一致

### 6.6 离线阅读 + 进度恢复（序列图 SD-2）

```mermaid
sequenceDiagram
    autonumber
    actor U as 用户
    participant RD as ReaderUI
    participant PS as PositionService
    participant RS as RenderService
    participant BS as BookStore

    U->>RD: 打开书籍 / 点击"继续阅读"
    RD->>PS: get_position(book_id)
    PS->>BS: 读 progress.json
    BS-->>PS: ReadingPosition{version_id, chapter, anchor, ratio}
    PS-->>RD: 定位到指定版本+章节
    RD->>RS: render_chapter(book_id, version_id, chapter)
    RS->>BS: 读取工作区 site/ 下章节内容（v1 为当前版本）
    BS-->>RS: 章节 HTML / markdown
    RS-->>RD: 渲染完成（dangerous_inner_html）
    U->>RD: 滚动阅读
    RD->>PS: 节流保存 scroll_ratio / 切章保存
    PS->>BS: 写回 progress.json
```

### 6.7 元数据写入可靠性（JSON，不用 SQLite）

几十本书量级的元数据总量 < 100KB，访问形态为按 `book_id` 的简单读写、单进程单写者（串行化即可），无关系查询需求——**JSON 文本足够，不引入 SQLite**（避免 C 依赖/Android 交叉编译复杂度与 schema 迁移，保持依赖纯 Rust）。可靠性由以下约定保证：

| 场景 | 风险 | 解法 |
|---|---|---|
| 写入一半被杀（滚动存进度） | 文件损坏 | **原子写**：写 `*.tmp` + 同目录 `rename` 覆盖（Android/Linux 上原子），要么旧文件要么新文件，无半写状态 |
| 启动读到坏文件 | 无法解析 | 覆盖前保留 `.bak`，启动解析失败回退备份 |
| `add_book`/更新多步中断（建仓库 → 写 `.mdor/` → 写 `library.json`） | 半完成状态 | `library.json` **最后写** = 提交点：中断后书架无此书/仍为旧版本；孤儿 `books/<id>/` 目录启动时清理 |
| 断电/内核崩溃 | rename 未落盘 | 原子写可加 `fsync`（Android 上可选） |

- 原子写封装为 `store` 内工具函数 `write_json_atomic(path, &data)`，core 内统一复用
- 读取统一走 `read_json_capped(path, MAX_META_BYTES)`：**读文件前按字节数上限拦截**（默认 1MB，远超正常元数据量级），把"超大文档耗尽 CPU/内存"这类 DoS 彻底关死（纵深防御，见 §6.8）
- 此策略适用于 `library.json`、`progress.json`；`.mdor/versions/<sha>.json` 为只增写、失败可重写，同样走原子写

### 6.8 解析器安全对照（serde_json 选型依据）

选型时逐项对照了其他 JSON 解析器生态近期公开的漏洞，结论：**以下均为 Java/C 生态问题，`serde_json` 不受影响**。

| 漏洞 | 属于 | 漏洞本质 | 对 `serde_json` 的适用性 |
|---|---|---|---|
| CVE-2026-18401（Jackson-core 异步解析器数字长度绕过） | Java | 非阻塞/异步解析路径漏掉 `maxNumberLength` 校验 → 无限分配 + O(n²) 大数解析 → DoS | 无此类"异步流式解析"API；本项目只用 `from_str` 整块解析自有小文件，不适用 |
| CVE-2026-29062（Jackson-core 嵌套深度限制绕过） | Java | `DataInput`/`Reader` 路径漏掉 `maxNestingDepth` 校验 → 栈溢出 DoS | **已内置防护**：`from_str` 路径默认 128 层递归限制（serde_json PR #163），无已知绕过通告；仅有的 serde#3023 边缘情况（`IgnoredAny` 处理程序构造的 10 万层 `Value`）在本项目"解析自有 <100KB 元数据"的流程中不可达 |
| CVE-2026-9563（Eclipse Parsson 无文档大小上限） | Java | 无默认 max 文档大小 → 超大文档耗尽 CPU/内存 | serde_json 默认同样无大小上限（各主流解析器共性），其危害前提是"解析攻击者可控的网络 JSON"——本项目解析的是**应用自生成的本地元数据**；另以 `read_json_capped()` 1MB 读入 guard 补齐纵深防御 |
| QVD-2026-45876（Fastjson2 反序列化 RCE） | Java | `@type` AutoType 哈希碰撞绕过白名单 + `jar:` URL 类加载 → RCE | **结构性不可能**：Rust/serde 无多态反序列化——JSON 无 `@type` 机制、不从 JSON 加载类、无反身构造副作用，`Deserialize` 全为编译期实现，反序列化无法触发任意代码 |
| USN-7973-1（cJSON 多个内存安全漏洞） | C | OOB 读/写、大数 DoS | 内存安全由 Rust 语言保证（`serde_json` unsafe 极少、无 OOB）；大数解析进 `f64`/`i64`/`u64`，无放大性分配 |

**结论**：异步限制绕过/深度绕过/RCE/内存安全这几类 Java/C 生态漏洞，在 `serde_json` + Rust 内存模型下结构性不适用；唯一设计上同源的"无文档大小上限"以读入 guard 补齐。依赖层面的"无已知未修复漏洞"由 `cargo audit`（§12.1）持续验证。

---

## 7. 版本控制设计

### 7.1 版本标识

两场景统一：**`version_id` = tag 指向的 commit sha**；用户可见的"版本" = 私有命名空间 tag `refs/mdor/versions/<seq>`。

| 来源 | commit 来源 | 说明 |
|---|---|---|
| GitHub（场景 1） | 上游仓库 commit | clone/fetch 上游历史，拉取点在最新 HEAD 打版本 tag，不向上游写任何内容 |
| 静态站点（场景 2） | 自建 commit | 每次抓取 = 一个 commit（parent=HEAD），随即打版本 tag |

### 7.2 快照模型（git 基座 + tag 统一版本）

- 每本书是一个由 gix 管理的 git 仓库：`books/<id>/`（对象库 `.git/` + 工作区）
  - **场景 1**：仓库 = 上游仓库克隆，历史原样保留（`git log`/`git diff` 直接可用），我们只加 tag、不 commit
  - **场景 2**：仓库 = 自建链，每次抓取一个 commit
- **用户版本 = tag**（`refs/mdor/versions/<seq>`，序号单调递增，避免与上游自身 tag 冲突），`version_id` = tag 指向的 commit sha
- 每次内容有变化的抓取/更新：场景 1 打新 tag、场景 2 生成 commit 并打新 tag；内容树 hash 未变化时跳过，不产生空提交
- 对象库天然**内容寻址去重**：跨版本未变的文件只存一份（blob/tree 复用），无需额外去重机制
- `HEAD` 即当前版本，`current` 语义由 git ref 承载；版本列表 = 列 `refs/mdor/versions/*`
- 历史版本读取 = **按需 checkout 目标 commit 到工作区**（单一工作区、原地覆盖，无空间累加）
- 书籍元数据（`.mdor/`：toc/meta，按 commit sha 索引）位于仓库根下但**未被 git 跟踪**，两场景都不污染历史

### 7.3 更新流程（序列图 SD-3）

```mermaid
sequenceDiagram
    autonumber
    actor U as 用户
    participant UP as UpdateService
    participant AD as SourceAdapter
    participant VER as Versioning
    participant MIG as PathMigrator (v1 默认)
    participant BS as BookStore

    U->>UP: 点击"更新"（书架 / 书籍详情）
    UP->>AD: remote_version(url)
    AD-->>UP: 远端版本标识
    alt 远端版本 ≠ 当前版本
        UP->>AD: fetch(url, 暂存目录)
        AD-->>UP: FetchResult { version_id, toc }
        UP->>VER: 对比内容树 hash，跳过空提交
        VER->>VER: 场景1 fetch 上游到新 HEAD / 场景2 生成 commit 写入工作区
        VER->>VER: 打新版本 tag refs/mdor/versions/v<seq+1>，更新 HEAD
        UP->>MIG: 阅读位置按章节路径映射（v1 默认）
        MIG-->>UP: MigratedPosition（新 commit + 映射后的章节）
        UP-->>U: 更新完成（历史 tag 与 commit 保留）
    else 远端版本 == 当前版本
        UP-->>U: 已是最新版本
    end
```

### 7.4 存储基座：为何 v1 起就用 gix

**结论：v1 起即以 gix（纯 Rust 的 git 实现）作为存储基座，每书一个 git 仓库，并以私有 tag 记录版本。** 版本历史 UI、多版本阅读、数据同步都不是 v1 功能，但存储基座一旦选定就应指向长期正确的那个——版本 tag 随每次抓取/更新**自然积累**，将来只需开放功能，而不用回头改造存储层。

**为什么是 git/gix 而不是"目录快照 + 手写版本链"：**

- commit 承载内容快照、commit 图承载历史关系、**tag 承载"版本"语义**、ref（HEAD）= 当前指针、内容寻址对象库 = 去重与校验——版本管理需要的每一件事都被 git 封装好了；自己用目录 + `index.json` 实现，等于重写一个简化且不完整的 git
- 历史版本读取 = 按需 checkout 到单一工作区，与日常 `git checkout` 完全一致，无空间累加
- 未来若做**数据同步**，git 协议（fetch/push、partial clone 按需懒加载 blob）直接复用，无需自建传输层

**诚实成本（gix 封装掉了解析，剩下的工程成本）：**

| 成本项 | 说明 | 应对 |
|---|---|---|
| 依赖体积/内存 | gix 依赖树大，Android 上编译时间、APK 体积、内存上升 | 按需裁剪 feature；每书独立小仓库，控制单仓对象库规模 |
| 写路径 | 由"复制目录"变为"一次 commit"（gix 封装，无解析负担） | 场景1 只 fetch+打 tag、不 commit；场景2 直接使用 `gix` 高层 API（`commit` / tree / blob 读取） |
| 读历史版本 | checkout 目标 commit 到工作区；从对象库直接读 blob 为非必要优化 | 版本功能开放时按切换频率实测二选一 |
| GC / 清理 | 初期"删除版本" = 仅删 tag，不清理 git 历史；后续真回收磁盘需 shallow 截断 + gc | 初期两场景一致（只删 tag）；shallow 回收延后，两场景统一作为用户设置项 |

**被否定的替代方案：**

- **全量目录快照 + COW 硬链接 + `index.json` 版本链**（早期规划）：需手写版本关系、去重、原子性，且无同步传输能力；空间收益（文件级去重）被 gix 对象库天然覆盖
- **git2（libgit2）**：C 依赖，Android 交叉编译麻烦，排除
- **blob 直接读（对象解析桥）作为历史读取主路径**：不必要——单一工作区 checkout 已满足"切状态阅读"，且不占额外空间

> 版本 tag 机制自 v1 生效（每次抓取/更新都打 tag，能力自然积累）；版本 UI / 多版本阅读（方案 D）/ 数据同步为后续里程碑开放的功能（见 §8、§10），开放时无需改造存储层，只需新增读取与展示。

---

## 8. 阅读位置在版本变动后的处理方案

### 8.1 设计决策

**v1 行为（更新追最新，`path` 策略）**：v1 不开放版本历史 UI。更新后阅读位置**按章节路径映射**到新版本（`chapter_path` 不变则直连，章节消失则 TOC 顺序回退相邻章节）；位置在更新时即迁移，不存在"读旧版"路径。

**方案 D（版本快照绑定）为后续开放（M5）**：阅读位置与具体 commit 强绑定：

- 记录时绑定 `version_id`（commit sha，该记录的位置必然可回放）
- 更新后，位置仍指向旧版本 commit；用户从书架打开时，**默认继续读旧版本对应位置**，进度零丢失
- 支持同一文档**多版本并行阅读**（版本历史界面选择任意版本）
- 版本切换 = 按需 checkout 目标 tag 指向的 commit 到工作区；版本列表 = 列 `refs/mdor/versions/*`；清理策略（保留最近 N 版）在版本功能开放时实现

### 8.2 插件化迁移架构

实际"跳转"行为做成插件，未来可让用户选择不同策略。当前内置 `path`（v1 默认），其余后续逐个加入。

```rust
// core/migration/mod.rs
#[async_trait]
pub trait PositionMigrator: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    /// 将旧版本位置迁移到目标版本（可能返回"保持旧版本"）
    async fn migrate(
        &self,
        from: &VersionSnapshot,
        to: &VersionSnapshot,
        pos: &ReadingPosition,
    ) -> Result<MigratedPosition>;
}
```

### 8.3 插件清单

| 插件 id | 名称 | 状态 | 行为 |
|---|---|---|---|
| `path` | PathMigrator | **内置（v1 默认）** | 按 `chapter_path` 直接映射到新版本，路径消失则 TOC 顺序回退相邻章节 |
| `snapshot` | SnapshotMigrator | 预留（M5 开放） | 位置绑定旧版 commit，旧版本仍在 → 迁移结果即"读旧版原位置"；若用户选择追最新，则按 TOC 同名路径映射 |
| `anchor` | AnchorMigrator | 预留 | 增加标题锚点映射，锚点消失回退章节开头，标题改动用模糊匹配 |
| `fingerprint` | FingerprintMigrator | 预留 | 记录阅读位置附近文本指纹，新版本全文检索命中后精确定位 |

> v1 更新默认用 `path` 追最新；方案 D（多版本阅读 + 快照直连，M5）保证"永不丢位置"；其余插件是"用户主动追最新版"时的可选策略，将来通过设置界面选择。

### 8.4 版本切换 / 位置迁移（序列图 SD-4，M5 开放）

```mermaid
sequenceDiagram
    autonumber
    actor U as 用户
    participant VH as VersionHistoryUI
    participant PS as PositionService
    participant MIG as PositionMigrator (SnapshotMigrator 等)
    participant BS as BookStore
    participant RD as ReaderUI

    U->>VH: 打开版本历史，选择目标版本
    VH->>PS: resolve_position(book_id, target_version, 当前位置)
    PS->>BS: 读取当前 ReadingPosition{version_id, chapter, ...}
    PS->>MIG: migrate(from_commit, to_commit, pos)
    alt 目标版本 = 位置版本
        MIG-->>PS: 快照直连：原章节原位置
    else 目标版本 = 其他版本（追最新）
        MIG-->>PS: 按策略映射章节/锚点（可换插件）
    end
    PS->>BS: checkout 目标 tag 指向的 commit 到工作区
    PS-->>RD: 打开目标版本对应章节
    RD-->>U: 进入阅读（保留/重置位置由策略决定）
```

---

## 9. 存储布局

```
<app data dir>/
├── library.json                 # 书架元数据（Book 列表 + current_version = HEAD commit sha）
├── progress.json                # 阅读位置（ReadingPosition 按 book_id 索引）
└── books/
    └── <book_id>/               # gix 管理的 git 仓库（场景1: 上游克隆；场景2: 自建链）
        ├── .git/                # 对象库 + refs（含版本 tag refs/mdor/versions/<seq>，内容寻址去重）
        ├── src/, book.toml ...  # 工作区（场景1: 上游 mdBook 源文件；场景2: 镜像内容于 site/）
        ├── site/                # 工作区（场景2: 当前版本镜像内容，webview 直接读文件）
        └── .mdor/               # 书籍元数据（仓库根下但未被 git 跟踪）
            └── versions/
                └── <commit sha>.json  # 每版本 toc/meta（按 commit sha 索引）
```

- 用户版本 = 私有 tag `refs/mdor/versions/<seq>`；版本列表 = 列 tag；`current` 语义即 git `HEAD`
- `.mdor/` 未被 git 跟踪：toc/meta 按 commit sha 索引，两场景都不污染仓库历史（场景1 尤其关键——克隆的上游历史不被任何附加 commit 改动）
- 历史版本读取 = 按需 checkout 目标 tag 指向的 commit 到工作区（单一工作区，无空间累加）
- 应用级元数据（`library.json`/`progress.json`）位于仓库之外，避免与版本内容混存

> Android 下根目录通过 `android_activity`/JNI 取 `getFilesDir()`；桌面开发回退到用户数据目录，便于 `cargo test` 与调试。

---

## 10. 里程碑

| 阶段 | 内容 | 验证方式 |
|---|---|---|
| **M0** | Android 环境搭建（SDK/NDK/JDK、rustup android targets、dioxus-cli） | `dx serve --platform desktop` 跑通 |
| **M1** | 脚手架：core 模块 + trait + 存储层（gix 基座）+ 书架骨架（中文 UI） | `cargo run` / `cargo test` |
| **M2** | `StaticSiteSource` + 递归镜像下载 | 用真实 mdBook 站点离线镜像 |
| **M3** | 阅读器：内容抽取、资源协议、目录抽屉、滚动进度 | 桌面全流程 |
| **M4** | `GitHubSource`：git clone/fetch 上游仓库（保留历史）+ SUMMARY 解析 + markdown 渲染 | 真实仓库测试 |
| **M5** | 版本功能开放：版本历史 UI + 按需 checkout 多版本阅读 + SnapshotMigrator（方案 D）+ 清理策略（初期删版本 tag；后续 shallow 截断 + gc 为可选设置项，两场景统一） | 修改源站后更新，验证旧版本位置可回放 |
| **M6** | Android 打包（APK）、权限/存储目录/cleartext 配置 | 模拟器/真机验证 |

---

## 11. 风险与待定项

| 风险/待定 | 影响 | 应对 |
|---|---|---|
| wry 自定义协议在 Android 的兼容性 | 阅读页图片/资源加载 | 备选：内嵌本地 `tiny_http` 服务器 |
| Android 数据目录获取 | 存储路径 | JNI `getFilesDir()`；桌面回退 |
| mdBook 高级扩展（`{{#playground}}`、LaTeX、mermaid） | GitHub 源渲染保真度 | 首版仅支持 `{{#include}}`，其余列出 |
| `dangerous_inner_html` 不执行 `<script>` | 搜索/导航原生 JS 失效 | 由 Dioxus 自绘 TOC/导航替代（预期行为） |
| gix 依赖体积/内存（Android） | APK 大小、启动内存 | 按需裁剪 gix feature；每书独立小仓库控制对象库规模 |
| checkout 切版本与 webview 渲染协调 | 切版本时工作区文件被替换 | 先加载章节到内存再切换，或切换后 reload |
| gix 在 Android 的可用性 | 存储基座能否正常跑 | M1 桌面验证 + M6 真机验证 |
| 版本历史的存储占用 | 设备空间 | gix 对象库内容寻址去重；保留最近 N 版初期只删版本 tag（历史与对象保留），后续磁盘回收统一做 shallow 截断 + gc（用户可选设置） |
| 上游仓库体积 / Git LFS | 场景1 磁盘占用与图片渲染 | 依赖 gix 对象去重；LFS 仓库 clone 仅得指针文件，首版提示暂不支持 |
| 静态站点镜像边界 | 防止越界爬取 | 限同源 + 深度/大小上限 |

---

## 12. 项目文件结构

采用 **Cargo workspace** 承载 core / ui 分层（对应 §2 设计原则 2）。`mdor-core` 为纯 Rust、平台无关库，桌面可直接 `cargo test`；`mdor-app` 为 Dioxus UI 二进制（`dx` 构建目标）。输入适配器与位置迁移插件均为 core 内模块、编译期注册（对应 §4 / §8）。

```
mdor/
├── Cargo.toml                 # [workspace] members + [workspace.dependencies] 统一锁定依赖版本
├── rust-toolchain.toml        # 固定 1.97.1 + android targets（arm64-v8a / x86_64）
├── .gitignore                 # /target、mobile/android 构建产物、fixtures 下载缓存
├── README.md
├── doc/                       # 本架构文档、mdor.c4 等
│
├── crates/
│   ├── mdor-core/             # 纯 Rust 库，平台无关，桌面可直接 cargo test
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs               # 统一错误类型
│   │       ├── model/                 # §5 数据模型（纯数据，无 IO）
│   │       │   ├── book.rs            #   Book
│   │       │   ├── snapshot.rs        #   VersionSnapshot / SnapshotMeta
│   │       │   ├── toc.rs             #   TocEntry
│   │       │   └── position.rs        #   ReadingPosition / MigratedPosition
│   │       ├── store/                 # §6.6/6.7 BookStore：文件系统持久化（§9 存储布局，JSON 原子写）
│   │       │   ├── mod.rs             #   BookStore 聚合入口，base_dir 注入
│   │       │   ├── util.rs            #   write_json_atomic + read_json_capped（§6.7 原子写/1MB 读入 guard）
│   │       │   ├── library.rs         #   library.json 读写（原子写，提交点）
│   │       │   ├── progress.rs        #   progress.json 读写（原子写）
│   │       │   └── snapshot.rs        #   git 仓库快照：clone/fetch 上游（场景1）/ 自建链 commit（场景2）、版本 tag 管理、工作区 checkout（gix）
│   │       ├── source/                # §4 输入插件（编译期注册）
│   │       │   ├── mod.rs             #   SourceKind / SourceAdapter trait / FetchResult
│   │       │   ├── registry.rs        #   SourceRegistry（detect 遍历）
│   │       │   ├── static_site.rs     #   StaticSiteSource：递归镜像 + <main> 抽取（自建链）
│   │       │   └── github.rs          #   GitHubSource：git clone/fetch 上游 + SUMMARY 解析 + md 渲染
│   │       ├── versioning.rs          # §7 commit/版本 tag 生成、HEAD/ref 维护、对象去重（gix 对象库）
│   │       ├── migration/             # §8 位置迁移插件
│   │       │   ├── mod.rs             #   PositionMigrator trait
│   │       │   ├── path.rs            #   PathMigrator（v1 更新默认，按章节路径映射）
│   │       │   └── snapshot.rs        #   SnapshotMigrator（方案 D，M5 开放）
│   │       ├── render/                # §6.5 RenderService 统一渲染管线
│   │       │   ├── mod.rs             #   render_chapter 入口
│   │       │   ├── html_extract.rs    #   scraper 抽 <main> + 资源链接重写
│   │       │   ├── markdown.rs        #   pulldown-cmark → HTML
│   │       │   └── resources.rs       #   mdor-book:// 协议路由
│   │       └── services/              # §6 服务编排层
│   │           ├── book_manager.rs    #   add/remove/update 编排
│   │           ├── update_service.rs  #   更新编排（§7.3 SD-3）
│   │           └── position_service.rs#   进度读写 + 迁移调用（§8.4 SD-4）
│   │
│   └── mdor-app/              # Dioxus UI 二进制（dx 构建目标）
│       ├── Cargo.toml
│       ├── Dioxus.toml        # [application] [android] [asset] 配置，dx serve --project 指向此目录
│       ├── assets/            # CSS / 字体 / 图标（dx 打包资源）
│       ├── mobile/            # dx 生成的 Android 原生工程（构建产物，勿手改）
│       └── src/
│           ├── main.rs        # dioxus::launch 入口 + Android getFilesDir()/桌面回退路径解析
│           ├── app.rs         # Router + 主题
│           ├── state.rs       # GlobalSignal：BookManager 句柄、当前书籍/版本
│           ├── screens/       # §3.1 UI 四屏
│           │   ├── library.rs #   书架
│           │   ├── add_book.rs#   添加书籍
│           │   ├── reader.rs  #   阅读（dangerous_inner_html + 滚动节流存进度）
│           │   └── versions.rs#   版本历史
│           └── components/    # 抽屉、列表、按钮等复用组件
│
└── fixtures/                  # 测试样例（core 集成测试用，不随包发布）
    ├── mdbook-static/         # 预构建的静态 mdBook 站点（M2 镜像测试）
    └── github-sample/         # 带 SUMMARY.md 的 markdown 仓库样例（M4 解析测试）
```

### 12.1 关键设计决策

- **数据目录注入而非硬编码**：`BookStore::new(base_dir)` 接收路径；`mdor-app` 启动时按平台解析（Android 走 JNI `getFilesDir()`，桌面走 `dirs`），core 保持平台无关（对应 §11 风险项）。
- **核心与 UI 解耦验证**：全部业务逻辑（含渲染管线）在 core，`mdor-app` 只做「拿到 HTML 注入 + 交互」；验证方式即 `cargo test -p mdor-core` 桌面直跑。
- **`mdor-book://` 协议处理放 `render/resources.rs`**，跨平台复用；wry 协议兼容问题的 `tiny_http` 备选方案（§11）仅影响 app 侧接入点。
- **异步运行时**：core 用 tokio；reqwest 配 rustls（Android 无 OpenSSL）。
- **依赖版本统一**：reqwest / scraper / pulldown-cmark / serde / gix 等在根 `[workspace.dependencies]` 钉一次。
- **依赖安全审计 = `cargo audit`（零配置）**：本地定期或 CI 跑，对照 RustSec Advisory Database（RUSTSEC），保证"无已知未修复漏洞"可持续验证而非一次性判断；漏洞存在时退出码非 0。**不引入 `cargo deny`**：许可证合规（licenses）对离线阅读器非刚需、来源检查（sources）冗余（依赖全来自 crates.io 且 `Cargo.toml` 自持）。若日后关心 APK 体积，用 cargo 自带 `cargo tree -d` 按需排查重复版本（multiple-versions），无需整套 deny。
- **存储基座 = gix（每书一个 git 仓库，链 + tag 统一版本），day-one 引入**：场景1 clone/fetch 上游保留其历史、场景2 自建链 commit；用户版本 = 私有 tag `refs/mdor/versions/<seq>`、HEAD=当前指针、对象库=去重；版本/同步能力随每次抓取自然积累（打 tag 即记录版本），存储层无需将来改造（代价与取舍见 §7.4）。历史版本读取统一走"按需 checkout 单一工作区"，不引入 blob 直接读。

### 12.2 里程碑映射

- **M1**：建 workspace + `mdor-core` 骨架（model / store / source trait / versioning / migration trait + 单测）+ gix 存储基座 + `mdor-app` 书架壳 → `cargo test -p mdor-core` + 书架可跑
- **M2 / M4**：补 `static_site.rs`（自建链 + 版本 tag）/ `github.rs`（clone 上游 + 版本 tag），用 `fixtures/` 做集成测试（HTTP mock：`httpmock` dev-dep）
- **M5**：补 `migration/snapshot.rs`（方案 D）+ 版本历史 UI + checkout 切换 + 清理策略（初期删版本 tag；后续可选 shallow 截断 + gc，两场景统一）
- **M6**：`mobile/` 生成 + Android 打包

---

*本文件为架构规划稿，随实现推进持续更新。*

