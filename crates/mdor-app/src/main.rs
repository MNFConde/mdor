use std::path::PathBuf;
use std::sync::Arc;

use mdor_core::services::app_service::AppService;

use crate::app::App;
use crate::local_http::LocalHttpServer;
use crate::state::APP;

mod app;
mod components;
mod local_http;
mod screens;
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

    // 本地资源分发服务器：动态端口 + 独立线程（切片3，D-04 / SD-5）。
    // 绑定失败即降级为不启用资源服务（app 仍可启动，核心功能不受阻）。
    let http_server = LocalHttpServer::bind(base_dir.join("books")).ok();
    let port = match &http_server {
        Some(s) => s.port,
        None => {
            tracing::warn!("本地资源服务器绑定失败，降级为不启用（port=0）");
            0
        }
    };

    let app = Arc::new(AppService::with_port(base_dir, port).expect("初始化 AppService"));
    if APP.set(app).is_err() {
        tracing::error!("APP 重复初始化，忽略本次设置");
    }

    dioxus::launch(App);
}
