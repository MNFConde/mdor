//! StaticSiteSource（§4.1）：托管静态站点的镜像适配器。
//!
//! M2 交付范围（26-08-31 定案）：递归镜像（同源 + 入口路径前缀双重限界）+ TOC 构建；
//! `<main>` 抽取与资源链接重写属渲染管线（§6.5），留 M3。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::source::{FetchResult, SourceAdapter, SourceKind};

/// 镜像边界与限额（§11 防越界爬取）。
///
/// 判定规则（26-08-31 定案）：目标 URL 必须**同时**满足同源与入口路径前缀，
/// 才会被抓取。`doc.rust-lang.org/book/` 的页面会链接同源的 `/std/` 等外书路径，
/// 仅同源会爬出书籍范围，故前缀是硬边界。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorOptions {
    /// 入口 URL 的 origin（`https://example.com`），全小写 host，无尾斜杠。
    pub origin: String,
    /// 入口 URL 的路径前缀（`/book/`），无尾斜杠；根路径入口为空串（限 origin 全站）。
    pub path_prefix: String,
    /// 递归页面发现的深度上限（入口页 = 0）。
    pub max_depth: u32,
    /// 单文件大小上限（字节）。
    pub max_file_bytes: u64,
    /// 镜像总量上限（字节，所有文件之和）。
    pub max_total_bytes: u64,
    /// 镜像文件数上限（页面 + 资源）。
    pub max_files: usize,
}

impl MirrorOptions {
    /// 从入口 URL 推导边界：origin 全小写；路径取目录前缀（入口页自身路径归入前缀内）。
    ///
    /// `https://Example.com/book/index.html` → origin `https://example.com`、
    /// prefix `/book`；`https://a.b/` → prefix 空串（限全站）。
    pub fn from_entry(entry_url: &str) -> Result<Self> {
        let parsed = url::Url::parse(entry_url).map_err(|_| Error::InvalidUrl(entry_url.into()))?;
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Err(Error::InvalidUrl(entry_url.into()));
        }
        let host = parsed
            .host_str()
            .ok_or_else(|| Error::InvalidUrl(entry_url.into()))?
            .to_lowercase();
        let port = match parsed.port() {
            Some(p) => format!(":{p}"),
            None => String::new(),
        };
        let origin = format!("{}://{}{}", parsed.scheme(), host, port);

        // 路径前缀 = 入口文件所在目录（含目录自身名），保证目录外引用不越界。
        let path = decoded_path(&parsed);
        let dir = match path.rfind('/') {
            Some(i) => &path[..i],
            None => "",
        };
        Ok(Self {
            origin,
            path_prefix: dir.to_string(),
            max_depth: Self::DEFAULT_MAX_DEPTH,
            max_file_bytes: Self::DEFAULT_MAX_FILE_BYTES,
            max_total_bytes: Self::DEFAULT_MAX_TOTAL_BYTES,
            max_files: Self::DEFAULT_MAX_FILES,
        })
    }

    /// 默认递归深度。
    pub const DEFAULT_MAX_DEPTH: u32 = 8;
    /// 默认单文件上限：5 MiB（mdBook 单页/资源远小于此）。
    pub const DEFAULT_MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;
    /// 默认总量上限：128 MiB（离线书籍合理量级，超出说明误入爬取）。
    pub const DEFAULT_MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
    /// 默认文件数上限。
    pub const DEFAULT_MAX_FILES: usize = 5000;

    /// 判定目标 URL 是否在镜像边界内（同源 + 路径前缀 + http(s) scheme）。
    #[must_use]
    pub fn allows(&self, target: &str) -> bool {
        let Ok(parsed) = url::Url::parse(target) else {
            return false;
        };
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return false;
        }
        let Some(host) = parsed.host_str() else {
            return false;
        };
        let port = match parsed.port() {
            Some(p) => format!(":{p}"),
            None => String::new(),
        };
        if format!("{}://{}{}", parsed.scheme(), host.to_lowercase(), port) != self.origin {
            return false;
        }
        let path = decoded_path(&parsed);
        self.path_prefix.is_empty()
            || path == self.path_prefix
            || path.starts_with(&format!("{}/", self.path_prefix))
    }
}

