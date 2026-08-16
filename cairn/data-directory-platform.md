---
type: project_topic
status: active
summary: "数据目录双端注入：Windows exe 同目录便携式（产品决策、不可写报错不回退）vs Android getFilesDir() 沙箱；JNI 路径运行时才可得；差异收敛在数据根一层"
tags: [mdor, storage, data-directory, android, windows, portable, jni]
contains: [decision, lesson, procedure]
created: "2026-08-16"
updated: "2026-08-16"
related: [diff.md, decisions.md, project.md]
authoring_mode: ai_generated
---
# 数据目录双端注入

## 背景

两端对"应用数据能放哪"的约束本质不同——Windows 是原生进程、系统不设限（位置是**产品选择**）；Android 是沙箱强制（应用只有取舍）。决策记录见 D-13；规范见 `doc/project.md` §9；对比见 `doc/diff.md` §3。

## Lessons

1. **Windows 便携式是产品决策，不是系统要求**：原生进程与登录用户同权限，写 C 盘任意目录、用户目录、exe 同目录都合法——选"exe 同目录"是不污染 `%APPDATA%`/系统目录、数据随 exe 走、整目录可拷贝迁移；约束只在权限：**目录不可写直接报错，不回退**到系统用户目录。
2. **Android 沙箱强制：只有两块**：内部私有目录 `getFilesDir()`（`/data/data/<包名>/files`，免权限、只有本应用可读、随卸载删除）vs 外部存储（需运行时权限、用户可插拔/清理、其他应用可读）。mdor 选内部：免权限申请、数据与应用同生命周期；代价是**卸载即丢**（离线书可重下，可接受）。
3. **JNI 路径只能在运行时取**：`main()` 阶段拿不到 `getFilesDir()`——路径注入只能在启动时经 cfg 分支完成（桌面 `current_exe()` 同理）。
4. **差异收敛在"数据根"一层**：根之下 `data/bookstore/` 是产品自定义结构、平台无关；`BookStore::new(base_dir)` 注入点，core 只依赖 `bookstore/` 这一层，不出现 `cfg(target_os)` 分支。

## Current Conclusions

- **D-13 数据目录注入**：`BookStore::new(base_dir)` 接收路径，core 平台无关；`mdor-app` 启动时按平台解析数据根（cfg 分支）——Android 走 JNI `getFilesDir()`；Windows 走 `std::env::current_exe()` 的 **exe 同目录**（不存在则 `create_dir_all`，不可写直接报错）。
- 两端存储结构**对称**：数据根/`data/bookstore/`（`library.json` + `progress.json` + `books/<book_id>/` gix 仓库 + `.mdor/versions/`）。

## Practice Guide

- 数据根解析在 `mdor-app` 启动 cfg 分支；core 平台无关。
- 详情见 `doc/diff.md` §3、`doc/project.md` §9、D-13。
