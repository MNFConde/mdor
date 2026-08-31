//! 版本快照模型（§5）：一次获取的内容快照及其元数据。

use std::path::PathBuf;

use crate::model::toc::TocEntry;

/// 一次获取的内容快照（书籍的一个版本，§5；由 tag `refs/mdor/versions/<seq>` 标记）。
#[derive(Debug, Clone, PartialEq)]
pub struct VersionSnapshot {
    /// 版本 tag 指向的 commit sha（`version_id`）。
    pub version_id: String,
    /// 仓库工作区路径（场景1: 上游文件；场景2: `books/<id>/site/`）。
    pub workdir: PathBuf,
    /// 章节树。
    pub toc: Vec<TocEntry>,
    /// 获取时间、来源版本标识、内容树 hash。
    pub meta: SnapshotMeta,
}

/// 快照元数据（§5 `SnapshotMeta`）。
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotMeta {
    /// 获取时间（unix 秒）。
    pub fetched_at: i64,
    /// 来源版本标识（GitHub: commit SHA；静态站: ETag/内容树 hash）。
    pub source_version: Option<String>,
    /// 内容树 hash（git tree oid）。
    pub content_tree_hash: String,
}
