//! 文档来源抽象（§4）：来源类型、适配器 trait 与注册表。

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::model::toc::TocEntry;

/// 文档来源类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// git 托管平台（github/gitee/gitlab 等）。
    GitForge,
    /// 静态网站（html、md、pdf 等）。
    StaticSite,
}

/// 一次获取的内容快照（§4）：书籍结构与版本标识，供添加/更新流程消费。
#[derive(Debug, Clone)]
pub struct FetchResult {
    /// 远端版本标识。
    pub version_id: String,
    /// 书名（书架显示）。
    pub title: String,
    /// 章节树。
    pub toc: Vec<TocEntry>,
    /// 镜像文件集（相对路径 → 字节；场景 2 落库输入。GitHub 适配器走独立路径）。
    pub files: std::collections::HashMap<std::path::PathBuf, Vec<u8>>,
}

impl FetchResult {
    /// 文件集访问（落库编排消费；避免泄漏 HashMap 具体类型）。
    #[must_use]
    pub fn files(&self) -> &std::collections::HashMap<std::path::PathBuf, Vec<u8>> {
        &self.files
    }
}

/// 输入适配器 trait（§4）：所有文档来源实现此接口，经 [`SourceRegistry`] 注册、
/// 按 URL 探测。内置适配器 M2（StaticSite）/ M4（GitHub）实现。
#[async_trait::async_trait]
pub trait SourceAdapter: Send + Sync {
    /// 适配器对应的来源类型。
    fn kind(&self) -> SourceKind;
    /// 适配器名（日志 / 调试）。
    fn name(&self) -> &'static str;
    /// 探测：该适配器是否认识这个 URL。
    fn detect(&self, url: &str) -> bool;
    /// 获取/刷新：从 URL 拉取内容写入 dest，返回书籍结构与版本标识。
    async fn fetch(&self, url: &str, dest: &Path) -> Result<FetchResult>;
    /// 版本标识：返回当前远端版本字符串（用于变更检测）。
    async fn remote_version(&self, url: &str) -> Result<Option<String>>;
}

pub mod registry;
pub mod static_site;
pub use registry::SourceRegistry;
pub use static_site::StaticSiteSource;
