---
type: project_topic
status: active
summary: "PowerShell 5.1 处理无 BOM UTF-8 中文文件的坑：Get-Content 默认按 ANSI/GBK 误读、Set-Content -Encoding UTF8 带 BOM；安全路径 = .NET API 显式 UTF8Encoding($false)；损坏可经 git 历史字节级恢复"
tags: [cairn, powershell, encoding, utf8, windows, tooling]
contains: [lesson, procedure, experience]
created: "2026-08-21"
updated: "2026-08-21"
related: [windows-scripts.md]
authoring_mode: ai_generated
---
# PowerShell 5.1 中文文本编码坑

## 背景

批量替换 cairn/ 知识专题文档的小节标题时（2026-08-21），用 PS5.1 `Get-Content -Raw` + `Set-Content -Encoding UTF8` 处理无 BOM 的 UTF-8 中文 Markdown，造成全部 14 个文件字节级损坏；从 `experiment/cairn-track` 分支按 blob 字节完整恢复后重做。

## 教训

1. **`Get-Content` 无 `-Encoding` 时按系统 ANSI 代码页解码**（中文系统 = GBK/cp936）：无 BOM 的 UTF-8 文件被逐字节误读成乱码字符串，此时任何内存修改 + 回写都会把乱码固化为真损坏。
2. **损坏是部分不可逆的**：GBK 解码遇到无法映射的字节序列会产生 `?`（0x3F），原字节信息当场丢失——事后逆向转码（GBK 编码回字节再按 UTF-8 解码）只能恢复未触碰区域。
3. **`Set-Content -Encoding UTF8`（PS5.1）写入带 BOM**：与仓库既有的无 BOM UTF-8 约定不一致；PS5.1 没有 `utf8NoBOM` 选项（PS 6+ 才有）。
4. **控制台显示不可信**：工具管道捕获的中文输出可能是显示伪影（乱码 ≠ 文件损坏）；判断文件是否损坏必须程序化断言（如扫描 U+FFFD、校验已知关键词码位），不能靠肉眼。
5. **gitignore 不等于没有备份**：cairn/ 在 master 被忽略，但实验分支 `experiment/cairn-track` 跟踪了它——分支历史就是恢复源。

## 当前结论

- **安全路径**：读写中文文本一律走 .NET API 并显式指定编码——
  ```powershell
  $utf8 = New-Object System.Text.UTF8Encoding($false)   # 无 BOM
  $s = [System.IO.File]::ReadAllText($path, $utf8)
  [System.IO.File]::WriteAllText($path, $s, $utf8)
  ```
- **字节级恢复姿势**：`cmd /c "git show <branch>:<path> > <file>"` 取原始 blob 字节（PowerShell 管道会再编码，不可用）；用 `git hash-object <file>` 对比 `git ls-tree` 的 blob SHA 验证字节一致。
- **正则替换注意**：`-replace` 的 `\s*` 会吞掉标题后的空行；替换串中 `$1` 后紧跟数字会被解析成多位组号（用 `${1}` 消歧）。

## 实践指南

- 批量改中文文件前：先确认有备份（git 历史 or 临时副本），改完做程序化校验（U+FFFD 扫描 + 与源 diff 仅含预期行）。
- 相关：bash 脚本侧的 Windows 姿势见 [windows-scripts.md](windows-scripts.md)。
