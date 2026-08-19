use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div {
            h1 { "书架" }
            p { "mdor — 移动端 mdBook 离线阅读器" }
        }
    }
}
