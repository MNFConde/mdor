//! 集成测试共享工具（tests/ 目录首建，26-08-31 定案）。
//!
//! `test_support`（crate 内 `pub(crate)`）不外借集成测试，此处独立实现。

#![deny(missing_docs)]

use std::fs;
use std::path::PathBuf;

/// 在系统临时目录下创建 `mdor-it-{name}-{pid}` 独占目录（先清残留）。
pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mdor-it-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("创建测试临时目录");
    dir
}

/// 预构建 mdBook fixture 站点根（`fixtures/mdbook-static/`，mdbook build 产物）。
#[allow(dead_code)]
pub fn mdbook_fixture_root() -> PathBuf {
    let candidates = [
        PathBuf::from("../fixtures/mdbook-static"),
        PathBuf::from("../../fixtures/mdbook-static"),
    ];
    for dir in &candidates {
        if dir.join("index.html").is_file() {
            return dir.canonicalize().expect("fixture 目录存在");
        }
    }
    panic!("找不到 fixtures/mdbook-static/（候选 {candidates:?}）；集成测试需以仓库内相对路径运行");
}
