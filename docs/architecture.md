# Coder 技术架构文档

> 版本: 0.1.0 | 最后更新: 2026-05-05

---

## 1. 项目概述

### 1.1 项目定位

Coder 是一款用 Rust 编写的 AI 驱动开发工具，集成了 Claude Code 和 OpenCode 的核心能力。它通过终端用户界面 (TUI) 与 AI 模型交互，提供代码编写、调试、代码审查、团队协作等全栈开发体验。

### 1.2 技术栈

| 层面 | 技术选型 | 用途 |
|------|---------|------|
| 语言 | Rust 2021 edition | 系统编程、高性能、内存安全 |
| 异步运行时 | Tokio (full) | 异步 I/O、任务调度、流处理 |
| TUI 框架 | Ratatui 0.29 + Crossterm 0.28 | 终端 UI 渲染与事件处理 |
| CLI 框架 | Clap 4.5 (derive + env) | 命令行参数解析 |
| HTTP 客户端 | Reqwest 0.12 (json, stream, socks) | AI API 调用、Web 抓取 |
| 序列化 | Serde + Serde JSON | 配置解析、消息序列化 |
| 日志 | Tracing + Tracing Subscriber | 结构化日志和诊断 |
| 数据库 | libSQL 0.6 (SQLite) | 持久化存储 |
| 语法高亮 | Syntect 5.2 | 代码块语法着色 |
| LSP | Tower-lsp 0.20 | 语言服务器协议集成 |
| 版本控制 | gix 0.66 | Git 操作（纯 Rust 实现） |
| Web 服务器 | Axum 0.7 (ws) | HTTP API 与 WebSocket |
| Docker | Bollard 0.17 | Docker 容器管理 |
| OAuth | oauth2 5.0 | OAuth 2.0 认证流程 |

### 1.3 设计哲学

- **模块化分层**：核心抽象（Provider、Tool、Agent）+ 可选功能（feature-gated）
- **不可变性优先**：数据传递以创建新实例为主，避免就地修改
- **异步优先**：全异步 I/O 栈，从 AI 请求到工具执行均为异步
- **扩展性**：通过 trait 定义接口，第三方可实现自定义 Provider、Tool、Skill
- **渐进式复杂度**：简单场景（`--print`）到完整 TUI，同一核心支撑不同运行模式

---

## 2. 系统架构

