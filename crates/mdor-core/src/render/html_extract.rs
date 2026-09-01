//! 章节 HTML 抽取与链接重写（§6.5 RenderService）。
//!
//! 职责二：
//! 1. [``extract_main``]：用 scraper 文档树抽取 `<main>`（`#content` 优先、回退
//!    `main`）的 `inner_html()`（ChildrenOnly），丢弃外边框标签与页头/侧栏/脚本。
//! 2. [``rewrite_links``]：把 `<main>` 内的相对资源引用（`href`/`src`，如
//!    `./img/logo.svg`）统一改写为本地分发前缀。因 scraper 0.27 属性不可变
//!    （无 setter，见 [`crate::render::resources`]），只能「先序列化再文本替换」：
//!    相对路径 → 占位符 `${资源前缀}` → `${书根上下文}`。占位符不含
//!    `&`/`<`/`>`/`"`，故 html5ever 序列化转义不影响文本替换（见模块注记）。

use regex::{Captures, Regex};
use scraper::{Html, Selector};

use super::resources::{LOCAL_PREFIX, VERSION_SEGMENT, local_url};

/// 章节正文缺失时返回错误消息（mdBook 0.3/0.4 无 `<main>` 降级后仍无则报错）。
pub const MISSING_MAIN: &str = "章节无 <main> 内容";

/// 抽取 `<main>`（`#content` 优先、回退 `main`）的 inner HTML。
///
/// 返回 `None` 表示文档中找不到任何 `<main>`。
pub fn extract_main(document: &str) -> Option<String> {
    let html = Html::parse_document(document);

    // 优先 `main#content`（mdBook 0.3+ 存在）；解析失败或选中为空则回退 `main`。
    let main_id = Selector::parse("main#content")
        .ok()
        .filter(|s| html.select(s).next().is_some());
    let main = main_id
        .or_else(|| Selector::parse("main").ok())
        .and_then(|s| html.select(&s).next());

    main.map(|node| node.inner_html())
}

/// 把 `<main>` 内相对资源引用改写为本地分发 URL。
///
/// 两阶段：
/// 1. 相对/书根路径（`./x.svg`、`/img/x.svg` 等，不含 `#` 锚点 / `http(s)` /
///    `data:` 等 scheme）→ 占位符 `${LOCAL_PREFIX}/{VERSION_SEGMENT}/<rel>`。
/// 2. 占位符根 → `${ctx_prefix}`（形如 [`resources::url_prefix_root`] 输出）。
///
/// `ctx_prefix` 为运行时书根上下文的完整前缀（如 [`resources::url_prefix_root`] 输出，
/// 形如 `http://127.0.0.1:PORT/books/<id>/mdor-res/v1/`），替换相对引用根后末端
/// 即衔接 `needle` 之后的资源相对路径。
#[must_use]
pub fn rewrite_links(html: &str, ctx_prefix: &str) -> String {
    let placeholder = rebase_relatives(html);
    let needle = format!("{LOCAL_PREFIX}/{VERSION_SEGMENT}/");
    placeholder.replacen(&needle, ctx_prefix, usize::MAX)
}

