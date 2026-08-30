---
type: project_topic
status: active
summary: "gix 在 Windows 的坑（长路径/大小写/autocrlf 配置规避）与 Windows 文件系统语义坑（保留设备名/路径分隔符/fixtures）、变更检测定案；D-09 已两端实测敲定（Linux 2026-08-27 / Windows 2026-08-30，碰撞 checkout 静默覆盖实证）；gix 0.87 API 实测要点（平台无关，M2/M4 复用）"
tags: [mdor, gix, git, windows, autocrlf, versioning]
contains: [lesson, decision, procedure]
created: "2026-08-16"
updated: "2026-08-30"
related: [diff.md, decisions.md]
authoring_mode: ai_generated
---
# gix 与 Windows 文件系统坑

## 背景

mdor 以 gix（纯 Rust git 实现）为存储基座，每书一个 git 仓库（`doc/project.md` §7.4 / D-01）。gix 是库而非 CLI、没有 `git config` 命令，Windows 特有坑需在配置侧规避。机制梳理与完整讨论见 `doc/diff.md` §4.5；决策记录见 D-08 / D-09。另 `doc/diff.md` §4.3 的 Windows 文件系统语义坑（保留设备名/路径分隔符/fixtures 大小写）属 **core 通用**（非仅 gix），一并整理于此。

## 教训

