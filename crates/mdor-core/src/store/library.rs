//! 书架元数据存取（§9）：`<bookstore>/library.json`。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::book::Book;

use super::util::{Durability, MAX_META_BYTES, read_json_capped, write_json_atomic};

/// library.json 顶层结构（Book 列表；包一层便于将来加字段不破坏格式）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Library {
    /// 书架上的书籍。
    pub books: Vec<Book>,
}

/// 书架元数据存取（§9：`<bookstore>/library.json`）。
///
/// 提交点语义：add_book/更新多步中 library.json 最后写，中断即回退到旧状态（§6.7）。
pub struct LibraryStore {
    path: PathBuf,
}

impl LibraryStore {
    /// 以 bookstore 目录为基座构造（路径 = `<bookstore>/library.json`）。
    #[must_use]
    pub fn new(base_dir: &Path) -> Self {
        Self {
            path: base_dir.join("library.json"),
        }
    }

    /// 载入书架；文件不存在返回空书架（首次启动）。
    pub fn load(&self) -> Result<Library> {
        match read_json_capped(&self.path, MAX_META_BYTES) {
            Ok(library) => Ok(library),
            Err(Error::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
                Ok(Library::default())
            }
            Err(e) => Err(e),
        }
    }

    /// 原子保存（Fsync 档：低频高价值，断电兜底）。
    pub fn save(&self, library: &Library) -> Result<()> {
        write_json_atomic(&self.path, library, Durability::Fsync)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::book::tests::sample_book;
    use crate::source::SourceKind;
    use crate::test_support::temp_dir;
    use std::fs;

    #[test]
    fn save_then_load_roundtrip() {
        let dir = temp_dir("library_roundtrip");
        let store = LibraryStore::new(&dir);
        let library = Library {
            books: vec![sample_book("https://example.com/book1")],
        };

        store.save(&library).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded, library);
    }

    #[test]
    fn load_missing_returns_empty() {
        let dir = temp_dir("library_empty");
        let store = LibraryStore::new(&dir);

        let library = store.load().unwrap();
        assert!(library.books.is_empty());
    }

    #[test]
    fn corrupt_library_errors() {
        let dir = temp_dir("library_corrupt");
        let store = LibraryStore::new(&dir);
        fs::write(dir.join("library.json"), b"{broken").unwrap();

        assert!(matches!(store.load(), Err(Error::Json { .. })));
    }

    #[test]
    fn serializes_source_kind_snake_case() {
        let dir = temp_dir("library_kind");
        let store = LibraryStore::new(&dir);
        let mut book = sample_book("https://example.com/book1");
        book.source_kind = SourceKind::StaticSite;
        store.save(&Library { books: vec![book] }).unwrap();

        let raw = fs::read_to_string(dir.join("library.json")).unwrap();
        assert!(
            raw.contains(r#""source_kind": "static_site""#),
            "应序列化为 snake_case：{raw}"
        );
    }
}
