use std::path::PathBuf;
use std::sync::Arc;

use dioxus::prelude::*;

use mdor_core::services::app_service::AppService;

use crate::state::APP;

mod state;

/// 桌面数据根（D-13）：exe 同目录 `data/`（便携式）；不存在则 create_dir_all，
/// 不可写直接报错、不回退到系统用户目录。Android 分支（getFilesDir）M6 补。
fn data_root() -> PathBuf {
    let exe = std::env::current_exe().expect("解析当前可执行文件路径");
    exe.parent().expect("可执行文件路径含父目录").join("data")
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    tracing::info!("mdor 启动");

    // 命令队列消费者需要 tokio 运行时上下文
    let runtime = tokio::runtime::Runtime::new().expect("启动 tokio 运行时");
    let _guard = runtime.enter();

    let base_dir = data_root().join("bookstore");
    tracing::info!(base = %base_dir.display(), "初始化数据目录");
    let app = Arc::new(AppService::new(base_dir).expect("初始化 AppService"));
    if APP.set(app).is_err() {
        tracing::error!("APP 重复初始化，忽略本次设置");
    }

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        Library {}
    }
}

/// 书架屏：经 AppService 渲染 library.json 真实数据（切片5 验收）。
#[component]
fn Library() -> Element {
    let app = match APP.get() {
        Some(app) => app,
        None => return rsx! { p { "正在初始化…" } },
    };
    let books = match app.library() {
        Ok(books) => books,
        Err(e) => return rsx! { p { "书架加载失败：{e}" } },
    };

    rsx! {
        div {
            h1 { "书架" }
            p { "mdor — 移动端 mdBook 离线阅读器" }
            if books.is_empty() {
                p { "书架空空 — 添加书籍功能 M2 开放" }
            } else {
                for book in &books {
                    div {
                        key: "{book.id}",
                        h2 { "{book.title}" }
                        p { "{book.url}" }
                        p { "版本 {book.current_version}" }
                    }
                }
            }
        }
    }
}
