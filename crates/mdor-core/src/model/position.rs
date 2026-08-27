use serde::{Deserialize, Serialize};

/// 阅读位置 —— 与具体版本绑定（§5）。
///
/// v1 行为：更新追最新（path 策略，§8.1），位置在更新时按章节路径映射到新版本；
/// 绑定 `version_id` 是为将来方案 D（快照绑定，M5）预留，v1 记录的是当前版本。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadingPosition {
    /// 所属书籍 id（目录名 / 进度索引键）。
    pub book_id: String,
    /// 位置所属版本（commit sha）。
    pub version_id: String,
    /// 章节路径（TOC 中的相对路径）。
    pub chapter_path: String,
    /// 标题锚点（mdBook 输出自带，章节内多标题跳转）。
    pub heading_anchor: Option<String>,
    /// 章节内滚动比例 0.0~1.0。
    pub scroll_ratio: f32,
    /// 保存时间（unix 秒）。
    pub saved_at: i64,
}

/// 位置迁移结果（§5 / §8.1）。
#[derive(Debug, Clone, PartialEq)]
pub struct MigratedPosition {
    /// 迁移到的版本（commit sha）。
    pub target_version: String,
    /// 迁移后的章节路径。
    pub target_chapter: String,
    /// 迁移后的标题锚点。
    pub target_anchor: Option<String>,
    /// 采用的迁移策略。
    pub strategy: MigrateStrategy,
}

/// 迁移策略（v1 仅 path；方案 D / anchor / fingerprint 后续追加，§8.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrateStrategy {
    /// 按章节路径映射（v1 默认）。
    Path,
}
