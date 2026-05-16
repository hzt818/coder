# 项目代码全面检查 Spec

## Why
对 Coder 项目（Rust 终端 AI 编程助手）进行系统性代码检查，包括编译、lint、测试、代码质量和安全性，确保代码库健康状态。

## What Changes
- 运行 `cargo check` 检查编译错误
- 运行 `cargo clippy` 检查代码规范
- 运行 `cargo test` 检查测试通过率
- 对核心模块进行代码审查
- 检查安全性问题（硬编码密钥、不安全代码等）
- 生成检查报告

## Impact
- Affected specs: 无（纯检查任务）
- Affected code: 无（不修改代码，仅检查）

## ADDED Requirements

### Requirement: 编译检查
系统 SHALL 对项目执行 `cargo check --all-targets --all-features` 并报告所有编译错误和警告。

#### Scenario: 编译成功
- **WHEN** 执行 `cargo check`
- **THEN** 报告编译状态（成功/失败），列出所有错误和警告数量

#### Scenario: 特征组合检查
- **WHEN** 执行不同特征组合的编译检查
- **THEN** 确保默认特征和无默认特征均能编译

### Requirement: Lint 检查
系统 SHALL 对项目执行 `cargo clippy --all-targets --all-features` 并报告所有 Clippy 警告和错误。

#### Scenario: Clippy 检查
- **WHEN** 执行 `cargo clippy`
- **THEN** 报告 Clippy 警告数量、级别分布、并分类汇总问题

### Requirement: 测试检查
系统 SHALL 对项目执行 `cargo test --all-features` 并报告测试通过率和失败详情。

#### Scenario: 测试运行
- **WHEN** 执行 `cargo test`
- **THEN** 报告测试总数、通过数、失败数、跳过数，列出失败测试详情

### Requirement: 代码质量审查
系统 SHALL 审查核心模块代码质量，包括：
- 错误处理完整性
- 异步代码安全性
- 资源管理（文件句柄、连接等）
- unsafe 代码使用情况

#### Scenario: 核心模块审查
- **WHEN** 审查 agent/、ai/、tool/、core/ 等核心模块
- **THEN** 生成代码质量评分和建议

### Requirement: 安全检查
系统 SHALL 检查以下安全问题：
- 硬编码的密钥、令牌、密码
- unsafe 代码块数量与位置
- 不安全的依赖版本
- 潜在的死锁或竞态条件

#### Scenario: 安全扫描
- **WHEN** 扫描整个代码库
- **THEN** 列出所有安全隐患及其位置

### Requirement: 依赖检查
系统 SHALL 检查 Cargo.toml 中的依赖项：
- 过时依赖
- 已知安全漏洞（通过 `cargo audit`）
- 许可证兼容性

#### Scenario: 依赖审计
- **WHEN** 执行 `cargo audit`
- **THEN** 报告是否有已知漏洞的依赖

### Requirement: 覆盖率检查（可选）
系统 MAY 检查测试覆盖率情况。

#### Scenario: 覆盖率报告
- **WHEN** 使用 tarpaulin 或 llvm-cov 检查覆盖率
- **THEN** 报告行覆盖率和分支覆盖率