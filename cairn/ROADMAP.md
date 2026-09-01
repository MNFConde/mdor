# mdor 路线图

**当前焦点**：M2 已落地并全量验收（StaticSiteSource 递归镜像 + TOC 构建 + 场景 2 自建链落库 + tree 级大小写碰撞检测落库 + add_book/update_book 门面接线；httpmock 集成测试 14 例；真实站点 TRPL 验收通过；77 测试绿 + 门禁全绿 + 覆盖率 90.7%）；下一里程碑 **M3**（阅读器：内容抽取、资源协议、目录抽屉、滚动进度）。里程碑详表见 `doc/project.md` §10。

## 里程碑

- [x] M0 桌面开发环境搭建（VS/MSVC v14.50、rust-toolchain 1.97.1、dioxus-cli）
- [x] M1 workspace + mdor-core 骨架 + gix 存储基座 + AppService + 书架骨架 + ci.yml
- [x] M2 StaticSiteSource 递归镜像下载（自建链 + 版本 tag；httpmock 集成测试 + tests/ 目录 + cargo-llvm-cov 覆盖率接入）
- [ ] M3 阅读器：内容抽取、资源协议、目录抽屉、滚动进度
- [ ] M4 GitHubSource + SUMMARY 解析 + markdown 渲染
- [ ] M5 版本功能开放（历史 UI / 多版本阅读 / SnapshotMigrator / 清理策略；SnapshotMigrator 引入 proptest property-based 测试）
- [ ] M6 Android 打包（APK、权限、存储目录、cleartext）
- [ ] M7 CI 与发布（GitHub Actions + release）

## 开放问题

1. 风险与待定项以 `doc/project.md` §11 为准（如 D1 触摸滚动性能待 M6 真机）；达成共识后沉淀为 cairn/ 知识专题文档。（gix 三坑配置策略已实测敲定，见 D-09 / [gix-windows-pitfalls.md](gix-windows-pitfalls.md)）
