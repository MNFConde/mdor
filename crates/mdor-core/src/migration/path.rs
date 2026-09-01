//! path 迁移策略（§8.1 v1 默认）：按章节路径映射，消失章节回退相邻。

use crate::error::{Error, Result};
use crate::migration::PositionMigrator;
use crate::model::position::{MigrateStrategy, MigratedPosition, ReadingPosition};
use crate::model::snapshot::VersionSnapshot;
use crate::model::toc::TocEntry;

/// v1 默认迁移器：按章节路径映射（§8.1 path 策略）。
///
/// `chapter_path` 在新版本 TOC 中存在 → 直连；消失 → 按 TOC 前序索引回退相邻章节；
/// 目标 TOC 为空 → 报错（无法定位）。
pub struct PathMigrator;

/// 在目标 TOC 中定位章节路径；不存在则按源 TOC 中位置回退相邻，再兜底首个章节。
fn map_path(from_toc: &[TocEntry], to_toc: &[TocEntry], chapter_path: &str) -> Option<String> {
    let to_flat: Vec<&TocEntry> = to_toc.iter().flat_map(|e| e.flat()).collect();

    if let Some(found) = to_flat.iter().find(|e| e.path == chapter_path) {
        return Some(found.path.clone());
    }

    // 章节消失：按源 TOC 前序索引，取目标 TOC 同索引相邻条目
    let from_flat: Vec<&TocEntry> = from_toc.iter().flat_map(|e| e.flat()).collect();
    let index = from_flat.iter().position(|e| e.path == chapter_path)?;
    let fallback_index = index.min(to_flat.len().saturating_sub(1));
    to_flat.get(fallback_index).map(|e| e.path.clone())
}

#[async_trait::async_trait]
impl PositionMigrator for PathMigrator {
    fn id(&self) -> &'static str {
        "path"
    }

    fn name(&self) -> &'static str {
        "按章节路径映射"
    }

    async fn migrate(
        &self,
        from: &VersionSnapshot,
        to: &VersionSnapshot,
        pos: &ReadingPosition,
    ) -> Result<MigratedPosition> {
        let target_chapter = map_path(&from.toc, &to.toc, &pos.chapter_path)
            .ok_or_else(|| Error::Unsupported("目标版本无可用章节（TOC 为空）"))?;
        Ok(MigratedPosition {
            target_version: to.version_id.clone(),
            target_chapter,
            target_anchor: pos.heading_anchor.clone(),
            strategy: MigrateStrategy::Path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::snapshot::SnapshotMeta;
    use std::path::PathBuf;

    fn toc(entries: &[(&str, &str)]) -> Vec<TocEntry> {
        entries
            .iter()
            .map(|(title, path)| TocEntry {
                title: (*title).to_string(),
                path: (*path).to_string(),
                children: vec![],
            })
            .collect()
    }

    fn snapshot(version: &str, toc: Vec<TocEntry>) -> VersionSnapshot {
        VersionSnapshot {
            version_id: version.to_string(),
            workdir: PathBuf::new(),
            toc,
            meta: SnapshotMeta {
                fetched_at: 0,
                source_version: None,
                content_tree_hash: String::new(),
                case_collisions: Vec::new(),
            },
        }
    }

    fn position(chapter: &str) -> ReadingPosition {
        ReadingPosition {
            book_id: "b".to_string(),
            version_id: "v1".to_string(),
            chapter_path: chapter.to_string(),
            heading_anchor: None,
            scroll_ratio: 0.5,
            saved_at: 0,
        }
    }

    #[tokio::test]
    async fn direct_match_keeps_chapter() {
        let migrator = PathMigrator;
        let from = snapshot("v1", toc(&[("一", "ch1.html"), ("二", "ch2.html")]));
        let to = snapshot("v2", toc(&[("一", "ch1.html"), ("三", "ch3.html")]));
        let result = migrator
            .migrate(&from, &to, &position("ch1.html"))
            .await
            .unwrap();
        assert_eq!(result.target_chapter, "ch1.html");
        assert_eq!(result.target_version, "v2");
        assert_eq!(result.strategy, MigrateStrategy::Path);
    }

    #[tokio::test]
    async fn missing_chapter_falls_back_to_adjacent() {
        let migrator = PathMigrator;
        let from = snapshot(
            "v1",
            toc(&[("一", "ch1.html"), ("二", "ch2.html"), ("三", "ch3.html")]),
        );
        let to = snapshot(
            "v2",
            toc(&[("甲", "a.html"), ("乙", "b.html"), ("丙", "c.html")]),
        );
        // ch2.html 在源 TOC 索引 1 → 目标同索引相邻 b.html
        let result = migrator
            .migrate(&from, &to, &position("ch2.html"))
            .await
            .unwrap();
        assert_eq!(result.target_chapter, "b.html");
    }

    #[tokio::test]
    async fn empty_target_toc_errors() {
        let migrator = PathMigrator;
        let from = snapshot("v1", toc(&[("一", "ch1.html")]));
        let to = snapshot("v2", vec![]);
        let result = migrator.migrate(&from, &to, &position("ch1.html")).await;
        assert!(result.is_err());
    }
}