### 2.1 分层架构图

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI Layer (main.rs)                   │
│              Clap CLI  →  三种运行模式分发                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐  │
│   │ TUI Mode │  │ Headless │  │  Print   │  │ API Server │  │
│   │ (ratatui)│  │ (stdio)  │  │ (oneshot)│  │ (axum)     │  │
│   └────┬─────┘  └────┬─────┘  └────┬─────┘  └─────┬──────┘  │
│        └──────────────┴─────────────┴──────────────┘         │
│                            │                                  │
├────────────────────────────┼──────────────────────────────────┤
│                     Core Engine                               │
│   ┌──────────────────────────────────────────────────┐      │
│   │                   Agent (ReAct Loop)              │      │
│   │  ┌─────────┐  ┌──────────┐  ┌─────────────────┐  │      │
│   │  │Context  │  │Provider  │  │ ToolRegistry    │  │      │
│   │  │Manager  │──│(AI API)  │──│ (Tool 集合)      │  │      │
│   │  └─────────┘  └──────────┘  └─────────────────┘  │      │
│   └──────────────────────────────────────────────────┘      │
├──────────────────────────────────────────────────────────────┤
│                    AI Provider Layer                           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────────┐   │
│  │ OpenAI   │ │Anthropic │ │ Google   │ │ Custom        │   │
│  │ Compat.  │ │ Claude   │ │ Gemini   │ │ (User Defined)│   │
│  └──────────┘ └──────────┘ └──────────┘ └───────────────┘   │
├──────────────────────────────────────────────────────────────┤
│                    Tool Layer                                  │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐     │
│  │Bash  │ │File  │ │File  │ │File  │ │Glob  │ │Grep  │     │
│  │      │ │Read  │ │Write │ │Edit  │ │      │ │      │     │
│  ├──────┤ ├──────┤ ├──────┤ ├──────┤ ├──────┤ ├──────┤     │
│  │Web   │ │Web   │ │Docs  │ │Task  │ │Plan  │ │ ...  │     │
│  │Fetch │ │Search│ │      │ │      │ │      │ │      │     │
│  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘     │
├──────────────────────────────────────────────────────────────┤
│               Feature Systems (Phase 1, feature-gated)        │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐    │
│  │Team  │ │Skill │ │Sub-  │ │Memory│ │ LSP  │ │ MCP  │    │
│  │      │ │      │ │agent │ │      │ │      │ │      │    │
│  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘    │
├──────────────────────────────────────────────────────────────┤
│                         Storage Layer                         │
│              ┌──────────────────────────────┐                │
│              │  Database trait → SQLite/libSQL               │
│              │  (Sessions, Memories, Events, Config)         │
│              └──────────────────────────────┘                │
└──────────────────────────────────────────────────────────────┘
```

### 2.2 三层架构说明

**展示层 (Presentation)**: 四种运行模式共享同一核心引擎
- TUI 模式: 全功能终端界面，ratatui 渲染
- Headless 模式: 标准 I/O 交互
- Print 模式: 一次性查询后退出
- Server 模式: Axum HTTP + WebSocket API

**核心引擎 (Core Engine)**: Agent ReAct 循环，协调 Provider 和 ToolRegistry

**基础设施 (Infrastructure)**: AI Provider 实现、Tool 实现、持久化层

### 2.3 源文件结构

```
src/
├── main.rs                 # CLI 入口，三种模式分发
├── lib.rs                  # 库根，feature-gated 模块声明
│
├── config/                 # 配置系统 (5 级优先级)
│   ├── settings.rs         # Settings + AiSettings + UiSettings 等
│   ├── provider_config.rs  # ProviderConfig (type, api_key, model)
│   └── theme.rs            # 主题色配置
│
├── ai/                     # AI Provider 抽象层
│   ├── provider.rs         # Provider trait 定义
│   ├── types.rs            # Message, ContentBlock, StreamEvent 等
│   ├── openai.rs           # OpenAI 兼容实现
│   ├── anthropic.rs        # Anthropic Claude 实现
│   ├── google.rs           # Google Gemini 实现
│   └── custom.rs           # 自定义 Provider
│
├── agent/                  # Agent ReAct 循环
│   ├── loop.rs             # Agent 结构体 + run_stream
│   ├── context.rs          # 上下文管理 (消息 + 系统提示词)
│   ├── dispatch.rs         # Agent 分发逻辑
│   └── types.rs            # AgentType 枚举
│
├── tool/                   # Tool 系统
│   ├── registry.rs         # ToolRegistry
│   ├── mod.rs              # Tool trait
│   ├── bash.rs / file_*.rs # 核心工具
│   ├── web_fetch.rs / web_search.rs / docs.rs
│   ├── task.rs / plan.rs   # 任务与规划工具
│   └── git.rs / docker.rs / db_query.rs  # 可选工具 (feature-gated)
│
├── tui/                    # 终端用户界面
│   ├── app.rs              # App 状态机
│   ├── chat_panel.rs       # 对话渲染
│   ├── input.rs            # 输入处理
│   ├── help.rs             # 帮助系统
│   └── theme.rs            # TUI 色彩主题
│
├── session/                # 会话管理
│   ├── manager.rs          # SessionManager (文件持久化)
│   └── history.rs          # 对话历史
│
├── server/                 # HTTP API 服务 (phase 2)
│   ├── router.rs           # 路由定义
│   ├── handler_session.rs  # 会话 CRUD
│   ├── handler_tools.rs    # 工具执行
│   └── ws.rs               # WebSocket
│
├── storage/                # 存储层 (phase 1)
│   ├── db.rs               # Database trait
│   ├── sqlite.rs           # SQLite 实现
│   └── migrate.rs          # 数据库迁移
│
├── team/                   # 团队协作 (phase 1)
├── skill/                  # 技能系统 (phase 1)
├── subagent/               # 子代理系统 (phase 1)
├── memory/                 # 记忆系统 (phase 1)
├── mcp/                    # MCP 协议 (phase 1)
├── lsp/                    # LSP 集成 (phase 1)
├── permission/             # 权限系统 (phase 2)
├── sync/                   # 云同步 (phase 2)
├── voice/                  # 语音模块 (phase 2)
├── oauth/                  # OAuth (phase 2)
├── computer/               # 电脑操控 (phase 2)
├── analytics/              # 分析统计 (phase 2)
└── util/                   # 工具函数
    ├── path.rs             # 路径解析
    ├── format.rs           # 格式化工具
    └── template.rs         # 模板处理
```

---

## 3. 核心抽象

### 3.1 Provider Trait — AI 提供商抽象

`src/ai/provider.rs`

所有 AI 提供商必须实现 `Provider` trait，这是系统与外部 AI API 交互的唯一接口：

```rust
#[async_trait]
pub trait Provider: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn supports_thinking(&self) -> bool { false }

    async fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        config: &GenerateConfig,
    ) -> anyhow::Result<StreamHandler>;

    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        config: &GenerateConfig,
    ) -> anyhow::Result<Message> { /* 基于 chat_stream 的默认实现 */ }
}
```

**设计要点**:
- `StreamHandler` = `tokio::sync::mpsc::Receiver<StreamEvent>` — 基于 channel 的异步流
- `chat()` 有默认实现，基于 `chat_stream()` 聚合结果
- `supports_thinking()` 用于标识是否支持扩展思考能力（如 Claude 的 thinking）
- `Send + Sync` 约束确保 Provider 可以在 Tokio 任务间安全共享

**当前实现**:

| 实现 | 特性门控 | 支持 API |
|------|---------|---------|
| `OpenAIProvider` | `ai-openai` | OpenAI / DeepSeek / Ollama / Groq / MiniMax |
| `AnthropicProvider` | `ai-anthropic` | Claude 系列 (含 thinking) |
| `GoogleProvider` | `ai-google` | Gemini 系列 |
| `CustomProvider` | `ai-opencode` | 用户自定义请求/响应模板 |

**工厂函数** (`src/ai/mod.rs`):
```rust
pub fn create_provider(
    name: &str,
    config: ProviderConfig,
    model_override: Option<String>,
) -> anyhow::Result<Box<dyn Provider>>
```
基于 `config.provider_type` 字符串 (`"openai"`, `"anthropic"`, `"google"`, `"custom"`) 分发到不同的 Provider 构造函数。

### 3.2 Tool Trait — 工具抽象

`src/tool/mod.rs`

所有工具必须实现 `Tool` trait：

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> serde_json::Value;       // JSON Schema 输入定义
    async fn execute(&self, args: serde_json::Value) -> ToolResult;
    fn requires_permission(&self) -> bool { false }
}
```

