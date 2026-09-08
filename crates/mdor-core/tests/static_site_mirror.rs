//! StaticSiteSource 递归镜像集成测试（M2 切片2）：httpmock 假站，不依赖真实网络。

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use httpmock::prelude::*;
use mdor_core::source::static_site::{FileSet, MirrorOptions, mirror_site};
use mdor_core::source::{SourceAdapter, SourceKind, StaticSiteSource};

/// 把 fixture 站点文件灌入 mock server：返回 (path → bytes) 清单。
///
/// 相对路径（fixture 根下）即 URL path（site 部署在服务器根）。
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
async fn mirrors_fixture_site_end_to_end() {
    let server = MockServer::start();
    let served = serve_fixture(&server);

    let entry = format!("{}/index.html", server.base_url());
    let files = mirror_site(&entry, &MirrorOptions::from_entry(&entry).unwrap())
        .await
        .expect("镜像成功");

    // 断言分两层：
    // ① 全部 HTML 页面（mdBook 页面从 index.html 导航可达）+ css/img/woff2/js
    //    资源（css url() / html src|href 引用可达）必须入集；
    // ② 顶层杂项与仅由 JS 变量引用的文件（404.html、toc.html、searchindex-*.js ——
    //    经 window.path_to_searchindex_js 动态加载，非静态 src/href）不作要求。
    // 字节保真（D-09：工作区字节 ≡ 上游字节）对所有入集文件断言。
    let must_have = served
        .keys()
        .filter(|p| {
            let p = p.as_str();
            p.ends_with(".html") && p != "/404.html" && p != "/toc.html"
                || p.contains("/css/")
                || p.contains("/img/")
                || p.ends_with(".css")
                || (p.ends_with(".js") && !p.contains("searchindex"))
                || (p.contains("/fonts/") && (p.ends_with(".woff2") || p.ends_with(".css")))
        })
        .cloned()
        .collect::<Vec<_>>();
    assert!(!must_have.is_empty(), "fixture 应含页面与资源");
    for path in &must_have {
        let rel = path.trim_start_matches('/');
        assert!(
            files.contains_key(std::path::Path::new(rel)),
            "可达文件 {rel} 应在镜像集内"
        );
    }
    for (path, bytes) in &served {
        let rel = path.trim_start_matches('/');
        if let Some(actual) = files.get(std::path::Path::new(rel)) {
            assert_eq!(actual, bytes, "{rel} 字节应与 fixture 一致");
        }
    }
}

#[tokio::test]
async fn mirror_respects_path_prefix_boundary() {
    let server = MockServer::start();
    // /book/ 内两页 + 前缀外一页（不应被抓）。
    let pages = [
        ("/book/index.html", "<a href=\"ch1.html\">next</a>"),
        ("/book/ch1.html", "<p>chapter 1</p>"),
        ("/std/index.html", "<p>std docs（越界）</p>"),
    ];
    for (path, body) in pages {
        server.mock(|when, then| {
            when.method(GET).path(path);
            then.status(200).body(body);
        });
    }

    let entry = format!("{}/book/index.html", server.base_url());
    let files = mirror_site(&entry, &MirrorOptions::from_entry(&entry).unwrap())
        .await
        .expect("镜像成功");

    assert!(files.contains_key(std::path::Path::new("book/index.html")));
    assert!(files.contains_key(std::path::Path::new("book/ch1.html")));
    assert!(
        !files.contains_key(std::path::Path::new("std/index.html")),
        "前缀外路径不得被抓取（26-08-31 定案硬边界）"
    );
}

#[tokio::test]
async fn mirror_rejects_oversized_file() {
    let server = MockServer::start();
    let big = vec![b'x'; 2048];
    server.mock(|when, then| {
        when.method(GET).path("/big.html");
        then.status(200).body(big);
    });

    let entry = format!("{}/big.html", server.base_url());
    let mut opts = MirrorOptions::from_entry(&entry).unwrap();
    opts.max_file_bytes = 1024;
    let err = mirror_site(&entry, &opts).await.unwrap_err();
    assert!(
        matches!(
            err,
            mdor_core::error::Error::MirrorLimit {
                limit: "file_bytes",
                ..
            }
        ),
        "应报单文件超限：{err}"
    );
}

