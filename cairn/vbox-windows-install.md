---
type: project_topic
status: active
summary: "VirtualBox Windows 自定义目录安装坑链：「invalid installation directory」实为 7.0.14+ 目录安全校验；官方 icacls 配方的 Deny ACE 反噬管理员致 MSI 1303；正确做法 inheritance:r 断继承不授写；%TEMP% MSI 日志排查法"
tags: [mdor, virtualbox, windows, installer, acl, icacls, troubleshooting]
contains: [lesson, procedure, experience]
created: "2026-08-24"
updated: "2026-08-24"
related: [decisions.md, env.md]
authoring_mode: ai_generated
---
# VirtualBox Windows 自定义目录安装坑

## 背景

为搭建 mdor 开发环境（见 [D-16](../doc/decisions.md#d-16-开发环境三端架构)）在 Windows 宿主机安装 VirtualBox 7.2.16 并指定自定义安装目录 `D:\VirtualBox`，连续踩中两道门：先报「invalid installation directory」类错误，改 ACL 后又报「没有足够的特权访问」。本文记录完整坑链与解法，适用于任意 VirtualBox 7.x 自定义目录安装场景。

## 教训

1. **报错名是社区讹传**：网上（含中文社区大量文章）流传的「invalid installation dictionary」并非官方文案——真实错误为 **"The chosen installation directory is invalid, as it does not meet the security requirements"**。搜「dictionary」搜不到解法，搜官方原文或「无效安装目录 安全要求」才有结果。
2. **7.0.14 起有安装目录安全校验**：自定义目录（及其**全部父目录**）必须满足 DACL 要求——普通用户（Users / Authenticated Users）不可写入/重命名，且禁用继承。装到一级目录只需处理该目录自身；多级父目录每层都要满足。默认 `C:\Program Files\Oracle\VirtualBox` 天然合规。
3. **官方手册的 icacls Deny 配方会反噬安装器本身**：按官方文档对目录 `/deny *S-1-5-32-545:(DE,WD,AD,WEA,WA)` 和 `/deny *S-1-5-11:(DE,WD,AD,WEA,WA)` 后，目录校验能通过（MSI 属性 `VBox_Target_Dir_Is_Valid = 1`），但安装中途创建子目录时报 **MSI 错误 1303「安装程序没有足够的特权访问该目录」并整体回滚**。根因：任何登录账户的令牌都必然包含 `Authenticated Users`（S-1-5-11）组——包括以管理员身份运行的安装器进程；ACL 规范里 Deny 恒优先于 Allow，于是管理员自己也被 Deny 挡住。论坛上该配方「时灵时不灵」即源于此。
4. **正确配方：不用 Deny，断继承即可**：
   ```bat
   icacls D:\VBox /remove:d *S-1-5-11 *S-1-5-32-545   # 先清掉旧 Deny
   icacls D:\VBox /remove:g *S-1-5-11 *S-1-5-32-545   # 清掉残留授权（如继承转显式的 Modify）
   icacls D:\VBox /inheritance:r                       # 断开继承、丢弃继承来的 ACE
   icacls D:\VBox /grant *S-1-5-32-544:(OI)(CI)(F)     # Administrators 完全控制
   icacls D:\VBox /grant *S-1-5-18:(OI)(CI)(F)         # SYSTEM 完全控制
   icacls D:\VBox /grant *S-1-5-32-545:(OI)(CI)(RX)    # Users 仅读+执行
   ```
   「普通用户无写权限」靠**不授予**而非显式拒绝实现——同样满足安全校验，且不挡管理员与 SYSTEM。注意 `/inheritance:d`（转显式保留）会把父目录的高权限 ACE 固化为显式条目残留，须配合 remove 清干净。
5. **排查入口是 `%TEMP%\MSI*.LOG`**：GUI 报错文案模糊，真正的诊断信息在 MSI 日志里——搜 `VBox_Target_Dir_Is_Valid`（0 = 目录校验拒绝，弹窗发生在选完目录点 Install 时；1 = 通过，若仍失败看后续）与 `Note: 1: <错误码>`（1303 = 权限不足，附具体卡住的路径）。
6. **镜像站文件可用官方 SHA256SUMS 校验**：`https://download.virtualbox.org/virtualbox/<版本>/SHA256SUMS` 列全平台包哈希；实测某国内镜像重命名的 `virtualbox-Win-latest.exe` 与官方 `VirtualBox-7.2.16-174877-Win.exe` 字节一致——文件没问题时应转向环境因素排查，不要在重下上空转。

## 当前结论

- 自定义目录安装的标准流程：**管理员 CMD**（勿用 PowerShell，`*S-1-5-…` 通配符 SID 在其中会被展开导致命令失效）执行第 4 条配方 → 管理员身份运行安装器 → 选该目录。
- 无 ACL 折腾需求的场景直接装默认 `C:\Program Files`，事后把虚拟磁盘/默认 VM 目录指到数据盘即可。
- 安装完成后弹出「VM folder contains files that were used for unattended guest OS installation」清理提示，删除的是无人值守应答辅助文件，不影响已装系统，放心删。

## 实践指南

- 装完验证：`& "D:\VirtualBox\VBoxManage.exe" --version`；扩展包用 `VBoxManage extpack install --replace <file>` 导入（自动落入主程序目录 ExtensionPacks，无独立安装路径）。
- 本机落地结果：VirtualBox 7.2.16 r174877 装于 `D:\VirtualBox`；ubuntu-dev 虚拟机信息与三端架构定位见 [env.md 开发环境拓扑](../doc/env.md#开发环境拓扑) 与 [D-16](../doc/decisions.md#d-16-开发环境三端架构)。
