//! 本地资源分发 HTTP 服务器（D-04 / SD-5）。
//!
//! 阅读页渲染出的资源 URL 形如 `http://127.0.0.1:{PORT}/books/{book_id}/mdor-res/v1/...`，
//! 由 mdBook 阅读场景的 WebView 发起请求。本模块在 app 层用 `tiny_http` 起一个仅
//! 绑定回环地址的极简服务器，负责把这类 `/mdor-res/v1/` 路径解析回书内相对路径
//! 并从工作区 `site/` 目录读取字节返回。
//!
//! 设计要点（对应切片 3 验收）：
//! 1. [`bind`] 动态端口：`bind("127.0.0.1:0")` 由系统分配可用端口（B2）。
//! 2. 独立线程池：服务在独立 `std::thread` 上阻塞接受请求，不占 UI 线程（B1）。
//! 3. 白名单校验：路径经 [`mdor_core::render::resources::resolve_local_url`] 与
//!    [`sanitize_relative`] 双重规范化 + 穿越拦截，越界一律 403/404（B5）。
//! 4. `Cache-Control: no-store`：动态端口 + 版本切换场景禁用缓存（B6/D-04）。

use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use mdor_core::render::resources::{resolve_local_url, sanitize_relative};
use tiny_http::{Header, Response, Server};

/// 本地资源服务器句柄（app 层持有，跨线程共享）。
///
/// 构造即绑定端口并启动独立服务线程；`drop` 时关闭监听（listener 取出后由
/// [`Server`] 内部持有，`Server` 释放即停止接受新连接）。
#[must_use]
pub struct LocalHttpServer {
    /// 实际绑定的端口（`bind` 动态分配后回读）。
    pub port: u16,
    /// 资源书根（`<bookstore>/books`，核心白名单校验后的落盘根）。
    _books_root: PathBuf,
    /// 服务线程句柄（不 join，持有以保活；服务随进程退出而终止）。
    _thread: thread::JoinHandle<()>,
}

impl LocalHttpServer {
    /// 绑定回环动态端口并启动服务线程。
    ///
    /// `books_root` 为书籍仓库根（`<bookstore>/books`），资源请求经白名单校验后
    /// 解析到 `<books_root>/<book_id>/site/<rel>` 读取。绑定失败返回 `Err`（端口
    /// 耗尽/权限等，app 启动时可降级为不启用资源服务）。
    pub fn bind(books_root: PathBuf) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        let server = Server::http(addr)?;
        let port = server
            .server_addr()
            .to_ip()
            .expect("回环地址必为 IP")
            .port();
        let server_root = Arc::new(books_root);
        let thread_root = Arc::clone(&server_root);

        let thread = thread::Builder::new()
            .name("mdor-local-http".to_string())
            .spawn(move || serve(server, thread_root))?;

        Ok(Self {
            port,
            _books_root: (*server_root).clone(),
            _thread: thread,
        })
    }
}

/// 阻塞接受并处理请求直至服务器关闭（服务线程入口）。
fn serve(server: Server, books_root: Arc<PathBuf>) {
    for request in server.incoming_requests() {
        handle_request(request, &books_root);
    }
}

/// 处理单个请求：解析路径 → 白名单校验 → 读文件返回；越界/缺失返回 403/404。
fn handle_request(request: tiny_http::Request, books_root: &Path) {
    let url = request.url();
    match route(url, books_root) {
        Ok(served) => {
            let response = Response::from_data(served.bytes)
                .with_header(no_store())
                .with_header(content_type_header(served.mime_type));
            let _ = request.respond(response);
        }
        Err(code) => {
            let _ = request.respond(Response::from_string("").with_status_code(code));
        }
    }
}

/// 路由结果：`Ok(文件内容 + MIME)` 或 `Err(HTTP 状态码)`。
type RouteResult = Result<ServedFile, u16>;

/// 已解析的资源文件（字节 + MIME 类型）。
struct ServedFile {
    bytes: Vec<u8>,
    mime_type: &'static str,
}

/// URL → 文件：校验 → 映射 → 读盘。
///
/// URL 形如 `/books/{book_id}/mdor-res/v1/<rel>`。处理流程：
/// 1. [`extract_book_id`] 剥 `/books/{book_id}/` 前缀，命中非该形态即 404。
/// 2. [`resolve_local_url`] 剥离剩余路径的 `/mdor-res/v1/` 前缀并解析 `.`/`..`
///    （越界返回 `None` → 404，非资源路径）。
/// 3. [`sanitize_relative`] 对相对路径再做一次穿越与绝对路径拦截（防 `C:/`、前导
///    `..`，越界 `None` → 403）。
/// 4. 拼 `<books_root>/<book_id>/site/<rel>` 读文件；不存在/目录 → 404。
fn route(url: &str, books_root: &Path) -> RouteResult {
    let Some((book_id, rest_url)) = extract_book_id(url) else {
        return Err(404);
    };
    let Some(rel) = resolve_local_url(rest_url) else {
        return Err(404);
    };
    let Some(rel_path) = sanitize_relative(&rel, false) else {
        return Err(403);
    };

    let path = books_root.join(book_id).join("site").join(rel_path);
    let Ok(bytes) = std::fs::read(&path) else {
        return Err(404);
    };
    Ok(ServedFile {
        bytes,
        mime_type: mime_for(&path),
    })
}

