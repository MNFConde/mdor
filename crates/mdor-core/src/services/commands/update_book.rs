//! 更新书籍命令（SD-3，§6.9 命令化 / §7.3）：remote_version → fetch →
//! 落库（D-08 变更检测内置）→ 位置迁移 → 元数据更新。

use crate::error::{Error, Result};
use crate::model::position::ReadingPosition;
use crate::services::AppContext;
use crate::services::commands::{Command, Progress as StageProgress};
use crate::services::snapshot_pipeline::{SnapshotOptions, commit_snapshot, save_version_record};
use crate::store::library::Library;
use crate::store::progress::Progress;

/// 更新书籍命令（SD-3 流程命令化）：承载一次完整更新流程，命令队列串行执行。
///
/// 中断语义（§6.9）：各阶段幂等——重复执行时 D-08 检测内容无变化即跳过；
/// library.json / progress.json 原子写保证单步中断无半写状态。
pub struct UpdateBookCommand {
    /// 目标书籍 id。
    pub book_id: String,
}

#[async_trait::async_trait]
impl Command for UpdateBookCommand {
    fn name(&self) -> &'static str {
        "update_book"
    }

    fn progress(&self) -> Option<StageProgress> {
        Some(StageProgress::Checking)
    }

    async fn execute(&self, ctx: &AppContext) -> Result<()> {
        let library = ctx.store.library().load()?;
        let book = library
            .books
            .iter()
            .find(|b| b.id == self.book_id)
            .ok_or_else(|| Error::NotFound(format!("书籍 {} 不在书架", self.book_id)))?
            .clone();

        let adapter = ctx
            .registry
            .detect(&book.url)
            .ok_or_else(|| Error::NoSource(book.url.clone()))?;

        // ① remote_version 预检：能在下载前发现"已是最新"时即省流量。
        // 预检只是优化——探测失败（如入口路径形态不同）不阻断更新，降级为
        // None 走完整 fetch，由 D-08 内容树检测兜底判断有无变化。
        let remote = adapter.remote_version(&book.url).await.unwrap_or_else(|e| {
            tracing::debug!(book_id = %book.id, error = %e, "remote_version 预检失败，降级走完整更新");
            None
        });
        if let Some(remote) = &remote
            && let Some(current) = current_source_version(ctx, &book.id)
            && current.as_deref() == Some(remote)
        {
            tracing::info!(book_id = %book.id, "远端版本未变，已是最新");
            return Ok(());
        }

        // ② fetch（镜像 + TOC）。
        let fetched = adapter.fetch(&book.url, &ctx.store.books_root()).await?;

        // ③ 落库（D-08 变更检测内置：内容无变化跳过空提交）。
        let repo_root = ctx.store.books_root().join(&book.id);
        let snapshot = commit_snapshot(
            &repo_root,
            fetched.files(),
            remote.clone(),
            &SnapshotOptions::default(),
        )?;
        if !snapshot.committed {
            tracing::info!(book_id = %book.id, "内容与当前版本一致，无需更新");
            return Ok(());
        }
        save_version_record(
            &repo_root,
            &snapshot.version_id,
            fetched.toc.clone(),
            &snapshot.meta,
        )?;

        // ④ 位置迁移（v1 path 策略，§8.1）：更新追最新。
        //
        // 迁移失败（如新旧版本均无 TOC）不阻断更新主流程——内容已落库，
        // 旧位置保留，仅记 warn（§8.1 位置迁移是更新的附带步骤而非前提）。
        let pos = ctx
            .store
            .progress()
            .load()?
            .positions
            .get(&book.id)
            .cloned();
        if let Some(pos) = pos {
            let from = crate::model::snapshot::VersionSnapshot {
                version_id: book.current_version.clone(),
                workdir: repo_root.join("site"),
                toc: current_toc(ctx, &book.id, &book.current_version),
                meta: crate::model::snapshot::SnapshotMeta::default(),
            };
            let to = crate::model::snapshot::VersionSnapshot {
                version_id: snapshot.version_id.clone(),
                workdir: repo_root.join("site"),
                toc: fetched.toc.clone(),
                meta: snapshot.meta.clone(),
            };
            match ctx.migrator.migrate(&from, &to, &pos).await {
                Ok(migrated) => {
                    let mut progress = ctx.store.progress().load()?;
                    progress.positions.insert(
                        book.id.clone(),
                        ReadingPosition {
                            book_id: book.id.clone(),
                            version_id: migrated.target_version,
                            chapter_path: migrated.target_chapter,
                            heading_anchor: migrated.target_anchor,
                            scroll_ratio: pos.scroll_ratio,
                            saved_at: now_unix(),
                        },
                    );
                    ctx.store.progress().save(&Progress {
                        positions: progress.positions,
                    })?;
                }
                Err(e) => {
                    tracing::warn!(
                        book_id = %book.id,
                        error = %e,
                        "位置迁移失败，保留旧位置"
                    );
                }
            }
        }

        // ⑤ library.json 更新 current_version / updated_at（原子写）。
        let mut library = ctx.store.library().load()?;
        if let Some(b) = library.books.iter_mut().find(|b| b.id == book.id) {
            b.current_version = snapshot.version_id.clone();
            b.updated_at = now_unix();
        }
        ctx.store.library().save(&Library {
            books: library.books,
        })?;

        tracing::info!(
            book_id = %book.id,
            version = %snapshot.version_id,
            tag = ?snapshot.tag_seq,
            "书籍更新完成"
        );
        Ok(())
    }
}

/// 当前版本 meta 的 source_version（预检比对用；无记录返回 None → 走完整更新）。
fn current_source_version(ctx: &AppContext, book_id: &str) -> Option<Option<String>> {
    let library = ctx.store.library().load().ok()?;
    let book = library.books.iter().find(|b| b.id == book_id)?;
    let record = ctx
        .store
        .version_meta(book_id)
        .load(&book.current_version)
        .ok()??;
    Some(record.meta.source_version)
}

/// 当前版本的 TOC（迁移源快照用；无记录给空 TOC → 迁移报错可感知）。
fn current_toc(ctx: &AppContext, book_id: &str, version: &str) -> Vec<crate::model::toc::TocEntry> {
    ctx.store
        .version_meta(book_id)
        .load(version)
        .ok()
        .flatten()
        .map(|r| r.toc)
        .unwrap_or_default()
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
