# mdor 路线图

**当前焦点**：规划期——M0 桌面构建为唯一当前可用目标（`dx serve --platform desktop`）；下一里程碑 **M1**（workspace + mdor-core 骨架 + gix 存储基座 + 书架骨架 + 轻量 ci.yml）。里程碑详表见 `doc/project.md` §10。

## 里程碑

- [ ] M0 桌面开发环境搭建（VS/MSVC v14.50、rust-toolchain 1.97.1、dioxus-cli）
- [ ] M1 workspace + mdor-core 骨架 + gix 存储基座 + AppService + 书架骨架 + ci.yml
- [ ] M2 StaticSiteSource 递归镜像下载（自建链 + 版本 tag）
- [ ] M3 阅读器：内容抽取、资源协议、目录抽屉、滚动进度
- [ ] M4 GitHubSource + SUMMARY 解析 + markdown 渲染
- [ ] M5 版本功能开放（历史 UI / 多版本阅读 / SnapshotMigrator / 清理策略）
- [ ] M6 Android 打包（APK、权限、存储目录、cleartext）
- [ ] M7 CI 与发布（GitHub Actions + release）

## 开放问题

1. 风险与待定项以 `doc/project.md` §11 为准（如 gix 三坑配置策略待 M1 实测、D1 触摸滚动性能待 M6 真机）；达成共识后沉淀为 cairn/ 知识专题文档。
