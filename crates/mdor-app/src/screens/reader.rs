use dioxus::prelude::*;

use crate::components::toc_drawer::TocDrawer;
use crate::state::{AppRoute, NAV, READER};

/// 计算章节内滚动比例（0.0~1.0），供进度保存与恢复。
///
/// 分母为 `scroll_height - client_height`（可滚动差值）；不可滚动（差值 ≤ 0）返回 0。
/// 结果 clamp 到 0.0~1.0，避免浮点边界溢出。
#[must_use]
pub fn scroll_ratio(scroll_top: f64, scroll_height: f64, client_height: f64) -> f32 {
    let max = scroll_height - client_height;
    if max <= 0.0 {
        return 0.0;
    }
    ((scroll_top / max).clamp(0.0, 1.0)) as f32
}

/// 打开一本书：spawn 异步调用 `open_reading`，成功后写入 [`READER`] 并切到阅读路由。
///
/// 供书架「阅读」按钮与切章/返回复用；失败时仅打日志（UI 停留原屏）。
pub fn open_book(book_id: String) {
    let Some(app) = crate::state::APP.get() else {
        tracing::warn!("AppService 未初始化，无法打开书籍");
        return;
    };
    let app = app.clone();
    READER.with_mut(|r| {
        r.book_id = Some(book_id.clone());
        r.open = None;
        r.chapter_path = None;
        r.initial_anchor = None;
        r.scroll_ratio = 0.0;
    });
    NAV.with_mut(|n| *n = AppRoute::Reader);

    spawn(async move {
        match app.open_reading(&book_id).await {
            Ok(opened) => {
                let chapter = opened.chapter_html.title.clone();
                READER.with_mut(|r| {
                    r.book_id = Some(book_id.clone());
                    r.open = Some(opened.clone());
                    r.chapter_path = opened.position.as_ref().map(|p| p.chapter_path.clone());
                    r.initial_anchor = opened.initial_anchor.clone();
                    r.scroll_ratio = opened.scroll_ratio;
                });
                tracing::info!(book_id, chapter, "open_reading 渲染完成");
            }
            Err(e) => tracing::warn!(book_id, error = %e, "open_reading 失败"),
        }
    });
}

/// 切章：保存当前进度后加载目标章节（目录抽屉 / 上一页下一页）。
pub fn switch_chapter(book_id: String, chapter_path: String) {
    let Some(app) = crate::state::APP.get() else {
        return;
    };
    let app = app.clone();
    let prev = READER.with(|r| {
        (
            r.scroll_ratio,
            r.initial_anchor.clone(),
            r.chapter_path.clone(),
        )
    });
    if prev.0 > 0.0
        && let Some(current_path) = prev.2
    {
        let _ = app.save_progress(&book_id, &current_path, prev.1, prev.0);
    }

    spawn(async move {
        match app.render_chapter(&book_id, &chapter_path) {
            Ok(rendered) => {
                READER.with_mut(|r| {
                    if let Some(open) = r.open.as_mut() {
                        open.chapter_html = rendered;
                    }
                    r.chapter_path = Some(chapter_path.clone());
                    r.initial_anchor = None;
                    r.scroll_ratio = 0.0;
                });
            }
            Err(e) => {
                tracing::warn!(book_id, chapter = %chapter_path, error = %e, "切章渲染失败")
            }
        }
    });
}

/// 阅读屏：注入 `dangerous_inner_html` + 滚动节流存进度 + TOC 抽屉。
#[component]
pub fn ReaderScreen() -> Element {
    let reader = READER.read();
    let Some(book_id) = reader.book_id.clone() else {
        return rsx! {
            div {
                p { "未打开书籍" }
                button { onclick: move |_| NAV.with_mut(|n| *n = AppRoute::Library), "返回书架" }
            }
        };
    };

    let html = reader.chapter_html().to_string();
    let title = reader.chapter_title().to_string();
    let toc = reader.toc().to_vec();
    let anchors = reader.anchors().to_vec();
    let current = reader.chapter_path.clone();

    // 安装一次正文链接拦截（切章/正文重建不重复安装；依赖 book_id 定位书籍）。
    // `document::eval` 必须在组件作用域内调用（desktop 的 Document provider 挂在根
    // scope，spawn 出的任务可能拿不到 no-op 文档），故在 use_effect 里创建 Eval，
    // 再 spawn 长活 recv 循环消费切章消息。
    let intercept_book_id = book_id.clone();
    use_effect(move || {
        let script = r#"
            const mark = "/mdor-res/v1/";
            if (window.__mdor_listener_installed) return;
            window.__mdor_listener_installed = true;
            document.addEventListener("click", (ev) => {
                const a = ev.target && ev.target.closest && ev.target.closest("a[href]");
                if (!a) return;
                const href = a.getAttribute("href") || "";
                const idx = href.indexOf(mark);
                if (idx < 0) return;
                const path = href.slice(idx + mark.length);
                if (!path || path.includes("..")) return;
                ev.preventDefault();
                dioxus.send(path);
            }, true);
        "#;
        let mut eval = document::eval(script);
        let book = intercept_book_id.clone();
        spawn(async move {
            // 长活循环：读到章节路径即切章；通道关闭（渲染器切换/组件卸载）则退出。
            while let Ok(path) = eval.recv::<String>().await {
                switch_chapter(book.clone(), path);
            }
        });
    });

    rsx! {
        div {
            style { "display: flex; flex-direction: column; height: 100vh; margin:0; padding:0;" }
            // 顶部栏：返回 / 标题 / 目录抽屉按钮
            header {
                style { "display:flex; align-items:center; gap:8px; padding:8px; border-bottom:1px solid #ddd;" }
                button {
                    onclick: move |_| NAV.with_mut(|n| *n = AppRoute::Library),
                    "← 返回"
                }
                button {
                    onclick: move |_| crate::components::toc_drawer::toggle_drawer(),
                    "目录"
                }
                h2 { "{title}" }
            }
            // 正文可滚动区
            div {
                style { "flex:1; overflow-y:auto; padding:16px; " }
                div {
                    dangerous_inner_html: html,
                    onscroll: move |e| {
                        let ratio = scroll_ratio(e.scroll_top(), e.scroll_height() as f64, e.client_height() as f64);
                        READER.with_mut(|r| {
                            r.scroll_ratio = ratio;
                            r.initial_anchor = None;
                        });
                    },
                }
            }
            TocDrawer {
                book_id: book_id.clone(),
                toc: toc,
                anchors: anchors,
                current: current,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::scroll_ratio;

    #[test]
    fn scroll_ratio_normalizes() {
        assert_eq!(scroll_ratio(100.0, 1100.0, 100.0), 0.1);
        assert_eq!(scroll_ratio(0.0, 1100.0, 100.0), 0.0);
    }

    #[test]
    fn scroll_ratio_clamps_and_guards() {
        // 超出范围 clamp 到 1.0
        assert_eq!(scroll_ratio(5000.0, 1100.0, 100.0), 1.0);
        // 不可滚动（差值 ≤ 0）
        assert_eq!(scroll_ratio(0.0, 100.0, 100.0), 0.0);
        assert_eq!(scroll_ratio(10.0, 100.0, 200.0), 0.0);
    }
}