**设计要点**:
- `schema()` 返回 JSON Schema 描述工具的输入参数，AI 据此生成正确的参数
- `ToolResult` 包含 `success`, `output`, `error`, `metadata` 四个字段
- `requires_permission()` 控制是否在执行前需要用户授权
- `SharedTool = Arc<dyn Tool>` 支持工具共享引用

**ToolResult 结构**:
```rust
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}
```

### 3.3 ToolRegistry — 工具注册中心

`src/tool/registry.rs`

```rust
pub struct ToolRegistry {
    tools: HashMap<String, SharedTool>,
}
```

核心方法:
- `register(tool)` — 注册工具
- `get(name)` — 按名称查找
- `tool_defs()` — 转换为 AI API 所需的 `Vec<ToolDef>` 格式
- `execute(name, args)` — 异步执行工具

**默认注册** (`Default::default()`) 包含:
- 核心工具: `bash`, `file_read`, `file_write`, `file_edit`, `glob`, `grep`, `question`
- 网络工具: `web_fetch`, `web_search`
- 条件工具 (feature-gated): `git`, `docker`, `db_query`, `oauth`, `worktree`

### 3.4 Agent ReAct 循环

`src/agent/loop.rs`

Agent 是系统的核心编排器：

```rust
pub struct Agent {
    provider: Box<dyn Provider>,
    tools: ToolRegistry,
    context: Context,
    agent_type: AgentType,
    session: Session,
    #[cfg(feature = "permission")]
    permission_evaluator: Option<PermissionEvaluator>,
}
```

**核心循环** (`run_stream`):

```
用户输入
    │
    ▼
添加到 Context
    │
    ▼
构建请求 (系统提示词 + 消息历史)
    │
    ▼
Provider.chat_stream()  ──→ 逐块发送 TextChunk 事件
    │
    ▼ (检测到工具调用)
提取 ToolCall
    │
    ▼
检查权限 → ToolRegistry.execute() → 执行工具
    │
    ▼
将工具结果添加到 Context
    │
    ▼
┌─────────────────────────────────────┐
│ 循环 (最多 10 轮)                    │
│ 直到无更多工具调用 → 结束            │
└─────────────────────────────────────┘
    │
    ▼
发送 Done 事件 (含用量统计)
```

**AgentEvent 枚举** — TUI 消费的事件流：
```rust
pub enum AgentEvent {
    ThinkingStart { provider, model },
    TextChunk(String),
    ToolCallStart { id, name },
    ToolResult { tool_name, result },
    Done { stop_reason, usage },
    Error(String),
}
```

**三种运行模式**:
| 方法 | 用途 | 返回 |
|------|------|------|
| `run_simple(query)` | `--print` 模式 | `Result<String>` |
| `run_interactive()` | Headless 模式 | `Result<()>` (stdin/stdout) |
| `run_stream(input)` | TUI 模式 | `Receiver<AgentEvent>` |

### 3.5 Context Manager — 上下文管理

`src/agent/context.rs`

```rust
pub struct Context {
    messages: Vec<Message>,   // 完整消息历史
    max_tokens: u64,          // 触发压缩前的最大 token 数
    system_prompt: Option<String>,  // 系统提示词
}
```

关键方法:
- `build_request()` — 在消息列表前添加系统提示词，构建 API 请求
- `add_message(msg)` — 追加消息到上下文
- `compact()` — 保留最近 10 条消息（暂时直接截断，未来实现摘要压缩）
- `clear()` — 清空消息历史（保留系统提示词）

**AgentType 系统提示词**:
每种 AgentType 有独立的 system prompt，定义在 `src/agent/types.rs`:
- `Coding`: 完整工具访问，通用编码
- `Research`: 专注于信息检索
- `Debug`: 系统性调试方法论
- `Plan`: 规划分析模式（只读工具）
- `Review`: 代码审查专家

---

## 4. AI 提供商实现细节

### 4.1 OpenAI 兼容提供商

`src/ai/openai.rs`

支持 OpenAI API 格式的所有服务（OpenAI、DeepSeek、Ollama、Groq、MiniMax 等）。

