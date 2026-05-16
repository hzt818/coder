# Checklist

- [ ] `cargo check --all-targets` 默认特征编译通过，无错误
- [ ] `cargo check --all-targets --all-features` 编译通过，无错误
- [ ] `cargo check --no-default-features` 编译通过，无错误
- [ ] `cargo clippy --all-targets --all-features` 无 critical/error 级别警告
- [ ] Clippy 警告已分类统计（correctness, style, complexity, perf, suspicious）
- [ ] `cargo test --all-features` 测试全部通过
- [ ] 无测试失败或 panic
- [ ] `cargo audit` 无已知安全漏洞依赖
- [ ] 代码中无硬编码密钥、令牌或密码
- [ ] unsafe 代码块已记录数量和位置
- [ ] 无过度使用 unwrap/expect 的情况
- [ ] tokio::spawn 使用正确，无任务泄漏风险
- [ ] 核心模块代码质量评分 ≥ 良好
- [ ] 最终报告包含优先级排序的建议列表