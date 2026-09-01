//! 阅读渲染管线（§6.5 RenderService）。
//!
//! 把章节原始 HTML 加工为可注入 Dioxus `dangerous_inner_html` 的片段：
//! 抽取 `<main>` → 资源引用重写为书根前缀 → 内联书籍 CSS（[D-06 方案 1](doc/decisions.md#方案-1-includebytes-内嵌)）。

pub mod html_extract;
pub mod resources;

use crate::error::{Error, Result};

use html_extract::{extract_main, rewrite_links};
use resources::LOCAL_PREFIX;

/// 阅读页样式表内嵌集合（D-06 方案 1：include_bytes!/include_str! 编入发布二进制）。
///
/// 覆盖 mdBook 默认输出所需：变量/通用/布局/打印；字体与代码高亮同理。渲染时
/// 合并为一个 `<style>` 块注入。
const BOOK_CSS: &[&str] = &[
    include_str!("../../../../fixtures/mdbook-static/css/variables-8adf115d.css"),
    include_str!("../../../../fixtures/mdbook-static/css/general-e96d0476.css"),
    include_str!("../../../../fixtures/mdbook-static/css/chrome-d279d366.css"),
    include_str!("../../../../fixtures/mdbook-static/css/print-9e4910d8.css"),
    include_str!("../../../../fixtures/mdbook-static/fonts/fonts-9644e21d.css"),
    include_str!("../../../../fixtures/mdbook-static/highlight-493f70e1.css"),
];

/// 渲染结果（§6.5 / §6.6 `dangerous_inner_html` 注入源）。
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedChapter {
    /// 已渲染的正文 HTML（`<main>` 抽取 + 资源重写 + `<style>` 内联）。
    pub html: String,
    /// 章节标题（取自 `<h1>` 首个文本；无则空串）。
    pub title: String,
    /// 章节内所有 `id` 锚点（供跳转定位，§6.6 `heading_anchor`）。
    pub anchors: Vec<String>,
}

impl RenderedChapter {
    /// 空渲染结果（无可用章节时的降级占位，门面不致命）。
    #[must_use]
    pub fn empty() -> Self {
        Self {
            html: String::new(),
            title: String::new(),
            anchors: Vec::new(),
        }
    }
}

/// 渲染单章正文（§6.5 StaticSite 路径）。
///
/// 输入为章节原始 HTML 字节；`ctx_prefix` 为书中根 URL 前缀
/// （见 [`resources::url_prefix_root`]），调用方传入运行时 PORT 与 book_id。
/// 处理流程：UTF-8 解码 → `extract_main`（缺失返回 `Unsupported`）→ `rewrite_links`
/// → 注入内联 `<style>`。
pub fn render_chapter(html: &[u8], ctx_prefix: &str) -> Result<RenderedChapter> {
    let raw = String::from_utf8_lossy(html);
    let main = extract_main(&raw).ok_or_else(|| Error::Unsupported(html_extract::MISSING_MAIN))?;
    let body = rewrite_links(&main, ctx_prefix);
    let title = extract_title(&body);
    let anchors = extract_anchors(&body);

    let styled = inject_css(&body);
    Ok(RenderedChapter {
        html: styled,
        title,
        anchors,
    })
}

/// 抽取首个 `<h1>` 的文本标题。
fn extract_title(body: &str) -> String {
    let Some(first) = body.find("<h1") else {
        return String::new();
    };
    let Some(end) = body[first..].find("</h1>") else {
        return String::new();
    };
    let slice = &body[first..first + end];
    // 去掉标签后余文本（含 `<a>` 内文本），再折叠空白。
    let mut out = String::new();
    let mut in_tag = false;
    for ch in slice.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// 抽取正文内所有 `id` 锚点（去重、保序）。
fn extract_anchors(body: &str) -> Vec<String> {
    let re = regex::Regex::new(r#"id="([^"]+)""#).expect("id 匹配正则");
    let mut out = Vec::new();
    for caps in re.captures_iter(body) {
        let id = caps.get(1).expect("id").as_str().to_string();
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// 把内联 CSS 合并为一个 `<style>` 块（D-06 方案 1）。
fn inject_css(body: &str) -> String {
    let css = BOOK_CSS.join("\n");
    format!(
        r#"<style data-mdor="inline-theme">{css}</style><main id="mdor-content" data-mdor-prefix="{LOCAL_PREFIX}">{body}</main>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::resources::url_prefix_root;

    const CH1: &str = include_str!("../../../../fixtures/mdbook-static/ch1.html");

    #[test]
    fn render_chapter_strips_page_chrome_and_injects_style() {
        let ctx = url_prefix_root(8080, "abc");
        let rendered = render_chapter(CH1.as_bytes(), &ctx).expect("渲染成功");
        assert!(
            rendered.html.contains("<style data-mdor=\"inline-theme\">"),
            "应注入内联 <style>"
        );
        assert!(!rendered.html.contains("<!DOCTYPE"), "应剥离文档声明");
        assert!(
            rendered
                .html
                .contains(r#"src="http://127.0.0.1:8080/books/abc/mdor-res/v1/img/logo.svg""#),
            "img 应改写为书根绝对 URL"
        );
        assert!(rendered.html.contains("<h1"), "应含章节标题");
    }

    #[test]
    fn render_chapter_missing_main_returns_unsupported() {
        let err = render_chapter(b"<p>no main</p>", "http://x/").unwrap_err();
        assert!(
            matches!(err, Error::Unsupported(_)),
            "应为 Unsupported: {err}"
        );
    }

    #[test]
    fn extract_title_gets_h1_text() {
        let body = extract_main(CH1).unwrap();
        let title = extract_title(&body);
        assert!(title.contains("第一章"), "标题应取自 <h1> 文本: {title:?}");
    }

    #[test]
    fn extract_anchors_finds_ids() {
        let body = extract_main(CH1).unwrap();
        let anchors = extract_anchors(&body);
        assert!(
            anchors.iter().any(|a| a == "第一章-入门"),
            "应含章节锚点: {anchors:?}"
        );
    }
}
