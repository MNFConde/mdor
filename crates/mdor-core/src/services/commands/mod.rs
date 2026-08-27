//! 命令对象与命令队列（§6.9）：把"一次更新"封装为数据对象，队列串行执行。

pub mod update_book;

use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::error::{Error, Result};
use crate::services::AppContext;

/// 命令执行阶段（§6.9，UI 订阅展示"正在更新…"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// 检查版本。
    Checking,
    /// 下载内容。
    Downloading,
    /// 提交快照。
    Committing,
    /// 迁移位置。
    Migrating,
}

impl Progress {
    /// 人类可读阶段名。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Checking => "检查版本",
            Self::Downloading => "下载内容",
            Self::Committing => "提交快照",
            Self::Migrating => "迁移位置",
        }
    }
}

/// 命令对象（§6.9）：一次完整流程，经命令队列串行执行。
#[async_trait::async_trait]
pub trait Command: Send + Sync + 'static {
    /// 命令名（日志）。
    fn name(&self) -> &'static str;
    /// 当前阶段（供 UI 订阅）。
    fn progress(&self) -> Option<Progress> {
        None
    }
    /// 执行一次完整流程；ctx 持有全部模块句柄。
    async fn execute(&self, ctx: &AppContext) -> Result<()>;
}

/// 命令队列：一次只执行一条（串行化 = §6.7 单进程单写者）。
///
/// `spawn` 需在 tokio 运行时上下文内调用（内部 `tokio::spawn` 要求）。
pub struct CommandQueue {
    tx: mpsc::UnboundedSender<Box<dyn Command>>,
}

impl CommandQueue {
    /// 创建队列并启动消费者任务。
    #[must_use]
    pub fn spawn(ctx: Arc<AppContext>) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<Box<dyn Command>>();
        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                let name = cmd.name();
                let start = Instant::now();
                match cmd.execute(&ctx).await {
                    Ok(()) => tracing::info!(
                        command = name,
                        elapsed_ms = start.elapsed().as_millis(),
                        "命令完成"
                    ),
                    Err(e) => tracing::warn!(
                        command = name,
                        error = %e,
                        elapsed_ms = start.elapsed().as_millis(),
                        "命令失败"
                    ),
                }
            }
        });
        Self { tx }
    }

    /// 入队一条命令（无界通道，即时返回）。
    pub fn enqueue(&self, cmd: Box<dyn Command>) -> Result<()> {
        self.tx
            .send(cmd)
            .map_err(|e| Error::Git(format!("命令队列已关闭：{e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::path::PathMigrator;
    use crate::source::SourceRegistry;
    use crate::store::BookStore;

    struct TestCommand {
        name: &'static str,
        tx: mpsc::UnboundedSender<&'static str>,
    }

    #[async_trait::async_trait]
    impl Command for TestCommand {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn execute(&self, _ctx: &AppContext) -> Result<()> {
            self.tx.send(self.name).unwrap();
            Ok(())
        }
    }

    #[tokio::test]
    async fn queue_executes_commands_serially() {
        let ctx = Arc::new(AppContext {
            store: BookStore::new(std::env::temp_dir().join("mdor-test-queue")),
            registry: SourceRegistry::new(),
            migrator: Arc::new(PathMigrator),
        });
        let queue = CommandQueue::spawn(ctx);
        let (tx, mut rx) = mpsc::unbounded_channel();

        queue
            .enqueue(Box::new(TestCommand {
                name: "a",
                tx: tx.clone(),
            }))
            .unwrap();
        queue
            .enqueue(Box::new(TestCommand { name: "b", tx }))
            .unwrap();

        let mut done = Vec::new();
        for _ in 0..2 {
            done.push(rx.recv().await.unwrap());
        }
        assert_eq!(done, vec!["a", "b"]);
    }
}
