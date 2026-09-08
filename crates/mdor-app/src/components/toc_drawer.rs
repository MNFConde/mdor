use dioxus::desktop::use_window;
use dioxus::prelude::*;

use mdor_core::model::toc::TocEntry;

use crate::screens::reader::switch_chapter;

/// 全局目录抽屉开关信号。
pub static DRAWER: GlobalSignal<bool> = GlobalSignal::new(|| false);

/// 在当前组件作用域内切换抽屉开关（供阅读屏按钮触发）。
pub fn toggle_drawer() {
    DRAWER.with_mut(|d| *d = !*d);
}

/// 目录抽屉：Dioxus 自绘 TOC 树 + 子章节锚点跳转。
///
/// - `toc`：`open_reading` 返回的章节树。
/// - `anchors`：当前章节内 `id` 锚点（子章节内标题跳转）。
/// - `current`：当前章节路径（高亮）。
#[component]
pub fn TocDrawer(
    book_id: String,
    toc: Vec<TocEntry>,
    anchors: Vec<String>,
    current: Option<String>,
) -> Element {
    let drawer_open = DRAWER.read();
    if !*drawer_open {
        return rsx! {};
    }

    rsx! {
        div {
            style {
                "position:fixed; top:0; left:0; bottom:0; width:280px; max-width:80vw; \
                 background:#fff; border-right:1px solid #ddd; overflow-y:auto; z-index:1000; \
                 padding:8px; box-shadow: 2px 0 8px rgba(0,0,0,0.1);"
            }
            button {
                onclick: move |_| DRAWER.with_mut(|d| *d = false),
                "关闭"
            }
            // 章节树
            for entry in &toc {
                TocNode {
                    entry: entry.clone(),
                    current: current.clone(),
                    book_id: book_id.clone(),
                }
            }
            // 页内锚点（当前章节内的标题）
            if !anchors.is_empty() {
                p { style { "margin-top:12px; font-weight:bold;" } "本章锚点" }
                for a in anchors.clone() {
                    AnchorItem { anchor: a }
                }
            }
        }
        // 遮罩：点击关闭抽屉
        div {
            onclick: move |_| DRAWER.with_mut(|d| *d = false),
            style { "position:fixed; inset:0; background:rgba(0,0,0,0.3); z-index:999;" }
        }
    }
}

/// 单个页内锚点条目：点击时经 WebView 滚动到指定 id 元素，并关闭抽屉。
#[component]
fn AnchorItem(anchor: String) -> Element {
    let window = use_window();
    let anchor = anchor.clone();
    rsx! {
        div {
            onclick: move |_| {
                let _ = window.webview.evaluate_script(&format!(
                    "var el=document.getElementById({:?}); if(el){{el.scrollIntoView();}}",
                    anchor
                ));
                DRAWER.with_mut(|d| *d = false);
            },
            style { "padding:4px 8px; color:#0366d6; cursor:pointer;" }
            "#{anchor}"
        }
    }
}

/// 单个章节节点：有子章节则递归渲染。
#[component]
fn TocNode(entry: TocEntry, current: Option<String>, book_id: String) -> Element {
    let is_current = current.as_deref() == Some(entry.path.as_str());
    let active = if is_current {
        "font-weight:bold; color:#0366d6;"
    } else {
        ""
    };
    let click = {
        let entry = entry.clone();
        let book_id = book_id.clone();
        move |_| {
            DRAWER.with_mut(|d| *d = false);
            switch_chapter(book_id.clone(), entry.path.clone());
        }
    };

    rsx! {
        div {
            style { "padding:4px 0;" }
            div {
                onclick: click,
                style { "cursor:pointer; {active}" }
                "{entry.title}"
            }
            if !entry.children.is_empty() {
                div {
                    style { "padding-left:14px;" }
                    for child in &entry.children {
                        TocNode {
                            entry: child.clone(),
                            current: current.clone(),
                            book_id: book_id.clone(),
                        }
                    }
                }
            }
        }
    }
}
