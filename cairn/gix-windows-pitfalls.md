---
type: project_topic
status: active
summary: "gix 在 Windows 的坑（长路径/大小写/autocrlf 配置规避）与 Windows 文件系统语义坑（保留设备名/路径分隔符/fixtures）、变更检测定案与待 M1 实测项"
tags: [mdor, gix, git, windows, autocrlf, versioning]
contains: [lesson, decision, procedure, open_question]
created: "2026-08-16"
updated: "2026-08-16"
related: [diff.md, decisions.md]
authoring_mode: ai_generated
---
# gix 与 Windows 文件系统坑

## 背景

mdor 以 gix（纯 Rust git 实现）为存储基座，每书一个 git 仓库（`doc/project.md` §7.4 / D-01）。gix 是库而非 CLI、没有 `git config` 命令，Windows 特有坑需在配置侧规避。机制梳理与完整讨论见 `doc/diff.md` §4.5；决策记录见 D-08 / D-09。另 `doc/diff.md` §4.3 的 Windows 文件系统语义坑（保留设备名/路径分隔符/fixtures 大小写）属 **core 通用**（非仅 gix），一并整理于此。

## Lessons

1. **三坑性质不同，不能一刀切**：
   - **autocrlf**：gix 默认遵循配置（含 system/global），会真做 LF↔CRLF 转换——**必须**显式压为 `false`，有明确手段。
   - **ignorecase**：clone/init 时经 `create::Options::fs_capabilities` 探测文件系统并写入 git-config（NTFS 上大概率自动 `core.ignorecase=true`）；只能让索引比较大小写不敏感，**救不了物理冲突**。
   - **longpaths**：260 限制是 Win32 API 限制而非 NTFS；gix 走 Rust `std::fs`（宽字符 API + 超长路径自动 `\\?\`），大概率不需要；设它仅为 git CLI 互操作 / 防御。
2. **全局约定是毒药（关键风险）**：gix 会读到用户机器全局配置——Git for Windows 常写 system 级 `core.autocrlf=true`（实锤案例 helix #6467）。mdor 要求工作区字节 = 上游字节，任何 CRLF 转换都破坏它且与 Android 行为不一致。不能靠「用户改全局配置」这类约定，必须由 mdor 主动在更高优先级压掉。
3. **变更检测别用 gix status 的 stat 快路径**：`lstat()` 的 size/mtime 对比被 mdor 全量重写流击穿——每次 fetch 全量重写工作区 → 每个文件 mtime 都是新的 → 永远落慢路径（全量读盘 + 逐文件 hash），且依赖写盘时序与 stat 缓存边缘情况。

#### Windows 文件系统语义坑（core 通用，非仅 gix，`doc/diff.md` §4.3）

4. **保留设备名不能作文件名**：`CON`/`PRN`/`AUX`/`NUL` 等在 Windows 不能用作文件名（Android 无此限制）——命名避开。
5. **路径分隔符必须走 `Path`/`PathBuf`**：Windows 用 `\`、Android 用 `/`，硬编码拼接必出错；代码里不出现分隔符字面量，fixtures 路径也用 `Path` 抽象（§4.4）。
6. **fixtures 规避同名不同大小写**：NTFS 大小写不敏感、ext4 敏感——仓库同时有 `Foo.md`/`foo.md` 时 Windows checkout 冲突/丢文件（gix 侧注意）；fixtures 避免同名不同大小写；URL 编码层也有 404 风险（`doc/diff.md` §2.3）。

## Current Conclusions

- **配置施加点（A + B 叠加，D-09，待 M1 实测后敲定）**：
  1. `snapshot.rs` 的 clone/init 路径：成功后、checkout 前执行 `apply_windows_safety_config()`，写 repo-local：`core.autocrlf=false`（必须）、`core.longpaths=true`（防御 + git CLI 互操作）；`core.ignorecase` 交给 gix 探测。
  2. AppService 统一仓库打开入口：`config_overrides` 兜底 `core.autocrlf=false`，保证进程内行为确定。
- **变更检测定案（D-08）**：检测层 = 原始字节 hash（下载字节 vs 上个 commit blob hash，前提 autocrlf=false）；展示层 = gix diff（树对象级，与过滤器无关）。分工：hash 回答「内容变没变」，gix diff 回答「变了什么」。**gix status 检测层被否决**（接受字节分叉 → 跨平台同步存储不可用）。
- **大小写冲突不在配置层解决（D-09 定案）**：fetch/clone 后对 tree 做大小写冲突检测，对象层恒两条目——同 blob 归一为一个资源；异 blob 双渲染+标注（默认）/ 报错。**Windows 退化**：NTFS 物理只能落一个文件 → 「单渲染+标注」；跨平台真双渲染绑定 blob 直接读能力（D-10，v1 默认不引入）。
- autocrlf=false 使「磁盘字节 ≡ blob 字节 ≡ 两端字节」，把 gix 当字节透明存储用。

## Open Questions

- 待 M1 实测项（D-09）：gix 在 Windows clone 是否自动写 `core.ignorecase=true`；checkout 超 260 路径是否无碍；模拟 Git for Windows system autocrlf=true 时压成 false 后 checkout 不再转换；碰撞路径 checkout 实际行为；tree 级大小写冲突检测在 fixtures 验证；同 blob / 异 blob 判定（读两路径 blob oid 是否相等）。

## Practice Guide

- gix 提供三个程序化配置入口：`Repository::config_mut()`（落盘 local）、`gix::open::Options::config_overrides`（纯内存）、`gix::config::tree`（类型化 key）。
- 文件系统语义坑（保留设备名/Path 分隔符/fixtures 大小写）见 `doc/diff.md` §4.3/§4.4。
- 详情见 `doc/diff.md` §4.5 与 D-08 / D-09。
