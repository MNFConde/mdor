//! 统一错误类型：单一 `Error` 枚举贯穿全部模块，变体随用随建（不一次建全）。

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
    /// git 仓库初始化失败。
    #[error("git 初始化失败（{path}）：{source}")]
    GitInit {
        /// 仓库路径。
        path: PathBuf,
        /// gix init 错误。
        source: Box<gix::init::Error>,
    },
    /// git 仓库打开失败。
    #[error("git 打开失败（{path}）：{source}")]
    GitOpen {
        /// 仓库路径。
        path: PathBuf,
        /// gix open 错误。
        source: Box<gix::open::Error>,
    },
    /// git 引用操作失败。
    #[error("git 引用操作失败：{0}")]
    GitRef(#[from] Box<gix::reference::edit::Error>),
    /// git 对象读取失败。
    #[error("git 对象读取失败：{0}")]
    GitFind(#[from] Box<gix::object::find::existing::Error>),
    /// git 对象写入失败。
    #[error("git 对象写入失败：{0}")]
    GitWrite(#[from] Box<gix::object::write::Error>),
    /// git 对象解析（peel）失败。
    #[error("git 对象解析失败：{0}")]
    GitPeel(#[from] Box<gix::object::peel::to_kind::Error>),
    /// git 索引构建失败。
    #[error("git 索引构建失败：{0}")]
    GitIndex(#[from] Box<gix::index::init::from_tree::Error>),
    /// git 索引写盘失败。
    #[error("git 索引写盘失败：{0}")]
    GitIndexWrite(#[from] Box<gix::index::file::write::Error>),
    /// git 检出配置失败。
    #[error("git 检出配置失败：{0}")]
    GitCheckoutOptions(#[from] Box<gix::config::checkout_options::Error>),
    /// git 检出失败。
    #[error("git 检出失败：{0}")]
    GitCheckout(#[from] Box<gix_worktree_state::checkout::Error>),
    /// git 仓库保护配置读取失败。
    #[error("git 保护配置读取失败：{0}")]
    GitProtect(#[from] Box<gix::config::boolean::Error>),
    /// git 配置加载失败。
    #[error("git 配置加载失败：{0}")]
    GitConfigLoad(#[from] Box<gix::config::file::init::Error>),
    /// git 配置写入失败。
    #[error("git 配置写入失败：{0}")]
    GitConfigSet(#[from] Box<gix::config::file::set_raw_value::Error>),
    /// 其他 git 操作失败（无专门变体时兜底）。
    #[error("git 操作失败：{0}")]
    Git(String),
    /// 命令队列已关闭（接收端已退出，入队失败）。
    #[error("命令队列已关闭：{0}")]
    QueueClosed(String),
    /// 实体不存在（如书架中查不到 book_id）。
    #[error("未找到：{0}")]
    NotFound(String),
    /// URL 未被任何已注册来源适配器认识（§4 detect 全部落空）。
    #[error("没有适配器认识该来源：{0}")]
    NoSource(String),
    /// 功能占位（对应里程碑尚未实现，标注所属阶段）。
    #[error("功能未实现：{0}")]
    Unsupported(&'static str),
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

/// 为装箱的 gix 错误变体生成 `From<T>`（大错误类型装箱避免 `Result` 过大，clippy::result_large_err）。
macro_rules! impl_from_boxed {
    ($($variant:ident => $ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for Error {
                fn from(source: $ty) -> Self {
                    Self::$variant(Box::new(source))
                }
            }
        )*
    };
}

impl_from_boxed! {
    GitRef => gix::reference::edit::Error,
    GitFind => gix::object::find::existing::Error,
    GitWrite => gix::object::write::Error,
    GitPeel => gix::object::peel::to_kind::Error,
    GitIndex => gix::index::init::from_tree::Error,
    GitIndexWrite => gix::index::file::write::Error,
    GitCheckoutOptions => gix::config::checkout_options::Error,
    GitCheckout => gix_worktree_state::checkout::Error,
    GitProtect => gix::config::boolean::Error,
    GitConfigLoad => gix::config::file::init::Error,
    GitConfigSet => gix::config::file::set_raw_value::Error,
}
