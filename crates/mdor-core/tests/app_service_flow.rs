//! AppService 门面级集成测试（M2 切片6）：add_book 接线 + 更新闭环（SD-1/SD-3）。
//!
//! httpmock 假站模拟静态 mdBook 站点：添加 → 书架出现 + §9 磁盘布局；
//! 改内容 → 更新出 v2；不改 → 跳过；阅读位置随 path 迁移（§8.1）。

mod common;

use async_trait::async_trait;
use httpmock::prelude::*;
use mdor_core::error::{Error, Result};
use mdor_core::model::position::ReadingPosition;
use mdor_core::services::AppContext;
use mdor_core::services::app_service::AppService;
use mdor_core::services::commands::Command;
use mdor_core::store::progress::Progress;
use std::collections::BTreeMap;
use tokio::sync::mpsc;

/// 哨兵命令：排在被测命令之后入队，执行即证明前者已完成（队列串行 FIFO）。
/// 代替盲 sleep/轮询等待——命令真实完成时点由通道确定性地通知测试。
struct Sentinel {
    tx: mpsc::UnboundedSender<()>,
}

#[async_trait]
impl Command for Sentinel {
    fn name(&self) -> &'static str {
        "test_sentinel"
    }

    async fn execute(&self, _ctx: &AppContext) -> Result<()> {
        self.tx
            .send(())
            .map_err(|_| Error::QueueClosed("哨兵通道已关闭".to_string()))
    }
}

/// 入队哨兵并等待其执行（命令队列完成被测命令的确定性信号）。
async fn await_queue_drained(service: &AppService) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    service
        .command_queue()
        .enqueue(Box::new(Sentinel { tx }))
        .expect("入队哨兵");
    tokio::time::timeout(std::time::Duration::from_secs(30), rx.recv())
        .await
        .expect("哨兵应在超时前执行（被测命令已完成）");
}

/// 最小两页站：index → ch1。
fn serve_v1(server: &MockServer) {
    server.mock(|when, then| {
        when.method(GET).path("/index.html");
        then.status(200)
            .header("content-type", "text/html; charset=utf-8")
            .body("<!DOCTYPE html><html><head><title>测试书</title></head><body><a href=\"ch1.html\">一</a></body></html>");
    });
    server.mock(|when, then| {
        when.method(GET).path("/ch1.html");
        then.status(200)
            .header("content-type", "text/html; charset=utf-8")
            .body("<!DOCTYPE html><html><head><title>第一章 - 测试书</title></head><body><p>正文</p></body></html>");
    });
}

#[tokio::test]
async fn add_book_end_to_end_persists_layout() {
    let server = MockServer::start();
    serve_v1(&server);

    let store_dir = common::temp_dir("svc_add");
    let service = AppService::new(store_dir.clone()).unwrap();

    let url = format!("{}/index.html", server.base_url());
    let book = service.add_book(&url).await.expect("添加成功");

    // 书架出现，元数据正确。
    assert_eq!(book.title, "测试书");
    assert_eq!(book.source_kind, mdor_core::source::SourceKind::StaticSite);
    assert_eq!(service.library().unwrap().len(), 1);

    // §9 磁盘布局：books/<id>/ 是 git 仓库，site/ 有内容，.mdor/versions/<sha>.json 落库。
    let book_root = store_dir.join("books").join(&book.id);
    assert!(book_root.join(".git").exists(), "应有 git 仓库");
    assert!(
        book_root.join("site/index.html").exists(),
        "工作区应物化 site/"
    );
    assert!(
        book_root
            .join(".mdor/versions")
            .join(format!("{}.json", book.current_version))
            .exists(),
        "版本元数据应落库"
    );

    // 重复添加拒绝。
    assert!(matches!(
        service.add_book(&url).await,
        Err(mdor_core::error::Error::AlreadyExists(_))
    ));
}

#[tokio::test]
async fn update_creates_v2_and_migrates_position() {
    let server = MockServer::start();
    serve_v1(&server);

    let store_dir = common::temp_dir("svc_update");
    let service = AppService::new(store_dir.clone()).unwrap();

    let url = format!("{}/index.html", server.base_url());
    let book = service.add_book(&url).await.expect("添加成功");
    let v1 = book.current_version.clone();

    // 预置阅读位置：在 ch1.html。
    let mut positions = BTreeMap::new();
    positions.insert(
        book.id.clone(),
        ReadingPosition {
            book_id: book.id.clone(),
            version_id: v1.clone(),
            chapter_path: "ch1.html".to_string(),
            heading_anchor: None,
            scroll_ratio: 0.3,
            saved_at: 0,
        },
    );
    // 进度文件经 AppContext 内部 store 写入（借 pub API：AppService 未暴露直写，
    // 测试用同目录重建 BookStore 写入）。
    let ctx_store = mdor_core::store::BookStore::new(store_dir.clone());
    ctx_store.progress().save(&Progress { positions }).unwrap();
    let _ = ctx_store;

    // 改 ch1 内容 → v2（reset 后重灌全部 mock，v2 版 ch1 内容不同）。
    server.reset();
    server.mock(|when, then| {
        when.method(GET).path("/index.html");
        then.status(200)
            .header("content-type", "text/html; charset=utf-8")
            .body("<!DOCTYPE html><html><head><title>测试书</title></head><body><noscript><iframe src=\"toc.html\"></iframe></noscript><a href=\"ch1.html\">一</a></body></html>");
    });
    server.mock(|when, then| {
        when.method(GET).path("/ch1.html");
        then.status(200)
            .header("content-type", "text/html; charset=utf-8")
            .body("<!DOCTYPE html><html><head><title>第一章 - 测试书</title></head><body><p>正文 v2</p></body></html>");
    });
    // toc.html（sidebar）内容不变：TOC 结构一致 → ch1 直连迁移。
    server.mock(|when, then| {
        when.method(GET).path("/toc.html");
        then.status(200).header("content-type", "text/html; charset=utf-8").body(
            "<ol class=\"chapter\"><li><span><a href=\"ch1.html\"><strong>1.</strong> 一</a></span></li></ol>",
        );
    });

    service.update_book(&book.id).expect("入队成功");
    await_queue_drained(&service).await;

    let v2 = service
        .library()
        .unwrap()
        .into_iter()
        .find(|b| b.id == book.id)
        .unwrap()
        .current_version;
    assert_ne!(v2, v1, "内容已变，更新应产生新版本");

    // 位置迁移：v2 中 ch1 仍在 → 直连；version_id 已是 v2。
    let opened = service.open_reading(&book.id).await.unwrap();
    let pos = opened.position.expect("位置应存在");
    assert_eq!(pos.version_id, v2, "位置应迁移到新版本");
    assert_eq!(pos.chapter_path, "ch1.html", "路径仍在应直连");
}

#[tokio::test]
async fn update_without_changes_is_noop() {
    let server = MockServer::start();
    serve_v1(&server);

    let store_dir = common::temp_dir("svc_noop");
    let service = AppService::new(store_dir).unwrap();

    let url = format!("{}/index.html", server.base_url());
    let book = service.add_book(&url).await.expect("添加成功");
    let v1 = book.current_version.clone();

    service.update_book(&book.id).expect("入队成功");

    // 确定性等待命令完成（内容未变 → D-08 跳过空提交，library 不变）。
    await_queue_drained(&service).await;
    let current = service
        .library()
        .unwrap()
        .into_iter()
        .find(|b| b.id == book.id)
        .unwrap()
        .current_version;
    assert_eq!(current, v1, "内容未变不应产生新版本");
}
