//! 薄门面 AppService（§6.9）：UI 唯一入口，单次用户操作 = 单次门面调用。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::error::{Error, Result};
use crate::migration::path::PathMigrator;
use crate::model::book::Book;
use crate::model::position::ReadingPosition;
use crate::model::toc::TocEntry;
use crate::render::{self, RenderedChapter};
use crate::services::AppContext;
use crate::services::commands::CommandQueue;
use crate::services::snapshot_pipeline::{SnapshotOptions, commit_snapshot, save_version_record};
use crate::source::static_site::StaticSiteSource;
use crate::store::BookStore;
use crate::store::library::Library;

/// `open_reading` 返回（§6.9 薄门面；SD-2 §6.6）。
#[derive(Debug, Clone)]
pub struct OpenReading {
    /// 打开的书。
    pub book: Book,
    /// 已保存的阅读位置（无则 None）。
    pub position: Option<ReadingPosition>,
    /// 当前版本的 TOC 章节树（空表示该书无可用版本信息）。
    pub toc: Vec<TocEntry>,
    /// 定位到的章节渲染结果（含正文 HTML；无可用章节时 html 为空）。
    pub chapter_html: RenderedChapter,
    /// 初始标题锚点（进度定位；无则 None）。
    pub initial_anchor: Option<String>,
    /// 初始滚动比例（进度定位；无则 0.0）。
    pub scroll_ratio: f32,
}

/// UI 唯一门面（§6.9 薄门面）：单次用户操作 = 单次门面调用，UI 不触达内部服务。
pub struct AppService {
    ctx: Arc<AppContext>,
    queue: CommandQueue,
}

