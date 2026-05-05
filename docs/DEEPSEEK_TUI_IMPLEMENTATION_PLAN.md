# DeepSeek TUI 功能移植计划

> **目标:** 将 DeepSeek TUI (v0.8.12) 的所有功能完整移植到 coder 项目  
> **当前版本:** coder v0.1.0  
> **源项目:** [Hmbown/DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI)  
> **计划日期:** 2026-05-05

---

## 目录

1. [现状概览](#1-现状概览)
2. [移植范围](#2-移植范围)
3. [阶段计划](#3-阶段计划)
4. [文件变更清单](#4-文件变更清单)
5. [依赖关系图](#5-依赖关系图)
6. [验证策略](#6-验证策略)

---

## 1. 现状概览

### 1.1 coder 项目现有能力

| 模块 | 状态 | 说明 |
|------|------|------|
| 配置系统 | ✅ 完成 | Settings, ProviderConfig, 环境变量解析, 分层配置 |
| AI Provider | ✅ 完成 | OpenAI, Anthropic, Google, Custom, OpenCode; 流式 + 非流式 |
| 工具系统 | ✅ 完成 | ToolRegistry, Tool trait, ToolResult, JSON Schema |
| 核心工具 | ✅ 完成 | bash, file_read, file_write, file_edit, glob, grep, question, web_fetch, web_search |
| 高级工具 | ⚠️ 基础 | docs, plan, task, ci (需增强) |
| 特性工具 | ⚠️ 基础 | git, docker, db_query, oauth, worktree (需增强) |
| 引擎 | ✅ 完成 | ReAct loop, 流式, Context, AgentType (5种), 事件系统 |
| 会话 | ⚠️ 基础 | Session, SessionManager, load/save (需增强持久化) |
| TUI | ✅ 完成 | ratatui, App, chat_panel, input, status_bar, help, theme, syntax |
| MCP | ✅ 完成 | JSON-RPC stdio client, server, Context7 |
| 权限 | ⚠️ 基础 | Policy, PermissionEvaluator, Action (需分层规则) |
| 子智能体 | ⚠️ 基础 | Spawn, Supervisor (需角色系统和增强) |
| 技能 | ⚠️ 基础 | Registry, Loader, Builtin (需社区同步/多路径发现) |
| 记忆 | ✅ 完成 | Store, Retrieve, AutoDream |
| 存储 | ✅ 完成 | Database trait, SQLite, 迁移 |
| LSP | ✅ 完成 | Client, Handler |
| 服务器 | ✅ 完成 | axum router, WebSocket |
| OAuth | ✅ 完成 | Flow, OpenCode |
| 计算机 | ✅ 完成 | Screenshot, Mouse, Keyboard |
| 语音 | ✅ 完成 | Input, Output |
| 团队 | ⚠️ 基础 | 基础任务模块 (需增强) |
| Worktree | ✅ 完成 | Manager |
| 同步 | ✅ 完成 | Cloud |
| 工具 | ✅ 完成 | Path, Format, Template |
| 特性标志 | ✅ 完成 | 23个 feature flags |

### 1.2 缺失功能清单（按优先级）

| # | 功能 | 优先级 | 复杂度 | DeepSeek TUI 对应模块 |
|---|------|--------|--------|----------------------|
| 1 | **apply_patch 工具** | P0 | 中 | tools/apply_patch |
| 2 | **list_dir 工具** | P0 | 低 | tools/file (list_dir) |
| 3 | **checklist_write 工具** | P0 | 低 | tools/checklist |
| 4 | **Plan/Agent/YOLO 三模式** | P0 | 中 | core/modes, execpolicy |
| 5 | **推理强度切换** | P0 | 低 | llm_client (reasoning_effort) |
| 6 | **实时成本跟踪** | P1 | 中 | pricing.rs, llm_client |
| 7 | **思考模式流式显示** | P1 | 低 | tui/streaming |
| 8 | **LSP 编辑后诊断注入** | P1 | 高 | core/engine/lsp_hooks |
| 9 | **审计日志** | P1 | 低 | audit.rs |
| 10 | **大工具输出路由** | P1 | 中 | core/capacity_flow |
| 11 | **智能上下文压缩** | P1 | 中 | compaction.rs |
| 12 | **崩溃恢复 + 离线队列** | P1 | 中 | session/checkpoints |
| 13 | **工作区回滚 (side-git)** | P1 | 高 | snapshot/ |
| 14 | **持久化任务队列** | P1 | 高 | task_manager.rs |
| 15 | **子智能体7角色系统** | P2 | 高 | tools/subagent (7 roles) |
| 16 | **Bash 参数匹配字典** | P2 | 中 | execpolicy/bash_arity |
| 17 | **分层权限规则** | P2 | 中 | execpolicy |
| 18 | **技能社区注册表同步** | P2 | 中 | skills/registry, commands/skills |
| 19 | **多语言 UI** | P2 | 中 | tui/i18n |
| 20 | **FIM 编辑工具** | P2 | 中 | tools/fim_edit |
| 21 | **可插拔沙箱后端** | P2 | 高 | sandbox/ |
| 22 | **RLM 递归语言模型** | P2 | 极高 | tools/rlm |
| 23 | **GitHub 工具** (issue/PR/comment) | P2 | 中 | tools/github |
| 24 | **用户定义斜杠命令** | P2 | 中 | commands/user_commands |
| 25 | **自动化 cron 系统** | P3 | 高 | automation_manager |
| 26 | **PR 尝试追踪** | P3 | 中 | tools/pr_attempt |
| 27 | **验证门 (task_gate_run)** | P3 | 中 | tools/task_gate |
| 28 | **运行时 HTTP/SSE API** | P3 | 高 | app-server |
| 29 | **Vim 模态编辑** | P3 | 低 | tui/input (vim mode) |
| 30 | **快捷键帮助面板** (F1) | P3 | 低 | tui/help |
| 31 | **命令面板** (Ctrl+K) | P3 | 中 | tui/command_palette |
| 32 | **多位置技能发现** | P3 | 中 | skills/loader |

---

## 2. 移植范围

### 范围说明

本计划涵盖将 DeepSeek TUI v0.8.12 的所有功能移植到 coder 项目。移植不是逐行复制，而是**概念移植**——在 coder 的现有架构上重新实现 DeepSeek TUI 的功能，并优先复用 coder 现有模块。

### 不纳入范围

- 与 DeepSeek V4 平台深度绑定的特定 API 逻辑（但保留推理强度等通用概念）
- DeepSeek 专有的定价模型（coder 使用通用定价配置）
- 特定于 macOS 的沙箱实现（但保留沙箱抽象层）

---

## 3. 阶段计划

### 阶段 0: 工具增强 (P0 工具)

**目标:** 补齐缺失的 P0 工具，确保模型有完整的工具集可用。

**文件变更:**

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/tool/apply_patch.rs` | **新建** | Unified diff 应用工具 |
| `src/tool/list_dir.rs` | **新建** | 结构化目录列表工具 |
| `src/tool/checklist.rs` | **新建** | 清单管理工具 (checklist_write/add/update/list) |
| `src/tool/mod.rs` | 修改 | 注册新工具 |
| `src/tool/registry.rs` | 修改 | 注册新工具到默认注册表 |
| `src/tool/task.rs` | 增强 | 改为 SQLite 持久化 |
| `src/tool/git.rs` | 增强 | 添加 `git_blame`、`git_log` 增强、`git_branch` |
| `src/tool/plan.rs` | 增强 | 添加 `update_plan` + 结构化清单 |

**键 API 设计:**

```rust
// apply_patch.rs
pub struct ApplyPatchTool;

// 输入: path, patch (unified diff 字符串)
// 输出: 成功/失败 + 修改的文件行数

// list_dir.rs
pub struct ListDirTool;

// 输入: path, show_hidden, max_depth
// 输出: 结构化目录树 (gitignore-aware)

// checklist.rs
pub struct ChecklistTool;

// 输入: action (write/add/update/list), goal, items[]
// 输出: 结构化清单
```

**依赖:** 无（独立工具实现）  
**复杂度:** 低-中  
**估算工时:** 2-3 天

---

### 阶段 1: 交互模式与引擎增强

**目标:** 实现 Plan/Agent/YOLO 三模式切换、推理强度控制、thinking 流式显示。

**文件变更:**

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/core/mode.rs` | **新建** | Mode enum + 模式行为定义 |
| `src/agent/loop.rs` | 修改 | 集成模式感知的审批逻辑 |
| `src/ai/types.rs` | 修改 | GenerateConfig 添加 reasoning_effort |
| `src/ai/provider.rs` | 修改 | Provider trait 添加 supports_thinking |
| `src/ai/openai.rs` | 修改 | 传递 reasoning_effort 参数 |
| `src/ai/anthropic.rs` | 修改 | 传递 thinking_budget 参数 |
| `src/tui/app.rs` | 修改 | 添加 mode 状态, reasoning_effort 状态 |
| `src/tui/ui.rs` | 修改 | Tab/Shift+Tab 快捷键, 模式指示器 |
| `src/tui/status_bar.rs` | 修改 | 显示当前模式和推理强度 |
| `src/tui/streaming.rs` | **新建** | 思考区块组件 |

**模式行为定义:**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Plan,   // 只读工具 + checklist_write 允许, shell/patch 禁止
    Agent,  // 默认: 工具调用带审批门禁
    YOLO,   // 自动批准所有工具
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReasoningEffort {
    Off,
    Low,
    High,
    Max,
    Auto,  // 根据提示自动选择
}
```

**依赖:** 阶段 0  
**复杂度:** 中  
**估算工时:** 3-4 天

---

### 阶段 2: 成本跟踪与上下文管理

**目标:** 实现实时 token 和成本跟踪、智能上下文压缩、崩溃恢复。

**文件变更:**

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/core/pricing.rs` | **新建** | 定价模型 + 成本估算 |
| `src/core/compaction.rs` | **新建** | 上下文智能压缩 |
| `src/core/checkpoint.rs` | **新建** | 检查点 + 崩溃恢复 + 离线队列 |
| `src/agent/context.rs` | 修改 | 集成压缩逻辑 |
| `src/agent/loop.rs` | 修改 | 添加 cost 事件 |
| `src/ai/types.rs` | 修改 | Usage 添加缓存命中/未命中字段 |
| `src/session/manager.rs` | 修改 | 添加检查点持久化 |
| `src/tui/app.rs` | 修改 | 添加 cost 显示状态 |
| `src/tui/status_bar.rs` | 修改 | 显示 token/cost 信息 |

**关键行为:**

- 上下文 > 500K token 时触发自动压缩
- 每次发送前写检查点到 `~/.coder/checkpoints/latest.json`
- 离线时消息排队到 `~/.coder/checkpoints/offline_queue.json`
- 启动时检测检查点并提示恢复

**依赖:** 阶段 1  
**复杂度:** 中  
**估算工时:** 3-4 天

---

### 阶段 3: 审计、大输出路由、LSP 增强

**目标:** 实现审计日志、大工具输出截断路由、LSP 编辑后诊断注入。

**文件变更:**

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/core/audit.rs` | **新建** | 追加式审计日志 |
| `src/core/capacity.rs` | **新建** | 大工具输出路由和截断 |
| `src/core/lsp_hooks.rs` | **新建** | LSP 编辑后诊断连接器 |
| `src/tool/mod.rs` | 修改 | ToolResult 添加大小跟踪 |
| `src/tool/registry.rs` | 修改 | 输出大小检查 + 路由 |
| `src/lsp/mod.rs` | 修改 | 暴露诊断收集接口 |
| `src/lsp/client.rs` | 修改 | 添加 `collect_diagnostics()` |
| `src/agent/loop.rs` | 修改 | 注入 LSP 诊断到上下文 |

**审计日志格式:**

```rust
#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub timestamp: String,
    pub event_type: AuditEventType,
    pub tool_name: Option<String>,
    pub approval_action: Option<String>,
    pub details: String,
}

pub enum AuditEventType {
    ToolExecution,
    ApprovalGranted,
    ApprovalDenied,
    CredentialAccess,
    ConfigChange,
}
```

**依赖:** 阶段 2  
**复杂度:** 中-高  
**估算工时:** 3-4 天

---

### 阶段 4: 工作区回滚与持久化任务队列

**目标:** 实现 side-git 快照机制和 SQLite 持久化任务队列。

**文件变更:**

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/core/snapshot.rs` | **新建** | Side-git 快照管理 |
| `src/core/task_manager.rs` | **新建** | 持久化任务队列 |
| `src/tool/snapshot_tool.rs` | **新建** | `/restore` 和 `revert_turn` 工具 |
| `src/tool/task.rs` | 增强 | 集成持久化任务管理器 |
| `src/tool/mod.rs` | 修改 | 注册新工具 |
| `src/tool/registry.rs` | 修改 | 注册 snapshot/task 工具 |
| `src/storage/sqlite.rs` | 修改 | 添加任务队列表 |
| `src/storage/migrate.rs` | 修改 | 添加迁移脚本 |
| `src/agent/loop.rs` | 修改 | 自动创建/清理快照 |

**Side-git 快照架构:**

```
~/.coder/snapshots/<project_hash>/<worktree_hash>/
  ├── .git/          # 独立 git repo (--git-dir + --work-tree)
  └── snapshots/     # 快照索引
      ├── pre_turn_1.json
      ├── post_turn_1.json
      └── ...
```

**任务队列 SQLite 表:**

```sql
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'pending',
    prompt TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    result TEXT,
    error TEXT
);

CREATE TABLE task_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    event_type TEXT NOT NULL,
    payload TEXT,
    created_at TEXT NOT NULL
);
```

**依赖:** 阶段 3  
**复杂度:** 高  
**估算工时:** 4-5 天

---

### 阶段 5: 子智能体7角色 + Bash 参数匹配 + 分层权限

**目标:** 实现完整的子智能体角色系统、Bash 参数匹配字典、分层权限规则。

**文件变更:**

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/subagent/roles.rs` | **新建** | 7种角色系统提示定义 |
| `src/subagent/spawn.rs` | 增强 | 集成角色系统和权限继承 |
| `src/subagent/mod.rs` | 修改 | 导出角色类型 |
| `src/execpolicy/arity.rs` | **新建** | Bash 参数匹配字典 |
| `src/execpolicy/mod.rs` | 修改 | 集成分层规则和 arity |
| `src/execpolicy/policy.rs` | 增强 | 添加 builtin/agent/user 层级 |
| `src/tool/subagent_tool.rs` | 修改 | 更新 agent_spawn 接口 |
| `src/tool/mod.rs` | 修改 | 注册新工具 |
| `src/tool/registry.rs` | 修改 | 注册 |

**子智能体角色:**

```rust
pub enum SubAgentRole {
    General,      // 灵活执行 (默认)
    Explore,      // 只读探索
    Plan,         // 分析设计
    Review,       // 审计评分
    Implementer,  // 精确实施
    Verifier,     // 验证报告
    Custom,       // 显式限制 (需 allowed_tools)
}
```

**Bash Arity 字典:**

```rust
// 内置已知命令的参数结构
// "git status" 匹配 "git status -s" 但不匹配 "git push"
// "cargo build" 匹配 "cargo build --release"
pub struct ArityDictionary {
    commands: HashMap<String, Vec<String>>,  // cmd → [subcommands...]
}

impl ArityDictionary {
    pub fn matches(&self, auto_allow: &str, actual: &str) -> bool;
}
```

**依赖:** 阶段 4  
**复杂度:** 高  
**估算工时:** 4-5 天

---

### 阶段 6: 技能系统增强 + GitHub 工具 + 用户命令

**目标:** 实现技能社区注册表同步、GitHub issue/PR 工具、用户定义斜杠命令。

**文件变更:**

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/skill/registry.rs` | 增强 | 添加 `/skills sync` 社区注册表 |
| `src/skill/loader.rs` | 增强 | 多路径发现 (6个路径) |
| `src/commands/mod.rs` | **新建** | 用户定义命令系统 |
| `src/commands/user_commands.rs` | **新建** | `$1`/`$2`/`$ARGUMENTS` 模板 |
| `src/tool/github.rs` | **新建** | GitHub issue/PR/comment 工具 |
| `src/tool/mod.rs` | 修改 | 注册 GitHub 工具 |
| `src/tool/registry.rs` | 修改 | 注册 |
| `src/tui/commands/mod.rs` | 修改 | 集成用户命令 |

**技能发现路径:**

1. `.agents/skills/**/SKILL.md`
2. `skills/**/SKILL.md`
3. `.opencode/skills/**/SKILL.md`
4. `.claude/skills/**/SKILL.md`
5. `~/.coder/skills/**/SKILL.md`
6. 用户配置路径

**GitHub 工具:**

```rust
// github_issue_context: 只读 Issue 上下文
// github_pr_context: 只读 PR 上下文
// github_comment: 需审批的评论
// github_close_issue: 需审批的关闭
```

**依赖:** 阶段 5  
**复杂度:** 中  
**估算工时:** 3-4 天

---

### 阶段 7: FIM 编辑 + 沙箱后端 + 多语言 UI

**目标:** 实现 FIM fill-in-the-middle 编辑、可插拔沙箱后端、多语言支持。

**文件变更:**

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/tool/fim_edit.rs` | **新建** | FIM 编辑工具 (fill-in-the-middle) |
| `src/sandbox/mod.rs` | **新建** | SandboxBackend trait |
| `src/sandbox/local.rs` | **新建** | 本地沙箱 (默认) |
| `src/sandbox/remote.rs` | **新建** | 远程沙箱 (OpenSandbox) |
| `src/tool/bash.rs` | 修改 | 集成沙箱路由 |
| `src/config/settings.rs` | 修改 | 沙箱配置 |
| `src/i18n/mod.rs` | **新建** | 国际化框架 |
| `src/i18n/translations.rs` | **新建** | 翻译 (en/ja/zh-Hans/pt-BR) |
| `src/tui/mod.rs` | 修改 | 集成 i18n |
| `src/tui/theme.rs` | 修改 | Color::Reset 适配 |

**FIM 编辑:**

```rust
// 发送 fill-in-the-middle 请求到 /beta 端点
// 输入: path, code_before, code_after, instructions
// 输出: 生成的代码
```

**SandboxBackend trait:**

```rust
#[async_trait]
pub trait SandboxBackend: Send + Sync {
    async fn execute(&self, command: &str, workdir: &str, timeout: u64) -> Result<SandboxResult, String>;
    fn name(&self) -> &str;
}
```

**依赖:** 阶段 6  
**复杂度:** 高  
**估算工时:** 4-5 天

---

### 阶段 8: RLM + 自动化 + PR 尝试

**目标:** 实现 RLM 递归语言模型、自动化 cron 系统、PR 尝试追踪。

**文件变更:**

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/tool/rlm.rs` | **新建** | RLM 工具 (Python REPL + llm_query) |
| `src/core/automation.rs` | **新建** | 自动化管理器 (cron 调度) |
| `src/tool/automation_tool.rs` | **新建** | 自动化管理工具 |
| `src/tool/pr_attempt.rs` | **新建** | PR 尝试追踪工具 |
| `src/tool/task_gate.rs` | **新建** | 验证门工具 |
| `src/tool/mod.rs` | 修改 | 注册新工具 |
| `src/tool/registry.rs` | 修改 | 注册 |
| `src/storage/sqlite.rs` | 修改 | 自动化/PR 表 |
| `src/storage/migrate.rs` | 修改 | 迁移脚本 |
| `src/config/settings.rs` | 修改 | 自动化配置 |

**RLM 架构:**

```
┌──────────────┐     ┌──────────────────────┐
│  rlm_query   │────→│  Python Sandbox REPL  │
│  (工具调用)   │     │                      │
└──────────────┘     │  llm_query()          │
                     │  llm_query_batched()  │
                     │  rlm_query()          │
                     └──────────┬───────────┘
                                │
                     ┌──────────▼───────────┐
                     │  1-16 并行子任务      │
                     │  (deepseek-v4-flash)  │
                     └──────────────────────┘
```

**依赖:** 阶段 7  
**复杂度:** 极高  
**估算工时:** 5-7 天

---

### 阶段 9: HTTP/SSE API + TUI 增强

**目标:** 实现运行时 HTTP/SSE API 服务、Vim 模态编辑、命令面板、帮助面板。

**文件变更:**

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/server/runtime.rs` | **新建** | 运行时 HTTP/SSE API |
| `src/server/mod.rs` | 修改 | 集成运行时 API |
| `src/tui/vim.rs` | **新建** | Vim 模态编辑 |
| `src/tui/command_palette.rs` | **新建** | Ctrl+K 命令面板 |
| `src/tui/help.rs` | 增强 | F1 可搜索帮助面板 |
| `src/tui/input.rs` | 修改 | 集成 Vim 模式 |
| `src/tui/mod.rs` | 修改 | 注册新组件 |
| `src/config/settings.rs` | 修改 | Vim 配置 |

**HTTP/SSE API 端点:**

| 端点 | 方法 | 说明 |
|------|------|------|
| `/v1/threads` | GET/POST | 管理会话线程 |
| `/v1/threads/{id}/turns` | POST | 创建轮次 |
| `/v1/threads/{id}/events` | GET | SSE 事件流 |
| `/v1/tasks` | GET/POST | 任务管理 |
| `/v1/health` | GET | 健康检查 |

**依赖:** 阶段 8  
**复杂度:** 高  
**估算工时:** 4-5 天

---

### 阶段 10: 最终集成与优化

**目标:** 全面测试、性能优化、文档补充、bug 修复。

**文件变更:**

| 文件 | 操作 | 说明 |
|------|------|------|
| 全模块 | 测试增强 | 补齐测试覆盖率到 80%+ |
| `src/lib.rs` | 修改 | 功能标志完整性检查 |
| `Cargo.toml` | 修改 | 最终特性标志调整 |
| `DOCS/*` | **新建** | 用户文档 |
| `README.md` | 修改 | 更新功能列表 |

**验证清单:**

- [ ] `cargo test --workspace --all-features` 通过
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过
- [ ] `cargo build --release` 成功
- [ ] TUI 所有模式可正常切换 (Plan/Agent/YOLO)
- [ ] 所有工具可正确执行
- [ ] 会话保存/恢复正常
- [ ] MCP 连接正常
- [ ] LSP 诊断正常

**依赖:** 阶段 9  
**复杂度:** 中  
**估算工时:** 3-4 天

---

## 4. 文件变更清单

### 新建文件 (约 30 个)

```
src/tool/apply_patch.rs          src/tool/rlm.rs
src/tool/list_dir.rs             src/tool/fim_edit.rs
src/tool/checklist.rs            src/tool/github.rs
src/core/mode.rs                 src/tool/pr_attempt.rs
src/core/pricing.rs              src/tool/task_gate.rs
src/core/compaction.rs           src/tool/snapshot_tool.rs
src/core/checkpoint.rs           src/tool/automation_tool.rs
src/core/audit.rs                src/execpolicy/arity.rs
src/core/capacity.rs             src/subagent/roles.rs
src/core/lsp_hooks.rs            src/commands/mod.rs
src/core/snapshot.rs             src/commands/user_commands.rs
src/core/task_manager.rs         src/sandbox/mod.rs
src/core/automation.rs           src/sandbox/local.rs
src/tui/streaming.rs             src/sandbox/remote.rs
src/tui/vim.rs                   src/i18n/mod.rs
src/tui/command_palette.rs       src/i18n/translations.rs
src/server/runtime.rs
```

### 修改文件 (约 25 个)

```
src/tool/mod.rs                  src/session/manager.rs
src/tool/registry.rs             src/lsp/mod.rs
src/tool/bash.rs                 src/lsp/client.rs
src/tool/task.rs                 src/server/mod.rs
src/tool/git.rs                  src/storage/sqlite.rs
src/tool/plan.rs                 src/storage/migrate.rs
src/tool/docker.rs               src/config/settings.rs
src/ai/types.rs                  src/tui/app.rs
src/ai/provider.rs               src/tui/ui.rs
src/ai/openai.rs                 src/tui/input.rs
src/ai/anthropic.rs              src/tui/mod.rs
src/agent/loop.rs                src/tui/status_bar.rs
src/agent/context.rs             src/tui/help.rs
src/subagent/spawn.rs            src/tui/theme.rs
src/subagent/mod.rs              src/lib.rs
src/execpolicy/mod.rs            Cargo.toml
src/execpolicy/policy.rs         README.md
src/skill/registry.rs
src/skill/loader.rs
src/permission/mod.rs
```

---

## 5. 依赖关系图

```
阶段 0 (工具增强)
  │
  ▼
阶段 1 (模式 + 推理强度) ◄─── 阶段 0
  │
  ▼
阶段 2 (成本 + 压缩 + 恢复) ◄─── 阶段 1
  │
  ▼
阶段 3 (审计 + 大输出 + LSP) ◄─── 阶段 2
  │
  ▼
阶段 4 (回滚 + 任务队列) ◄─── 阶段 3
  │
  ▼
阶段 5 (子智能体 + 权限 + arity) ◄─── 阶段 4
  │
  ▼
阶段 6 (技能 + GitHub + 用户命令) ◄─── 阶段 5
  │
  ▼
阶段 7 (FIM + 沙箱 + i18n) ◄─── 阶段 6
  │
  ▼
阶段 8 (RLM + 自动化 + PR) ◄─── 阶段 7
  │
  ▼
阶段 9 (API + 编辑器增强) ◄─── 阶段 8
  │
  ▼
阶段 10 (集成 + 测试 + 文档) ◄─── 阶段 9
```

---

## 6. 验证策略

### 每阶段验证

每个阶段完成后必须通过:

1. **编译检查:** `cargo build --all-features`
2. **单元测试:** `cargo test --workspace --all-features`
3. **Lint 检查:** `cargo clippy --workspace --all-targets --all-features -- -D warnings`
4. **功能验证:** 手动测试该阶段新增功能

### 集成测试重点

| 功能 | 测试方法 |
|------|---------|
| Plan/Agent/YOLO 模式 | 单元测试模式转换 + 权限行为 |
| 工具执行 | 每个工具 3+ 测试用例 (成功/失败/边界) |
| 会话持久化 | 创建 → 保存 → 恢复 → 验证消息完整 |
| MCP 连接 | Mock stdio 进程测试 JSON-RPC |
| 成本估算 | 已知 token 数 × 固定价格 = 验证计算结果 |
| 上下文压缩 | 构造过大会话 → 触发压缩 → 验证 token 减少 |
| 子智能体 | 生成 → 执行 → 收集结果 → 验证输出格式 |
| 快照回滚 | 创建 → 修改文件 → 快照 → 恢复 → 验证文件状态 |
| 任务队列 | 创建 → 持久化 → 重启 → 验证任务状态 |

### 端到端测试

```bash
# 编译测试
cargo build --release

# 单元 + 集成测试
cargo test --workspace --all-features

# 文档测试
cargo test --doc

# 性能基准 (可选)
cargo bench
```

---

## 7. 工时估算

| 阶段 | 描述 | 估算工时 |
|------|------|---------|
| 0 | 工具增强 | 2-3 天 |
| 1 | 模式与引擎 | 3-4 天 |
| 2 | 成本与上下文 | 3-4 天 |
| 3 | 审计与 LSP | 3-4 天 |
| 4 | 回滚与任务 | 4-5 天 |
| 5 | 子智能体与权限 | 4-5 天 |
| 6 | 技能与 GitHub | 3-4 天 |
| 7 | FIM 与沙箱 | 4-5 天 |
| 8 | RLM 与自动化 | 5-7 天 |
| 9 | API 与编辑器 | 4-5 天 |
| 10 | 集成与测试 | 3-4 天 |
| **总计** | | **38-50 天** |

> **注意:** 以上估算基于单人全职开发。使用多 agent 并行开发可显著缩短日历时间。

---

## 8. 并行执行策略

### 可并行阶段

以下阶段互相独立，可以并行开发：

- **阶段 0a** (apply_patch, list_dir, checklist) ↔ **阶段 0b** (task, git, plan 增强)
- **阶段 1** (模式) ↔ **阶段 3** (审计) [部分独立]
- **阶段 6** (技能 + GitHub) ↔ **阶段 7** (FIM + i18n)
- **阶段 8** (RLM) ↔ **阶段 9** (API) [需架构对齐]

### 建议团队分工

| 角色 | 负责阶段 |
|------|---------|
| 核心引擎工程师 | 阶段 0-2, 4, 8 |
| 工具/SDK 工程师 | 阶段 3, 5-7 |
| TUI/前端工程师 | 阶段 1(TUI部分), 9 |
| QA/测试工程师 | 全阶段 + 阶段 10 |

---

## 9. 风险与缓解

| 风险 | 影响 | 可能性 | 缓解 |
|------|------|--------|------|
| RLM (Python 沙箱) 复杂度高 | 延迟 | 高 | 先实现简化版 (仅 llm_query), 后续迭代增强 |
| Side-git 跨平台兼容问题 | 质量问题 | 中 | 优先支持 Linux/macOS, Windows 使用备用方案 |
| LSP 多语言服务器集成 | 集成难度 | 中 | 从 rust-analyzer 开始逐步扩展 |
| 自动化 cron 在非 7x24 环境 | 功能受限 | 低 | 任务持久化 + 启动时检查过期任务 |
| 特性标志膨胀 | 维护负担 | 中 | 阶段 10 统一清理, 合并相关标志 |
