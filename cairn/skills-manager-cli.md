---
type: project_topic
status: active
summary: "skills-manager CLI 备份同步机制与常用操作：中央库即普通 git 仓库（DB 不入 git，元数据从 skill 文件重建）；远端已有存档时用 git clone 拉取（git init + set-remote 会撞 unrelated histories）；本地改动 git commit → push 上行、git pull 下行（行级合并）；快照 tag sm-v* 可 versions/restore 回滚；勿用 raw --mirror/--all push 污染 refs/skills-manager/*。opencode 识别 skill 依赖 deploy 到 agent 全局目录的两步机制"
tags: [skills-manager, cli, git, sync, backup, deploy, presets, opencode]
contains: [lesson, procedure]
created: "2026-08-26"
updated: "2026-08-26"
related: [nix-env-tooling.md]
authoring_mode: ai_generated
---
# skills-manager CLI：备份同步机制与常用操作

## 背景

skills-manager 管理跨项目的 skill 中央库（默认 `~/.skills-manager`，库目录 `~/.skills-manager/skills/`）。本专题补充 [nix-env-tooling.md](nix-env-tooling.md)（打包/安装侧）未覆盖的**使用侧**知识：备份/多设备同步机制、远端拉取、本地上报、常用操作速查。

与 opencode 的关系（两步机制）：

1. **同步（git）只动库**：`git pull/push` 只更新中央库 `~/.skills-manager/skills/`。
2. **部署才让 agent 看见**：opencode 只从全局 `~/.config/opencode/skills/<name>/SKILL.md` 等目录发现 skill；需 `skills deploy --agent opencode ...`（或 preset deploy）写入后才生效。`git status` 里 `skills 的 deployed_to: []` = 只入库未部署。

## 方案与机制（备份/多设备同步）

- **中央库即普通 git 仓库**（官方 README：远端是 plain Git repo，`git clone` 任意处、无 lock-in）。git 仓库在 `~/.skills-manager/skills/.git`。
- **DB 不入 git**：`~/.skills-manager/skills-manager.db`（SQLite）只存运行时元数据，从 skill 文件重建。git 备份内容 = skill 目录 + `.skills-manager/{protocol.json,schema.json,scenarios/,skills/}` 元数据 JSON；预设/标签/agent 开关会备份；**密钥/机器信息绝不外传**；>100MB 特大 skill 本地排除（`.gitignore` 内置 `# skills-manager: end oversized skills` 标记段）。
- **上行（本地改动 → 远端）**：CLI 手动 = `git commit -m <msg>` + `git push`。GUI 是「停止编辑数分钟后自动 commit+push」。push 同时推快照 tag（命名 `sm-vYYYYMMDD-HHMMSS<sha>`，如 `sm-v-20260823-105327-3acd638`）。
- **下行（远端 → 本地）**：`git pull`，按 skill 做行级合并（line merge），rename 与 content 编辑可组合。
- **冲突永不阻塞/覆盖**：双方同改一个 skill 时其他照常同步，该 skill 保留本地版进 `pending_conflicts`「待处理」，可选 keep mine / use remote / keep both；任何选择前先打安全快照。merge 时 remote 触碰待处理项会阻塞 ff（`ff blocked (remote touches pending)`）。
- **快照/回滚**：`git versions` 列 tag，`git restore <TAG>` 切到某快照（restore 前先自动备份当前态，失败可回滚）。
- **同步协调 ref**：本地用 `refs/skills-manager/*` 记录同步状态；raw `git push --mirror/--all` 会把它们上传污染远端 → 用 `git prune-sync-refs` 清理（CLI 的 `git push` 不会推这些 ref）。

## 坑

1. **`git pull` 报 `no common history with the remote (unrelated histories)`**（本会话实证）：本机 `git init` 建了独立空骨架历史（提交 "Initial skill library snapshot"，无 skill），远端是另一台机器推的真实备份——两历史无共同祖先（`git status` 的 `upstream_health: "unrelated_histories"` 即此信号）。**本地无数据时修复**：`git fetch origin` + `git reset --hard origin/main`（丢弃空骨架提交，工作区变为远端内容；之后 pull/push 恢复快进）。正路是初始化时直接 `git clone`（见实践指南 Q1）。
2. **同步后 opencode 识别不到**：只同步了库、没部署（两步机制）。修复：`skills deploy --agent opencode <skills>...`（或 preset deploy）→ 重启 opencode 会话生效。
3. **raw `git push --mirror`/`--all` 污染 refs**：会把本地 `refs/skills-manager/*` 协调 ref 上传到备份远端；误 push 后 `git prune-sync-refs` 清理（本地 ref 保留）。
4. **`presets add-skill` 只改 DB 不部署**：membership 存 SQLite `scenario_skills`（scenario_id ↔ skill_id 多对多，含 sort_order），`skill_count`/`preset_ids` 即其投影；要生效还须 `presets deploy <preset> --agent opencode`。

