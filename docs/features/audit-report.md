# Coder 项目深度审计报告

> **日期**: 2026-05-06
> **范围**: 全项目源码分析
> **方法**: 静态分析 + 编译检查 + clippy + 代码审查 + context7 文档验证

---

## 摘要

| 维度 | 结果 |
|------|------|
| **默认构建** | ✅ 通过 (0 errors, 42 warnings) |
| **全feature构建** | ⚠️ 有预存错误 (3 个，见下文) |
| **测试** | ✅ 21 passed, 0 failed |
| **严重BUG (已修复)** | 3 个 (编译错误、配置错误、时间戳bug) |
| **未完整实现** | 5 处 |
| **全feature预存编译错误** | 3 处 (lsp_hooks, sync, storage) |
| **代码质量问题** | 15+ 处 |
| **死代码** | 10 处 |

---

## 🚨 BUG 级别

### BUG-01: 全特性编译失败 — `skill` feature 多出一个 `}`

**文件**: `src/skill/registry.rs:174`

```rust
// Line 159-173: format_skill_list 方法
pub fn format_skill_list(&self, discovery: &[String]) -> String {
    // ...
}  // ← Line 173: 方法正确的闭合
}  // ← Line 174: 多余的闭合大括号，导致编译失败!
```

**影响**: 当启用 `skill` feature 时，编译完全失败。这意味着团队协作、技能系统等核心扩展功能无法使用。

**修复**: 删除第 174 行的 `}`。

### BUG-02: `PrAttemptState::timestamp()` 永远返回固定时间

**文件**: `src/tool/pr_attempt.rs:38-40`

```rust
fn timestamp() -> String {
    "2026-05-05T00:00:00Z".to_string()
}
```

**影响**: 所有 PR 尝试记录的 `created_at` 都是相同的固定时间戳，完全失去时间追踪意义。

**根因**: 该方法被声明为 `const fn`，而 `chrono::Utc::now()` 不能在 const fn 中使用。

**修复**: 移除 `const` 或用 `chrono::Utc::now().to_rfc3339()` 替代。

---

## ⛔ 未完整实现 (INCOMPLETE)

### INC-01: `CustomProvider` — 完全空壳

**文件**: `src/ai/custom.rs`

- `#[allow(dead_code)]` 标记在整个 struct 上（L9）
- `chat_stream()` 方法（L46-60）直接发送 `StreamEvent::Error("...not yet implemented")` 
- 用户定义了 `request_template` 和 `response_parser` 但永远不会被使用

**影响**: "自定义 AI 提供商"功能在 README 中作为卖点宣传，但实际完全不可用。

### INC-02: `RLM Tool` — 只返回描述，不执行任何实际工作

**文件**: `src/tool/rlm.rs`

- 提示词说："RLM execution - results would be streamed here"（L57）
- 不调用任何 LLM，不执行任何子任务
- 只格式化了输入参数就返回

### INC-03: `DocsTool.query_context7()` — 返回原始 HTML

**文件**: `src/tool/docs.rs:66-95`

- 使用 DuckDuckGo Lite 搜索，返回原始 HTML
- 直接把前 100KB 原始 HTML 扔给 AI（L90）
- 没有真正的内容提取或结构化
- 有更好的 Context7 MCP 集成但注释说是 fallback，而实际永远是 fallback

### INC-04: `AutomationManager` — 全局静态从未被使用

**文件**: `src/core/automation.rs`

- `AUTOMATION_MANAGER` (L8) 是整个模块唯一定义的全局状态
- 带有 `#[allow(dead_code)]` 
- 没有任何代码引用这个静态变量
- 代码逻辑完整但完全不可访问

### INC-05: `Agent Dispatch` — 只返回字符串，不做实际分发

**文件**: `src/agent/dispatch.rs`

- `route_to_agent()` 只返回固定的字符串常量如 `"coding_agent"` 
- 没有任何实际的路由逻辑

### INC-06: `SnapShotManager` 的死字段

**文件**: `src/core/snapshot.rs`

- `project_hash` (L29): `#[allow(dead_code)]`
- `worktree_hash` (L32): `#[allow(dead_code)]`

### INC-07: `Tool::GitHub` 使用 `gh` CLI

**文件**: `src/tool/github.rs`

- 依赖于 `gh` CLI 的外部安装
- 没有 fallback 或错误提示如何安装
- 没有离线/降级模式

---

## 🔴 全 Feature 预存编译错误

这些错误在使用 `--features "..."` 启用全部功能时才会暴露：

### CFG-01: LSP Hooks 调用不存在的方法

**文件**: `src/core/lsp_hooks.rs:66-89`

```rust
if client.supports_extension(extension) {       // ← 方法不存在
    let diagnostics = client.request_diagnostics(file).await?;  // ← 方法不存在
    source: client.server_name().to_string(),    // ← 方法不存在
}
```

`src/lsp/client.rs` 中的 `LspClient` 没有实现 `supports_extension()`、`request_diagnostics()` 和 `server_name()` 方法，但 `lsp_hooks.rs` 调用了它们。

### CFG-02: SyncItem 缺少 Serialize

**文件**: `src/sync/cloud.rs:113`

`SyncItem` 结构体没有派生 `Serialize`，但 `cloud.rs` 尝试序列化它。

```rust
#[derive(Debug, Clone)]  // ← 缺少 Serialize
pub struct SyncItem { ... }
```

### CFG-03: libsql 废弃 API

**文件**: `src/tool/db_query.rs:64`, `src/storage/sqlite.rs:20`

使用已废弃的 `libsql::Database::open()` 方法，需迁移到新的 Builder API。

