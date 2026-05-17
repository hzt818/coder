# GitHub Workflow 全平台自动编译 & 自动修复 Bug Spec

## Why
项目已有基础的多平台编译工作流（build.yml / ci.yml），但缺少自动修复 Bug 的能力。需要统一并增强现有工作流，新增自动代码修复、自动回退、自动创建修复 PR 等能力，减少人工干预。

## What Changes
- **合并** build.yml 与 ci.yml 为一个统一的增强工作流
- 新增自动 Bug 修复能力：cargo fix、cargo fmt 自动修正、clippy auto-fix
- 新增编译失败自动重试与回退机制
- 新增失败时自动创建修复 Issue 或 PR
- 新增跨平台测试（Linux / macOS / Windows）矩阵
- 新增安全审计扫描（cargo audit）
- **BREAKING**: 删除旧的 ci.yml，统一为增强版 build.yml

## Impact
- Affected specs: 无
- Affected code: `.github/workflows/build.yml`, `.github/workflows/ci.yml`

## ADDED Requirements

### Requirement: 自动格式修复
系统 SHALL 在 cargo fmt 检查失败时自动运行 cargo fmt 并提交修复。

#### Scenario: 格式检查失败自动修复
- **WHEN** cargo fmt --check 返回非零退出码
- **THEN** 系统自动执行 cargo fmt，将修复后的代码提交为一个新 commit 或 PR

### Requirement: 自动 Clippy 修复
系统 SHALL 在 clippy 检查失败时自动运行 cargo clippy --fix 并提交修复。

#### Scenario: Clippy 检查失败自动修复
- **WHEN** cargo clippy 报告 warnings/errors
- **THEN** 系统自动执行 cargo clippy --fix --allow-dirty，将修复提交

### Requirement: 编译失败自动诊断与修复
系统 SHALL 在编译失败时自动分析错误日志并尝试自动修复。

#### Scenario: 编译失败自动重试
- **WHEN** cargo build 失败
- **THEN** 系统自动执行 cargo fix --allow-dirty 并重新编译，最多重试 3 次

### Requirement: 失败自动创建 Issue
系统 SHALL 在自动修复均失败后自动创建 GitHub Issue 记录问题。

#### Scenario: 无法自动修复时创建 Issue
- **WHEN** 所有自动修复尝试均失败
- **THEN** 系统自动创建包含错误日志的 GitHub Issue

### Requirement: 全平台编译矩阵
系统 SHALL 在 Linux x86_64、Linux aarch64、macOS x86_64、macOS aarch64、Windows x86_64 五个平台上编译 release 版本。

#### Scenario: 推送 tag 触发全平台编译
- **WHEN** 推送以 v 开头的 tag
- **THEN** 系统在五个平台上并行编译 release 版本并上传产物

### Requirement: 安全审计
系统 SHALL 在每次 CI 运行中执行 cargo audit 检查依赖安全漏洞。

#### Scenario: 发现安全漏洞
- **WHEN** cargo audit 发现已知漏洞
- **THEN** 系统在 CI 报告中标注并尝试自动更新受影响依赖

## REMOVED Requirements

### Requirement: 旧 ci.yml 工作流
**Reason**: 功能已合并到增强版 build.yml 中
**Migration**: 删除 `.github/workflows/ci.yml`，所有功能由 build.yml 覆盖