//! 静态资源 URL ↔ 相对路径双向映射（§6.5 RenderService / D-04）。
//!
//! 纯函数模块，无 socket / 无 IO。职责二：
//! 1. 把章节内 `href`/`src` 的相对资源引用，统一改写为本地离线前缀
//!    `LOCAL_PREFIX/v1/<rel>`（占位 absolute 前缀由调用方注入，见 [`build_ctx`]）。
//! 2. 把来自 WebView 的资源请求（`/mdor-res/...` 形态）解析回书内相对路径，
//!    并做目录穿越拦截（越界返回 `None`，B5）。

use std::path::{Component, Path, PathBuf};

/// 本地资源分发前缀（固定，不带端口；`/v1/` 为版本段占位）。
pub const LOCAL_PREFIX: &str = "/mdor-res";

/// 版本占位段（当前单版本分发，`/v1/` 预留切换多版本窗口）。
pub const VERSION_SEGMENT: &str = "v1";

/// 生成带书 id 的上下文 URL 前缀（最终形态，§6.5 PORT 注入链）。
///
/// 返回如 `http://127.0.0.1:{port}/books/{book_id}/`。early Core 阶段调用方可
/// 传入虚端口 `0`，替换范围（`/mdor-res/v1/`）不变。
#[must_use]
pub fn build_ctx(port: u16, book_id: &str) -> String {
    format!("http://127.0.0.1:{port}/books/{book_id}/")
}

/// 把书内相对路径格式化为本地分发 URL（`/mdor-res/v1/<rel>`）。
#[must_use]
pub fn local_url(rel: &str) -> String {
    let rel = rel.trim_start_matches('/');
    format!("{LOCAL_PREFIX}/{VERSION_SEGMENT}/{rel}")
}

/// 拼接书根上下文前缀（不含重复斜杠）。
///
/// 章节 HTML 中已把资源根改写为 `/mdor-res/v1/`，真实运行时需把该占位根
/// 替换为「宿主机地址 + 书 id」的完整前缀，见 [`rewrite_links`]。
#[must_use]
pub fn url_prefix_root(port: u16, book_id: &str) -> String {
    // build_ctx 已带有结尾 `/books/<id>/`；LOCAL_PREFIX 以 `/` 开头，合并去重。
    format!(
        "{}{LOCAL_PREFIX}/{VERSION_SEGMENT}/",
        build_ctx(port, book_id).trim_end_matches('/')
    )
}

/// 解析本地分发 URL（`/mdor-res/v1/<path>` 等形态）→ 规范化相对路径。
///
/// 仅识别带 [`LOCAL_PREFIX`] 的 URL；剥掉前缀与版本段后，剔除 `?query` /
/// `#fragment`，解析 `.`/`..`/空段；任何 `..` 上跳（越出书根白名单）即返回
/// `None`（目录穿越拦截，B5）。非本地分发 URL（无前缀）返回 `None`。
pub fn resolve_local_url(url: &str) -> Option<String> {
    let tail = strip_prefix(url)?;
    // 剔除 `?query` / `#fragment`。
    let end = tail
        .find('?')
        .or_else(|| tail.find('#'))
        .unwrap_or(tail.len());
    let path = &tail[..end];
    let rel = normalize_segments(path)?;
    let rel = rel.trim_start_matches('/').trim_end_matches('/');
    if rel.is_empty() {
        return None;
    }
    Some(rel.to_string())
}

/// 从 URL 中剥离预置前缀，返回资源相对路径（如 `img/logo.svg`）。
///
/// 若 URL 不含 [`LOCAL_PREFIX`] 或其后无有效资源段，返回 `None`。
fn strip_prefix(url: &str) -> Option<&str> {
    let idx = url.find(LOCAL_PREFIX)?;
    let tail = &url[idx + LOCAL_PREFIX.len()..];
    let tail = tail.trim_start_matches('/');
    // 跳过版本段（`v1/` 或 `v1`），资源路径紧随其后。
    let tail = tail.strip_prefix(VERSION_SEGMENT).unwrap_or(tail);
    let tail = tail.trim_start_matches('/');
    Some(tail)
}

/// 拆段规范化（解析 `.`/`..`/空段，返回拼接后的相对路径）。
///
/// 任一段 `..` 无法收敛（匹配不到可弹出的常规段，即越出书根白名单）返回
/// `None`（目录穿越拦截，B5）。返回的段序列均为普通段（无 `.`/`..`）。
fn normalize_segments(path: &str) -> Option<String> {
    let mut stack: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg.trim() {
            "" | "." => continue,
            ".." => {
                if let Some(popped) = stack.pop() {
                    debug_assert_ne!(popped, "..");
                    continue;
                }
                // 栈空仍上跳：越界。
                return None;
            }
            s => stack.push(s),
        }
    }
    if stack.is_empty() {
        return None;
    }
    Some(stack.join("/"))
}

