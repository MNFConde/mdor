//! 阅读位置在版本变动后的迁移（§8）：插件化迁移架构，v1 默认 path 策略。

use crate::error::Result;
use crate::model::position::{MigratedPosition, ReadingPosition};
use crate::model::snapshot::VersionSnapshot;

/// 位置迁移插件 trait（§8.2）。
#[async_trait::async_trait]
pub trait PositionMigrator: Send + Sync {
    /// 插件 id（`path` / `snapshot` / `anchor` / `fingerprint`，§8.3）。
    fn id(&self) -> &'static str;
    /// 插件名（设置界面展示）。
    fn name(&self) -> &'static str;
    /// 将旧版本位置迁移到目标版本（可能返回"保持旧版本"）。
    async fn migrate(
        &self,
        from: &VersionSnapshot,
        to: &VersionSnapshot,
        pos: &ReadingPosition,
    ) -> Result<MigratedPosition>;
}

pub mod path;
