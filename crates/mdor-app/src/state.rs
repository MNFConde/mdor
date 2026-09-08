use std::sync::{Arc, OnceLock};

use dioxus::signals::GlobalSignal;
use mdor_core::model::toc::TocEntry;
use mdor_core::services::app_service::{AppService, OpenReading};

/// 全局 AppService 句柄（main 启动时一次性初始化；AppService 初始化后不可变，
/// 无需响应式——Dioxus GlobalSignal 因 `UnsyncStorage` 非 Sync 不适用）。
/// M3 阅读器的「当前书籍/版本」等可变状态另用 [`NAV`] 的 GlobalSignal。
pub static APP: OnceLock<Arc<AppService>> = OnceLock::new();

/// 当前导航路由（§12 screens 四屏平面跳转；不含 URL 参数，见 D-17）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppRoute {
    /// 书架（默认落地页）。
    Library,
    /// 添加书籍。
    AddBook,
    /// 阅读（需要 [`ReaderState`] 已打开书籍）。
    Reader,
}

/// 阅读器可变状态（当前打开的书籍 / 章节 / 进度定位）。
#[derive(Clone, Debug)]
pub struct ReaderState {
    /// 当前打开的书籍 id（无则 Reader 屏不可达）。
    pub book_id: Option<String>,
    /// `open_reading` 返回的完整结果（含 TOC 与章节 HTML；切章时更新 chapter_html）。
    pub open: Option<OpenReading>,
    /// 当前章节路径（`ReadingPosition.chapter_path` 对齐）。
    pub chapter_path: Option<String>,
    /// 初始标题锚点（进度定位；切章后清空）。
    pub initial_anchor: Option<String>,
    /// 初始滚动比例（进度定位；切章后重置）。
    pub scroll_ratio: f32,
}

impl Default for ReaderState {
    fn default() -> Self {
        Self {
            book_id: None,
            open: None,
            chapter_path: None,
            initial_anchor: None,
            scroll_ratio: 0.0,
        }
    }
}

impl ReaderState {
    /// 当前打开的 TOC 章节树（未打开或空返回空切片）。
    #[must_use]
    pub fn toc(&self) -> &[TocEntry] {
        self.open.as_ref().map(|o| o.toc.as_slice()).unwrap_or(&[])
    }

    /// 当前章节正文 HTML（未打开/未渲染返回空串）。
    #[must_use]
    pub fn chapter_html(&self) -> &str {
        self.open
            .as_ref()
            .map(|o| o.chapter_html.html.as_str())
            .unwrap_or("")
    }

    /// 当前章节锚点（供目录抽取 / 页内跳转）。
    #[must_use]
    pub fn anchors(&self) -> &[String] {
        self.open
            .as_ref()
            .map(|o| o.chapter_html.anchors.as_slice())
            .unwrap_or(&[])
    }

    /// 当前章节标题（无则空串）。
    #[must_use]
    pub fn chapter_title(&self) -> &str {
        self.open
            .as_ref()
            .map(|o| o.chapter_html.title.as_str())
            .unwrap_or("")
    }
}

/// 全局导航信号（M3：当前路由 + 阅读器状态）。
///
/// `GlobalSignal` 依赖 Dioxus `UnsyncStorage`，只能在组件上下文内使用；`APP`
/// 因 `UnsyncStorage` 非 Sync 不适用，故 AppService 句柄仍走 `OnceLock`。
pub static NAV: GlobalSignal<AppRoute> = GlobalSignal::new(|| AppRoute::Library);

/// 全局阅读器状态信号。
pub static READER: GlobalSignal<ReaderState> = GlobalSignal::new(ReaderState::default);
