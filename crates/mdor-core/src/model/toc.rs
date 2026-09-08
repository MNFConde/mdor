//! 目录模型（§4/§5）：TOC 章节树条目。

use serde::{Deserialize, Serialize};

/// TOC 章节条目（§4 `FetchResult.toc`；也存于 `.mdor/versions/<sha>.json`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TocEntry {
    /// 章节标题。
    pub title: String,
    /// 章节相对路径（TOC 中的路径，§5 `ReadingPosition.chapter_path` 对齐）。
    pub path: String,
    /// 子章节。
    pub children: Vec<TocEntry>,
}

impl TocEntry {
    /// 按前序遍历收集全部条目（含自身与子孙）。
    #[must_use]
    pub fn flat(&self) -> Vec<&TocEntry> {
        let mut out = vec![self];
        for child in &self.children {
            out.extend(child.flat());
        }
        out
    }
}
