# mdor 测试小书

用于 M2 StaticSiteSource 镜像测试的预构建静态站点源。

- 3 章 + 1 嵌套子章节，含图片/css 引用，覆盖 TOC 嵌套与资源抓取场景
- 构建产物提交于本目录 `book/`（fixture 需可离线跟踪，CI 不装 mdbook）

重建命令（产物变更时）：

```sh
mdbook build
cp -r book ../fixtures/mdbook-static/
```