impl AppService {
    /// 初始化门面：BookStore 注入 + 孤儿清理 + 内置适配器注册 + 命令队列。
    ///
    /// 需在 tokio 运行时上下文内调用（命令队列消费者任务 `tokio::spawn` 要求）。
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        let store = BookStore::new(base_dir);
        let removed = store.cleanup_orphans()?;
        tracing::info!(?removed, "清理孤儿书籍目录");
        let mut registry = crate::source::SourceRegistry::new();
        registry.register(Box::new(StaticSiteSource::new()));
        let ctx = Arc::new(AppContext {
            store,
            registry,
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
    /// 流程：detect → fetch（镜像 + TOC）→ `books/<id>/` 落库（commit_snapshot）
    /// → library.json 最后写（提交点语义，§6.7；中断即孤儿目录启动清理兜底）。
    pub async fn add_book(&self, url: &str) -> Result<Book> {
        let start = Instant::now();
        let result = self.add_book_inner(url).await;
        match &result {
            Ok(book) => tracing::info!(
                url,
                book_id = %book.id,
                version = %book.current_version,
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
        let adapter = self
            .ctx
            .registry
            .detect(url)
            .ok_or_else(|| Error::NoSource(url.to_string()))?;

        // 重复添加拒绝（book_id = SHA-256(url) 派生，同 URL 恒同 id）。
        let book_id = Book::derive_id(url);
        let library = self.ctx.store.library().load()?;
        if library.books.iter().any(|b| b.id == book_id) {
            return Err(Error::AlreadyExists(format!("书籍已在书架：{url}")));
        }

        let fetched = adapter.fetch(url, &self.ctx.store.books_root()).await?;
        let repo_root = self.ctx.store.books_root().join(&book_id);
        let snapshot = commit_snapshot(
            &repo_root,
            fetched.files(),
            Some(fetched.version_id.clone()),
            &SnapshotOptions::default(),
        )?;
        save_version_record(
            &repo_root,
            &snapshot.version_id,
            fetched.toc.clone(),
            &snapshot.meta,
        )?;

        // 提交点：library.json 最后写。
        let now = now_unix();
        let book = Book {
            id: book_id,
            source_kind: adapter.kind(),
            url: url.to_string(),
            title: if fetched.title.is_empty() {
                "未命名书籍".to_string()
            } else {
                fetched.title.clone()
            },
            current_version: snapshot.version_id.clone(),
            added_at: now,
            updated_at: now,
        };
        self.ctx.store.library().save(&Library {
            books: {
                let mut books = library.books;
                books.push(book.clone());
                books
            },
        })?;
        Ok(book)
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

        let ctx = self.ctx.clone();
        let (toc, html, anchor, ratio) = Self::locate_and_render(&ctx, &book, position.as_ref())?;

        Ok(OpenReading {
            book,
            position,
            toc,
            chapter_html: html,
            initial_anchor: anchor,
            scroll_ratio: ratio,
        })
    }

    /// 定位章节并渲染（SD-2 §6.6）。
    ///
    /// - 无可用版本（`current_version` 空 / TOC 缺失）→ 返回空 toc + 空渲染。
    /// - 有进度且进度章节存在于 TOC → 按进度章节渲染；否则回退 TOC 首章
    ///   （失败保留旧位置场景，§5 迁移）。
    /// - 章节文件缺失或内容无 `<main>` → 回退首章；首章也异常则返回空渲染。
    fn locate_and_render(
        ctx: &AppContext,
        book: &Book,
        position: Option<&ReadingPosition>,
    ) -> Result<(Vec<TocEntry>, RenderedChapter, Option<String>, f32)> {
        if book.current_version.is_empty() {
            return Ok((Vec::new(), RenderedChapter::empty(), None, 0.0));
        }
        let Some(record) = ctx
            .store
            .version_meta(&book.id)
            .load(&book.current_version)?
        else {
            return Ok((Vec::new(), RenderedChapter::empty(), None, 0.0));
        };
        if record.toc.is_empty() {
            return Ok((Vec::new(), RenderedChapter::empty(), None, 0.0));
        }

        let flat: Vec<&TocEntry> = record.toc.iter().flat_map(|t| t.flat()).collect();
        let (chapter, anchor, ratio) = match position {
            Some(p) if flat.iter().any(|t| t.path == p.chapter_path) => (
                p.chapter_path.clone(),
                p.heading_anchor.clone(),
                p.scroll_ratio,
            ),
            _ => (flat[0].path.clone(), None, 0.0),
        };

        let rendered = Self::render_chapter_from_workdir(ctx, book, &chapter);
        match rendered {
            Ok(html) => Ok((record.toc, html, anchor, ratio)),
            // 定位章节渲染失败 → 回退首章；仍失败则留空渲染（薄门面不致命）。
            Err(_) => match Self::render_chapter_from_workdir(ctx, book, &flat[0].path) {
                Ok(html) => Ok((record.toc, html, None, 0.0)),
                Err(_) => Ok((record.toc, RenderedChapter::empty(), None, 0.0)),
            },
        }
    }

    /// 从工作区 `site/<chapter>` 读取字节并渲染为正文。
    fn render_chapter_from_workdir(
        ctx: &AppContext,
        book: &Book,
        chapter: &str,
    ) -> Result<RenderedChapter> {
        let rel = std::path::PathBuf::from(chapter);
        if rel.is_absolute()
            || rel
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(Error::Unsupported("章节路径含穿越"));
        }
        let path = ctx.store.books_root().join(&book.id).join("site").join(rel);
        let bytes = std::fs::read(&path).map_err(|e| Error::io(&path, e))?;
        let prefix = render::resources::url_prefix_root(0, &book.id);
        render::render_chapter(&bytes, &prefix)
    }

    /// 保存阅读进度（SD-2 §6.6，§6.9 非命令：短/无网络/无并发）。
    ///
    /// 更新 `<bookstore>/progress.json` 中该书的 [`ReadingPosition`]：
    /// `version_id` 取书籍当前版本、`saved_at` 记当前 unix 秒；`anchor`/`scroll_ratio`
    /// 由调用方传入（UI 节流/切章时上报）。
    pub fn save_progress(
        &self,
        book_id: &str,
        chapter_path: &str,
        anchor: Option<String>,
        scroll_ratio: f32,
    ) -> Result<()> {
        let start = Instant::now();
        let anchor_log = anchor.as_deref().unwrap_or("").to_string();
        let result = self.save_progress_inner(book_id, chapter_path, anchor, scroll_ratio);
        match &result {
            Ok(()) => tracing::info!(
                book_id,
                chapter = chapter_path,
                anchor = %anchor_log,
                scroll_ratio,
                elapsed_ms = start.elapsed().as_millis(),
                "save_progress 成功"
            ),
            Err(e) => tracing::warn!(
                book_id,
                error = %e,
                elapsed_ms = start.elapsed().as_millis(),
                "save_progress 失败"
            ),
        }
        result
    }

    fn save_progress_inner(
        &self,
        book_id: &str,
        chapter_path: &str,
        anchor: Option<String>,
        scroll_ratio: f32,
    ) -> Result<()> {
        let library = self.ctx.store.library().load()?;
        let book = library
            .books
            .into_iter()
            .find(|b| b.id == book_id)
            .ok_or_else(|| Error::NotFound(format!("书籍 {book_id} 不在书架")))?;

        let store = self.ctx.store.progress();
        let mut progress = store.load()?;
        progress.positions.insert(
            book_id.to_string(),
            ReadingPosition {
                book_id: book_id.to_string(),
                version_id: book.current_version,
                chapter_path: chapter_path.to_string(),
                heading_anchor: anchor,
                scroll_ratio,
                saved_at: now_unix(),
            },
        );
        store.save(&progress)
    }

    /// 更新书籍（SD-3 薄门面）：封装为命令入队串行执行（§6.9 命令化）。
    pub fn update_book(&self, book_id: &str) -> Result<()> {
        self.queue.enqueue(Box::new(
            crate::services::commands::update_book::UpdateBookCommand {
                book_id: book_id.to_string(),
            },
        ))
    }

    /// 命令队列句柄。
    #[must_use]
    pub fn command_queue(&self) -> &CommandQueue {
        &self.queue
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
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
    async fn add_book_unknown_scheme_errors() {
        let dir = temp_dir("appsvc_add");
        let service = AppService::new(dir).unwrap();
        // StaticSite 已注册（M2）：http(s) 会被识别；非 http(s) 才 NoSource。
        assert!(matches!(
            service.add_book("ftp://example.com/book1").await,
            Err(Error::NoSource(_))
        ));
        assert!(matches!(
            service.add_book("not a url").await,
            Err(Error::NoSource(_))
        ));
    }

    #[tokio::test]
    async fn add_book_rejects_duplicate_url() {
        let dir = temp_dir("appsvc_dup");
        let service = AppService::new(dir).unwrap();
        // 重复添加拒绝：fetch 之前即拦截（book_id 由 URL 派生）。
        // 首次添加需要真实 fetch，此处直接预置 library 验证拦截逻辑。
        let book = sample_book("https://example.com/dup");
        service
            .ctx
            .store
            .library()
            .save(&Library { books: vec![book] })
            .unwrap();
        assert!(matches!(
            service.add_book("https://example.com/dup").await,
            Err(Error::AlreadyExists(_))
        ));
    }
}