1. **三坑性质不同，不能一刀切**：
   - **autocrlf**：gix 默认遵循配置（含 system/global），会真做 LF↔CRLF 转换——**必须**显式压为 `false`，有明确手段。
   - **ignorecase**：clone/init 时经 `create::Options::fs_capabilities` 探测文件系统并写入 git-config（NTFS 上**已实证**自动 `core.ignorecase=true`）；只能让索引比较大小写不敏感，**救不了物理冲突**。
   - **longpaths**：260 限制是 Win32 API 限制而非 NTFS；gix 走 Rust `std::fs`（宽字符 API + 超长路径自动 `\\?\`），大概率不需要；设它仅为 git CLI 互操作 / 防御。
2. **全局约定是毒药（关键风险）**：gix 会读到用户机器全局配置——Git for Windows 常写 system 级 `core.autocrlf=true`（实锤案例 helix #6467）。mdor 要求工作区字节 = 上游字节，任何 CRLF 转换都破坏它且与 Android 行为不一致。不能靠「用户改全局配置」这类约定，必须由 mdor 主动在更高优先级压掉。
3. **变更检测别用 gix status 的 stat 快路径**：`lstat()` 的 size/mtime 对比被 mdor 全量重写流击穿——每次 fetch 全量重写工作区 → 每个文件 mtime 都是新的 → 永远落慢路径（全量读盘 + 逐文件 hash），且依赖写盘时序与 stat 缓存边缘情况。

#### Windows 文件系统语义坑（core 通用，非仅 gix，`doc/diff.md` §4.3）

4. **保留设备名不能作文件名**：`CON`/`PRN`/`AUX`/`NUL` 等在 Windows 不能用作文件名（Android 无此限制）——命名避开。
5. **路径分隔符必须走 `Path`/`PathBuf`**：Windows 用 `\`、Android 用 `/`，硬编码拼接必出错；代码里不出现分隔符字面量，fixtures 路径也用 `Path` 抽象（§4.4）。
6. **fixtures 规避同名不同大小写**：NTFS 大小写不敏感、ext4 敏感——仓库同时有 `Foo.md`/`foo.md` 时 Windows checkout 冲突/丢文件（gix 侧注意）；fixtures 避免同名不同大小写；URL 编码层也有 404 风险（`doc/diff.md` §2.3）。

## 当前结论

- **配置施加点（A + B 叠加，D-09，已经 M1 两端实测敲定）**：
  1. `snapshot.rs` 的 clone/init 路径：成功后、checkout 前执行 `apply_windows_safety_config()`，写 repo-local：`core.autocrlf=false`（必须）、`core.longpaths=true`（防御 + git CLI 互操作）；`core.ignorecase` 交给 gix 探测。
  2. AppService 统一仓库打开入口：`config_overrides` 兜底 `core.autocrlf=false`，保证进程内行为确定。
- **变更检测定案（D-08）**：检测层 = 原始字节 hash（下载字节 vs 上个 commit blob hash，前提 autocrlf=false）；展示层 = gix diff（树对象级，与过滤器无关）。分工：hash 回答「内容变没变」，gix diff 回答「变了什么」。**gix status 检测层被否决**（接受字节分叉 → 跨平台同步存储不可用）。
- **大小写冲突不在配置层解决（D-09 定案）**：fetch/clone 后对 tree 做大小写冲突检测，对象层恒两条目——同 blob 归一为一个资源；异 blob 双渲染+标注（默认）/ 报错。**Windows 退化**：NTFS 物理只能落一个文件 → 「单渲染+标注」；跨平台真双渲染绑定 blob 直接读能力（D-10，v1 默认不引入）。
- autocrlf=false 使「磁盘字节 ≡ blob 字节 ≡ 两端字节」，把 gix 当字节透明存储用。

## 开放问题

- **D-09 两端实测已全部完成**（Linux 2026-08-27 切片3 / Windows 2026-08-30 六项回归测试，commit 905666b），无遗留实测项。完整结论见 D-09「实测结论」。
- **Windows 实测关键发现**（2026-08-30）：大小写碰撞 checkout **静默覆盖**（无告警无报错，树序后者胜出、前者字节丢失）——gix 不提供任何知情信号，**tree 级检测前置（D-09 定案 3）是实现「标注」的唯一可行位置**，异 blob 未检测即 checkout = 内容静默丢失。「树序后者胜出」是观察到的实现行为而非 gix 承诺语义，已钉为回归断言（`case_collision_checkout_overwrites_silently`）；实现标注时以 checkout 后实际物理内容为准，勿依赖该顺序。
- **排期定案（2026-08-30）**：定案 3 拆三件——① tree 级检测 + ② 检测结果落库（`SnapshotMeta` 增碰撞字段）归 **M2**（M4 GitHubSource 复用同一检测函数；报错选项 M2 接线）；③ 归一 + 标注渲染归 **M3**（「Windows 单渲染+标注」= 工作区直读 D-10 自然退化 + 标注层，标注不依赖 blob 直接读；Android 物理两文件自然双渲染，M6 验证）。

## gix API 实测（0.87，平台无关，M2/M4 复用）

- **checkout 无高层 Repository API**：`gix::Repository` 不暴露 checkout，须直依赖 `gix-worktree-state`（传递依赖，加为直接依赖即可）。配方：`index::State::from_tree(tree, &repo.objects, validate)` → `index::File::from_state` → `repo.checkout_options(attributes::Source::IdMapping)`（设 `overwrite_existing`）→ `gix_worktree_state::checkout(...)` → `index.write`。progress 参数传 `&progress::Discard` 即可（`count()`/`bytes()` 返回 `Option<Unit>`，非 `Count` impl）。
- **`repo.config.protect_options()` 是 pub(crate) 外部不可用**：`index::State::from_tree` 的 validate 参数用 `gix::validate::path::component::Options::default()`（默认全开最安全，防 untrusted 书内容路径穿越）。
- **`config_snapshot_mut()` 只改进程内配置、不落盘**（doc 明示 in-memory only）：repo-local 配置（D-09 的 autocrlf/longpaths）持久化须走 `gix::config::File::from_path_no_includes(<git_dir>/config, Source::Local)` + `set_raw_value_by` + `write_to`。
- **`gix::init(path)` 在 0.87 直接返回 `Repository`**（非 ThreadSafeRepository）；写对象用 `repo.write_blob`/`write_object`（返回 `Id`，`.detach()` 取 `ObjectId`）；引用操作走 `repo.edit_reference(RefEdit)`（`refs/mdor/versions/...` 自定义 ref 用此；`tag_reference` 只写 `refs/tags/` 不适合私有版本命名空间）。
- **commit 对象类型**：`gix::objs::Commit`（`parents` 是 smallvec，`vec![id].into()` 即可）；签名 `gix::actor::Signature` + `gix::date::Time::now_local_or_utc()`；树条目须按 git 树序排序（目录名按「名 + /」参与字节比较）。

## 实践指南

- gix 提供三个程序化配置入口：`Repository::config_mut()`（落盘 local）、`gix::open::Options::config_overrides`（纯内存）、`gix::config::tree`（类型化 key）。
- 文件系统语义坑（保留设备名/Path 分隔符/fixtures 大小写）见 `doc/diff.md` §4.3/§4.4。
- 详情见 `doc/diff.md` §4.5 与 D-08 / D-09。
