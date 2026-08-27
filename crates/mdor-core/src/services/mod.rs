//! 服务编排层（§6.9）：薄门面 AppService + 按需命令化。

use std::sync::Arc;

use crate::migration::PositionMigrator;
use crate::source::SourceRegistry;
use crate::store::BookStore;

pub mod app_service;
pub mod commands;

/// 命令上下文（§6.9）：持有全部模块句柄，供命令对象执行。
pub struct AppContext {
    /// 存储聚合。
    pub store: BookStore,
    /// 来源适配器注册表。
    pub registry: SourceRegistry,
    /// 位置迁移器（v1 默认 path）。
    pub migrator: Arc<dyn PositionMigrator>,
}
