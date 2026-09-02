use dioxus::prelude::*;

use crate::screens::reader::open_book;
use crate::state::{AppRoute, NAV};

/// 书架屏：经 AppService 渲染 library.json 真实数据（切片5 验收）。
#[component]
pub fn LibraryScreen() -> Element {
    let app = match crate::state::APP.get() {
        Some(app) => app,
        None => return rsx! { p { "正在初始化…" } },
    };
    let books = match app.library() {
        Ok(books) => books,
        Err(e) => return rsx! { p { "书架加载失败：{e}" } },
    };

    // 预物化书籍元数据为 owned 元组，避免 rsx 闭包借用 `books` 导致 'static 不满足。
    let rows: Vec<(String, String, String, String)> = books
        .iter()
        .map(|b| {
            (
                b.id.clone(),
                b.title.clone(),
                b.url.clone(),
                b.current_version.clone(),
            )
        })
        .collect();

    rsx! {
        div {
            h1 { "书架" }
            p { "mdor — 移动端 mdBook 离线阅读器" }
            button {
                onclick: move |_| NAV.with_mut(|n| *n = AppRoute::AddBook),
                "添加书籍"
            }
            if rows.is_empty() {
                p { "书架空空 — 添加书籍功能 M2 开放" }
            } else {
                for (bid, title, url, version) in rows {
                    div {
                        key: "{bid}",
                        h2 { "{title}" }
                        p { "{url}" }
                        p { "版本 {version}" }
                        button {
                            onclick: {
                                let bid = bid.clone();
                                move |_| {
                                    open_book(bid.clone());
                                    NAV.with_mut(|n| *n = AppRoute::Reader);
                                }
                            },
                            "阅读"
                        }
                    }
                }
            }
        }
    }
}
