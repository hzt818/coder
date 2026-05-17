# Checklist

- [x] 新 build.yml 包含全平台编译矩阵（5 target: Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64）
- [x] 新 build.yml 包含多 feature 组合编译验证（默认特性、最小特性、全特性）
- [x] 新 build.yml lint 阶段包含 fmt check + clippy 并作为前置依赖
- [x] 新 build.yml test 阶段覆盖 Linux/macOS/Windows 三平台
- [x] npm 发布 job 正常工作（仅 master/tag/manual 触发）
- [x] GitHub Release job 在 tag 推送时正常工作
- [x] cargo fmt 失败时自动执行 cargo fmt 并创建修复 PR
- [x] cargo clippy 失败时自动执行 cargo clippy --fix 并创建修复 PR
- [x] cargo build 失败时自动 cargo fix 重试最多 3 次
- [x] 所有自动修复失败后自动创建包含错误日志的 GitHub Issue
- [x] cargo audit job 存在并能发现安全漏洞
- [x] 旧 ci.yml 已删除
- [x] workflow_dispatch 手动触发支持
- [x] concurrency 配置防止重复运行
- [x] rust-cache 配置正确启用缓存