**请求构建**: `build_request()` 将内部 `Message` 转换为 OpenAI Chat Completions 格式：
```json
{
  "model": "deepseek-chat",
  "messages": [{"role": "user", "content": "..."}],
  "tools": [{"type": "function", "function": {"name": "...", "parameters": {...}}}],
  "stream": true
}
```

**流式解析**: `parse_sse_stream_public()` 解析 SSE (Server-Sent Events) 流，处理 `data: ...` 行和 `[DONE]` 终止标记。每个 JSON chunk 经 `process_sse_data()` 分发为 `StreamEvent`。

**关键数据结构**:
- `PendingToolCall` — 处理增量 `tool_calls` delta（OpenAI 的流式工具调用可能是分块的）

### 4.2 Anthropic Claude 提供商

`src/ai/anthropic.rs`

**消息映射**: 将内部 `Message` 转换为 Anthropic Messages API 格式：
- `System` 角色 → 提取到顶层 `system` 字段
- `Tool` 角色 → 转换为 `role: "user"`（Anthropic 的 tool_result 格式）
- 支持 `anthropic-beta: thinking-2025-01-01` 头用于扩展思考

**SSE 事件处理**（Anthropic 特有的事件类型）:
| 事件类型 | 处理逻辑 |
|---------|---------|
| `message_start` | 提取初始用量统计 |
| `content_block_start` | 识别 text 或 tool_use 类型 |
| `content_block_delta` | `text_delta` → TextChunk; `input_json_delta` → 累积工具参数 |
| `content_block_stop` | 完成工具参数收集 → 发送 ToolCallStart |
| `message_delta` | 更新 stop_reason 和用量 |
| `message_stop` | 流结束 |

### 4.3 自定义提供商

`src/ai/custom.rs`

用户通过 `request_template` 和 `response_parser` 定义自己的 API 交互格式，实现与任意 HTTP API 的集成。

---

## 5. 数据流

### 5.1 一次完整对话的数据流

以用户输入 "找出项目中所有 TODO" 为例：

```
Step 1: 用户输入
  TUI Input → App.send_message() → Agent.run_stream("找出项目中所有 TODO")
  
Step 2: 构建请求
  Context.build_request()
    → [System Prompt] + [User: "找出项目中所有 TODO"]
  
Step 3: Provider 调用
  OpenAIProvider.chat_stream(messages, [grep, file_read, ...], config)
    → HTTP POST https://api.openai.com/v1/chat/completions (SSE stream)
    → 返回 Receiver<StreamEvent>
  
Step 4: 流式响应
  StreamEvent::TextChunk("我来搜索项目中的 TODO 注释...")
  StreamEvent::ToolCallStart { id: "call_1", name: "grep", arguments: '{"pattern":"TODO"}' }
  
Step 5: 工具执行
  Agent 检查权限 → ToolRegistry.execute("grep", {"pattern": "TODO"})
    → GrepTool 执行 ripgrep 搜索
    → 返回 ToolResult { success: true, output: "src/main.rs:42: // TODO: ..." }
  
Step 6: 工具结果加入上下文
  Context.add_message(Message::tool_result("call_1", "src/main.rs:42: // TODO: ..."))
  
Step 7: 继续循环
  Provider.chat_stream([..., tool_result])
    → StreamEvent::TextChunk("在 src/main.rs 第 42 行发现了一个 TODO...")
    → StreamEvent::Done { stop_reason: "end_turn" }
  
Step 8: 呈现结果
  AgentEvent::TextChunk → TUI 渲染到 chat_panel
  AgentEvent::Done → TUI 回到 Input 模式
```

### 5.2 事件流转架构

```
用户键盘输入
    │
    ▼
crossterm::event::read()  (输入线程)
    │
    ▼
App 状态机处理
    │
    ▼
Agent.run_stream() ──→ tokio::spawn ──→ mpsc::Receiver<AgentEvent>
    │                                              │
    │                                              ▼
    │                                       TUI 事件循环
    │                                       App.handle_event()
    │                                              │
    │                                              ▼
    │                                       ratatui::Frame 渲染
    │                                              │
    └──────────────────────────────────────────────┘
```

---

## 6. Session 与持久化

### 6.1 会话生命周期

```rust
pub struct Session {
    pub id: String,           // UUID v4
    pub created_at: String,   // RFC 3339
    pub updated_at: String,
    pub title: String,
    pub messages: Vec<Message>,
    pub metadata: HashMap<String, String>,
}
```

**持久化策略**:
- **SessionManager** (`src/session/manager.rs`): JSON 文件存储于 `~/.coder/sessions/{uuid}.json`
- **SqliteDb** (`src/storage/sqlite.rs`): 通过 Database trait 统一接口的 SQLite 实现
- **自动保存**: 每次消息后调用 `auto_save()`

### 6.2 Database Trait

`src/storage/db.rs`

