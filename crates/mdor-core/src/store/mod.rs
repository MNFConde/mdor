//! BookStore 聚合入口（§9）：`base_dir` 注入，统一持有各 store 与书籍仓库根。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

use self::library::LibraryStore;
use self::progress::ProgressStore;

pub mod library;
pub mod progress;
pub mod snapshot;
pub mod util;

/// BookStore 聚合（§9 存储布局）：core 只见 `bookstore/` 这一层，平台无关。
pub struct BookStore {
    base_dir: PathBuf,
}

impl BookStore {
    /// 以 bookstore 目录为基座构造（数据根解析在 app 层完成，D-13）。
    #[must_use]
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// bookstore 基目录。
    #[must_use]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// 书架元数据（`library.json`）。
    #[must_use]
    pub fn library(&self) -> LibraryStore {
        LibraryStore::new(&self.base_dir)
    }

    /// 阅读进度（`progress.json`）。
    #[must_use]
    pub fn progress(&self) -> ProgressStore {
        ProgressStore::new(&self.base_dir)
    }

    /// 书籍仓库根目录（`<bookstore>/books`）。
    #[must_use]
    pub fn books_root(&self) -> PathBuf {
        self.base_dir.join("books")
    }

    /// 清理孤儿书籍目录（§6.7 提交点设计配套 / §11）。
    ///
    /// 判定标准：`books/<id>/` 目录存在但 `library.json` 中无该 book_id 即为孤儿
    /// （add_book 在写 library.json 前中断的残留）。启动时同步执行，返回清理的 id 列表。
    ///
    /// 设计（26-08-27 定稿）：同步执行、不开后台线程——add_book 时序为「先建
    /// `books/<id>/` → 最后写 library.json」，后台删除与建目录之间存在 TOCTOU
    /// 竞态（删前复查只能缩小窗口关不死）；成本为微秒~毫秒级（几十本书量级）。
    pub fn cleanup_orphans(&self) -> Result<Vec<String>> {
        let library = self.library().load()?;
        let live: HashSet<&str> = library.books.iter().map(|b| b.id.as_str()).collect();
        let books_root = self.books_root();
        if !books_root.exists() {
            return Ok(Vec::new());
        }

        let mut removed = Vec::new();
        for entry in std::fs::read_dir(&books_root).map_err(|e| Error::io(&books_root, e))? {
            let entry = entry.map_err(|e| Error::io(&books_root, e))?;
            if !entry
                .file_type()
                .map_err(|e| Error::io(&books_root, e))?
                .is_dir()
            {
                continue;
            }
            let id = entry.file_name().to_string_lossy().into_owned();
            if !live.contains(id.as_str()) {
                let path = entry.path();
                std::fs::remove_dir_all(&path).map_err(|e| Error::io(&path, e))?;
                removed.push(id);
            }
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::book::tests::sample_book;
    use crate::store::library::Library;
    use crate::test_support::temp_dir;
    use std::fs;

    fn mk_orphan(books_root: &Path, id: &str) {
        fs::create_dir_all(books_root.join(id).join("site")).unwrap();
        fs::write(books_root.join(id).join("site/index.html"), b"x").unwrap();
    }

    #[test]
    fn removes_orphans_keeps_live() {
        let dir = temp_dir("orphan_live");
        let store = BookStore::new(dir.clone());
        let live_book = sample_book("https://example.com/book1");
        store
            .library()
            .save(&Library {
                books: vec![live_book.clone()],
            })
            .unwrap();

        let books_root = store.books_root();
        mk_orphan(&books_root, &live_book.id);
        mk_orphan(&books_root, "aaaaaaaaaaaaaaaa"); // 不在书架 → 孤儿

        let removed = store.cleanup_orphans().unwrap();
        assert_eq!(removed, vec!["aaaaaaaaaaaaaaaa".to_string()]);
        assert!(
            books_root.join(&live_book.id).exists(),
            "在册书籍目录应保留"
        );
        assert!(
            !books_root.join("aaaaaaaaaaaaaaaa").exists(),
            "孤儿目录应删除"
        );
    }

    #[test]
    fn missing_library_treats_all_as_orphans() {
        let dir = temp_dir("orphan_nolib");
        let store = BookStore::new(dir.clone());
        mk_orphan(&store.books_root(), "aaaaaaaaaaaaaaaa");

        let removed = store.cleanup_orphans().unwrap();
        assert_eq!(removed.len(), 1);
    }

    #[test]
    fn no_books_root_is_noop() {
        let dir = temp_dir("orphan_empty");
        let store = BookStore::new(dir);
        assert!(store.cleanup_orphans().unwrap().is_empty());
    }

    #[test]
    fn non_dir_entries_ignored() {
        let dir = temp_dir("orphan_non_dir");
        let store = BookStore::new(dir.clone());
        fs::create_dir_all(store.books_root()).unwrap();
        fs::write(store.books_root().join("readme.txt"), b"x").unwrap();

        let removed = store.cleanup_orphans().unwrap();
        assert!(removed.is_empty(), "非目录条目不应视为孤儿");
        assert!(store.books_root().join("readme.txt").exists());
    }
}