#[tokio::test]
async fn mirror_rejects_file_count_overflow() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/index.html");
        then.status(200)
            .body("<a href=\"p1.html\">1</a><a href=\"p2.html\">2</a>");
    });
    for p in ["/p1.html", "/p2.html"] {
        server.mock(move |when, then| {
            when.method(GET).path(p);
            then.status(200).body("<p>x</p>");
        });
    }

    let entry = format!("{}/index.html", server.base_url());
    let mut opts = MirrorOptions::from_entry(&entry).unwrap();
    opts.max_files = 2;
    let err = mirror_site(&entry, &opts).await.unwrap_err();
    assert!(
        matches!(
            err,
            mdor_core::error::Error::MirrorLimit {
                limit: "file_count",
                ..
            }
        ),
        "应报文件数超限：{err}"
    );
}

#[tokio::test]
async fn mirror_dedups_shared_resources() {
    let server = MockServer::start();
    // 两页共享同一 css；css 只应被抓一次（seen 去重 → 不重复入队）。
    let css_hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let hits = css_hits.clone();
    server.mock(move |when, then| {
        when.method(GET).path("/style.css");
        hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        then.status(200).body("body{}");
    });
    server.mock(|when, then| {
        when.method(GET).path("/index.html");
        then.status(200)
            .body("<link rel=\"stylesheet\" href=\"style.css\"><a href=\"p2.html\">2</a>");
    });
    server.mock(|when, then| {
        when.method(GET).path("/p2.html");
        then.status(200)
            .body("<link rel=\"stylesheet\" href=\"style.css\">");
    });

    let entry = format!("{}/index.html", server.base_url());
    let files = mirror_site(&entry, &MirrorOptions::from_entry(&entry).unwrap())
        .await
        .expect("镜像成功");
    assert_eq!(files.len(), 3, "两页 + 一份 css");
    assert_eq!(
        css_hits.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "共享资源只应请求一次（已抓去重）"
    );
}

#[tokio::test]
async fn mirror_skips_cross_origin_links() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/index.html");
        then.status(200).body(
            "<a href=\"https://other.example.com/x.html\">外站</a><a href=\"mailto:a@b.c\">邮件</a>",
        );
    });

    let entry = format!("{}/index.html", server.base_url());
    let files = mirror_site(&entry, &MirrorOptions::from_entry(&entry).unwrap())
        .await
        .expect("镜像成功");
    assert_eq!(files.len(), 1, "外站与 mailto 链接均不抓");
}

#[tokio::test]
async fn mirror_returns_404_as_error() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/index.html");
        then.status(404).body("missing");
    });

    let entry = format!("{}/index.html", server.base_url());
    let err = mirror_site(&entry, &MirrorOptions::from_entry(&entry).unwrap())
        .await
        .unwrap_err();
    assert!(
        matches!(err, mdor_core::error::Error::HttpStatus { status: 404, .. }),
        "非 2xx 应报 HttpStatus：{err}"
    );
}

#[tokio::test]
async fn adapter_fetch_returns_version_id() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/index.html");
        then.status(200)
            .header("content-type", "text/html; charset=utf-8")
            .body("<!DOCTYPE html><html><body><h1>hello</h1></body></html>");
    });

    let src = StaticSiteSource::new();
    let entry = format!("{}/index.html", server.base_url());
    let result = src
        .fetch(&entry, &common::temp_dir("fetch_dest"))
        .await
        .unwrap();
    assert_eq!(
        result.version_id.len(),
        16,
        "版本标识 = SHA-256 前 16 位 hex"
    );
    assert_eq!(src.kind(), SourceKind::StaticSite);
    assert_eq!(src.name(), "StaticSiteSource");
}

