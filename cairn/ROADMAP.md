# mdor 路线图

**当前焦点**：M1 已验收（workspace + mdor-core 骨架、gix 存储基座、AppService 薄门面 + 命令骨架、书架 UI 渲染真实数据，`cargo test -p mdor-core` 41 绿 + 门禁全绿 + **CI core-quality 实跑绿**；D-09 gix 三坑配置两端实测敲定——Linux 2026-08-27 / Windows 2026-08-30 六项回归测试；CI 首跑抓 MissingCommitter 产品级 bug 已修复）；下一里程碑 **M2**（StaticSiteSource 静态站递归镜像 + 自建链版本 tag + tree 级大小写碰撞检测落库 + httpmock 集成测试，fixtures/mdbook-static/）。里程碑详表见 `doc/project.md` §10。

## 里程碑

- [x] M0 桌面开发环境搭建（VS/MSVC v14.50、rust-toolchain 1.97.1、dioxus-cli）
- [x] M1 workspace + mdor-core 骨架 + gix 存储基座 + AppService + 书架骨架 + ci.yml
- [ ] M2 StaticSiteSource 递归镜像下载（自建链 + 版本 tag）
- [ ] M3 阅读器：内容抽取、资源协议、目录抽屉、滚动进度
- [ ] M4 GitHubSource + SUMMARY 解析 + markdown 渲染
- [ ] M5 版本功能开放（历史 UI / 多版本阅读 / SnapshotMigrator / 清理策略）
- [ ] M6 Android 打包（APK、权限、存储目录、cleartext）
- [ ] M7 CI 与发布（GitHub Actions + release）

## 开放问题

1. 风险与待定项以 `doc/project.md` §11 为准（如 D1 触摸滚动性能待 M6 真机）；达成共识后沉淀为 cairn/ 知识专题文档。（gix 三坑配置策略已实测敲定，见 D-09 / [gix-windows-pitfalls.md](gix-windows-pitfalls.md)）