/// 解码后的 URL path（百分号解码仅用于前缀比较；本地文件名另经 sanitize）。
fn decoded_path(url: &url::Url) -> String {
    let raw = url.path();
    // 仅在含 % 时尝试解码；解码失败保持原样（比较容错）。
    if raw.contains('%')
        && let Ok(decoded) = percent_decode(raw)
    {
        return decoded;
    }
    raw.to_string()
}

fn percent_decode(raw: &str) -> std::result::Result<String, ()> {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).map_err(|_| ())?;
            let byte = u8::from_str_radix(hex, 16).map_err(|_| ())?;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| ())
}

/// StaticSiteSource（§4.1）：递归镜像同源页面/资源（自建链场景 2）。
#[derive(Debug, Default)]
pub struct StaticSiteSource;

impl StaticSiteSource {
    /// 构造适配器。
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl SourceAdapter for StaticSiteSource {
    fn kind(&self) -> SourceKind {
        SourceKind::StaticSite
    }

    fn name(&self) -> &'static str {
        "StaticSiteSource"
    }

    /// 探测：http(s) URL 即认识（静态站无统一 URL 模式；GitHub URL 由 GitHubSource
    /// 的更具体规则优先，注册顺序保证 M4 时后者先答）。
    fn detect(&self, url: &str) -> bool {
        let Ok(parsed) = url::Url::parse(url) else {
            return false;
        };
        matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some()
    }

    /// 获取/刷新：镜像到内存文件集并返回结构（§4.2 SD-1）。
    async fn fetch(&self, url: &str, _dest: &Path) -> Result<FetchResult> {
        let opts = MirrorOptions::from_entry(url)?;
        let files = mirror_site(url, &opts).await?;
        let toc = build_toc(&files);
        let title = extract_title(&files);
        tracing::info!(
            files = files.len(),
            toc = toc.len(),
            title,
            "静态站镜像完成"
        );
        Ok(FetchResult {
            version_id: content_version(&files),
            title,
            toc,
            files,
        })
    }

    /// 版本标识：入口页字节的 SHA-256 前缀（静态站无 commit SHA/ETag 可靠来源；
    /// 内容树 hash（tree oid）在落库后由 SnapshotMeta 承载，§5）。
    async fn remote_version(&self, url: &str) -> Result<Option<String>> {
        let opts = MirrorOptions::from_entry(url)?;
        let head = http_get(&entry_html_url(&opts)).await?;
        let digest = Sha256::digest(&head);
        Ok(Some(short_hex(&digest)))
    }
}

/// 文件集（相对路径 → 字节）：镜像引擎的产物与落库输入（切片4）。
pub type FileSet = HashMap<PathBuf, Vec<u8>>;

/// 由文件集字节派生稳定版本标识（SHA-256 前 16 位 hex；与 `Book::derive_id` 同风格）。
#[must_use]
pub fn content_version(files: &FileSet) -> String {
    let mut hasher = Sha256::new();
    let mut paths: Vec<&PathBuf> = files.keys().collect();
    paths.sort();
    for path in paths {
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update(&files[path]);
    }
    short_hex(&hasher.finalize())
}

/// 摘要字节 → 前 16 位 hex（sha2 0.10 手写转换，避免引入 hex crate）。
fn short_hex(digest: &[u8]) -> String {
    digest[..8].iter().map(|b| format!("{b:02x}")).collect()
}

fn entry_html_url(opts: &MirrorOptions) -> String {
    if opts.path_prefix.is_empty() {
        format!("{}/", opts.origin)
    } else {
        format!("{}{}/", opts.origin, opts.path_prefix)
    }
}

/// 单次 HTTP GET（reqwest，rustls，D-11）。
async fn http_get(url: &str) -> Result<Vec<u8>> {
    let resp = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|e| Error::Http(e.to_string()))?;
    let status = resp.status();
    let bytes = resp.bytes().await.map_err(|e| Error::Http(e.to_string()))?;
    if !status.is_success() {
        return Err(Error::HttpStatus {
            url: url.to_string(),
            status: status.as_u16(),
        });
    }
    Ok(bytes.to_vec())
}