#[tokio::test]
async fn fetch_fixture_returns_real_toc_and_title() {
    let server = MockServer::start();
    let _served = serve_fixture(&server);

    let src = StaticSiteSource::new();
    let entry = format!("{}/index.html", server.base_url());
    let result = src
        .fetch(&entry, &common::temp_dir("fetch_toc"))
        .await
        .expect("fixture 镜像成功");

    // 真实 mdBook 产物：书名 + 3 章嵌套 TOC（SUMMARY.md 定义的形状）。
    assert_eq!(result.title, "mdor 测试小书");
    assert_eq!(result.toc.len(), 3, "三个顶层章节");
    assert_eq!(result.toc[0].title, "第一章 入门");
    assert_eq!(result.toc[0].path, "ch1.html");
    assert_eq!(result.toc[0].children.len(), 1, "第一章含子章节");
    assert_eq!(result.toc[0].children[0].title, "安装");
    assert_eq!(result.toc[0].children[0].path, "ch1-1.html");
    assert_eq!(result.toc[2].title, "第三章 附录");

    // TOC 路径与镜像文件集对齐（chapter_path → 工作区文件）。
    for entry in result.toc.iter().flat_map(|e| e.flat()) {
        assert!(
            std::fs::metadata(common::mdbook_fixture_root().join(&entry.path)).is_ok(),
            "TOC 路径 {} 应对应 fixture 内文件",
            entry.path
        );
    }
}

#[tokio::test]
async fn adapter_remote_version_changes_with_content() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/");
        then.status(200).body("<html>v1</html>");
    });

    let src = StaticSiteSource::new();
    let entry = format!("{}/", server.base_url());
    let v1 = src
        .remote_version(&entry)
        .await
        .unwrap()
        .expect("有版本标识");

    server.reset();
    server.mock(|when, then| {
        when.method(GET).path("/");
        then.status(200).body("<html>v2</html>");
    });
    let v2 = src
        .remote_version(&entry)
        .await
        .unwrap()
        .expect("有版本标识");

    assert_ne!(v1, v2, "内容变则版本标识变");
}

/// FileSet 类型别名导出可用性（落库输入契约，切片4 消费）。
#[test]
fn fileset_alias_usable() {
    let mut files: FileSet = HashMap::new();
    files.insert(PathBuf::from("index.html"), b"x".to_vec());
    assert_eq!(files.len(), 1);
}

/// M2 验收（§10）：真实 mdBook 站点离线镜像。
///
/// 依赖真实网络，不进 CI（`#[ignore]`），手工执行：
/// `cargo test -p mdor-core --test static_site_mirror real_site -- --ignored --nocapture`
#[tokio::test]
#[ignore = "依赖真实网络，M2 手工验收用"]
async fn real_site_doc_rust_lang_book_mirror() {
    const ENTRY: &str = "https://doc.rust-lang.org/book/";

    let opts = MirrorOptions::from_entry(ENTRY).unwrap();
    let files = mirror_site(ENTRY, &opts).await.expect("真实站点镜像成功");

    // 基本量级：mdBook 站点应有几十个页面 + 资源。
    assert!(files.len() >= 30, "镜像文件数应 ≥30，实际 {}", files.len());
    let pages = files
        .keys()
        .filter(|p| p.extension().is_some_and(|e| e == "html"))
        .count();
    assert!(pages >= 10, "HTML 页面应 ≥10，实际 {pages}");

    // 字节保真抽查：入口页可读且为 HTML。
    // 入口 https://doc.rust-lang.org/book/ → 前缀 /book → 落盘 book/index.html。
    let index = files
        .get(std::path::Path::new("book/index.html"))
        .or_else(|| files.get(std::path::Path::new("index.html")))
        .expect("入口页应入集");
    let text = String::from_utf8_lossy(index);
    assert!(
        text.contains("The Rust Programming Language"),
        "入口页应为 TRPL 书页"
    );

    // TOC 从真实 sidebar 构建。
    let toc = mdor_core::source::static_site::build_toc(&files);
    assert!(
        toc.len() >= 3,
        "真实书 TOC 应有多个章节，实际 {}",
        toc.len()
    );
    eprintln!(
        "镜像 {}/{} 页面；TOC 顶层 {} 章",
        pages,
        files.len(),
        toc.len()
    );
}
