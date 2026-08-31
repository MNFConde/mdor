//! 来源适配器注册表（§6.2）：detect 链式探测。

use crate::source::SourceAdapter;

/// 输入适配器注册表（§6.2）：持有全部已注册适配器，`detect` 依次询问。
///
/// 新增来源 = 新增一个实现 [`SourceAdapter`] 的模块并注册，核心与 UI 零改动。
/// M1 无内置适配器（StaticSite 留 M2、GitHub 留 M4）。
pub struct SourceRegistry {
    adapters: Vec<Box<dyn SourceAdapter>>,
}

impl Default for SourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceRegistry {
    /// 空注册表（M1：无内置适配器）。
    #[must_use]
    pub fn new() -> Self {
        Self {
            adapters: Vec::new(),
        }
    }

    /// 注册一个适配器。
    pub fn register(&mut self, adapter: Box<dyn SourceAdapter>) {
        self.adapters.push(adapter);
    }

    /// 探测：依次询问已注册适配器，返回首个认识的（§4.2 SD-1）。
    pub fn detect(&self, url: &str) -> Option<&dyn SourceAdapter> {
        self.adapters
            .iter()
            .find(|a| a.detect(url))
            .map(|a| a.as_ref())
    }

    /// 已注册适配器数量（调试 / 日志）。
    #[must_use]
    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    /// 是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }
}
