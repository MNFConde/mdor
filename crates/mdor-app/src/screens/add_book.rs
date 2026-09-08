use dioxus::prelude::*;

use crate::state::{AppRoute, NAV};

/// 添加书籍屏：URL 输入 → `add_book`（URL 探测/校验仍在 core）。
#[component]
pub fn AddBookScreen() -> Element {
    let mut url = use_signal(String::new);
    let busy = use_signal(|| false);
    let message = use_signal(String::new);

    let app = match crate::state::APP.get() {
        Some(app) => app,
        None => return rsx! { p { "正在初始化…" } },
    };

    let do_add = {
        let app = app.clone();
        move |_: dioxus::events::MouseEvent| {
            let target = url.read().trim().to_string();
            let mut message = message;
            let mut busy = busy;
            if target.is_empty() {
                message.set("请输入书籍 URL".to_string());
                return;
            }
            let app = app.clone();
            busy.set(true);
            spawn(async move {
                match app.add_book(&target).await {
                    Ok(book) => {
                        message.set(format!("添加成功：{}", book.title));
                        busy.set(false);
                        // 返回书架查看新书。
                        NAV.with_mut(|n| *n = AppRoute::Library);
                    }
                    Err(e) => {
                        message.set(format!("添加失败：{e}"));
                        busy.set(false);
                    }
                }
            });
        }
    };

    rsx! {
        div {
            h1 { "添加书籍" }
            input {
                placeholder: "输入书籍 URL（https://...）",
                oninput: move |e| url.set(e.value().clone()),
            }
            button {
                disabled: *busy.read(),
                onclick: do_add,
                if *busy.read() { "添加中…" } else { "添加" }
            }
            button {
                onclick: move |_| NAV.with_mut(|n| *n = AppRoute::Library),
                "返回书架"
            }
            p { "{message}" }
        }
    }
}
