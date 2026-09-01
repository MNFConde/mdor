//! 快照落库编排（§4.2 SD-1 BM 段 / §7.2 场景 2）：镜像文件集 → 自建链 commit
//! → 版本 tag → `.mdor/versions` 落库，含 D-08 变更检测与 D-09 碰撞检测。

use std::path::Path;

use crate::error::{Error, Result};
use crate::model::snapshot::{CaseCollisionRecord, SnapshotMeta};
use crate::source::static_site::FileSet;
use crate::store::snapshot::BookRepo;
use crate::store::version_meta::VersionRecord;
use crate::versioning::next_version_seq;

use super::SnapshotResult;

/// 落库选项（M2 接线 D-09 定案 3 的报错消费分支）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SnapshotOptions {
    /// 异 blob 大小写碰撞 → 报错拒绝落库（true）/ 仅落库记录待标注（false，默认）。
    pub error_on_diff_blob_collision: bool,
}

/// 将文件集作为一次抓取落入书籍仓库（场景 2 自建链，§7.2）。
///
/// 流程：init/open 仓库 → `site/` 相对路径写 blob 建 commit → D-08 变更检测
/// （根 tree oid 相同跳过空提交，返回 `committed: false`）→ 打版本 tag →
/// 碰撞检测（D-09）→ `.mdor/versions/<sha>.json` 落库 → HEAD 置新 commit。
pub fn commit_snapshot(
    repo_root: &Path,
    files: &FileSet,
    source_version: Option<String>,
    opts: &SnapshotOptions,
) -> Result<SnapshotResult> {
    std::fs::create_dir_all(repo_root).map_err(|e| Error::io(repo_root, e))?;
    let repo = if repo_root.join(".git").exists() {
        BookRepo::open(repo_root)?
    } else {
        BookRepo::init(repo_root)?
    };

    // 文件集 → (site/<rel>, bytes)；路径排序保证 tree 构建输入稳定。
    let mut entries: Vec<(std::path::PathBuf, Vec<u8>)> = files
        .iter()
        .map(|(rel, bytes)| (Path::new("site").join(rel), bytes.clone()))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let parent = repo.head_commit()?;

    // D-08 变更检测前置：先 stage tree 与上个 commit 的 tree 比对
    // （autocrlf=false 下字节身份等价），相同则跳过空提交，不写 commit。
    let tree = repo.stage_tree(&entries)?;
    if let Some(parent) = parent
        && repo.tree_of(parent)? == tree
    {
        tracing::info!(head = %parent, "内容无变化，跳过空提交（D-08）");
        return Ok(SnapshotResult {
            version_id: format!("{parent}"),
            committed: false,
            tag_seq: None,
            meta: SnapshotMeta {
                fetched_at: now_unix(),
                source_version,
                content_tree_hash: format!("{tree}"),
                case_collisions: Vec::new(),
            },
        });
    }

    let commit = repo.commit_tree(
        tree,
        &format!("mdor snapshot {source_version:?}"),
        parent.into_iter().collect(),
    )?;

    // 版本 tag：现存最大序号 + 1；HEAD 置新 commit；工作区物化（§9 直读）。
    let existing = repo.list_versions()?;
    let seq = next_version_seq(existing.iter().map(|(s, _)| *s));
    repo.create_version_tag(seq, commit)?;
    repo.set_head(commit)?;
    repo.checkout(commit)?;

    // D-09 碰撞检测（tree 级，平台无关）。
    let collisions = repo.scan_case_collisions(commit)?;
    let diff_blob = collisions.iter().any(|c| !c.same_blob);
    if diff_blob && opts.error_on_diff_blob_collision {
        return Err(Error::CaseCollision {
            paths: collisions
                .iter()
                .filter(|c| !c.same_blob)
                .flat_map(|c| c.paths.iter().map(|p| p.to_string_lossy().into_owned()))
                .collect(),
        });
    }
    let collision_records: Vec<CaseCollisionRecord> = collisions
        .iter()
        .map(|c| CaseCollisionRecord {
            paths: c
                .paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            same_blob: c.same_blob,
        })
        .collect();

    // `.mdor/versions/<sha>.json` 落库（TOC 由调用方补齐前先落 meta 骨架——
    // 本函数接收 files 集合，TOC 构建在 source 侧完成后随 record 保存）。
    let meta = SnapshotMeta {
        fetched_at: now_unix(),
        source_version,
        content_tree_hash: format!("{tree}"),
        case_collisions: collision_records,
    };
    Ok(SnapshotResult {
        version_id: format!("{commit}"),
        committed: true,
        tag_seq: Some(seq),
        meta,
    })
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 落库后保存版本记录（TOC 在 source 侧构建完成后的补写入口）。
pub fn save_version_record(
    repo_root: &Path,
    commit_sha: &str,
    toc: Vec<crate::model::toc::TocEntry>,
    meta: &SnapshotMeta,
) -> Result<()> {
    let store = crate::store::version_meta::VersionMetaStore::new(repo_root);
    store.save(
        commit_sha,
        &VersionRecord {
            toc,
            meta: meta.clone(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::path::PathBuf;

    fn fileset(pairs: &[(&str, &str)]) -> FileSet {
        pairs
            .iter()
            .map(|(p, c)| (PathBuf::from(p), c.as_bytes().to_vec()))
            .collect()
    }

    #[test]
    fn first_snapshot_commits_and_tags() {
        let root = temp_dir("pipe_first").join("book");
        let files = fileset(&[("index.html", "A"), ("css/x.css", "body{}")]);

        let result = commit_snapshot(
            &root,
            &files,
            Some("v0".into()),
            &SnapshotOptions::default(),
        )
        .unwrap();

        assert!(result.committed);
        assert_eq!(result.tag_seq, Some(1));
        assert_eq!(result.version_id.len(), 40, "commit sha");
        assert!(
            root.join("site/index.html").exists(),
            "工作区应物化 site/ 相对路径"
        );
        assert!(root.join(".git").exists());
    }

    #[test]
    fn unchanged_content_skips_empty_commit() {
        let root = temp_dir("pipe_unchanged").join("book");
        let files = fileset(&[("index.html", "A")]);

        let first = commit_snapshot(
            &root,
            &files,
            Some("v0".into()),
            &SnapshotOptions::default(),
        )
        .unwrap();
        assert!(first.committed);

        let second = commit_snapshot(
            &root,
            &files,
            Some("v0".into()),
            &SnapshotOptions::default(),
        )
        .unwrap();
        assert!(!second.committed, "内容相同应跳过空提交（D-08）");
        assert_eq!(second.tag_seq, None);
        assert_eq!(second.version_id, first.version_id, "版本停在上个 commit");

        // tag 不新增
        let repo = BookRepo::open(&root).unwrap();
        assert_eq!(repo.list_versions().unwrap().len(), 1);
    }

    #[test]
    fn changed_content_creates_second_version() {
        let root = temp_dir("pipe_changed").join("book");
        let v1 = fileset(&[("index.html", "A")]);
        let v2 = fileset(&[("index.html", "B"), ("new.html", "N")]);

        let first = commit_snapshot(&root, &v1, None, &SnapshotOptions::default()).unwrap();
        let second = commit_snapshot(&root, &v2, None, &SnapshotOptions::default()).unwrap();

        assert!(second.committed);
        assert_eq!(second.tag_seq, Some(2));
        assert_ne!(first.version_id, second.version_id);

        let repo = BookRepo::open(&root).unwrap();
        assert_eq!(repo.list_versions().unwrap().len(), 2);
        assert_eq!(
            repo.head_commit().unwrap().map(|id| id.to_string()),
            Some(second.version_id.clone())
        );
    }

    #[test]
    fn same_blob_collision_recorded_diff_blob_errors() {
        // 同 blob 碰撞：内容一致 → 落库记录 same_blob=true
        let root = temp_dir("pipe_same").join("book");
        let files = fileset(&[("Readme.html", "SAME"), ("readme.html", "SAME")]);
        let result = commit_snapshot(&root, &files, None, &SnapshotOptions::default()).unwrap();
        assert_eq!(result.meta.case_collisions.len(), 1);
        assert!(result.meta.case_collisions[0].same_blob);
        assert_eq!(result.meta.case_collisions[0].paths.len(), 2);

        // 异 blob 碰撞 + 报错选项 → Err
        let root2 = temp_dir("pipe_diff").join("book");
        let files2 = fileset(&[("Readme.html", "UPPER"), ("readme.html", "lower")]);
        let err = commit_snapshot(
            &root2,
            &files2,
            None,
            &SnapshotOptions {
                error_on_diff_blob_collision: true,
            },
        )
        .unwrap_err();
        assert!(matches!(err, Error::CaseCollision { .. }));

        // 异 blob 碰撞 + 默认选项 → 落库记录
        let root3 = temp_dir("pipe_diff_ok").join("book");
        let result3 = commit_snapshot(&root3, &files2, None, &SnapshotOptions::default()).unwrap();
        assert!(!result3.meta.case_collisions[0].same_blob);
    }

    #[test]
    fn version_record_roundtrip_via_store() {
        let root = temp_dir("pipe_record").join("book");
        let files = fileset(&[("index.html", "A")]);
        let result = commit_snapshot(&root, &files, None, &SnapshotOptions::default()).unwrap();

        let toc = vec![crate::model::toc::TocEntry {
            title: "首".to_string(),
            path: "index.html".to_string(),
            children: vec![],
        }];
        save_version_record(&root, &result.version_id, toc.clone(), &result.meta).unwrap();

        let store = crate::store::version_meta::VersionMetaStore::new(&root);
        let record = store.load(&result.version_id).unwrap().expect("记录存在");
        assert_eq!(record.toc, toc);
        assert_eq!(record.meta.content_tree_hash, result.meta.content_tree_hash);
    }
}