/// 从入口页 `<title>` 抽书名。
///
/// mdBook 页面标题为「章节名 - 书名」形态：取最后一个 `-` 之后的部分为书名；
/// 无 `-` 时整串即书名（无则回退 "StaticSite"）。
fn extract_title(files: &FileSet) -> String {
    use scraper::Selector;
    let Some(index) = files.get(Path::new("index.html")) else {
        return "StaticSite".to_string();
    };
    let doc = scraper::Html::parse_fragment(&String::from_utf8_lossy(index));
    let title_sel = Selector::parse("title").expect("选择器合法");
    doc.select(&title_sel)
        .next()
        .map(|t| t.text().collect::<String>().trim().to_string())
        .filter(|t| !t.is_empty())
        .map(|t| {
            t.rsplit_once(" - ")
                .map(|(_, book)| book.trim().to_string())
                .unwrap_or(t)
        })
        .unwrap_or_else(|| "StaticSite".to_string())
}

/// 递归镜像入口（切片2）：从入口页出发，广度优先抓取页面与资源。
///
/// 边界（`opts`）：同源 + 路径前缀 + 深度/单文件/总量/文件数限额（§11）。
/// 页面 = text/html（解析 `<a href>` 继续发现页面、`src/href` 抓资源）；
/// 资源 = 其余 content-type，入文件集后不再递归。
/// 返回相对路径（URL path 落盘形态，已 sanitize）→ 字节。
pub async fn mirror_site(entry_url: &str, opts: &MirrorOptions) -> Result<FileSet> {
    use std::collections::VecDeque;

    let entry_path = url::Url::parse(entry_url)
        .map_err(|_| Error::InvalidUrl(entry_url.into()))?
        .path()
        .to_string();

    let mut files = FileSet::new();
    let mut total_bytes: u64 = 0;
    // URL 规范串（去掉 fragment）→ 已抓取。
    let mut seen: HashSet<String> = HashSet::new();
    // (url, depth)：depth 仅对「页面递归发现」计数；资源不递归。
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();

    let normalize = |u: &str| -> Option<String> {
        let parsed = url::Url::parse(u).ok()?;
        Some(format!(
            "{}://{}{}{}",
            parsed.scheme(),
            parsed.host_str()?.to_lowercase(),
            parsed.port().map(|p| format!(":{p}")).unwrap_or_default(),
            parsed.path()
        ))
    };

    let push = move |url: String,
                     depth: u32,
                     seen: &mut HashSet<String>,
                     queue: &mut VecDeque<(String, u32)>|
          -> bool {
        if seen.insert(url.clone()) {
            queue.push_back((url, depth));
            true
        } else {
            false
        }
    };

    let entry = normalize(entry_url).ok_or_else(|| Error::InvalidUrl(entry_url.into()))?;
    push(entry, 0, &mut seen, &mut queue);

    while let Some((url, depth)) = queue.pop_front() {
        if files.len() >= opts.max_files {
            return Err(Error::MirrorLimit {
                limit: "file_count",
                detail: format!("已抓 {} 文件（上限 {}）", files.len(), opts.max_files),
            });
        }
        if total_bytes > opts.max_total_bytes {
            return Err(Error::MirrorLimit {
                limit: "total_bytes",
                detail: format!("已抓 {total_bytes} 字节（上限 {}）", opts.max_total_bytes),
            });
        }

        let bytes = http_get(&url).await?;
        if bytes.len() as u64 > opts.max_file_bytes {
            return Err(Error::MirrorLimit {
                limit: "file_bytes",
                detail: format!("{url} {} 字节超上限 {}", bytes.len(), opts.max_file_bytes),
            });
        }
        total_bytes += bytes.len() as u64;

        let rel = sanitize_rel_path(
            url::Url::parse(&url)
                .map_err(|_| Error::InvalidUrl(url.clone()))?
                .path(),
        )?;

        // HTML：发现页面与资源链接；CSS：发现 url() 资源引用（mdBook 字体/背景图）。
        // 其余（js/图片/字体）不递归。
        let is_html = looks_like_html(&bytes);
        let is_css = rel
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("css"));
        if (is_html || is_css) && depth < opts.max_depth {
            let links = if is_html {
                extract_links(&bytes)
            } else {
                extract_css_urls(&bytes)
                    .into_iter()
                    .map(|u| (u, false))
                    .collect()
            };
            for (link, is_page) in links {
                let Some(absolute) = resolve_url(&url, &link) else {
                    continue;
                };
                if !opts.allows(&absolute) {
                    continue;
                }
                let Some(norm) = normalize(&absolute) else {
                    continue;
                };
                let next_depth = if is_page { depth + 1 } else { depth };
                let _ = push(norm, next_depth, &mut seen, &mut queue);
            }
        }
        // 目录 URL（…/）补 index.html 落盘名由 sanitize 决定；页面/资源统一入集。
        files.insert(rel, bytes);
    }

    // 入口页不因抓取顺序变化：文件集无序，落库侧 content_version 已排序。
    let _ = entry_path;
    tracing::debug!(files = files.len(), total_bytes, "镜像文件集构建完成");
    Ok(files)
}

