//! 每书 git 仓库操作（gix，§9 / §7.4）：init / 自建 commit / 版本 tag / checkout，
//! 以及 D-09 gix 三坑配置规避的两个施加点。
//!
//! 版本 tag 语义（序号生成等）见 [`crate::versioning`]。

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use gix::hash::ObjectId;
use gix::objs::bstr::BString;
use gix::objs::tree::{EntryKind, EntryMode};
use gix::objs::{Commit, Tree};
use gix::progress;
use gix::refs::Target;
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit};

use crate::error::{Error, Result};
use crate::versioning;

/// 每书一个 git 仓库（场景2 自建链 / 场景1 上游克隆，§7.2）。
pub struct BookRepo {
    repo: gix::Repository,
}

impl BookRepo {
    /// 初始化新仓库（场景2）：`gix::init` + 应用 repo-local 安全配置（D-09 施加点 1）。
    pub fn init(path: &Path) -> Result<Self> {
        let repo = gix::init(path).map_err(|e| Error::GitInit {
            path: path.to_path_buf(),
            source: Box::new(e),
        })?;
        let book = Self { repo };
        book.apply_safety_config()?;
        tracing::debug!(path = %path.display(), "初始化书籍仓库");
        Ok(book)
    }

    /// 打开已有仓库（统一入口，D-09 施加点 2）：`config_overrides` 兜底
    /// `core.autocrlf=false`，保证进程内行为确定，压掉 system/global 配置。
    pub fn open(path: &Path) -> Result<Self> {
        let opts = gix::open::Options::default().config_overrides(["core.autocrlf=false"]);
        let repo = gix::open_opts(path, opts).map_err(|e| Error::GitOpen {
            path: path.to_path_buf(),
            source: Box::new(e),
        })?;
        let book = Self { repo };
        book.apply_safety_config()?;
        tracing::debug!(path = %path.display(), "打开书籍仓库");
        Ok(book)
    }

    /// 写 repo-local 配置：`core.autocrlf=false`（必须，D-09）+ `core.longpaths=true`
    /// （防御 + git CLI 互操作）。`core.ignorecase` 交给 gix 探测，不在此设。
    fn apply_safety_config(&self) -> Result<()> {
        let config_path = self.repo.git_dir().join("config");
        let mut file = gix::config::File::from_path_no_includes(
            config_path.clone(),
            gix::config::Source::Local,
        )
        .unwrap_or_default();
        file.set_raw_value_by("core", None, "autocrlf", "false")?;
        file.set_raw_value_by("core", None, "longpaths", "true")?;
        let writer = std::fs::File::create(&config_path).map_err(|e| Error::io(&config_path, e))?;
        file.write_to(&mut std::io::BufWriter::new(writer))
            .map_err(|e| Error::io(&config_path, e))?;
        Ok(())
    }

    /// 将文件集合（相对路径 → 字节）作为一次快照自建一个 commit，返回 commit id。
    ///
    /// 相同字节 blob 由对象库内容寻址去重（D-01，§7.4）。场景1 的 clone/fetch 上游走
    /// 独立路径（M4），本方法只服务场景2 自建链。
    pub fn commit_workdir(
        &self,
        files: &[(PathBuf, Vec<u8>)],
        message: &str,
        parents: Vec<ObjectId>,
    ) -> Result<ObjectId> {
        let mut file_ids = Vec::with_capacity(files.len());
        for (path, bytes) in files {
            let id = self.repo.write_blob(bytes)?;
            file_ids.push((path.clone(), id.detach()));
        }
        let tree = build_tree(&self.repo, &file_ids)?;

        let time = gix::date::Time::now_local_or_utc();
        let signature = gix::actor::Signature {
            name: "mdor".into(),
            email: "mdor@localhost".into(),
            time,
        };
        let parent_count = parents.len();
        let commit = Commit {
            tree,
            parents: parents.into(),
            author: signature.clone(),
            committer: signature,
            encoding: None,
            message: BString::from(message),
            extra_headers: vec![],
        };
        let id = self.repo.write_object(&commit)?.detach();
        // 物化工作区：workdir 恒等于当前版本内容（§9，本地 http 服务直读）。
        self.checkout(id)?;
        tracing::debug!(files = files.len(), message, parents = parent_count, %id, "自建快照 commit");
        Ok(id)
    }

