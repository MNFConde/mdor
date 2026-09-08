---
type: project_topic
status: active
summary: "Rust 测试基建坑（httpmock / 集成测试）：httpmock 0.8 同路径多 mock 匹配序不定（HashMap 迭代序，重灌必须 server.reset()）；#[cfg(test)] 代码对集成测试不可见（集成测试链接的 lib 不带 cfg(test)，诊断输出失效根因）；tests/common/mod.rs dead_code 与共享工具边界（M4/M5 集成测试积累同归此篇）"
tags: [mdor, rust, testing, httpmock, integration-tests]
contains: [lesson, procedure]
created: "2026-08-31"
updated: "2026-08-31"
related: []
authoring_mode: ai_generated
---
# Rust 测试基建坑（httpmock / 集成测试）

## 背景

M2 首建 `crates/mdor-core/tests/` 集成测试目录（httpmock 假站镜像测试 + AppService 门面级流程测试），踩到的测试基建坑沉淀于此；M4（GitHubSource）/ M5（SnapshotMigrator）集成测试积累同归此篇。规范位置：`doc/project.md` §10 M2 行 / `doc/diff.md` §8.2。

## 教训

1. **httpmock 0.8 同路径注册多个 mock 时匹配顺序不定**（2026-08-31 M2 实锤）：服务端内部以 `HashMap<mock_id, ActiveMock>` 存 mock，匹配取 `values().find(...)`——**迭代序非注册序**。同路径叠加注册新 mock 当「覆盖」用是伪覆盖：先匹配到哪一个不确定，测试不稳定。**对策**：重灌场景必须 `server.reset()`（删全部 mock + 历史）后重新注册全部所需 mock，禁止新旧叠加。
2. **`#[cfg(test)]` 代码对集成测试不可见**：集成测试是独立二进制，链接的 lib 编译**不带** `cfg(test)`——lib 内 `#[cfg(test)] eprintln!` / `#[cfg(test)]` 工具函数在集成测试运行时不存在。本次排查「命令队列是否执行」时诊断输出静默缺失，误导为「任务没跑」，实际是输出被编译期剔除。**对策**：集成测试阶段的临时诊断直接写裸 `eprintln!`（排查完删除），不套 `cfg(test)`；测试专用设施要么走 pub API，要么放 `tests/common/`。
3. **门面级流程测试等异步命令队列：轮询 + sleep 让步**：`#[tokio::test]` 默认 current_thread runtime，`tokio::spawn` 的命令队列消费者任务只在测试任务 await 让步点运行。等待「library 状态变化」用 `for + tokio::time::sleep(20ms)` 轮询（上限设足，如 500 次），break 条件为轮询目标出现——不要假设 enqueue 后同步完成，也不要用固定一次 sleep 硬等。

## 实践指南

- **tests/common/mod.rs 的 dead_code**：共享工具按每个测试二进制独立编译，未被某二进制使用的 `pub fn` 报 dead_code → `#[allow(dead_code)]` 加在函数上（或整个 mod）是常规解，不要为了消警删工具。
- **crate 内 `pub(crate) test_support` 不外借**（2026-08-31 定案，LOG）：集成测试只能暴露 pub API，crate 内测试共享资产强行外借会迫使 pub 化 + 跨层依赖倒挂；`tests/common/mod.rs` 独立实现（如 `temp_dir`），接受与 lib 内版本的有限重复。
- **fixture 查找按仓库内相对路径多候选**：集成测试 CWD 是 crate 目录，fixture 在仓库根（`../fixtures/...`）；`tests/common` 提供多候选 + `canonicalize` 的查找函数，panic 信息列出已试候选路径。
- **真网络验收测试钉 `#[ignore]`**：依赖真实站点的验收（如 `real_site_doc_rust_lang_book_mirror`）标 `#[ignore = "依赖真实网络，M2 手工验收用"]`，手工 `cargo test -- --ignored --nocapture` 执行，不进 CI；CI 只跑 httpmock 假站测试。
- **httpmock 断言请求次数**：需要验证「只请求一次」（去重行为）时，闭包内对 `Arc<AtomicUsize>` 计数再断言，比 `mock.assert()` 的次数语义更直接。
