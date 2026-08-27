use crate::error::{Error, Result};
use crate::services::AppContext;
use crate::services::commands::{Command, Progress};

/// 更新书籍命令（SD-3 流程命令化，§6.9）：承载一次完整更新流程。
///
/// M1 占位：真实管线（remote_version → fetch → commit → 迁移，D-08 变更检测）
/// M2 落地；命令携带执行阶段，Android 切后台/被杀重试可跳过已完成步骤。
pub struct UpdateBookCommand {
    /// 目标书籍 id。
    pub book_id: String,
}

#[async_trait::async_trait]
impl Command for UpdateBookCommand {
    fn name(&self) -> &'static str {
        "update_book"
    }

    fn progress(&self) -> Option<Progress> {
        Some(Progress::Checking)
    }

    async fn execute(&self, _ctx: &AppContext) -> Result<()> {
        let _ = &self.book_id;
        Err(Error::Unsupported("更新书籍管线（M2）"))
    }
}
