use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    GitForge,   // github/gitee/gitlab 等
    StaticSite, // 静态网站（html、md、pdf 等）
}