    /// 设置 HEAD 指向给定 commit（detached；`HEAD` 即当前版本，§7.2）。
    pub fn set_head(&self, commit: ObjectId) -> Result<()> {
        self.edit_ref("HEAD", Target::Object(commit), PreviousValue::Any)
    }

    /// 创建版本 tag `refs/mdor/versions/v<seq>`（必须不存在），返回完整引用名。
    pub fn create_version_tag(&self, seq: u32, commit: ObjectId) -> Result<String> {
        let name = versioning::version_tag_ref(seq);
        self.edit_ref(&name, Target::Object(commit), PreviousValue::MustNotExist)?;
        tracing::debug!(tag = %name, %commit, "创建版本 tag");
        Ok(name)
    }

    /// 列出所有版本 tag：(序号, commit)，按序号升序。
    pub fn list_versions(&self) -> Result<Vec<(u32, ObjectId)>> {
        let platform = self
            .repo
            .references()
            .map_err(|e| Error::Git(e.to_string()))?;
        let mut out = Vec::new();
        for item in platform.all().map_err(|e| Error::Git(e.to_string()))? {
            let reference = item.map_err(|e| Error::Git(e.to_string()))?;
            let name = reference.name().to_string();
            if let Some(seq) = versioning::version_seq_of(&name) {
                out.push((seq, reference.id().detach()));
            }
        }
        out.sort_by_key(|(seq, _)| *seq);
        Ok(out)
    }

    /// HEAD 指向的 commit；仓库尚无提交时为 `None`。
    pub fn head_commit(&self) -> Result<Option<ObjectId>> {
        let mut head = self.repo.head().map_err(|e| Error::Git(e.to_string()))?;
        Ok(head
            .try_peel_to_id()
            .map_err(|e| Error::Git(e.to_string()))?
            .map(|id| id.detach()))
    }

    /// 检出目标 commit 的树到工作区（单一工作区、硬切换，§7.2 历史读取）。
    ///
    /// 硬切换语义（等价 `git reset --hard`）：删除目标树中不存在的旧文件，
    /// 覆盖其余文件；`.git/` 与 `.mdor/`（未被 git 跟踪的元数据）不参与。
    pub fn checkout(&self, commit: ObjectId) -> Result<()> {
        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| Error::Git("仓库无工作区".to_string()))?;
        let object = self.repo.find_object(commit)?;
        let root_tree = object.peel_to_tree()?.id;

        let mut target_paths = Vec::new();
        collect_tree_paths(&self.repo, root_tree, Path::new(""), &mut target_paths)?;
        let keep: HashSet<PathBuf> = target_paths.into_iter().collect();
        remove_untracked_files(workdir, &keep)?;

        let index = gix::index::State::from_tree(
            &root_tree,
            &self.repo.objects,
            gix::validate::path::component::Options::default(),
        )?;
        let mut index = gix::index::File::from_state(index, self.repo.index_path());

        let mut opts = self
            .repo
            .checkout_options(gix_worktree::stack::state::attributes::Source::IdMapping)?;
        opts.overwrite_existing = true;

        let files = progress::Discard;
        let bytes = progress::Discard;
        let interrupt = AtomicBool::new(false);
        gix_worktree_state::checkout(
            &mut index,
            workdir,
            self.repo
                .objects
                .clone()
                .into_arc()
                .map_err(|e| Error::io(workdir, e))?,
            &files,
            &bytes,
            &interrupt,
            opts,
        )?;
        index.write(Default::default())?;
        Ok(())
    }

    /// 编辑单条引用（公共封装：HEAD 与版本 tag 共用）。
    fn edit_ref(&self, name: &str, new: Target, expected: PreviousValue) -> Result<()> {
        let edit = RefEdit {
            change: Change::Update {
                log: LogChange::default(),
                expected,
                new,
            },
            name: name
                .try_into()
                .map_err(|e| Error::Git(format!("无效引用名 {name}：{e}")))?,
            deref: false,
        };
        self.repo.edit_reference(edit)?;
        Ok(())
    }
}