/// 把相对路径拆解为「目录穿越白名单校验」：仅允许落在书根内。
///
/// 供上层把解析出的相对路径映射回磁盘前做最终确认。绝对路径（`/` 或 `C:/`
/// 起始）或任何前导 `..` 上跳且 `allow_escape` 为 `false` 时返回 `None`。
#[must_use]
pub fn sanitize_relative(rel: &str, allow_escape: bool) -> Option<PathBuf> {
    let rel = rel.trim_end_matches('/');
    if rel.starts_with('/') || rel.contains(":\\") || rel.contains(":") {
        return None;
    }
    let mut out = PathBuf::new();
    let mut depth: i32 = 0;
    for comp in Path::new(rel).components() {
        match comp {
            Component::Normal(seg) => {
                out.push(seg);
                depth += 1;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if depth <= 0 {
                    if allow_escape {
                        return Some(PathBuf::from(".."));
                    }
                    return None;
                }
                depth -= 1;
                if !out.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if out.as_os_str().is_empty() {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_ctx_formats_host_and_book() {
        assert_eq!(build_ctx(8080, "abc"), "http://127.0.0.1:8080/books/abc/");
        assert_eq!(build_ctx(0, "abc"), "http://127.0.0.1:0/books/abc/");
    }

    #[test]
    fn local_url_prefixes_rel() {
        assert_eq!(local_url("img/logo.svg"), "/mdor-res/v1/img/logo.svg");
        assert_eq!(local_url("/ch2.html"), "/mdor-res/v1/ch2.html");
        assert_eq!(local_url(""), "/mdor-res/v1/");
    }

    #[test]
    fn url_prefix_root_concatenates() {
        assert_eq!(
            url_prefix_root(8080, "abc"),
            "http://127.0.0.1:8080/books/abc/mdor-res/v1/"
        );
    }

    #[test]
    fn resolve_strips_host_and_prefix() {
        assert_eq!(
            resolve_local_url("http://127.0.0.1:0/mdor-res/v1/img/logo.svg"),
            Some("img/logo.svg".to_string())
        );
        assert_eq!(
            resolve_local_url("/mdor-res/v1/css/variables.css"),
            Some("css/variables.css".to_string())
        );
    }

    #[test]
    fn resolve_strips_query_and_fragment() {
        assert_eq!(
            resolve_local_url("/mdor-res/v1/ch2.html?v=1#section"),
            Some("ch2.html".to_string())
        );
        assert_eq!(
            resolve_local_url("/mdor-res/v1/ch1.html#第一章-入门"),
            Some("ch1.html".to_string())
        );
    }

    #[test]
    fn resolve_traversal_rejected() {
        assert_eq!(resolve_local_url("/mdor-res/v1/../secret.html"), None);
        assert_eq!(resolve_local_url("/mdor-res/v1/a/../../secret.html"), None);
    }

    #[test]
    fn resolve_dot_segments_normalized() {
        assert_eq!(
            resolve_local_url("/mdor-res/v1/img/./logo.svg"),
            Some("img/logo.svg".to_string())
        );
    }

    #[test]
    fn resolve_empty_returns_none() {
        assert_eq!(resolve_local_url("/mdor-res/v1/"), None);
        assert_eq!(resolve_local_url("/mdor-res/v1"), None);
    }

    #[test]
    fn resolve_ignores_non_prefix_url() {
        assert_eq!(resolve_local_url("/assets/app.js"), None);
        assert_eq!(resolve_local_url("http://cdn/x.js"), None);
    }

    #[test]
    fn sanitize_relative_rejects_escape() {
        assert_eq!(sanitize_relative("../../secret", false), None);
        assert_eq!(sanitize_relative("a/../../b", false), None);
        assert_eq!(sanitize_relative("a/../b", false), Some(PathBuf::from("b")));
    }

    #[test]
    fn sanitize_relative_allow_escape_with_leading_dotdot() {
        assert_eq!(
            sanitize_relative("../outside", true),
            Some(PathBuf::from(".."))
        );
    }

    #[test]
    fn sanitize_relative_rejects_absolute() {
        assert_eq!(sanitize_relative("/etc/passwd", false), None);
        assert_eq!(sanitize_relative("C:/win", false), None);
    }
}