/// 从请求 URL 中提取 `/books/{book_id}/` 段，返回 `(book_id, 剩余 URL)`。
///
/// `url` 形如 `/books/{book_id}/mdor-res/v1/img/logo.svg`。命中前缀则剩余路径为
/// `/mdor-res/v1/...`；否则返回 `None`（非书内资源路径 → 404）。
fn extract_book_id(url: &str) -> Option<(&str, &str)> {
    let rest = url.strip_prefix("/books/")?;
    let end = rest.find('/')?;
    let book_id = &rest[..end];
    if book_id.is_empty() {
        return None;
    }
    // 剩余路径从 book_id 分隔符后开始。
    let rest_url = &rest[end..];
    if rest_url.is_empty() {
        return None;
    }
    Some((book_id, rest_url))
}

/// 按扩展名推断 MIME（覆盖 mdBook 产物：html/css/js/svg/png/woff2/ttf）。
fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("json") => "application/json; charset=utf-8",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// `Cache-Control: no-store`（D-04：动态端口 + 版本切换禁缓存）。
fn no_store() -> Header {
    Header::from_bytes(&b"Cache-Control"[..], &b"no-store"[..]).expect("Cache-Control 头合法")
}

/// `Content-Type` 响应头。
fn content_type_header(mime: &str) -> Header {
    Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()).expect("Content-Type 头合法")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_book_id_captures_two_segment_form() {
        assert_eq!(
            extract_book_id("/books/abc123/mdor-res/v1/img/logo.svg"),
            Some(("abc123", "/mdor-res/v1/img/logo.svg"))
        );
    }

    #[test]
    fn extract_book_id_rejects_non_book_path() {
        assert_eq!(extract_book_id("/mdor-res/v1/img/logo.svg"), None);
        assert_eq!(extract_book_id("/books/"), None);
        assert_eq!(extract_book_id("/books/abc123"), None, "缺后续段应拒绝");
    }

    #[test]
    fn mime_for_common_extensions() {
        assert_eq!(
            mime_for(&PathBuf::from("a.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(mime_for(&PathBuf::from("b.css")), "text/css; charset=utf-8");
        assert_eq!(mime_for(&PathBuf::from("c.svg")), "image/svg+xml");
        assert_eq!(mime_for(&PathBuf::from("d.png")), "image/png");
        assert_eq!(mime_for(&PathBuf::from("e.woff2")), "font/woff2");
        assert_eq!(
            mime_for(&PathBuf::from("f.bin")),
            "application/octet-stream"
        );
    }

    #[test]
    fn mime_for_missing_ext_is_octet_stream() {
        assert_eq!(
            mime_for(&PathBuf::from("noext")),
            "application/octet-stream"
        );
    }

    /// 直接对 route 做白名单与穿越拦截测试（无真实磁盘文件，读盘失败即 404）。
    #[test]
    fn route_resolves_prefix_and_captures_book() {
        let root = PathBuf::from("/tmp/bookstore/books");
        // 命中前缀 → book_id/rel 抽取正确，最终因文件不存在返回 404（非 403/越界）。
        assert!(matches!(
            route("/books/abc/mdor-res/v1/img/logo.svg", &root),
            Err(404)
        ));
    }

    #[test]
    fn route_rejects_traversal() {
        let root = PathBuf::from("/tmp/bookstore/books");
        // ../ 越界 → 403（sanitize_relative 拦截）。
        assert!(matches!(
            route("/books/abc/mdor-res/v1/../secret.svg", &root),
            Err(403 | 404)
        ));
        // 空路径 → 404。
        assert!(matches!(
            route("/books/abc/mdor-res/v1/", &root),
            Err(403 | 404)
        ));
    }

    /// 真实磁盘字节：把资源写入临时书根，验证 200 + 字节一致。
    #[test]
    fn route_serves_real_bytes() {
        let dir = std::env::temp_dir().join(format!("mdor-http-it-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let book_root = dir.join("abc123").join("site");
        std::fs::create_dir_all(&book_root).expect("创建书根");
        std::fs::write(book_root.join("logo.svg"), b"<svg/>").expect("写文件");

        let served = route("/books/abc123/mdor-res/v1/logo.svg", &dir).expect("文件可读");
        assert_eq!(served.bytes, b"<svg/>");
        assert_eq!(served.mime_type, "image/svg+xml");
    }

    /// 真实绑定 + TCP 请求：验证 `bind` 动态端口、字节返回与 no-store 头。
    #[test]
    fn bind_serves_over_real_socket() {
        let dir = std::env::temp_dir().join(format!("mdor-http-sock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let book_root = dir.join("abc123").join("site");
        std::fs::create_dir_all(&book_root).expect("创建书根");
        std::fs::write(book_root.join("logo.svg"), b"<svg/>").expect("写文件");

        let server = LocalHttpServer::bind(dir).expect("绑定成功");
        assert_ne!(server.port, 0, "动态端口应非 0");

        let stream = std::net::TcpStream::connect(("127.0.0.1", server.port)).expect("连接成功");
        use std::io::Write;
        let mut stream = stream;
        write!(
            stream,
            "GET /books/abc123/mdor-res/v1/logo.svg HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .expect("发送请求");
        stream.flush().expect("flush");
        let mut buf = String::new();
        use std::io::Read;
        stream.read_to_string(&mut buf).expect("读取响应");
        assert!(buf.starts_with("HTTP/1.1 200"), "应返回 200: {buf}");
        assert!(
            buf.contains("Cache-Control: no-store"),
            "应含 no-store: {buf}"
        );
        assert!(
            buf.contains("Content-Type: image/svg+xml"),
            "应含 MIME: {buf}"
        );
        assert!(buf.ends_with("<svg/>"), "正文应为字节: {buf}");
    }
}
