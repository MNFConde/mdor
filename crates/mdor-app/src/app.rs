use dioxus::prelude::*;

use crate::screens::add_book::AddBookScreen;
use crate::screens::library::LibraryScreen;
use crate::screens::reader::ReaderScreen;
use crate::state::{AppRoute, NAV};

/// 根组件：按 [`NAV`] 路由分发到四屏（§12 screens；D-17 手写 AppRoute 枚举）。
#[component]
pub fn App() -> Element {
    let route = NAV.read();
    match *route {
        AppRoute::Library => LibraryScreen(),
        AppRoute::AddBook => AddBookScreen(),
        AppRoute::Reader => ReaderScreen(),
    }
}
