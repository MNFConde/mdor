//! mdor-core: 核心库，纯 rust，平台无关的核心业务库
#![deny(missing_docs)]
pub mod error;
pub mod migration;
pub mod model;
pub mod services;
pub mod source;
pub mod store;
pub mod versioning;

/// 测试共享辅助：crate 内单测经 `#[cfg(test)]` 引用，避免各测试模块重复定义。
#[cfg(test)]
pub(crate) mod test_support {
    use std::fs;
    use std::path::PathBuf;

    /// 在系统临时目录下创建 `mdor-test-{name}-{pid}` 独占目录（先清残留）。
    pub(crate) fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mdor-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("创建测试临时目录");
        dir
    }
}
