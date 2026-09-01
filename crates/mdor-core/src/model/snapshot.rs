//! 版本快照模型（§5）：一次获取的内容快照及其元数据。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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

/// 大小写碰撞记录的落库形态（D-09 定案 3 件②：随 SnapshotMeta 持久化）。
///
/// 同 blob 可归一（M3 渲染层消费）；异 blob 内容真不同，Windows 物理单文件会
/// 静默丢失一方（树序后者胜出），消费方按报错选项拦截（M2）或标注（M3）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseCollisionRecord {
    /// 碰撞路径组（≥2 条；大小写折叠后相同）。
    pub paths: Vec<String>,
    /// 是否同 blob（oid 相等）。
    pub same_blob: bool,
}

/// 快照元数据（§5 `SnapshotMeta`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMeta {
    /// 获取时间（unix 秒）。
    pub fetched_at: i64,
    /// 来源版本标识（GitHub: commit SHA；静态站: ETag/内容树 hash）。
    pub source_version: Option<String>,
    /// 内容树 hash（git tree oid）。
    pub content_tree_hash: String,
    /// 大小写碰撞记录（无碰撞为空表；D-09 定案 3，M2 落库）。
    #[serde(default)]
    pub case_collisions: Vec<CaseCollisionRecord>,
}
