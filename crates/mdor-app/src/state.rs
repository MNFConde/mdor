use std::sync::{Arc, OnceLock};

use mdor_core::services::app_service::AppService;

/// 全局 AppService 句柄（main 启动时一次性初始化；AppService 初始化后不可变，
/// 无需响应式——Dioxus GlobalSignal 因 `UnsyncStorage` 非 Sync 不适用）。
/// M3 阅读器的「当前书籍/版本」等可变状态再引入 GlobalSignal。
pub static APP: OnceLock<Arc<AppService>> = OnceLock::new();