/// 是否为 HTML（Content-Type 不可靠时按内容嗅探；含完整页与片段）。
fn looks_like_html(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(512)];
    let lower: Vec<u8> = head.iter().map(|b| b.to_ascii_lowercase()).collect();
    let lower = String::from_utf8_lossy(&lower);
    lower.contains("<!doctype html")
        || lower.contains("<html")
        || lower.contains("<a ")
        || lower.contains("<link")
        || lower.contains("<img")
        || lower.contains("<p>")
        || lower.contains("<div")
}

/// 从 CSS 抽取 `url(...)` 资源引用（含引号/不带引号两种形态；data: 由 resolve_url 过滤）。
fn extract_css_urls(css: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(css);
    let mut out = Vec::new();
    let lower = text.to_lowercase();
    let mut search_from = 0;
    while let Some(pos) = lower[search_from..].find("url(") {
        let start = search_from + pos + 4;
        let Some(end_rel) = lower[start..].find(')') else {
            break;
        };
        let raw = text[start..start + end_rel].trim();
        let unquoted = raw
            .strip_prefix('"')
            .and_then(|r| r.strip_suffix('"'))
            .or_else(|| raw.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')))
            .unwrap_or(raw);
        out.push(unquoted.to_string());
        search_from = start + end_rel;
    }
    out
}

/// 从镜像文件集构建 TOC（§4.1）：解析 mdBook sidebar（`ol.chapter`）。
///
/// sidebar 来源：mdBook ≥0.5 为独立 `toc.html`（iframe 兜底）；mdBook 0.4.x
/// 直接内嵌在每页 HTML（入口页 `index.html`）。sidebar 文件可能在任意子目录
/// （入口带路径前缀时，如 `book/toc.html`）——按文件名在任意层级查找。都找不到
/// 时返回空 TOC。链接为站点相对路径，与 `ReadingPosition.chapter_path` 对齐（§5）。
#[must_use]
pub fn build_toc(files: &FileSet) -> Vec<crate::model::toc::TocEntry> {
    use crate::model::toc::TocEntry;
    use scraper::Selector;

    let file_name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned());
    let sidebar_html = files
        .iter()
        .find(|(p, _)| file_name(p).as_deref() == Some("toc.html"))
        .map(|(_, b)| b)
        .or_else(|| {
            files
                .iter()
                .find(|(p, _)| file_name(p).as_deref() == Some("index.html"))
                .map(|(_, b)| b)
        });
    let Some(sidebar_html) = sidebar_html else {
        return Vec::new();
    };
    let doc = scraper::Html::parse_fragment(&String::from_utf8_lossy(sidebar_html));
    let chapter_ol = Selector::parse("ol.chapter").expect("选择器合法");
    let Some(root_ol) = doc.select(&chapter_ol).next() else {
        return Vec::new();
    };

    let link_sel = Selector::parse("a[href]").expect("选择器合法");
    let li_sel = Selector::parse(":scope > li").expect("选择器合法");
    let section_ol_sel = Selector::parse(":scope > ol.section").expect("选择器合法");

    /// 递归解析 `<li>`：取首个 `<a>` 为本章条目，子 `<ol.section>` 为子章节。
    fn parse_li(
        li: scraper::ElementRef<'_>,
        link_sel: &Selector,
        li_sel: &Selector,
        section_sel: &Selector,
    ) -> Option<TocEntry> {
        let a = li.select(link_sel).next()?;
        let href = a.value().attr("href")?.split('#').next()?.to_string();
        // mdBook 条目文本形如 "1.2. 标题"：剥掉前导序号段（数字与点组成）。
        let full = a.text().collect::<String>();
        let title = section_number_regex()
            .find(full.trim())
            .and_then(|m| full.trim().get(m.end()..))
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .unwrap_or(full.trim())
            .to_string();
        let children = li
            .select(section_sel)
            .next()
            .map(|ol| {
                ol.select(li_sel)
                    .filter_map(|child| parse_li(child, link_sel, li_sel, section_sel))
                    .collect()
            })
            .unwrap_or_default();
        Some(TocEntry {
            title,
            path: href,
            children,
        })
    }

    root_ol
        .select(&li_sel)
        .filter_map(|li| parse_li(li, &link_sel, &li_sel, &section_ol_sel))
        .collect()
}

