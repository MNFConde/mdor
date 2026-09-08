//! 书籍元数据模型（library.json 单条记录）。

use crate::source::SourceKind;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 书架上的书籍元数据（library.json 中每条）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Book {
    /// book_id：由来源 URL 派生（`derive_id`），全链路主键（目录名 / 去重 / 进度索引）
    pub id: String,
    /// 来源类型：更新时据此选择适配器
    pub source_kind: SourceKind,
    /// 原始网址（用于更新）
    pub url: String,
    /// 书名（书架显示）
    pub title: String,
    /// 当前版本（HEAD commit sha）
    pub current_version: String,
    /// 添加时间（unix 秒）
    pub added_at: i64,
    /// 最近更新时间（unix 秒）
    pub updated_at: i64,
}

impl Book {
    /// book_id：SHA-256(url) 十六进制前 16 位。
    /// 目录名安全、跨平台稳定；title 不参与（书名可变，id 必须稳定）。
    #[must_use]
    pub fn derive_id(url: &str) -> String {
        let digest = Sha256::digest(url.as_bytes());
        digest[..8]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// 构造测试用 Book（共享给各模块 roundtrip 测试）。
    pub(crate) fn sample_book(url: &str) -> Book {
        Book {
            id: Book::derive_id(url),
            source_kind: SourceKind::GitForge,
            url: url.to_string(),
            title: "示例书籍".to_string(),
            current_version: String::new(),
            added_at: 1_752_000_000,
            updated_at: 1_752_000_000,
        }
    }

    #[test]
    fn same_url_same_id() {
        assert_eq!(
            Book::derive_id("https://example.com/book1"),
            Book::derive_id("https://example.com/book1")
        );
    }

    #[test]
    fn different_url_different_id() {
        assert_ne!(
            Book::derive_id("https://example.com/book1"),
            Book::derive_id("https://example.com/book2")
        );
    }

    #[test]
    fn id_is_16_hex_chars() {
        let id = Book::derive_id("https://example.com/book1");
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
