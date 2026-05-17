# Tasks

- [x] Task 1: 创建增强版 build.yml — 统一合并 ci.yml 和 build.yml
  - [x] SubTask 1.1: 保留原 build.yml 的全平台编译矩阵（5 个 target）
  - [x] SubTask 1.2: 合并 ci.yml 的多 feature 组合编译验证（默认/最小/全特性）
  - [x] SubTask 1.3: 统一 lint 阶段（fmt + clippy）并作为所有后续 job 的前置依赖
  - [x] SubTask 1.4: 统一 test 阶段并增加跨平台测试矩阵（Linux/macOS/Windows）
  - [x] SubTask 1.5: 保留 npm 发布与 GitHub Release 功能

- [x] Task 2: 新增自动 Bug 修复能力 — auto-fix job
  - [x] SubTask 2.1: 实现 cargo fmt 失败时自动修正并创建修复 commit 的逻辑
  - [x] SubTask 2.2: 实现 cargo clippy 失败时自动 cargo clippy --fix 并创建修复 commit 的逻辑
  - [x] SubTask 2.3: 实现编译失败时 cargo fix --allow-dirty 自动重试（最多 3 次）
  - [x] SubTask 2.4: 实现自动修复 PR 创建（使用 peter-evans/create-pull-request action）

- [x] Task 3: 新增自动 Issue 创建 — 失败兜底
  - [x] SubTask 3.1: 当所有自动修复都失败后自动创建 GitHub Issue
  - [x] SubTask 3.2: Issue 内容包含失败阶段、错误日志、触发分支/PR 信息

- [x] Task 4: 新增安全审计 — cargo audit
  - [x] SubTask 4.1: 添加 cargo audit job，检查依赖安全漏洞
  - [x] SubTask 4.2: 发现漏洞时尝试 cargo update 自动更新并重试审计

- [x] Task 5: 删除旧 ci.yml
  - [x] SubTask 5.1: 删除 .github/workflows/ci.yml 文件

# Task Dependencies
- [Task 2] 依赖 [Task 1] — auto-fix 逻辑需嵌入到统一的 build.yml 中
- [Task 3] 依赖 [Task 2] — Issue 创建是 auto-fix 失败后的兜底
- [Task 4] 可与 [Task 1~3] 并行
- [Task 5] 依赖 [Task 1] — 确保新 build.yml 已覆盖旧 ci.yml 所有功能后再删除