/// 从 HTML 抽取链接：返回 (链接, 是否页面链接)。
///
/// 页面链接 = `<a href>`；资源链接 = `src` 属性 + `<link href>`。
fn extract_links(html: &[u8]) -> Vec<(String, bool)> {
    use scraper::Selector;

    let text = String::from_utf8_lossy(html);
    let doc = scraper::Html::parse_fragment(&text);
    let mut out = Vec::new();

    let a_sel = Selector::parse("a[href]").expect("选择器合法");
    for el in doc.select(&a_sel) {
        if let Some(href) = el.value().attr("href") {
            out.push((href.to_string(), true));
        }
    }
    // 资源：img/script/source/video/audio/embed/iframe 的 src；link 的 href。
    // mdBook ≥0.5 sidebar 经 `<iframe src="toc.html">` 引用，TOC 构建依赖该文件。
    for tag in [
        "img", "script", "source", "video", "audio", "embed", "iframe",
    ] {
        let sel = Selector::parse(&format!("{tag}[src]")).expect("选择器合法");
        for el in doc.select(&sel) {
            if let Some(src) = el.value().attr("src") {
                out.push((src.to_string(), false));
            }
        }
    }
    let link_sel = Selector::parse("link[href]").expect("选择器合法");
    for el in doc.select(&link_sel) {
        if let Some(href) = el.value().attr("href") {
            out.push((href.to_string(), false));
        }
    }
    // noscript 兜底：html5ever 把 `<noscript>` 内容降级为转义文本节点，
    // mdBook 的 `<iframe src="toc.html">` 正在其内——对文本做 src/href 正则找回。
    for el in doc.select(&Selector::parse("noscript").expect("选择器合法")) {
        let text = el.text().collect::<String>();
        for cap in noscript_src_regex().captures_iter(&text) {
            if let Some(m) = cap.get(1) {
                out.push((m.as_str().to_string(), false));
            }
        }
    }
    out
}

fn noscript_src_regex() -> &'static regex::Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r#"(?:src|href)=["\']([^"\']+)["\']"#).expect("正则合法"))
}

fn section_number_regex() -> &'static regex::Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"^\d+(\.\d+)*\.?\s*").expect("正则合法"))
}

/// 相对/绝对链接 → 绝对 URL（基于当前页面 URL；跳过 fragment-only、mailto、data 等非 http 引用）。
fn resolve_url(base: &str, link: &str) -> Option<String> {
    let trimmed = link.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("mailto:")
        || trimmed.starts_with("javascript:")
        || trimmed.starts_with("data:")
    {
        return None;
    }
    url::Url::parse(base)
        .ok()?
        .join(trimmed)
        .ok()
        .map(|u| u.to_string())
}

