use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::position::ReadingPosition;

use super::util::{Durability, MAX_META_BYTES, read_json_capped, write_json_atomic};

/// progress.json 顶层结构（ReadingPosition 按 book_id 索引，BTreeMap 保证
/// 序列化键序稳定、人类可读 diff）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// book_id → 阅读位置。
    pub positions: BTreeMap<String, ReadingPosition>,
}

/// 阅读进度存取（§9：`<bookstore>/progress.json`）。
///
/// RenameOnly 档：高频低价值，仅 rename 原子、不做 fsync（D-03 / §6.7）。
pub struct ProgressStore {
    path: PathBuf,
}

impl ProgressStore {
    /// 以 bookstore 目录为基座构造（路径 = `<bookstore>/progress.json`）。
    #[must_use]
    pub fn new(base_dir: &Path) -> Self {
        Self {
            path: base_dir.join("progress.json"),
        }
    }

    /// 载入进度；文件不存在返回空（首次启动）。
    pub fn load(&self) -> Result<Progress> {
        match read_json_capped(&self.path, MAX_META_BYTES) {
            Ok(progress) => Ok(progress),
            Err(Error::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
                Ok(Progress::default())
            }
            Err(e) => Err(e),
        }
    }

    /// 原子保存（RenameOnly 档）。
    pub fn save(&self, progress: &Progress) -> Result<()> {
        write_json_atomic(&self.path, progress, Durability::RenameOnly)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mdor-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("创建测试临时目录");
        dir
    }

    fn sample_position(book_id: &str) -> ReadingPosition {
        ReadingPosition {
            book_id: book_id.to_string(),
            version_id: "abc123".to_string(),
            chapter_path: "chapters/intro.html".to_string(),
            heading_anchor: Some("heading".to_string()),
            scroll_ratio: 0.42,
            saved_at: 1_752_000_000,
        }
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = temp_dir("progress_roundtrip");
        let store = ProgressStore::new(&dir);
        let progress = Progress {
            positions: BTreeMap::from([
                ("book1".to_string(), sample_position("book1")),
                ("book2".to_string(), sample_position("book2")),
            ]),
        };

        store.save(&progress).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded, progress);
    }

    #[test]
    fn load_missing_returns_empty() {
        let dir = temp_dir("progress_empty");
        let store = ProgressStore::new(&dir);

        assert!(store.load().unwrap().positions.is_empty());
    }

    #[test]
    fn keys_serialize_in_sorted_order() {
        let dir = temp_dir("progress_keys");
        let store = ProgressStore::new(&dir);
        let progress = Progress {
            positions: BTreeMap::from([
                ("z".to_string(), sample_position("z")),
                ("a".to_string(), sample_position("a")),
            ]),
        };

        store.save(&progress).unwrap();
        let raw = fs::read_to_string(dir.join("progress.json")).unwrap();

        assert!(
            raw.find(r#""a""#).unwrap() < raw.find(r#""z""#).unwrap(),
            "BTreeMap 键应升序序列化：{raw}"
        );
    }
}