```rust
#[async_trait]
pub trait Database: Send + Sync {
    async fn save_session(&self, session: &Session) -> Result<()>;
    async fn load_session(&self, id: &str) -> Result<Option<Session>>;
    async fn list_sessions(&self) -> Result<Vec<SessionSummary>>;
    async fn delete_session(&self, id: &str) -> Result<()>;
    async fn save_memory(&self, memory: &Memory) -> Result<()>;
    async fn search_memory(&self, query: &str) -> Result<Vec<Memory>>;
    async fn list_session_memories(&self, session_id: &str) -> Result<Vec<Memory>>;
    async fn get_config(&self, key: &str) -> Result<Option<String>>;
    async fn set_config(&self, key: &str, value: &str) -> Result<()>;
    async fn track_event(&self, event: &Event) -> Result<()>;
}
```

覆盖四种持久化需求：会话、记忆、配置、遥测事件。

### 6.3 会话恢复

`main.rs` 的 `--session` 参数支持恢复历史会话：
```
coder --session <session-id>
```

恢复流程：SessionManager.load() → 反序列化消息 → Agent.context.add_message() 逐个恢复 → 进入交互模式。

---

## 7. 配置系统

### 7.1 五级优先级

```
1. CLI 参数 (最高)    --model, --provider, --headless, --print, -c
2. 环境变量            CODER_PROVIDER, CODER_MODEL, CODER_CONFIG
3. 项目配置            ./coder.toml (项目级覆盖)
4. 用户配置            ~/.coder/config.toml (全局默认)
5. 硬编码默认值        代码中 Default trait 实现 (最低)
```

### 7.2 Settings 结构

`src/config/settings.rs`

```rust
pub struct Settings {
    pub ai: AiSettings,            // AI 提供商 + 模型配置
    pub ui: UiSettings,            // 主题、行号、语法高亮
    pub tools: ToolSettings,       // 超时、输出限制、确认模式
    pub session: SessionSettings,  // 自动保存间隔、压缩阈值
    pub storage: StorageSettings,  // 数据库类型和 URL
}
```

### 7.3 ${ENV_VAR} 解析

配置文件中 `${OPENAI_API_KEY}` 格式的字符串在加载时自动解析为环境变量值。这避免了在配置文件中硬编码密钥。

### 7.4 ProviderConfig 结构

`src/config/provider_config.rs`

```rust
pub struct ProviderConfig {
    pub provider_type: String,       // "openai" | "anthropic" | "google" | "custom"
    pub api_key: Option<String>,     // 支持 ${ENV_VAR}
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_version: Option<String>,
    pub max_tokens: Option<u64>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub request_template: Option<String>,  // 自定义 Provider 用
    pub response_parser: Option<String>,   // 自定义 Provider 用
}
```

---

## 8. TUI 架构

### 8.1 App 状态机

`src/tui/app.rs`

四种模式的状态转换：

```
         启动
           │
           ▼
        Input ◄────────────────┐
           │                   │
           │ 用户发送消息        │
           ▼                   │
       Streaming ─────────────►│
           │  AI 响应结束       │
           │                   │
           │ [Tab]             │
           ▼                   │
        Detail ◄──────────────►│
           │                   │
        Confirm ◄──────────────┘
           │  (权限确认弹窗)
           │
           ▼
        退出 (Ctrl+C / /quit)
```

### 8.2 输入系统

**特殊前缀检测**:
| 前缀 | 含义 | 行为 |
|------|------|------|
| `!` | Shell 命令 | 直接执行系统命令 |
| `?` | 帮助 | 显示帮助信息 |
| `/` | 斜杠命令 | 内置命令（help, tools, git 等） |
| `@` | 提及 | 自动补全（工具名、Agent 类型、技能） |

**@提及自动补全**:
- 工具名: `@bash`, `@file_read`, `@grep`
- Agent 类型: `@agent:coding`, `@agent:research`
- 技能: `@skill:code-review`, `@skill:plan`

### 8.3 TUI 布局

```
┌────────────────────────────────────────────────────────────┐
│  🦀 Coder v0.1.0          model: claude-sonnet-4-6         │  ← 标题栏
├────────────────────────────────────────────────────────────┤
│                                                            │
│  ┌────────────────────────────────────────────────────┐    │
│  │  ◉ User · 14:30:22                                │    │
│  │  找出所有 TODO 注释                                │    │  ← 对话面板
│  │                                                    │    │
│  │  ◉ Assistant · 14:30:25                           │    │
│  │  我来搜索项目中的 TODO...                           │    │
│  │                                                    │    │
│  │  ┌─ ⏳ grep ─────────────────────────────────────┐ │    │
│  │  │ src/main.rs:42: // TODO: add error handling  │ │    │
│  │  └───────────────────────────────────────────────┘ │    │
│  │                                                    │    │
│  └────────────────────────────────────────────────────┘    │
├────────────────────────────────────────────────────────────┤
│ > 找出所有 TODO                           [Ctrl+Enter 发送] │  ← 输入栏
├────────────────────────────────────────────────────────────┤
│ 🦀 tools:12 | session:3 | tokens:1.2k | mode:input | Ready│  ← 状态栏
└────────────────────────────────────────────────────────────┘
```

