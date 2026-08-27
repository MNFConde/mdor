use std::path::PathBuf;

use thiserror::Error;

/// mdor-core 统一错误类型（变体随用随建，不一次建全）。
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// 统一错误类型。
#[derive(Debug, Error)]
pub enum Error {
    /// IO 错误（携带路径定位）。
    #[error("IO 错误（{path}）：{source}")]
    Io {
        /// 出错的路径。
        path: PathBuf,
        /// 底层 IO 错误。
        source: std::io::Error,
    },
    /// JSON 序列化/反序列化错误（携带路径定位）。
    #[error("JSON 错误（{path}）：{source}")]
    Json {
        /// 出错的路径。
        path: PathBuf,
        /// serde_json 错误。
        source: serde_json::Error,
    },
    /// 元数据文件超限拦截（`read_json_capped` 纵深防御，§6.7 / D-02）。
    #[error("元数据文件超限（{path}）：{size} 字节超过上限 {max} 字节")]
    Capped {
        /// 超限文件路径。
        path: PathBuf,
        /// 实际大小（字节）。
        size: u64,
        /// 允许上限（字节）。
        max: u64,
    },
}

impl Error {
    /// 包装 IO 错误并携带路径。
    #[must_use]
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// 包装 JSON 错误并携带路径。
    #[must_use]
    pub fn json(path: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        Self::Json {
            path: path.into(),
            source,
        }
    }
}
