# Tasks

- [ ] Task 1: 编译检查 — 运行 `cargo check --all-targets --all-features` 并收集编译错误/警告
  - [ ] SubTask 1.1: 运行 `cargo check --all-targets` 默认特征
  - [ ] SubTask 1.2: 运行 `cargo check --all-targets --all-features`
  - [ ] SubTask 1.3: 运行 `cargo check --no-default-features`
  - [ ] SubTask 1.4: 汇总编译检查结果

- [ ] Task 2: Lint 检查 — 运行 `cargo clippy --all-targets --all-features` 并收集 Clippy 警告
  - [ ] SubTask 2.1: 运行 `cargo clippy --all-targets --all-features` 
  - [ ] SubTask 2.2: 按类别统计 Clippy 警告（correctness, style, complexity, perf, suspicious）
  - [ ] SubTask 2.3: 汇总 Lint 检查结果

- [ ] Task 3: 测试检查 — 运行 `cargo test --all-features` 并收集测试结果
  - [ ] SubTask 3.1: 运行 `cargo test --all-features`
  - [ ] SubTask 3.2: 解析测试输出，统计通过/失败/跳过数
  - [ ] SubTask 3.3: 如有失败，列出失败测试详情

- [ ] Task 4: 依赖安全检查 — 运行 `cargo audit` 检查依赖漏洞
  - [ ] SubTask 4.1: 尝试运行 `cargo audit`
  - [ ] SubTask 4.2: 如不可用，检查 Cargo.lock 中关键依赖版本

- [ ] Task 5: 代码安全扫描 — 检查硬编码密钥、unsafe 代码、不安全模式
  - [ ] SubTask 5.1: 搜索硬编码密钥/令牌/密码模式
  - [ ] SubTask 5.2: 统计 unsafe 代码块数量和位置
  - [ ] SubTask 5.3: 检查 unwrap/expect 使用情况
  - [ ] SubTask 5.4: 检查 tokio::spawn 是否正确使用

- [ ] Task 6: 核心模块代码审查 — 审查 agent/、ai/、tool/、core/ 目录
  - [ ] SubTask 6.1: 审查 agent/ 模块代码质量
  - [ ] SubTask 6.2: 审查 ai/ 模块代码质量
  - [ ] SubTask 6.3: 审查 tool/ 模块代码质量
  - [ ] SubTask 6.4: 审查 core/ 模块代码质量
  - [ ] SubTask 6.5: 审查 tui/ 模块代码质量

- [ ] Task 7: 生成最终检查报告 — 汇总所有检查结果
  - [ ] SubTask 7.1: 汇总编译、Lint、测试结果
  - [ ] SubTask 7.2: 汇总安全和质量审查结果
  - [ ] SubTask 7.3: 生成优先级排序的建议列表

# Task Dependencies
- Task 2 依赖 Task 1（编译通过后再 Lint 更有效）
- Task 5, Task 6 可与 Task 1-4 并行执行
- Task 7 依赖所有前序任务完成