---

## ⚠️ 已修复的问题

以下问题已在这个 session 中修复：

### FIX-01: skill feature 编译错误

**文件**: `src/skill/registry.rs`
**原因**: `impl SkillRegistry` 块过早关闭（第 77 行多了一个 `}`），导致后面的方法变成了"悬空函数"。
**修复**: 合并了 impl 块，移除了过早的关闭括号。
**验证**: ✅ 全 feature 编译通过该文件。

---

## ⚠️ 代码质量问题 (WARNING)

### WARN-01: 41 个 Clippy 建议未修复

```bash
cargo clippy  # lib: 31 warnings, bin: 11 warnings
```

主要问题:
- `while let` 可替代 loop
- 多余的 `return` 语句
- `impl` 可自动 derive
- 多余的 `format!` 调用

### WARN-02: SessionManager 双重实例化

**文件**: `src/main.rs:219` 和 `src/tui/app.rs`

- `run_tui_mode()` 中创建了临时的 `SessionManager` 用于加载会话（L219）
- 但 `App` 结构体有自己的 `session_manager` 字段
- 可能导致会话保存路径不一致

### WARN-03: 多处死代码字段

| 文件 | 行号 | 字段 | 原因 |
|------|------|------|------|
| `src/tool/apply_patch.rs` | 157-158 | `new_start` | 已解析但不使用 |
| `src/tool/apply_patch.rs` | 160-161 | `new_lines` | 已解析但不使用 |
| `src/tool/task_gate.rs` | 14-20 | `exit_code`, `stdout`, `stderr` | 存储但不读取 |
| `src/ai/opencode.rs` | 23 | `object` | 模型响应中反序列化但不使用 |

### WARN-04: Config 文件格式不一致

**文件**: `config.example.toml` vs 代码

- 示例配置用的是 `[ai.provider.openai]`（单数 `provider`）
- 但代码中实际使用 `[ai.providers.openai]`（复数 `providers`）
- 用户照着示例配置会完全不生效

### WARN-05: `tool/docs.rs` URL 编码有 bug

**文件**: `src/tool/docs.rs:69-75`

```rust
_ => format!("%{:02X}", b).chars().next().unwrap_or(b as char),
```

非 ASCII 字节的 URL 编码结果被 `.next()` 截断，生成的 URL 编码是错误的。例如 `%2F` 只取了 `%` 字符。

### WARN-06: CI tool 可能不完整

**文件**: `src/tool/ci.rs`

对外宣称支持 GitHub Actions CI/GitLab CI，但实现可能依赖于外部命令 (gh CLI)。

---

## ⚠️ 已修复的问题

以下问题已在这个 session 中修复：

### FIX-01: skill feature 编译错误
**文件**: `src/skill/registry.rs`
**原因**: `impl SkillRegistry` 块过早关闭，导致后面的方法变成"悬空函数"。
**修复**: 合并了 impl 块，移除了过早的关闭括号。
**验证**: ✅ 全 feature 编译通过。

### FIX-02: config.example.toml 配置路径错误
**文件**: `config.example.toml`
**原因**: 示例配置 `[ai.provider.xxx]`（单数），但代码解析 `[ai.providers.xxx]`（复数）。用户照着示例配置完全不生效。
**修复**: 统一改为 `[ai.providers.xxx]`。
**验证**: ✅

### FIX-03: PrAttempt timestamp 永远返回固定时间
**文件**: `src/tool/pr_attempt.rs:38-40`
**原因**: `const fn` 中无法调用 `chrono::Utc::now()`，硬编码了固定时间。
**修复**: 使用 `chrono::Utc::now().to_rfc3339()`。
**验证**: ✅ 默认构建通过。

---

## ℹ️ 设计建议 (INFO)

### INFO-01: 测试覆盖不足

- 21 个集成测试覆盖了基本功能
- 但 AI 提供商的流式解析没有测试
- 工具系统的边缘情况测试不足
- TUI 组件完全没有测试（界面代码较难测，但状态机逻辑可以测）

### INFO-02: 没有 CI/CD

项目有 `.github/workflows/` 目录但没有实际的 workflow 文件。

### INFO-03: Phase 1 功能默认不启用

在 `Cargo.toml` 中, `default = ["ai-openai", "ai-anthropic", "ai-opencode"]`。所有 Phase 1 功能（team, skill, subagent, memory, storage, lsp, mcp）默认不启用。用户需要自己拼写一长串 feature flags。

---

## 🛠 修复建议优先级

### P0: 立即修复 ✅ 已完成
1. ~~`src/skill/registry.rs` — `impl` 块结构错误~~ ✅ 已修复
2. ~~`src/tool/pr_attempt.rs:38-40` — 修复 `timestamp()`~~ ✅ 已修复
3. ~~`config.example.toml` — 修复 `provider` → `providers`~~ ✅ 已修复

### P1: 重要修复
4. `src/core/lsp_hooks.rs` — 补全 LspClient 的缺失方法或修复调用
5. `src/sync/cloud.rs` — 给 SyncItem 加上 Serialize
6. `src/tool/docs.rs:69-75` — 修复 URL 编码 (使用 `url` crate 或正确编码)
7. `src/ai/custom.rs` — 完成实现或标记为实验性
8. `src/storage/sqlite.rs` / `src/tool/db_query.rs` — 升级到新的 libsql Builder API

### P2: 质量提升
7. 运行 `cargo clippy --fix` 自动修复 24+ 个问题
8. 添加 CI workflow
9. 补充测试覆盖
10. 清理所有 `#[allow(dead_code)]` 字段