/// URL path → 安全相对落盘路径（防目录穿越；目录 URL 补 `index.html`；无扩展名目录段补 `/index.html`）。
fn sanitize_rel_path(path: &str) -> Result<PathBuf> {
    let clean = path.trim_start_matches('/');
    if clean.is_empty() {
        return Ok(PathBuf::from("index.html"));
    }
    let mut out = PathBuf::new();
    for seg in clean.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            return Err(Error::MirrorLimit {
                limit: "path_escape",
                detail: format!("路径含 ..：{path}"),
            });
        }
        out.push(seg);
    }
    if out.as_os_str().is_empty() {
        return Ok(PathBuf::from("index.html"));
    }
    // 目录形态（尾 / 或最后段无扩展名且内容将是 html）：mdBook 页面 URL 均带 .html，
    // 此处仅处理目录 URL（…/）——最后一 seg 为空已被跳过，需补 index.html。
    if clean.ends_with('/') {
        out.push("index.html");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts_for(origin: &str, prefix: &str) -> MirrorOptions {
        MirrorOptions {
            origin: origin.to_string(),
            path_prefix: prefix.to_string(),
            max_depth: MirrorOptions::DEFAULT_MAX_DEPTH,
            max_file_bytes: MirrorOptions::DEFAULT_MAX_FILE_BYTES,
            max_total_bytes: MirrorOptions::DEFAULT_MAX_TOTAL_BYTES,
            max_files: MirrorOptions::DEFAULT_MAX_FILES,
        }
    }

    #[test]
    fn from_entry_derives_origin_and_prefix() {
        let o = MirrorOptions::from_entry("https://Example.com/book/index.html").unwrap();
        assert_eq!(o.origin, "https://example.com");
        assert_eq!(o.path_prefix, "/book");
        assert_eq!(o.max_depth, MirrorOptions::DEFAULT_MAX_DEPTH);
        assert_eq!(o.max_file_bytes, MirrorOptions::DEFAULT_MAX_FILE_BYTES);

        let root = MirrorOptions::from_entry("https://a.b/").unwrap();
        assert_eq!(root.origin, "https://a.b");
        assert_eq!(root.path_prefix, "", "根路径入口限全站");
    }

    #[test]
    fn from_entry_rejects_non_http_schemes() {
        assert!(matches!(
            MirrorOptions::from_entry("ftp://example.com/book/"),
            Err(Error::InvalidUrl(_))
        ));
        assert!(matches!(
            MirrorOptions::from_entry("file:///C:/x.html"),
            Err(Error::InvalidUrl(_))
        ));
        assert!(matches!(
            MirrorOptions::from_entry("not a url"),
            Err(Error::InvalidUrl(_))
        ));
    }

    #[test]
    fn allows_same_origin_within_prefix() {
        let o = opts_for("https://doc.example.com", "/book");
        assert!(o.allows("https://doc.example.com/book/index.html"));
        assert!(o.allows("https://doc.example.com/book/sub/page.html"));
        assert!(o.allows("https://doc.example.com/book"), "前缀目录本身");
    }

    #[test]
    fn rejects_cross_origin_and_out_of_prefix() {
        let o = opts_for("https://doc.example.com", "/book");
        assert!(!o.allows("https://other.com/book/x.html"), "跨源");
        assert!(
            !o.allows("http://doc.example.com/book/x.html"),
            "scheme 不同"
        );
        assert!(
            !o.allows("https://doc.example.com/std/vec/index.html"),
            "同源但前缀外（26-08-31 定案硬边界）"
        );
        assert!(
            !o.allows("https://doc.example.com/bookx/"),
            "前缀目录名碰撞"
        );
        assert!(!o.allows("not-a-url"));
    }

    #[test]
    fn allows_root_prefix_limits_origin_only() {
        let o = opts_for("https://a.b", "");
        assert!(o.allows("https://a.b/any/path.html"));
        assert!(!o.allows("https://a.b:8080/x.html"), "端口不同即异源");
    }

    #[test]
    fn percent_decoded_prefix_compared_decoded() {
        let o = opts_for("https://a.b", "/книга");
        assert!(o.allows("https://a.b/%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0/p.html"));
    }

    #[test]
    fn detect_accepts_http_https_only() {
        let src = StaticSiteSource::new();
        assert!(src.detect("https://doc.rust-lang.org/book/"));
        assert!(src.detect("http://localhost:8080/book/"));
        assert!(!src.detect("ftp://x.com/"));
        assert!(!src.detect("nonsense"));
    }

    #[test]
    fn build_toc_parses_mdbook_sidebar() {
        let toc_html = r#"<ol class="chapter">
            <li class="chapter-item"><span><a href="ch1.html"><strong>1.</strong> 第一章 入门</a></span>
                <ol class="section">
                    <li class="chapter-item"><span><a href="ch1-1.html"><strong>1.1.</strong> 安装</a></span></li>
                </ol>
            </li>
            <li class="chapter-item"><span><a href="ch2.html"><strong>2.</strong> 第二章 进阶</a></span></li>
        </ol>"#;
        let mut files: FileSet = HashMap::new();
        files.insert(PathBuf::from("toc.html"), toc_html.as_bytes().to_vec());
        files.insert(
            PathBuf::from("index.html"),
            "<title>测试书</title>".as_bytes().to_vec(),
        );

        let toc = build_toc(&files);
        assert_eq!(toc.len(), 2, "两个顶层章节");
        assert_eq!(toc[0].title, "第一章 入门");
        assert_eq!(toc[0].path, "ch1.html");
        assert_eq!(toc[0].children.len(), 1, "第一章含一个子章节");
        assert_eq!(toc[0].children[0].path, "ch1-1.html");
        assert_eq!(toc[1].title, "第二章 进阶");
        assert_eq!(toc[1].children.len(), 0);

        assert_eq!(extract_title(&files), "测试书");
    }

    #[test]
    fn build_toc_empty_without_sidebar() {
        let files: FileSet = HashMap::new();
        assert!(build_toc(&files).is_empty(), "无 toc.html → 空 TOC");

        let mut bad: FileSet = HashMap::new();
        bad.insert(PathBuf::from("toc.html"), b"<p>no sidebar</p>".to_vec());
        assert!(build_toc(&bad).is_empty(), "无 ol.chapter → 空 TOC");
    }

    #[test]
    fn build_toc_keeps_anchor_stripped_paths() {
        let toc_html = r#"<ol class="chapter"><li><a href="intro.html#sec-2">简介</a></li></ol>"#;
        let mut files: FileSet = HashMap::new();
        files.insert(PathBuf::from("toc.html"), toc_html.as_bytes().to_vec());
        let toc = build_toc(&files);
        assert_eq!(
            toc[0].path, "intro.html",
            "锚点应剥离，chapter_path 只到文件"
        );
    }

    #[test]
    fn extract_css_urls_handles_quoted_and_plain() {
        let css = b"@font-face { src: url('f.woff2') } .a { background: url(\"b.png\") } .c { list: url(d.svg) }";
        let urls = extract_css_urls(css);
        assert_eq!(urls, vec!["f.woff2", "b.png", "d.svg"]);
    }

    #[test]
    fn title_fallback_without_index() {
        assert_eq!(extract_title(&FileSet::new()), "StaticSite");
    }

    #[test]
    fn content_version_stable_and_path_sensitive() {
        let mut a: FileSet = HashMap::new();
        a.insert(PathBuf::from("index.html"), b"v1".to_vec());
        let mut b: FileSet = HashMap::new();
        b.insert(PathBuf::from("index.html"), b"v2".to_vec());
        let mut c: FileSet = HashMap::new();
        c.insert(PathBuf::from("index.html"), b"v1".to_vec());
        c.insert(PathBuf::from("css/x.css"), b"body{}".to_vec());

        assert_ne!(content_version(&a), content_version(&b), "内容变则版本变");
        assert_ne!(content_version(&a), content_version(&c), "文件集变则版本变");

        // 顺序无关（HashMap 迭代序不定，排序后稳定）。
        let mut c2: FileSet = HashMap::new();
        c2.insert(PathBuf::from("css/x.css"), b"body{}".to_vec());
        c2.insert(PathBuf::from("index.html"), b"v1".to_vec());
        assert_eq!(content_version(&c), content_version(&c2));
    }
}
