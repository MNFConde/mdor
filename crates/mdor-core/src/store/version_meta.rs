//! 书籍版本元数据存取（§9）：`books/<id>/.mdor/versions/<sha>.json`。
//!
//! 按 commit sha 索引的 toc/meta；位于仓库根下但**未被 git 跟踪**（checkout 与
//! 自建 commit 都不触碰），两场景都不污染历史（§7.2）。只增写、失败可重写，
//! 走 RenameOnly 原子写（§6.7）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::snapshot::SnapshotMeta;
use crate::model::toc::TocEntry;

use super::util::{Durability, MAX_META_BYTES, read_json_capped, write_json_atomic};

/// 单版本落库记录（§9：`<sha>.json` 内容）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VersionRecord {
    /// 章节树（TOC）。
    pub toc: Vec<TocEntry>,
    /// 快照元数据。
    pub meta: SnapshotMeta,
}

/// 每书版本元数据存取（`.mdor/versions/`）。
pub struct VersionMetaStore {
    dir: PathBuf,
}

impl VersionMetaStore {
    /// 以书籍仓库根（`books/<id>/`）为基座构造。
    #[must_use]
    pub fn new(book_repo_root: &Path) -> Self {
        Self {
            dir: book_repo_root.join(".mdor").join("versions"),
        }
    }

    /// 单版本记录路径。
    fn record_path(&self, commit_sha: &str) -> PathBuf {
        self.dir.join(format!("{commit_sha}.json"))
    }

    /// 落库单版本 toc/meta（原子写，RenameOnly 档）。
    pub fn save(&self, commit_sha: &str, record: &VersionRecord) -> Result<()> {
        write_json_atomic(
            &self.record_path(commit_sha),
            record,
            Durability::RenameOnly,
        )
    }

    /// 读取单版本 toc/meta；不存在返回 `None`。
    pub fn load(&self, commit_sha: &str) -> Result<Option<VersionRecord>> {
        match read_json_capped(&self.record_path(commit_sha), MAX_META_BYTES) {
            Ok(record) => Ok(Some(record)),
            Err(Error::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::snapshot::CaseCollisionRecord;
    use crate::test_support::temp_dir;

    #[test]
    fn save_then_load_roundtrip() {
        let root = temp_dir("vermeta_roundtrip");
        let store = VersionMetaStore::new(&root);
        let record = VersionRecord {
            toc: vec![crate::model::toc::TocEntry {
                title: "第一章".to_string(),
                path: "ch1.html".to_string(),
                children: vec![],
            }],
            meta: SnapshotMeta {
                fetched_at: 1_756_000_000,
                source_version: Some("abc".to_string()),
                content_tree_hash: "tree123".to_string(),
                case_collisions: vec![CaseCollisionRecord {
                    paths: vec!["A.html".to_string(), "a.html".to_string()],
                    same_blob: true,
                }],
            },
        };

        store.save("abc123", &record).unwrap();
        let loaded = store.load("abc123").unwrap().expect("记录存在");
        assert_eq!(loaded, record);
        assert!(
            root.join(".mdor/versions/abc123.json").exists(),
            "应落在 .mdor/versions/<sha>.json（§9）"
        );
    }

    #[test]
    fn load_missing_returns_none() {
        let root = temp_dir("vermeta_missing");
        let store = VersionMetaStore::new(&root);
        assert!(store.load("nope").unwrap().is_none());
    }

    #[test]
    fn sha256_long_key_safe() {
        let root = temp_dir("vermeta_longkey");
        let store = VersionMetaStore::new(&root);
        let sha = "a".repeat(64);
        store
            .save(&sha, &VersionRecord::default())
            .expect("64 位 sha 文件名可写");
        assert!(store.load(&sha).unwrap().is_some());
    }
}
