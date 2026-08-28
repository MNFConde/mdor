---
type: project_topic
status: active
summary: "PowerShell 5.1 编码坑三向记录：读侧——Get-Content 默认按 ANSI/GBK 误读无 BOM UTF-8；写侧——>/Out-File 默认输出 UTF-16 LE、Set-Content -Encoding UTF8 带 BOM；管道侧——捕获原生命令 UTF-8 stdout 按 GBK 解码致内容导出即损坏（不可逆）；stdin 侧——管道写原生命词 stdin 用 $OutputEncoding(默认 ASCII) 中文 → ? 字节级损坏；安全路径 = .NET API 显式 UTF8Encoding($false) + cmd /c 直通原始字节 + -F <UTF-8文件>；损坏可经 git 历史字节级恢复"
tags: [cairn, powershell, encoding, utf8, windows, tooling]
contains: [lesson, procedure, experience]
created: "2026-08-21"
updated: "2026-08-29"
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
6. **PS 5.1 `>` 重定向 / `Out-File` 默认输出 UTF-16 LE（带 `FF FE` BOM）**（2026-08-23）：导出的 opencode 会话备份 json 被 UTF-16 LE 写出，opencode 导入器按 UTF-8 解析即报「Unrecognized token '�'」——首个字节就是 BOM。读侧教训 1 的镜像坑：PS 5.1 各 cmdlet 默认编码不一致，任何「导出/落盘再被别的程序消费」的文件都不能依赖默认编码。
7. **PS 管道捕获原生命令的 UTF-8 stdout 按 GBK 解码，内容导出即损坏（不可逆）**（2026-08-23）：`opencode export <id> > file` 在 PS 5.1 下执行——opencode 输出 UTF-8 字节流，PS 按 [Console] 输出编码（GBK）解码成字符串再以 UTF-16 落盘，中文正文被固化为 GBK 假汉字 + PUA 私用区字符（U+E003 等，实测 652 个）+ `?` 替换。这是教训 2 的实例：事后把容器转回 UTF-8 也只是「正确编码的错误内容」，导入器 JSON 校验都过不了。**容器编码可修，管道损坏不可逆**——必须在捕获环节就绕开 PS 解码层。
8. **PS 5.1 管道写原生命词 stdin 用 `$OutputEncoding`（默认 ASCII），中文 → 字面 `?` 字节级损坏**（2026-08-29）：`@'…中文…'@ | git commit -F -` 写提交信息，每个中文字符都成了 0x3F（`cmd /c` 直读原始字节仍是 `?` = 真损坏非显示伪影）。与坑 7 同族但**方向相反**：stdout 捕获是「解码层误读」，stdin 写是「编码层丢弃」（ASCII 表达不了 CJK 直接替换成 `?`）。对策：中文内容别走 PS 管道喂原生命词——用 UTF-8 无 BOM 文件 `-F <file>`（如本会话 `git commit --amend -F <utf8文件>` 修复）；或先 `$OutputEncoding = [System.Text.Encoding]::UTF8` 再管道。提交后校验：`cmd /c "git log -1 --format=%B"` 不得含 `?`（显示层乱码≠损坏，见教训 4）。

## 当前结论

- **安全路径**：读写中文文本一律走 .NET API 并显式指定编码——
  ```powershell
  $utf8 = New-Object System.Text.UTF8Encoding($false)   # 无 BOM
  $s = [System.IO.File]::ReadAllText($path, $utf8)
  [System.IO.File]::WriteAllText($path, $s, $utf8)
  ```
- **UTF-16 LE 文件转 UTF-8 无 BOM**：`Get-Content -Raw` 能自动识别 BOM 正确读入，再按上面安全路径写回即可（2026-08-23 实测用于修复 opencode 会话备份导入）。
- **原生命令输出落盘用 cmd 直通原始字节**：`cmd /c "opencode export <id> > <file>"`——cmd 的 `>` 不解码 stdout，UTF-8 原样落盘。2026-08-23 实证对照：PS 管道版 652 个 PUA 字符报废；cmd 直通版 JSON 解析通过、4904 个汉字完好（残留 U+FFFD 仅在工具输出的二进制/进度条类内容里，不伤结构）。与字节级恢复姿势的 `cmd /c "git show …"` 同一原理。
- **字节级恢复姿势**：`cmd /c "git show <branch>:<path> > <file>"` 取原始 blob 字节（PowerShell 管道会再编码，不可用）；用 `git hash-object <file>` 对比 `git ls-tree` 的 blob SHA 验证字节一致。
- **正则替换注意**：`-replace` 的 `\s*` 会吞掉标题后的空行；替换串中 `$1` 后紧跟数字会被解析成多位组号（用 `${1}` 消歧）。

## 实践指南

- 批量改中文文件前：先确认有备份（git 历史 or 临时副本），改完做程序化校验（U+FFFD 扫描 + 与源 diff 仅含预期行）。
- 跨程序导出/导入后：程序化断言三件套——BOM 头字节检查、PUA（U+E000–U+F8FF）/U+FFFD 计数、已知关键词检索；控制台显示乱码不算数（教训 4）。
- 相关：bash 脚本侧的 Windows 姿势见 [windows-scripts.md](windows-scripts.md)。
