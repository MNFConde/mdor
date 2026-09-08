//! AppService 门面级集成测试（M3 切片2）：open_reading 渲染 + save_progress 闭环。
//!
//! 跑通 SD-2 §6.6 / §6.5 SDK-5：真实 mdBook fixture 站经 add_book 落库后，
//! open_reading 返回渲染好的正文（含 `<main>` 抽取、资源重写、内联 `<style>`）；
//! save_progress 落盘 progress.json，重开初始位置正确。薄门面（§6.9）验证。

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use httpmock::prelude::*;
use mdor_core::services::app_service::AppService;

/// 把 fixture 站点文件灌入 mock server（逐文件 mock，path 唯一无匹配歧义）。
fn serve_fixture(server: &MockServer) -> HashMap<String, Vec<u8>> {
    let root = common::mdbook_fixture_root();
    let mut served = HashMap::new();
    for entry in walk(&root) {
        let rel = entry
            .strip_prefix(&root)
            .expect("fixture 内文件")
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = std::fs::read(&entry).expect("fixture 文件可读");
        let path = format!("/{rel}");
        let body = bytes.clone();
        server.mock(|when, then| {
            when.method(GET).path(path.clone());
            then.status(200).body(body);
        });
        served.insert(path, bytes);
    }
    served
}

fn walk(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("目录可读") {
        let entry = entry.expect("目录条目");
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}

#[tokio::test]
async fn open_reading_renders_first_chapter_and_injects_css() {
    let server = MockServer::start();
    serve_fixture(&server);

    let store_dir = common::temp_dir("render_open");
    let service = AppService::new(store_dir.clone()).unwrap();

    let url = format!("{}/index.html", server.base_url());
    let book = service.add_book(&url).await.expect("添加成功");

    let opened = service.open_reading(&book.id).await.expect("打开阅读");

    // 版本元数据已落库 → TOC 章节树非空（≥3 章）。
    assert!(!opened.toc.is_empty(), "应返回 TOC 章节树");
    assert_eq!(opened.toc[0].path, "ch1.html", "首章应为 ch1.html");

    // 渲染正文：抽取 <main>（无 <!DOCTYPE>/<nav>）、含 <h1>、内联 <style>。
    assert!(
        opened
            .chapter_html
            .html
            .contains("<style data-mdor=\"inline-theme\">"),
        "应注入内联 <style>"
    );
    assert!(
        !opened.chapter_html.html.contains("<!DOCTYPE"),
        "应剥离文档声明"
    );
    assert!(opened.chapter_html.html.contains("<h1"), "应含章节标题");
    // 资源重写：img 相对引用已改为书根绝对 URL（PORT 占位 0）。
    assert!(
        opened
            .chapter_html
            .html
            .contains("/books/mdor-res/v1/img/logo.svg")
            || opened
                .chapter_html
                .html
                .contains("/books/&id;/mdor-res/v1/img/logo.svg")
            || opened
                .chapter_html
                .html
                .contains("/mdor-res/v1/img/logo.svg"),
        "img 应重写为书根前缀"
    );
    // 标题非空（取自 <h1>）。
    assert!(
        opened.chapter_html.title.contains("第一章"),
        "标题应取自 <h1>"
    );

    // 无进度 → 初始位置为空。
    assert!(opened.position.is_none());
    assert!(opened.initial_anchor.is_none());
}

#[tokio::test]
async fn save_progress_then_reopen_restores_initial_anchor() {
    let server = MockServer::start();
    serve_fixture(&server);

    let store_dir = common::temp_dir("render_progress");
    let service = AppService::new(store_dir.clone()).unwrap();

    let url = format!("{}/index.html", server.base_url());
    let book = service.add_book(&url).await.expect("添加成功");

    // 保存进度：定位到 ch1 的标题锚点 + 滚动比例。
    service
        .save_progress(&book.id, "ch1.html", Some("第一章-入门".to_string()), 0.42)
        .expect("保存成功");

    // progress.json 落盘验证。
    let progress_path = store_dir.join("progress.json");
    assert!(progress_path.exists(), "progress.json 应落盘");

    // 重开：初始位置应恢复（anchor + 滚动比例）。
    let opened = service.open_reading(&book.id).await.expect("重开成功");
    let pos = opened.position.expect("位置应存在");
    assert_eq!(pos.chapter_path, "ch1.html");
    assert_eq!(pos.heading_anchor.as_deref(), Some("第一章-入门"));
    assert_eq!(pos.scroll_ratio, 0.42);
    assert_eq!(opened.initial_anchor.as_deref(), Some("第一章-入门"));
    assert_eq!(opened.scroll_ratio, 0.42);
}
