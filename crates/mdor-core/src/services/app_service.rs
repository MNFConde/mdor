//! 薄门面 AppService（§6.9）：UI 唯一入口，单次用户操作 = 单次门面调用。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::error::{Error, Result};
use crate::migration::path::PathMigrator;
use crate::model::book::Book;
use crate::model::position::ReadingPosition;
use crate::services::AppContext;
use crate::services::commands::CommandQueue;
use crate::store::BookStore;

/// `open_reading` 返回（§6.9 薄门面；渲染部分 M3 填全）。
#[derive(Debug, Clone)]
pub struct OpenReading {
    /// 打开的书。
    pub book: Book,
    /// 已保存的阅读位置（无则 None）。
    pub position: Option<ReadingPosition>,
}

/// UI 唯一门面（§6.9 薄门面）：单次用户操作 = 单次门面调用，UI 不触达内部服务。
pub struct AppService {
    ctx: Arc<AppContext>,
    queue: CommandQueue,
}

impl AppService {
    /// 初始化门面：BookStore 注入 + 孤儿清理 + 命令队列。
    ///
    /// 需在 tokio 运行时上下文内调用（命令队列消费者任务 `tokio::spawn` 要求）。
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        let store = BookStore::new(base_dir);
        let removed = store.cleanup_orphans()?;
        tracing::info!(?removed, "清理孤儿书籍目录");
        let ctx = Arc::new(AppContext {
            store,
            registry: crate::source::SourceRegistry::new(),
            migrator: Arc::new(PathMigrator),
        });
        let queue = CommandQueue::spawn(ctx.clone());
        Ok(Self { ctx, queue })
    }

    /// 书架数据（UI 渲染源，读 library.json）。
    pub fn library(&self) -> Result<Vec<Book>> {
        self.ctx.store.library().load().map(|l| l.books)
    }

    /// 添加书籍（SD-1 薄门面，§4.2）：detect + 获取 + 入架合并为一次调用。
    ///
    /// M1 占位：内置适配器 M2/M4 落地前恒返回错误（`NoSource` / `Unsupported`）。
    pub async fn add_book(&self, url: &str) -> Result<Book> {
        let start = Instant::now();
        let result = self.add_book_inner(url).await;
        match &result {
            Ok(book) => tracing::info!(
                url,
                book_id = %book.id,
                elapsed_ms = start.elapsed().as_millis(),
                "add_book 成功"
            ),
            Err(e) => tracing::warn!(
                url,
                error = %e,
                elapsed_ms = start.elapsed().as_millis(),
                "add_book 失败"
            ),
        }
        result
    }

    async fn add_book_inner(&self, url: &str) -> Result<Book> {
        match self.ctx.registry.detect(url) {
            Some(_adapter) => Err(Error::Unsupported("来源获取管线（M2）")),
            None => Err(Error::NoSource(url.to_string())),
        }
    }

    /// 打开阅读（SD-2 薄门面，§6.6）：读位置 + 定位；渲染注入 M3 填全。
    pub async fn open_reading(&self, book_id: &str) -> Result<OpenReading> {
        let start = Instant::now();
        let result = self.open_reading_inner(book_id).await;
        match &result {
            Ok(opened) => {
                let version = opened
                    .position
                    .as_ref()
                    .map(|p| p.version_id.as_str())
                    .unwrap_or("无进度");
                tracing::info!(
                    book_id,
                    version,
                    elapsed_ms = start.elapsed().as_millis(),
                    "open_reading 成功"
                );
            }
            Err(e) => tracing::warn!(
                book_id,
                error = %e,
                elapsed_ms = start.elapsed().as_millis(),
                "open_reading 失败"
            ),
        }
        result
    }

    async fn open_reading_inner(&self, book_id: &str) -> Result<OpenReading> {
        let library = self.ctx.store.library().load()?;
        let book = library
            .books
            .into_iter()
            .find(|b| b.id == book_id)
            .ok_or_else(|| Error::NotFound(format!("书籍 {book_id} 不在书架")))?;
        let position = self
            .ctx
            .store
            .progress()
            .load()?
            .positions
            .get(book_id)
            .cloned();
        Ok(OpenReading { book, position })
    }

    /// 命令队列句柄（M2 更新流程接入）。
    #[must_use]
    pub fn command_queue(&self) -> &CommandQueue {
        &self.queue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::book::tests::sample_book;
    use crate::store::library::Library;
    use crate::test_support::temp_dir;

    #[tokio::test]
    async fn facade_library_and_open_reading() {
        let dir = temp_dir("appsvc");
        let service = AppService::new(dir.clone()).unwrap();

        assert!(service.library().unwrap().is_empty(), "初始书架为空");

        let book = sample_book("https://example.com/book1");
        service
            .ctx
            .store
            .library()
            .save(&Library {
                books: vec![book.clone()],
            })
            .unwrap();
        assert_eq!(service.library().unwrap(), vec![book.clone()]);

        let opened = service.open_reading(&book.id).await.unwrap();
        assert_eq!(opened.book.id, book.id);
        assert!(opened.position.is_none(), "无进度时位置为 None");

        assert!(matches!(
            service.open_reading("nope").await,
            Err(Error::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn add_book_without_adapter_errors() {
        let dir = temp_dir("appsvc_add");
        let service = AppService::new(dir).unwrap();
        assert!(matches!(
            service.add_book("https://example.com/book1").await,
            Err(Error::NoSource(_))
        ));
    }
}
