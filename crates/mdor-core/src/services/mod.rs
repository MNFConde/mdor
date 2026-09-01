//! 服务编排层（§6.9）：薄门面 AppService + 按需命令化。

pub mod app_service;
pub mod commands;
pub mod snapshot_pipeline;

use std::sync::Arc;

use crate::migration::PositionMigrator;
use crate::source::SourceRegistry;
use crate::store::BookStore;

/// 一次快照落库的结果（§4.2 / §7.2：commit sha + tag 序号 + 元数据）。
#[derive(Debug, Clone)]
pub struct SnapshotResult {
    /// 版本标识（tag 指向的 commit sha）。
    pub version_id: String,
    /// 是否真正生成新 commit（false = D-08 检测内容无变化跳过空提交）。
    pub committed: bool,
    /// 版本 tag 序号（跳过空提交时为 `None`）。
    pub tag_seq: Option<u32>,
    /// 快照元数据（内容树 hash / 碰撞记录等）。
    pub meta: crate::model::snapshot::SnapshotMeta,
}

/// 命令上下文（§6.9）：持有全部模块句柄，供命令对象执行。
pub struct AppContext {
    /// 存储聚合。
    pub store: BookStore,
    /// 来源适配器注册表。
    pub registry: SourceRegistry,
    /// 位置迁移器（v1 默认 path）。
    pub migrator: Arc<dyn PositionMigrator>,
}
