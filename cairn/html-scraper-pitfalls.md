---
type: project_topic
status: active
summary: "HTML 解析坑（scraper 0.27 / html5ever）：noscript 内容降级为转义文本节点致选择器抓不到（mdBook ≥0.5 toc.html 经 noscript iframe 引用，镜像引擎漏抓 sidebar，需正则兜底）；HTML 内容嗅探不能只认完整页（片段被误判非 HTML 致链接不发现）；scraper 0.27 属性不可变致 DOM 层无法改属性值（链接重写只能两段式字符串占位替换）；:scope 选择器/child_elements/mdBook 未闭合 li 自动修正等实测要点（M3 渲染管线复用）"
tags: [mdor, html, scraper, html5ever, html-parsing, mdbook]
contains: [lesson, procedure]
created: "2026-08-31"
updated: "2026-09-02"
related: []
authoring_mode: ai_generated
---
# HTML 解析坑（scraper / html5ever）

## 背景

M2 StaticSiteSource 镜像引擎（`source/static_site.rs`）与 TOC 构建（`build_toc`）首次大规模使用 scraper 0.27（底层 html5ever 0.39）；M3 渲染管线的 `<main>` 抽取将复用同一技术栈，坑与实测要点沉淀于此。规范位置：`doc/project.md` §4.1 / §6.5。

## 教训

1. **html5ever 把 `<noscript>` 内容降级为转义文本节点，选择器抓不到**（2026-08-31 M2 实锤）：`<noscript><iframe src="toc.html"></iframe></noscript>` 经 `Html::parse_fragment` 后，iframe 不是元素节点，而是 `&lt;iframe src="toc.html"&gt;` 形态的转义文本（`inner_html()` 可证）。mdBook ≥0.5 的 sidebar 正是经 noscript iframe 引用 `toc.html`（JS 可用时由 toc.js 注入，noscript 仅兜底）——镜像引擎靠 `iframe[src]` 选择器发现 toc.html 即漏抓，TOC 构建随之落空。**对策**：对 `noscript` 元素的文本做正则兜底，从文本中找回 `(?:src|href)=["']([^"']+)["']` 引用（`extract_links` 内实现）。
2. **HTML 内容嗅探不能只认完整页**：`<!doctype html` / `<html` 双判据会把**片段**（测试假站的 `<a href=...>`、`<link rel=...>` 等，无 `<html>` 包裹）误判为非 HTML → 链接不发现 → 镜像集缺页（集成测试三个用例同时失败的根因）。**对策**：嗅探清单放宽为 `<!doctype html` / `<html` / `<a ` / `<link` / `<img` / `<p>` / `<div` 任一命中；代价是极少数含这些字面量的资源文件会被多解析一次（解析无匹配无害）。
3. **scraper 0.27 属性不可变，DOM 层改不了属性值**（2026-09-02 M3 切片1 实锤）：`ElementRef::attr(name)` 只返回 `Option<&str>` 借用，无 setter；`Element.attrs` 是私有 `Vec<(QualName, StrTendril)>`，无任何修改 API。改写 `<main>` 内 `href`/`src` 等相对资源引用（`./img/logo.svg` → 本地分发前缀）**无法在 DOM 上原地改**。**对策**：两段式字符串替换——① 序列化 `inner_html()` 后，用正则把相对/书根路径改写为占位符根（如 `/mdor-res/v1/`），跳过 `#` 锚点 / `http(s)` / `data:` 等 scheme；② 用 `str::replacen` 把占位符根整体替换为书根上下文前缀。占位符不含 `&`/`<`/`>`/`"`，故 html5ever 序列化转义不影响文本替换（副作用：html5ever 会规范属性顺序与 `&`→`&amp;`，WebView 接受、对阅读无碍）。

## 实测要点（scraper 0.27 / html5ever 0.39）

- **`:scope > li` 在 `parse_fragment` 下可用**：`ElementRef::select` 以 scope 元素为界做子元素级匹配（`matches_with_scope_and_cache(Some(scope))`），实测对 `ol.chapter` 取直接 `li` 正确（descendant `li` 会混入嵌套层，必须带 `:scope >`）。
- **`child_elements()` 是直接子元素迭代的替代**：不走选择器、无解析歧义，两层结构简单时更直接。
- **mdBook ≥0.5 `toc.html` 的 `</li>` 缺失是常态**：模板输出不闭合 `li`，html5ever 依 HTML5 规范自动修正嵌套（后续 `li` 归同级）；解析逻辑以修正后树为准即可，不要假设源 HTML 闭合良好。
- **mdBook sidebar 双形态**：≥0.5 = 独立 `toc.html`（经 noscript iframe 引用）；0.4.x = 内嵌在每页 `index.html`（直接含 `ol.chapter`）。`build_toc` 按文件名在**任意层级**查找 `toc.html` 再回退 `index.html`（入口带路径前缀时 sidebar 落在 `book/toc.html` 等子路径，按顶级路径查会落空）。
- **页面 `<title>` 是「章节名 - 书名」形态**：书名取 `rsplit_once(" - ")` 的尾段；无 `-` 时整串即书名。
- **`parse_document` 与 `parse_fragment` 的树结构不同**：fragment 会补 wrapper，`ol.chapter` 仍可正常选中；两者对本项目的选择器均无影响，但跨两者共享选择器断言前需实测。

## 实践指南

- 从 HTML 提取引用时，资源发现清单覆盖：`a[href]`（页面）、`img/script/source/video/audio/embed/iframe[src]`（资源，iframe 是 mdBook sidebar 载体）、`link[href]`（样式）、`noscript` 文本正则兜底。
- 疑似「选择器没抓到」时先用 `inner_html()` / 树打印核对节点形态，确认是元素还是转义文本——html5ever 的规范行为与直觉不同的点都在这里（noscript 是唯一已知降级点）。
- mdBook 版本差异（0.4.x 内嵌 vs ≥0.5 独立 toc.html）在 `build_toc` 一处兼容，M3 渲染层抽 `<main>` 不受此影响（两版本页面结构一致）。