---

## 9. 特征门控系统

### 9.1 Cargo Features 结构

三个阶段的功能，每个通过 Cargo feature 控制编译：

```
# Phase 1 — 核心增强功能 (默认关闭)
team, skill, subagent, memory, storage, lsp, mcp

# Phase 2 — 高级功能
server, permission, sync, voice, oauth, analytics, computer, worktree
tools-docker, tools-db, tools-oauth, tools-computer

# Phase 3 — 平台适配器
adapters-telegram, adapters-feishu, adapters-slack

# AI 提供商
ai-openai, ai-anthropic, ai-google, ai-opencode

# 运行模式
tui

# 工具集
tools-core, tools-git
```

### 9.2 默认特性

```toml
[features]
default = ["tui", "ai-openai", "ai-anthropic", "ai-opencode", "tools-core"]
```

### 9.3 条件编译模式

在 `lib.rs` 中，模块声明通过 `#[cfg(feature = "...")]` 控制：

```rust
// Phase 1 — 可选加载
#[cfg(feature = "team")]
pub mod team;
#[cfg(feature = "skill")]
pub mod skill;
// ...

// Phase 2
#[cfg(feature = "server")]
pub mod server;
// ...
```

---

## 10. Phase 1+ 扩展系统

### 10.1 Team — 团队协作系统

`src/team/`

允许多个 Agent 实例协作完成复杂任务：

| 组件 | 职责 |
|------|------|
| `TeamManager` | 团队管理与任务分配 |
| `Teammate` | 团队成员（角色 + 状态） |
| `TeammateMessage` | 成员间消息（tokio channel 传递） |
| `TaskAssignment` | 任务描述与追踪 |

### 10.2 Skill — 技能系统

`src/skill/`

命名的可复用能力，支持结构化输入输出：

内置技能: `brainstorm`, `code_review`, `debug`, `plan`

系统通过 `SkillRegistry` 管理技能注册，通过 `SkillLoader` 加载。

### 10.3 Subagent — 子代理系统

`src/subagent/`

轻量级、短生命周期的子代理，具有隔离的上下文：

```
Supervisor
    │
    ├─ spawn_subagent(任务 A)  →  SubagentHandle  →  返回 SubagentResult
    ├─ spawn_subagent(任务 B)  →  SubagentHandle  →  返回 SubagentResult
    │
    └─ 收集结果，合并报告
```

- `SubagentHandle` — 用于与子代理通信
- `SpawnConfig` — 配置子代理类型、上下文大小、超时等

### 10.4 Memory — 记忆系统

`src/memory/`

对话记忆跨会话持久化，支持基于关键字的检索：

```
MemoryStore (JSON 文件 → ~/.coder/memory/)
    │
    ├─ MemoryRetrieval — 关键字匹配检索
    │
    └─ AutoDream — 后台记忆整合与压缩
```

### 10.5 MCP — Model Context Protocol

`src/mcp/`

双向 MCP 支持：
- **MCP Client** — 连接到外部 MCP 服务器，发现并使用远程工具
- **MCP Server** — 将 Coder 的工具暴露给其他 MCP 客户端
- **Context7** — 编程文档查询集成

### 10.6 LSP — 语言服务器协议

`src/lsp/`

通过 stdio 连接语言服务器，提供：

- 代码补全建议
- Hover 信息
- 跳转到定义
- 实时诊断

组件: `LspClient` (连接管理), `LspHandler` (请求处理), `LspServerConfig` (语言服务器配置)

---

## 11. Phase 2+ 功能架构

### 11.1 Permission — 权限系统

`src/permission/`

三层权限模型：Allow（允许）| Deny（拒绝）| Ask（询问用户）

```rust
pub struct Action {
    pub name: String,              // 操作名 (如 "file_write")
    pub resource: Option<String>,  // 资源路径 (如 "/tmp/test.txt")
}
```

`PermissionEvaluator` 评估策略集 (`PolicySet`)，决定工具的放行/拦截/确认。

### 11.2 Server — HTTP API

`src/server/`