/// 递归构建 git tree（支持嵌套路径），并按 git 树序（目录按名 + `/` 参与排序）排序。
fn build_tree(repo: &gix::Repository, files: &[(PathBuf, ObjectId)]) -> Result<ObjectId> {
    let mut blob_entries: Vec<(String, ObjectId)> = Vec::new();
    let mut dir_groups: std::collections::BTreeMap<String, Vec<(PathBuf, ObjectId)>> =
        std::collections::BTreeMap::new();

    for (path, oid) in files {
        let mut components = path.components();
        let first = components
            .next()
            .ok_or_else(|| Error::Git(format!("空相对路径：{path:?}")))?;
        let name = first.as_os_str().to_string_lossy().into_owned();
        let rest = components.as_path();
        if rest.as_os_str().is_empty() {
            blob_entries.push((name, *oid));
        } else {
            dir_groups
                .entry(name)
                .or_default()
                .push((rest.to_path_buf(), *oid));
        }
    }

    let mut entries: Vec<gix::objs::tree::Entry> = Vec::new();
    for (name, oid) in blob_entries {
        entries.push(entry(EntryKind::Blob, name, oid));
    }
    for (name, sub_files) in dir_groups {
        let sub_oid = build_tree(repo, &sub_files)?;
        entries.push(entry(EntryKind::Tree, name, sub_oid));
    }

    // git 树序：目录名按「名 + /」参与字节比较（git 约定），保证对象哈希与 git 一致。
    entries.sort_by(|a, b| {
        fn sort_key(e: &gix::objs::tree::Entry) -> Vec<u8> {
            let mut key = e.filename.to_vec();
            if e.mode.is_tree() {
                key.push(b'/');
            }
            key
        }
        sort_key(a).cmp(&sort_key(b))
    });

    let tree = Tree { entries };
    Ok(repo.write_object(&tree)?.detach())
}

fn entry(kind: EntryKind, name: String, oid: ObjectId) -> gix::objs::tree::Entry {
    gix::objs::tree::Entry {
        mode: EntryMode::from(kind),
        filename: name.into(),
        oid,
    }
}

/// 收集树内全部 blob 的（相对工作区）路径。
fn collect_tree_paths(
    repo: &gix::Repository,
    tree_id: ObjectId,
    prefix: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    let tree = repo.find_object(tree_id)?.peel_to_tree()?;
    let decoded = tree.decode().map_err(|e| Error::Git(e.to_string()))?;
    for entry in &decoded.entries {
        let rel = prefix.join(entry.filename.to_string());
        if entry.mode.is_tree() {
            collect_tree_paths(repo, entry.oid.to_owned(), &rel, out)?;
        } else {
            out.push(rel);
        }
    }
    Ok(())
}