/// 相对引用 → 占位符根（`href`/`src` 属性内）。
fn rebase_relatives(html: &str) -> String {
    let re = Regex::new(r#"(?i)(href|src)="([^"]*)""#).expect("href/src attr 匹配正则");
    re.replace_all(html, |caps: &Captures| {
        let attr = caps.get(1).expect("attr").as_str();
        let value = caps.get(2).expect("value").as_str();
        match rebase_value(value) {
            Some(rebased) => format!(r#"{attr}="{rebased}""#),
            None => caps.get(0).expect("full").as_str().to_string(),
        }
    })
    .into_owned()
}

/// 判断属性值是否为需改写的本地相对资源引用；是则返回改写后的占位符 URL。
fn rebase_value(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    // 内部锚点、外部 scheme、协议相对引用不改写。
    if v.starts_with('#')
        || v.starts_with("http://")
        || v.starts_with("https://")
        || v.starts_with("//")
        || v.starts_with("data:")
        || v.starts_with("mailto:")
        || v.starts_with("tel:")
        || v.starts_with("javascript:")
    {
        return None;
    }
    // 绝对书根路径或相对路径 → 去掉 `./` 前导后调 local_url。
    let rel = v
        .strip_prefix('/')
        .or_else(|| v.strip_prefix("./"))
        .unwrap_or(v);
    if rel.is_empty() {
        return None;
    }
    Some(local_url(rel))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::resources::url_prefix_root;

    const CH1: &str = include_str!("../../../../fixtures/mdbook-static/ch1.html");
    const CH3: &str = include_str!("../../../../fixtures/mdbook-static/ch3.html");

    #[test]
    fn extract_main_finds_content() {
        let body = extract_main(CH1).expect("ch1 应有 <main>");
        assert!(!body.contains("<!DOCTYPE"), "应剥离文档声明");
        assert!(!body.contains("<nav"), "应剥离 nav 导航");
        assert!(body.contains("<h1"), "应含正文章节标题");
        assert!(!body.contains("<body"), "应剥离 body 容器");
    }

    #[test]
    fn extract_main_drops_outer_tag() {
        let body = extract_main(CH1).unwrap();
        assert!(!body.contains("<main"), "inner_html 不应保留 <main> 边框");
        assert!(!body.contains("</main>"), "inner_html 不应保留 </main>");
    }

    #[test]
    fn extract_main_missing_returns_none() {
        assert!(extract_main("<p>无 main</p>").is_none());
        assert!(extract_main("").is_none());
    }

    #[test]
    fn rewrite_links_replaces_img_and_chapter_href() {
        let body = extract_main(CH1).unwrap();
        let ctx = url_prefix_root(8080, "abc");
        let out = rewrite_links(&body, &ctx);
        assert!(
            out.contains(r#"src="http://127.0.0.1:8080/books/abc/mdor-res/v1/img/logo.svg""#),
            "img src 应改写为书根绝对 URL: {out}"
        );
        assert!(
            out.contains(r#"href="http://127.0.0.1:8080/books/abc/mdor-res/v1/ch2.html""#),
            "ch2 章节 href 应改写: {out}"
        );
    }

    #[test]
    fn rewrite_links_keeps_internal_anchors() {
        let body = extract_main(CH1).unwrap();
        let ctx = url_prefix_root(8080, "abc");
        let out = rewrite_links(&body, &ctx);
        assert!(out.contains("#第一章-入门"), "内部锚点应保留: {out}");
        assert!(
            !out.contains("mdor-res/v1/#第一章-入门"),
            "锚点不应被改写前置换入: {out}"
        );
    }

    #[test]
    fn rewrite_links_touches_extension_link_in_ch3() {
        let body = extract_main(CH3).unwrap();
        let ctx = url_prefix_root(8080, "abc");
        let out = rewrite_links(&body, &ctx);
        assert!(
            out.contains(r#"href="http://127.0.0.1:8080/books/abc/mdor-res/v1/css/custom.css""#),
            "css 相对引用应改写: {out}"
        );
    }

    #[test]
    fn rebase_value_skips_external_schemes() {
        assert_eq!(
            rebase_value("https://cdn.example/x.js"),
            None,
            "外部 https 不改写"
        );
        assert_eq!(rebase_value("#anchor"), None, "# 锚点不改写");
        assert_eq!(rebase_value("data:image/png;base64,AA"), None);
        assert_eq!(rebase_value(""), None);
    }

    #[test]
    fn rebase_value_local_rel() {
        assert_eq!(
            rebase_value("./img/logo.svg"),
            Some("/mdor-res/v1/img/logo.svg".to_string())
        );
        assert_eq!(
            rebase_value("/css/variables.css"),
            Some("/mdor-res/v1/css/variables.css".to_string())
        );
        assert_eq!(
            rebase_value("ch2.html"),
            Some("/mdor-res/v1/ch2.html".to_string())
        );
    }
}