基于 Axum 的 RESTful API：

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/api/sessions` | 会话列表 |
| POST | `/api/sessions` | 创建会话 |
| GET | `/api/sessions/:id` | 会话详情 |
| POST | `/api/sessions/:id/chat` | 发送消息 (SSE 流式) |
| GET | `/api/tools` | 工具列表 |
| POST | `/api/tools/:name/exec` | 执行工具 |
| WS | `/api/ws` | WebSocket 实时通信 |
| GET | `/api/health` | 健康检查 |

`AppState` 包含共享的 `SessionManager`、`ToolRegistry` 和 `Provider`，通过 `Arc` 或 `Mutex` 包裹实现线程安全。

### 11.3 Sync — 云同步

`src/sync/`

支持三种同步方向: `Upload` / `Download` / `Bidirectional`

`SyncItem` 数据结构携带数据、时间戳和同步状态，支持冲突检测。

### 11.4 Voice — 语音模块

`src/voice/`

基于 `cpal` 的音频输入和 `hound` 的 WAV 编码，支持麦克风录音和播放。

### 11.5 OAuth — 认证模块

`src/oauth/`

OAuth 2.0 Authorization Code Flow 实现，支持外部服务集成。

### 11.6 Computer — 桌面操控

`src/computer/`

- `Screenshotter`: 屏幕截图
- `MouseController`: 鼠标控制
- `KeyboardController`: 键盘输入

### 11.7 Analytics — 分析统计

`src/analytics/`

事件追踪和用量统计，支持严重级别分类 (Debug / Info / Warning / Error / Critical) 和聚合指标计算。

### 11.8 Adapters — 平台适配器

`src/adapters/` (Phase 3)

IM 平台适配器（Telegram、飞书、Slack），将 Coder 暴露为聊天机器人。

### 11.9 Worktree — Git Worktree

`src/worktree/`

Git worktree 管理，支持创建、列出、删除隔离的工作目录。

---

## 12. 扩展点

### 12.1 添加新的 AI Provider

1. 在 `src/ai/` 下创建新文件（如 `ollama.rs`）
2. 实现 `Provider` trait
3. 在 `src/ai/mod.rs` 的 `create_provider()` 中添加分支
4. 添加 Cargo feature（可选）
5. 在 `Cargo.toml` 的 `[features]` 中注册

### 12.2 添加新的 Tool

1. 在 `src/tool/` 下创建新文件
2. 实现 `Tool` trait (`name`, `description`, `schema`, `execute`)
3. 在 `ToolRegistry::default()` 中注册
4. 如需要 feature gate，使用 `#[cfg(feature = "...")]`

### 12.3 添加新的 Skill

1. 在 `src/skill/builtin/` 下创建新文件
2. 实现 `Skill` trait
3. 在 `SkillRegistry` 中注册

### 12.4 添加新的 Adapter

1. 在 `src/adapters/` 下创建新目录
2. 实现平台消息到 Coder 内部格式的转换
3. 使用 `#[cfg(feature = "adapters-xxx")]` 门控

---

## 13. 构建系统

### 13.1 构建命令

```bash
# 默认构建 (TUI + OpenAI + Anthropic + 核心工具)
cargo build

# 完整构建 (所有功能)
cargo build --features "tui,ai-openai,ai-anthropic,ai-google,tools-core,tools-git,tools-docker,team,skill,subagent,memory,storage,server,mcp,lsp,sync,voice,oauth,analytics,computer,permission,worktree"

# 最小构建 (仅核心 + OpenAI)
cargo build --no-default-features --features "ai-openai,tools-core"

# 发布构建 (LTO + 优化 + 符号剥离)
cargo build --release
```

### 13.2 profile.release 优化

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

---

## 14. 模块依赖图

```
main.rs
  ├── config (独立)
  │   ├── settings
  │   ├── provider_config
  │   └── theme
  ├── ai
  │   ├── provider (trait)
  │   ├── types (Message, StreamEvent, etc.)
  │   ├── openai → reqwest
  │   ├── anthropic → reqwest
  │   ├── google → reqwest
  │   └── custom → reqwest
  ├── agent
  │   ├── loop (Agent) → ai::Provider + tool::ToolRegistry
  │   ├── context → ai::Message
  │   ├── dispatch → agent::types::AgentType
  │   └── types (AgentType)
  ├── tool
  │   ├── registry (ToolRegistry)
  │   ├── bash → tokio::process
  │   ├── file_read / file_write / file_edit
  │   ├── glob → glob crate
  │   ├── grep (纯文本搜索)
  │   ├── web_fetch → reqwest
  │   ├── web_search → reqwest
  │   └── ... (feature-gated tools)
  ├── session
  │   ├── manager → util::path
  │   └── history
  ├── tui → ratatui + crossterm
  │   ├── app (状态机) → agent::Agent
  │   ├── chat_panel
  │   ├── input
  │   └── help
  ├── storage (feature: storage)
  │   ├── db (trait)
  │   ├── sqlite → libsql
  │   └── migrate
  ├── server (feature: server) → axum
  │   ├── router
  │   ├── handler_session → session::manager
  │   ├── handler_tools → tool::registry
  │   └── ws
  ├── team (feature: team)
  ├── skill (feature: skill)
  ├── subagent (feature: subagent)
  ├── memory (feature: memory)
  ├── mcp (feature: mcp)
  ├── lsp (feature: lsp) → tower-lsp
  ├── permission (feature: permission)
  ├── sync (feature: sync)
  ├── voice (feature: voice) → cpal + hound
  ├── oauth (feature: oauth) → oauth2
  ├── computer (feature: computer) → enigo + screenshots
  ├── analytics (feature: analytics)
  ├── worktree (feature: worktree)
  └── util
      ├── path → dirs
      ├── format
      └── template
```

---

## 15. 命令行接口