/// 删除工作区中不在 `keep` 集合内的文件（`.git/` 与 `.mdor/` 除外）。
fn remove_untracked_files(workdir: &Path, keep: &HashSet<PathBuf>) -> Result<()> {
    fn walk(dir: &Path, rel: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        for entry in std::fs::read_dir(dir).map_err(|e| Error::io(dir, e))? {
            let entry = entry.map_err(|e| Error::io(dir, e))?;
            let child_rel = rel.join(entry.file_name());
            if entry.file_type().map_err(|e| Error::io(dir, e))?.is_dir() {
                if child_rel == Path::new(".git") || child_rel == Path::new(".mdor") {
                    continue;
                }
                walk(&entry.path(), &child_rel, out)?;
            } else {
                out.push(child_rel);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(workdir, Path::new(""), &mut files)?;
    for file in files {
        if !keep.contains(&file) {
            let path = workdir.join(&file);
            std::fs::remove_file(&path).map_err(|e| Error::io(&path, e))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_repo(name: &str) -> (PathBuf, BookRepo) {
        let dir = std::env::temp_dir().join(format!("mdor-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("创建测试临时目录");
        let repo = BookRepo::init(&dir).expect("init");
        (dir, repo)
    }

    fn files(pairs: &[(&str, &str)]) -> Vec<(PathBuf, Vec<u8>)> {
        pairs
            .iter()
            .map(|(p, c)| (PathBuf::from(p), c.as_bytes().to_vec()))
            .collect()
    }

    /// 沿树路径取条目 oid（辅助断言）。
    fn tree_lookup(repo: &gix::Repository, tree_id: ObjectId, path: &str) -> Option<ObjectId> {
        let mut current = tree_id;
        let comps: Vec<&str> = path.split('/').collect();
        for comp in &comps {
            let tree_obj = repo.find_object(current).expect("树存在");
            let tree = tree_obj.peel_to_tree().expect("peel 树");
            let decoded = tree.decode().expect("解码树");
            let found = decoded
                .entries
                .iter()
                .find(|e| e.filename == comp.as_bytes())?;
            if comp == comps.last().unwrap() {
                return Some(found.oid.to_owned());
            }
            current = found.oid.to_owned();
        }
        None
    }

    #[test]
    fn init_writes_safety_config() {
        let (dir, _) = temp_repo("init_config");
        let raw = fs::read_to_string(dir.join(".git/config")).unwrap();
        assert!(
            raw.contains("autocrlf = false"),
            "必须写 autocrlf=false：{raw}"
        );
        assert!(
            raw.contains("longpaths = true"),
            "必须写 longpaths=true：{raw}"
        );
    }

    #[test]
    fn commit_then_tag_lists_versions() {
        let (_, repo) = temp_repo("commit_tag");
        let id1 = repo
            .commit_workdir(
                &files(&[("index.html", "A"), ("guide/1.html", "B")]),
                "v1 内容",
                vec![],
            )
            .unwrap();
        repo.set_head(id1).unwrap();
        repo.create_version_tag(1, id1).unwrap();

        let id2 = repo
            .commit_workdir(
                &files(&[("index.html", "A"), ("guide/2.html", "C")]),
                "v2 内容",
                vec![id1],
            )
            .unwrap();
        repo.set_head(id2).unwrap();
        repo.create_version_tag(2, id2).unwrap();

        let versions = repo.list_versions().unwrap();
        assert_eq!(versions, vec![(1, id1), (2, id2)]);
        assert_eq!(repo.head_commit().unwrap(), Some(id2));
    }

    #[test]
    fn unchanged_blob_dedup_across_commits() {
        let (_, repo) = temp_repo("dedup");
        let id1 = repo
            .commit_workdir(&files(&[("index.html", "same")]), "v1", vec![])
            .unwrap();
        let id2 = repo
            .commit_workdir(
                &files(&[("index.html", "same"), ("extra.html", "new")]),
                "v2",
                vec![id1],
            )
            .unwrap();

        let root1 = repo
            .repo
            .find_object(id1)
            .unwrap()
            .peel_to_tree()
            .unwrap()
            .id;
        let root2 = repo
            .repo
            .find_object(id2)
            .unwrap()
            .peel_to_tree()
            .unwrap()
            .id;

        let blob1 = tree_lookup(&repo.repo, root1, "index.html").unwrap();
        let blob2 = tree_lookup(&repo.repo, root2, "index.html").unwrap();
        assert_eq!(blob1, blob2, "未变内容 blob 应去重（同一 oid）");
    }

    #[test]
    fn checkout_switches_workdir_content() {
        let (dir, repo) = temp_repo("checkout");
        let id1 = repo
            .commit_workdir(&files(&[("index.html", "A")]), "v1", vec![])
            .unwrap();
        repo.set_head(id1).unwrap();
        let id2 = repo
            .commit_workdir(
                &files(&[("index.html", "B"), ("guide/2.html", "C")]),
                "v2",
                vec![id1],
            )
            .unwrap();
        repo.set_head(id2).unwrap();

        repo.checkout(id1).unwrap();
        assert_eq!(fs::read_to_string(dir.join("index.html")).unwrap(), "A");
        assert!(
            !dir.join("guide/2.html").exists(),
            "切回 v1 不应有 v2 新增文件"
        );

        repo.checkout(id2).unwrap();
        assert_eq!(fs::read_to_string(dir.join("index.html")).unwrap(), "B");
        assert_eq!(fs::read_to_string(dir.join("guide/2.html")).unwrap(), "C");
    }

    #[test]
    fn autocrlf_false_preserves_crlf_bytes() {
        let (dir, repo) = temp_repo("crlf");
        let crlf = b"line1\r\nline2\r\n";
        let id = repo
            .commit_workdir(
                &[(PathBuf::from("win.html"), crlf.to_vec())],
                "CRLF 内容",
                vec![],
            )
            .unwrap();

        let root = repo
            .repo
            .find_object(id)
            .unwrap()
            .peel_to_tree()
            .unwrap()
            .id;
        let blob = tree_lookup(&repo.repo, root, "win.html").unwrap();
        let object = repo.repo.find_object(blob).unwrap();
        assert_eq!(object.kind, gix::objs::Kind::Blob);
        assert_eq!(object.data, crlf, "autocrlf=false 下工作区字节 ≡ blob 字节");
        assert_eq!(fs::read(dir.join("win.html")).unwrap(), crlf.to_vec());
    }

    // ===== D-09 Windows 侧实测（plan.todo M1 遗留；decisions.md D-09 待实测项）=====
    // 仅在 Windows 上编译执行（需 NTFS 语义 + Git for Windows system 配置；ubuntu CI 编译期剔除）。
    // 探针轮结论（2026-08-30，Windows 宿主 NTFS + gix 0.87.1 + Git for Windows system autocrlf=true）：
    //   ① init 自动探测写 core.ignorecase=true；② >260 长路径 checkout 无碍；
    //   ③ 裸 init 可见 system autocrlf=true（毒药环境实锤），双施加点压成 false；
    //   ④ 大小写碰撞 checkout 静默覆盖（无告警，树序后者胜出）——tree 级检测前置的实证依据。
    #[cfg(windows)]
    mod windows_gix {
        use super::*;

        /// 实测项 1：NTFS ignorecase 自动探测——gix init 时经 `Capabilities::probe`
        /// 探测文件系统并写入 `core.ignorecase`，且经 apply_safety_config 重写后仍保留。
        #[test]
        fn init_probes_ignorecase_true() {
            let (dir, repo) = temp_repo("win_ignorecase");
            let raw = fs::read_to_string(dir.join(".git/config")).unwrap();
            assert!(
                raw.contains("ignorecase = true"),
                "NTFS 上 gix init 应探测并写入 core.ignorecase=true：{raw}"
            );
            let caps = repo.repo.filesystem_options().unwrap();
            assert!(
                caps.ignore_case,
                "open 后 fs_capabilities 应报告 ignore_case=true"
            );
        }

        /// 实测项 2：>260 路径 checkout——gix 走 Rust std::fs（长路径自动 \\?\ 前缀），
        /// 预期无碍。临时目录基路径 ~60 字符 + 相对路径 ~300 字符，全路径 >350。
        #[test]
        fn checkout_long_path_beyond_260() {
            let (dir, repo) = temp_repo("win_longpath");
            let seg = "n".repeat(40);
            let mut rel = PathBuf::new();
            for _ in 0..7 {
                rel.push(&seg);
            }
            rel.push("leaf.html");
            let total = dir.join(&rel).to_string_lossy().len();
            assert!(total > 260, "构造的全路径应超 260：实际 {total}");

            let content = b"deep-content".to_vec();
            let id = repo
                .commit_workdir(&[(rel.clone(), content.clone())], "长路径", vec![])
                .unwrap();
            let path = dir.join(&rel);
            assert_eq!(fs::read(&path).unwrap(), content, "长路径文件字节应一致");

            repo.checkout(id).unwrap();
            assert_eq!(fs::read(&path).unwrap(), content, "硬切换复检字节应一致");
        }

        /// 实测项 3：system 级 `core.autocrlf=true`（本机 Git for Windows 实况，helix
        /// #6467 同款毒药环境）被压成 false——repo-local（施加点 1）+ open
        /// config_overrides（施加点 2）叠加后有效值必须为 false，且经 open 实例
        /// commit + checkout 的 CRLF 字节端到端保真。
        #[test]
        fn open_suppresses_system_autocrlf() {
            // 探针仓库放书籍工作区之外：裸 init（无 local autocrlf），验证 gix
            // 确实读到 system 级配置（毒药环境存在性前提）。
            let probe_path = std::env::temp_dir().join("mdor-d09-probe-raw");
            let _ = fs::remove_dir_all(&probe_path);
            let probe = gix::init(&probe_path).expect("裸 init 探针仓库");
            assert_eq!(
                probe.config_snapshot().boolean("core.autocrlf"),
                Some(true),
                "前提：gix 裸 init 应读到 system 级 autocrlf=true（本机毒药环境）"
            );

            let (dir, _unused) = temp_repo("win_autocrlf");
            let reopened = BookRepo::open(&dir).unwrap();
            assert_eq!(
                reopened.repo.config_snapshot().boolean("core.autocrlf"),
                Some(false),
                "open 后有效 autocrlf 必须为 false（双施加点压掉 system=true）"
            );

            let crlf = b"<p>win\r\nline</p>\r\n".to_vec();
            let id = reopened
                .commit_workdir(&[(PathBuf::from("win.html"), crlf.clone())], "CRLF", vec![])
                .unwrap();
            assert_eq!(
                fs::read(dir.join("win.html")).unwrap(),
                crlf,
                "open 实例 commit 后工作区字节应保真"
            );
            let root = reopened
                .repo
                .find_object(id)
                .unwrap()
                .peel_to_tree()
                .unwrap()
                .id;
            let blob = tree_lookup(&reopened.repo, root, "win.html").unwrap();
            assert_eq!(
                reopened.repo.find_object(blob).unwrap().data,
                crlf,
                "open 实例 commit 后 blob 字节应保真"
            );
        }

        /// 实测项 4：大小写碰撞 checkout 实际行为。
        /// 探针轮结论：commit 成功、checkout **静默覆盖**（无告警无报错），NTFS 物理单
        /// 文件，内容 = 树序后者（readme.html → "lower"）。钉为回归断言：
        /// 若 gix 行为变化（如加告警/报错），此处失败即提示重评 D-09 定案 3。
        #[test]
        fn case_collision_checkout_overwrites_silently() {
            let (dir, repo) = temp_repo("win_case_checkout");
            repo.commit_workdir(
                &files(&[("Readme.html", "UPPER"), ("readme.html", "lower")]),
                "大小写碰撞",
                vec![],
            )
            .expect("碰撞条目 commit 应成功（对象层两条目共存）");

            // NTFS 物理只能落一个文件；两种 case 的路径查询都命中同一物理文件。
            assert!(
                dir.join("Readme.html").exists() || dir.join("readme.html").exists(),
                "碰撞 checkout 应落一个物理文件"
            );
            let physical = fs::read_to_string(dir.join("Readme.html")).unwrap_or_else(|_| {
                fs::read_to_string(dir.join("readme.html")).expect("物理文件可读")
            });
            assert_eq!(
                physical, "lower",
                "静默覆盖：树序后者（readme.html）胜出，前一条目字节丢失"
            );
        }

        /// 实测项 5：tree 级大小写冲突检测的前提——对象层两条目恒共存。
        /// 用 build_tree 直构（绕过 checkout；物理层行为归实测项 4）。
        #[test]
        fn case_collision_tree_keeps_both_entries() {
            let (_, repo) = temp_repo("win_case_tree");
            let oid_up = repo.repo.write_blob(b"UPPER").unwrap().detach();
            let oid_low = repo.repo.write_blob(b"lower").unwrap().detach();
            let tree_id = build_tree(
                &repo.repo,
                &[
                    (PathBuf::from("Readme.html"), oid_up),
                    (PathBuf::from("readme.html"), oid_low),
                ],
            )
            .unwrap();
            let tree = repo
                .repo
                .find_object(tree_id)
                .unwrap()
                .peel_to_tree()
                .unwrap();
            let names: Vec<String> = tree
                .decode()
                .unwrap()
                .entries
                .iter()
                .map(|e| e.filename.to_string())
                .collect();
            assert!(
                names.contains(&"Readme.html".to_string())
                    && names.contains(&"readme.html".to_string()),
                "对象层应两条目共存：{names:?}"
            );
            assert_eq!(
                tree_lookup(&repo.repo, tree_id, "Readme.html").unwrap(),
                oid_up
            );
            assert_eq!(
                tree_lookup(&repo.repo, tree_id, "readme.html").unwrap(),
                oid_low
            );
        }

        /// 实测项 6：同/异 blob 判定——读两路径 blob oid 是否相等
        ///（D-09「同 blob 归一 / 异 blob 双渲染」的判定原语）。
        #[test]
        fn case_collision_blob_oid_discrimination() {
            let (_, repo) = temp_repo("win_case_oid");
            let same = b"same-bytes";
            let oids = [
                repo.repo.write_blob(same).unwrap().detach(),
                repo.repo.write_blob(same).unwrap().detach(),
                repo.repo.write_blob(b"A").unwrap().detach(),
                repo.repo.write_blob(b"B").unwrap().detach(),
            ];
            let tree_id = build_tree(
                &repo.repo,
                &[
                    (PathBuf::from("Same.html"), oids[0]),
                    (PathBuf::from("same.html"), oids[1]),
                    (PathBuf::from("Diff.html"), oids[2]),
                    (PathBuf::from("diff.html"), oids[3]),
                ],
            )
            .unwrap();
            let up = tree_lookup(&repo.repo, tree_id, "Same.html").unwrap();
            let low = tree_lookup(&repo.repo, tree_id, "same.html").unwrap();
            let d1 = tree_lookup(&repo.repo, tree_id, "Diff.html").unwrap();
            let d2 = tree_lookup(&repo.repo, tree_id, "diff.html").unwrap();
            assert_eq!(up, low, "同内容异 case：oid 相等 → 归一判定可行");
            assert_ne!(d1, d2, "异内容异 case：oid 不等 → 异 blob 判定可行");
        }
    }
}