## 实践指南

### Q1 远端已有存档，新装/新机器直接拉取

```bash
skills-manager-cli git clone git@github.com:<owner>/skills-manager-backup.git
# 或 HTTPS/PAT：skills-manager-cli git clone https://github.com/<owner>/skills-manager-backup.git
```

- clone 校验远端 `.skills-manager` 元数据完整（缺 `schema.json` 或 skills 目录报 `incomplete sync metadata snapshot`），成功后重建本地 DB。
- GUI 首启的「restore from backup」等价。
- **不要**先 `git init` 再 `set-remote` + `pull`——远端已有历史时必撞坑 1。
- 已 init 但远端后来接上历史（本会话场景）：走坑 1 的 `fetch` + `reset --hard origin/main`。

### Q2 本地改动同步到远端

```bash
skills-manager-cli git status      # ahead/behind/upstream_health 一目了然
skills-manager-cli git commit -m "备份说明"
skills-manager-cli git push        # 含快照 tag 一并推送
# 拉远端更新：
skills-manager-cli git pull        # 行级合并；堵冲突先处理 pending_conflicts
```

### 常用操作速查

| 分组 | 命令 | 说明 |
|---|---|---|
| git | `status` | ahead/behind/`upstream_health`/snapshot 状态 |
| git | `commit -m <msg>` / `push` / `pull` | 上行备份 / 下行合并 |
| git | `clone <URL>` / `init` / `set-remote <URL>` | 接入已有备份 / 本地新建 / 改远端 |
| git | `versions [--limit N]` / `restore <TAG>` | 列快照 / 回滚（先自动备份当前态） |
| git | `prune-sync-refs` | 清掉被 raw mirror push 上传的 `refs/skills-manager/*` |
| skills | `list [--query/--tag/--preset/--deployed-to]` `show <ref>` | 库查询（`deployed_to: []` = 未部署） |
| skills | `install <ref> [--local|--git|--skillssh] [--sync/--sync-preset]` | 安装（本地路径/git URL/owner-repo[@skill]）；`--sync` 装进当前预设并同步 agent |
| skills | `update [<ref>|--all]` `check [<ref>|--all]` | Git 型 skill 上游更新 / 状态核查 |
| skills | `remove <ref>...` | 移除（`-y`/`--dry-run`） |
| skills | `deploy --agent <agent> <ref>...` / `undeploy --agent <agent> <ref>...` | 写入/撤除 agent 全局技能目录 |
| skills | `sync [--preset <preset>]` | 按激活预设整体同步各工具部署 |
| skills | `export <ref> --dest <dir>` `adopt [path]` `tag add/remove/...` `search <q>` `set-source --git-url <url> <ref>` | 导出 / 收养现有 agent 目录技能 / 打标 / 搜索 / 就地改 git 源 |
| presets | `create <name> [--description]` `current` `list` `show <ref>` | 建组 / 当前组 / 列表 / 详情 |
| presets | `add-skill <preset名或id> <skill名或id>...` `remove-skill ...` | 改组员（只改 DB，需 deploy 生效） |
| presets | `deploy <ref> [--agent]` `undeploy <ref> [--agent]` `status <ref>` | 整组部署/撤除/状态 |
| presets | `apply <ref>` `deactivate <ref>` `delete <ref>` | legacy 独占切换 / 关闭 / 删除 |
| repo | `status` `set-path <path>` | 库路径信息 / 改中央库路径 |

## 开放问题

- 冲突裁决（`pending_conflicts` 的 keep mine/use remote/keep both）在 CLI 未见于命令面（`skills`/`presets`/`git` 全表无对应子命令），疑为 GUI-only——用 GUI「Backup → Needs attention」处理。待新版本 CLI 复核。
- 预设 membership 存在 metadata 往返机制（二进制含 `read_membership_files`/`replace_scenario_memberships_from_metadata`），但空预设时 scenario JSON 无 membership 字段；**有成员后是否把 membership 序列化进 scenario JSON（从而随 git 备份）未实测**——待 `presets add-skill` 后 `git status`/`git diff` 验证。
- `git clone`/`pull` 精确行为依据官方 README + 二进制内嵌字符串（`git2_engine`、snapshot merge）推断，未在本会话实测写入路径。