```bash
🦀 Coder - AI-powered development tool

Usage: coder [OPTIONS]

Options:
  --provider <PROVIDER>    AI provider [env: CODER_PROVIDER]
  --model <MODEL>          Model name override [env: CODER_MODEL]
  -c, --config <CONFIG>    Config file path [env: CODER_CONFIG]
  -s, --session <SESSION>  Session ID to resume
  --headless               Run in headless mode (no TUI)
  --print <PRINT>          One-shot query then exit
  -d, --directory <DIR>    Working directory [default: .]
  -v, --verbose            Enable verbose logging
  --help                   Print help
  --version                Print version
```

---

## 16. 安全考虑

### 16.1 密钥管理

- API 密钥通过 `${ENV_VAR}` 语法引用，不在配置文件中明文存储
- 所有 Provider 在构造时接收密钥，运行时不暴露

### 16.2 工具权限

- `requires_permission()` 标记敏感工具（如 `bash`）
- Permission 系统支持细粒度的 Allow/Deny/Ask 策略
- PermissionEvaluator 与 Agent 解耦，可替换实现

### 16.3 沙箱与限制

- Bash 命令有超时限制（默认 300 秒）和输出大小限制（默认 1MB）
- Web 请求有 30 秒超时
- Agent 循环最多 10 轮防止无限循环
- 上下文压缩保护 API 调用不超出 token 限制

### 16.4 终端恢复

- 信号处理器 (`SIGTERM` / `Ctrl+C`) 确保终端模式恢复
- 通过 `std::panic::set_hook` 在 panic 时恢复终端状态

---

## 17. 性能考量

### 17.1 异步性能

- 全 Tokio 异步架构，无阻塞 I/O
- AI API 调用和工具执行并行化（通过 channel 解耦）
- SSE 流式解析使用零拷贝缓冲区复用

### 17.2 Release 构建优化

```toml
[profile.release]
opt-level = 3        # 最大速度优化
lto = true           # 链接时优化（减小二进制大小 + 内联）
codegen-units = 1    # 单代码生成单元（更好优化）
strip = true         # 剥离符号（减小二进制）
```

### 17.3 上下文窗口管理

- 128K token 默认上下文限制
- 消息数量超过阈值时触发 `compact()`（当前策略: 保留最近 10 条）
- 未来计划: 基于 LLM 的智能摘要压缩

---

## 18. 未来架构方向

### Phase 3+ 规划

| 特性 | 状态 | 说明 |
|------|------|------|
| Adapters (Telegram) | 已规划 | IM 平台集成 |
| Adapters (飞书) | 已规划 | 企业协作集成 |
| Adapters (Slack) | 已规划 | 团队沟通集成 |
| MCP 扩展 | 规划中 | 更多 MCP 服务器集成 |
| 多模态支持 | 规划中 | 图片输入处理 |
| 增强记忆 | 规划中 | 向量数据库检索 |
| 插件系统 | 规划中 | WASM 插件 |

---

## 附录

### A. 配置文件示例

```toml
# ~/.coder/config.toml 或 ./coder.toml

[ai]
default_provider = "openai"

[ai.providers.openai]
type = "openai"
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1"
model = "gpt-4o"

[ai.providers.claude]
type = "anthropic"
api_key = "${ANTHROPIC_API_KEY}"
model = "claude-sonnet-4-6"

[ui]
theme = "coder-dark"
show_line_numbers = true
syntax_highlight = true
mouse_support = true

[tools]
confirm_before_exec = false
timeout_seconds = 300

[session]
auto_save_interval = 60
max_messages_before_compact = 100
```

### B. 关键文件索引

| 文件 | 核心内容 |
|------|---------|
| `src/main.rs` | CLI 入口，三种模式分发，信号处理 |
| `src/lib.rs` | 模块声明，feature 门控 |
| `src/ai/provider.rs` | Provider trait |
| `src/ai/types.rs` | Message, ContentBlock, StreamEvent, ToolCall |
| `src/agent/loop.rs` | Agent 结构体 + ReAct 循环 |
| `src/agent/context.rs` | Context 管理 + 系统提示词 |
| `src/tool/mod.rs` | Tool trait + ToolResult |
| `src/tool/registry.rs` | ToolRegistry + 默认注册 |
| `src/config/settings.rs` | Settings + Ai/Ui/Tool/Session/Storage 设置 |
| `src/tui/app.rs` | App 状态机 + 命令处理 |
| `src/storage/db.rs` | Database trait |
| `src/storage/sqlite.rs` | SQLite 实现 |

### C. 术语表

| 术语 | 说明 |
|------|------|
| ReAct | Reasoning + Acting 循环，AI 思考→行动→观察的迭代模式 |
| SSE | Server-Sent Events，流式 HTTP 响应协议 |
| TUI | Terminal User Interface，终端用户界面 |
| LSP | Language Server Protocol，语言服务器协议 |
| MCP | Model Context Protocol，模型上下文协议 |
| AgentType | 代理类型（Coding/Research/Debug/Plan/Review） |
| StreamEvent | 流式事件（TextChunk/ToolCallStart/Done/Error） |
| ToolCall | AI 发起的工具调用请求 |
| Context Window | 对话上下文窗口（当前 128K tokens） |
