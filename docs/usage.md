# Coder 使用文档

## 1. 简介

Coder 是一款基于 Rust 的 AI 编程辅助工具，融合了 Claude Code 和 OpenCode 的核心能力，在终端中提供智能代码开发体验。它支持多种 AI 提供商、丰富的工具系统、多 Agent 协作以及可扩展的插件体系。

### 核心特性

- **多模式运行**：TUI 交互模式（默认）、Headless 非交互模式、Print 一次性查询模式
- **多 AI 提供商**：支持 OpenAI 兼容接口、Anthropic Claude、Google Gemini 及自定义提供商
- **丰富的工具系统**：文件操作、代码搜索、命令执行、Web 搜索等
- **Slash 命令**：内置 Git 操作、代码审查、测试、搜索等命令
- **Agent 系统**：多种专业 Agent 类型（编程、研究、调试、规划、审查）
- **可扩展**：基于 Feature Flag 的阶段化功能（团队协作、技能系统、MCP、LSP 等）

---

## 2. 安装与构建

### 从源码构建

```bash
# 克隆项目
git clone <repository-url>
cd coder

# 默认构建（TUI + OpenAI + Anthropic + 核心工具）
cargo build --release

# 安装到系统路径
cargo install --path .
```

### 功能特性组合

Coder 使用 Cargo Feature Flags 控制功能开关，可按需启用：

#### 默认功能

```bash
# 等价于默认特性集
cargo build --release --features "tui,ai-openai,ai-anthropic,ai-opencode,tools-core"
```

#### 完整功能（Phase 1 + Phase 2 + Phase 3）

```bash
# 启用全部功能
cargo build --release --features "tui,ai-openai,ai-anthropic,ai-google,ai-opencode,tools-core,tools-git,tools-docker,tools-db,tools-oauth,tools-computer,team,skill,subagent,memory,storage,server,mcp,lsp,sync,voice,oauth,analytics,adapters-telegram,permission,computer,worktree"
```

#### 按阶段选择

| 阶段 | 功能 | 说明 |
|------|------|------|
| **核心** | `tui`, `ai-openai`, `ai-anthropic`, `ai-opencode`, `tools-core` | 基础使用必备 |
| **Phase 1** | `team`, `skill`, `subagent`, `memory`, `storage`, `lsp`, `mcp` | 团队协作、技能系统、MCP、LSP |
| **Phase 2** | `server`, `permission`, `sync`, `voice`, `oauth`, `analytics`, `computer`, `worktree`, `tools-docker`, `tools-db` | HTTP API、权限控制、云同步、语音、桌面自动化 |
| **Phase 3** | `adapters-telegram` | 平台适配器（Telegram 等） |

**常用构建示例：**

```bash
# 开发调试（含 Git 工具）
cargo build --features "tui,ai-openai,ai-anthropic,ai-opencode,tools-core,tools-git"

# 全功能开发
cargo build --features "$(grep '^###' Cargo.toml | head -1)"
```

---

## 3. 配置说明

Coder 使用 TOML 格式的配置文件，配置优先级从高到低：

1. **CLI 参数**（最高优先级）：`--model`、`--provider`
2. **环境变量**：`CODER_PROVIDER`、`CODER_MODEL`、`CODER_CONFIG`
3. **项目配置**：`./coder.toml`（项目目录下）
4. **用户配置**：`~/.coder/config.toml`
5. **默认值**（最低优先级）

### 完整配置参考

```toml
# ~/.coder/config.toml 或 ./coder.toml

[ai]
default_provider = "openai"

[ai.provider.openai]
type = "openai"
api_key = "${OPENAI_API_KEY}"          # 支持环境变量引用
base_url = "https://api.openai.com/v1"
model = "gpt-4o"
max_tokens = 4096
temperature = 0.7
top_p = 0.9

[ai.provider.claude]
type = "anthropic"
api_key = "${ANTHROPIC_API_KEY}"
# base_url 可选，默认使用 Anthropic API
model = "claude-sonnet-4-6"
max_tokens = 4096
temperature = 0.7
top_p = 0.9

[ai.provider.deepseek]
type = "openai"
api_key = "${DEEPSEEK_API_KEY}"
base_url = "https://api.deepseek.com/v1"
model = "deepseek-chat"

[ai.provider.gemini]
type = "google"
api_key = "${GEMINI_API_KEY}"
model = "gemini-2.0-flash"

[ai.provider.ollama]
type = "openai"
api_key = "ollama"
base_url = "http://localhost:11434/v1"
model = "llama3"

[ui]
theme = "coder-dark"                   # 主题：coder-dark, high-contrast
show_line_numbers = true                # 代码块显示行号
syntax_highlight = true                 # 启用语法高亮
mouse_support = true                    # 启用鼠标支持

[tools]
confirm_before_exec = false             # 工具执行前是否确认
timeout_seconds = 300                   # 工具执行超时（秒）
max_output_bytes = 1_000_000            # 工具输出最大字节数
allowed_tools = []                      # 允许的工具列表（空=全部允许）

[session]
auto_save_interval = 60                 # 自动保存间隔（秒）
max_messages_before_compact = 100       # 触发消息压缩的阈值
max_context_tokens = 128000             # 最大上下文 Token 数

[storage]
db_type = "sqlite"                      # 数据库类型（sqlite / postgres）
db_url = ""                             # 数据库连接 URL
```

### 环境变量引用

配置文件中使用 `${VAR_NAME}` 语法引用环境变量，运行时自动替换：

```toml
api_key = "${OPENAI_API_KEY}"   # 读取环境变量 OPENAI_API_KEY
base_url = "${CUSTOM_BASE_URL}" # 读取环境变量 CUSTOM_BASE_URL
```

若引用的环境变量未设置，则保留原字符串。

---

## 4. CLI 使用

### 命令行参数

```bash
🦀 Coder - AI-powered development tool

Usage: coder [OPTIONS]

Options:
  -c, --config <FILE>      配置文件路径（环境变量: CODER_CONFIG）
  -d, --directory <DIR>    工作目录 [默认: .]
  -h, --help               显示帮助信息
      --headless           Headless 模式（无 TUI 界面）
      --model <MODEL>      AI 模型名称（环境变量: CODER_MODEL）
      --print <QUERY>      Print 模式：一次性查询后退出
      --provider <NAME>    AI 提供商（环境变量: CODER_PROVIDER）
  -s, --session <ID>       恢复指定会话
  -v, --verbose            启用详细日志输出
  -V, --version            显示版本信息
```

### 运行模式

#### TUI 模式（默认）

```bash
# 直接启动 TUI
coder

# 指定工作目录
coder -d /path/to/project

# 指定 AI 提供商和模型
coder --provider claude --model claude-sonnet-4-6

# 使用自定义配置文件
coder -c /path/to/config.toml

# 恢复历史会话
coder -s session-id-here

# 启用详细日志
coder -v
```

#### Headless 模式

在终端中运行，无 TUI 界面，适合在 CI 或远程 SSH 会话中使用：

```bash
coder --headless
```

#### Print 模式

一次性查询，结果输出到 stdout 后退出，适合脚本中使用：

```bash
# 一次性查询
coder --print "解释 Rust 的所有权系统"

# 结合管道使用
coder --print "生成一个快速排序函数" > quicksort.rs

# 在脚本中使用
RESULT=$(coder --print "将 JSON 转为 TOML 格式")
```

---

## 5. TUI 使用

### 界面布局

TUI 界面从上到下分为四个区域：

```
┌──────────────────────────────────────────────────────┐
│  🦀 Coder  v0.1.0  ·  8 tools  ·  3 msgs            │  标题栏
├──────────────────────────────────────────────────────┤
│                                                      │
│  ┌─ Message ──────────────────────────────────────┐  │
│  │  user: 实现一个二分查找                         │  │  聊天面板
│  │  assistant: 好的，这里是 Rust 实现...            │  │
│  │  ┌─ Tool ──────────────────────────────────┐    │  │
│  │  │  file_write: src/binary_search.rs ✅    │    │  │
│  │  └─────────────────────────────────────────┘    │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
├──────────────────────────────────────────────────────┤
│ > 输入文本...                        Enter to send   │  输入区
├──────────────────────────────────────────────────────┤
│ 🦀 tools:8 | session:3 | tokens:1.2k/128k | input    │  状态栏
└──────────────────────────────────────────────────────┘
```

### 输入模式

TUI 支持四种输入模式：

| 前缀 | 模式 | 示例 | 说明 |
|------|------|------|------|
| `文本` | Chat | `写一个 Rust 斐波那契函数` | 发送给 AI 处理 |
| `!` | Shell | `!git status` | 直接执行 Shell 命令 |
| `?` | Help | `?git` | 查询帮助信息 |
| `/` | Slash | `/help` | 执行 Slash 命令 |

### 键盘快捷键

#### 输入模式

| 按键 | 功能 |
|------|------|
| `Enter` | 发送消息 |
| `Alt+Enter` | 输入换行 |
| `Ctrl+C` | 清空输入 |
| `Ctrl+U` | 清空输入 |
| `Ctrl+W` | 删除前一个词 |
| `Ctrl+O` | 打开详情面板 |
| `Ctrl+A / Home` | 光标到行首 |
| `Ctrl+E / End` | 光标到行尾 |
| `Ctrl+B / Left` | 光标左移 |
| `Ctrl+F / Right` | 光标右移 |
| `Ctrl+D / Delete` | 删除光标处字符 |
| `Up` | 上一条输入历史 |
| `Down` | 下一条输入历史 |
| `Tab` | 触发 @ 提及（输入中包含 @ 时） |
| `Esc` | 切换到 Normal 模式 |

#### Streaming 模式（AI 生成中）

| 按键 | 功能 |
|------|------|
| `Ctrl+C` | 中断 AI 回复 |
| `Ctrl+O` | 打开详情面板 |

#### Normal 模式

| 按键 | 功能 |
|------|------|
| `i` | 切换到输入模式 |
| `/` | 切换到输入模式并输入 `/` |
| `!` | 切换到输入模式并输入 `!` |
| `?` | 切换到输入模式并输入 `?` |
| `Up / PageUp` | 上滚聊天面板 |
| `Down / PageDown` | 下滚聊天面板 |
| `Ctrl+O` | 切换详情面板 |
| `Esc` | 退出（按两下） |

### @ 提及系统

输入 `@` 触发自动补全弹窗，支持选择：

- **工具**：`@bash`、`@file_read`、`@file_write`、`@file_edit`、`@glob`、`@grep`、`@web_fetch`、`@web_search` 等
- **Agent 类型**：`@agent:coding`、`@agent:research`、`@agent:debug`、`@agent:plan`、`@agent:review`
- **技能**：`@skill:brainstorm`、`@skill:code-review`、`@skill:plan`、`@skill:debug`、`@skill:tdd`

操作方式：

| 按键 | 功能 |
|------|------|
| `Tab / Down` | 选择下一项 |
| `Up` | 选择上一项 |
| `Enter` | 确认选择 |
| `Esc` | 取消 |

---

## 6. Slash 命令参考

### 信息类

| 命令 | 别名 | 功能 | 示例 |
|------|------|------|------|
| `/help` | `h` | 显示分类帮助 | `/help`, `/help git` |
| `/tools` | `t` | 列出所有可用工具 | `/tools` |
| `/model` | `m` | 显示/切换 AI 模型 | `/model`, `/model claude-sonnet-4-6` |
| `/context` | `ctx` | 显示上下文使用情况 | `/context` |

### Git 操作

| 命令 | 别名 | 功能 | 示例 |
|------|------|------|------|
| `/status` | `st` | 显示 Git 状态 | `/status` |
| `/diff` | - | 显示 Git 差异 | `/diff`, `/diff --staged` |
| `/commit` | - | 创建 Git 提交 | `/commit Add login feature` |
| `/pr` | - | 创建 Pull Request | `/pr` |

### 搜索类

| 命令 | 别名 | 功能 | 示例 |
|------|------|------|------|
| `/search` | `s` | 使用 ripgrep 搜索代码 | `/search fn main`, `/search TODO src/` |
| `/web_search` | `ws` | 搜索网络 | `/web_search Rust async patterns` |
| `/fetch` | `f` | 获取网页内容 | `/fetch https://example.com` |

### 操作类

| 命令 | 别名 | 功能 | 示例 |
|------|------|------|------|
| `/clear` | `c` | 清空当前对话 | `/clear` |
| `/compact` | - | 压缩对话上下文（保留最近 10 条） | `/compact` |
| `/summarize` | - | 对话摘要 | `/summarize` |
| `/review` | `r` | 审查代码变更 | `/review` |
| `/plan` | - | 创建实施计划 | `/plan Add authentication` |
| `/test` | - | 运行测试 | `/test`, `/test src/auth.rs` |
| `/lint` | - | 代码质量检查 | `/lint`, `/lint src/` |
| `/fix` | - | 修复问题 | `/fix unused variables` |
| `/explain` | - | 解释代码 | `/explain src/main.rs` |
| `/doc` | - | 生成文档 | `/doc src/lib.rs` |

### 配置类

| 命令 | 别名 | 功能 | 示例 |
|------|------|------|------|
| `/config` | - | 查看/设置配置 | `/config`, `/config theme dark` |
| `/init` | - | 初始化配置文件 | `/init` |
| `/memory` | - | 查看内存和会话信息 | `/memory` |
| `/quit` | `q`, `exit` | 退出 Coder | `/quit` |

---

## 7. AI 提供商配置

### OpenAI 兼容提供商

支持所有兼容 OpenAI API 格式的提供商，包括 OpenAI、DeepSeek、Ollama、Groq、MiniMax 等。

```toml
[ai.provider.openai]
type = "openai"
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1"
model = "gpt-4o"

[ai.provider.deepseek]
type = "openai"
api_key = "${DEEPSEEK_API_KEY}"
base_url = "https://api.deepseek.com/v1"
model = "deepseek-chat"

[ai.provider.ollama]
type = "openai"
api_key = "ollama"
base_url = "http://localhost:11434/v1"
model = "llama3"
```

### Anthropic（Claude）

```toml
[ai.provider.claude]
type = "anthropic"
api_key = "${ANTHROPIC_API_KEY}"
model = "claude-sonnet-4-6"
# 可选：自定义 API 版本
# api_version = "2024-10-01"
```

### Google（Gemini）

需要启用 `ai-google` 特性：

```toml
[ai.provider.gemini]
type = "google"
api_key = "${GEMINI_API_KEY}"
model = "gemini-2.0-flash"
```

### 自定义提供商

对于不完全兼容上述类型的 API，可使用 `custom` 类型，自定义请求和响应模板：

```toml
[ai.provider.custom]
type = "custom"
api_key = "${CUSTOM_API_KEY}"
base_url = "https://custom-api.example.com/v1"
model = "custom-model"
request_template = "..."   # 自定义请求模板
response_parser = "..."    # 自定义响应解析器
```

### 公共参数

所有提供商支持以下可选参数：

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `max_tokens` | u64 | 4096 | 最大生成 Token 数 |
| `temperature` | f64 | 0.7 | 采样温度（0-1） |
| `top_p` | f64 | 0.9 | Top-p 采样 |

---

## 8. 工具系统

Coder 内置丰富的工具，通过 Feature Flag 控制启用。

### 核心工具（默认）

| 工具名 | 功能 | 说明 |
|--------|------|------|
| `bash` | 执行 Shell 命令 | 在指定目录运行命令，支持超时控制 |
| `file_read` | 读取文件内容 | 支持指定行范围 |
| `file_write` | 写入文件 | 创建新文件或覆盖 |
| `file_edit` | 编辑文件 | 精确字符串替换 |
| `glob` | 文件模式匹配 | 按 Glob 模式查找文件 |
| `grep` | 内容搜索 | 基于 ripgrep 的正则搜索 |
| `question` | 向用户提问 | 获取用户输入 |
| `web_fetch` | 获取网页内容 | 抓取 URL 并转为 Markdown |
| `web_search` | 网络搜索 | 搜索互联网信息 |

### Git 工具（tools-git）

| 工具名 | 功能 |
|--------|------|
| `git` | Git 版本控制操作 |
| `worktree` | Git Worktree 管理 |

### Docker 工具（tools-docker）

| 工具名 | 功能 |
|--------|------|
| `docker` | Docker 容器操作 |

### 数据库工具（tools-db）

| 工具名 | 功能 |
|--------|------|
| `db_query` | 数据库查询 |

### OAuth 工具（tools-oauth）

| 工具名 | 功能 |
|--------|------|
| `oauth` | OAuth 2.0 授权流程 |

### 其他工具

| 工具名 | 功能 | 备注 |
|--------|------|------|
| `docs` | 文档查询 | - |
| `ci` | CI 操作 | - |
| `task` | 任务管理 | - |
| `plan` | 规划工具 | - |

---

## 9. Agent 类型

Coder 提供多种专业化 Agent 类型，适用于不同场景。

| 类型 | 显示名 | 说明 | 适用场景 |
|------|--------|------|----------|
| `Coding` | `coding`（默认） | 完整工具访问，通用编码 | 日常编程、代码生成、调试 |
| `Research` | `research` | 网络搜索和研究为主 | 技术调研、文档查阅、方案对比 |
| `Debug` | `debug` | 系统性调试，使用 Bash/Grep/LSP | Bug 定位、故障排查 |
| `Plan` | `plan` | 规划和分析模式（只读工具） | 需求分析、架构设计、实施规划 |
| `Review` | `review` | 代码审查专家 | 代码质量检查、安全审查 |

### 切换 Agent

在 TUI 中使用 `/model` 切换，或通过 `@agent:type` 提及在对话中临时调佣特定 Agent。

---

## 10. 阶段功能

### Phase 1 功能

#### Team（团队协作）

多 Agent 协作系统，基于 tokio 通道进行消息传递：

- `team`：团队管理
- `task`：任务分配
- `teammate`：团队成员
- `communication`：团队通信

#### Skill（技能系统）

可重用的命名能力模块，支持结构化输入输出：

- `brainstorm`：头脑风暴
- `code_review`：代码审查
- `debug`：调试
- `plan`：规划

可通过 `@skill:name` 在对话中调用。

#### Subagent（子 Agent）

短期存活、隔离上下文的子 Agent，由 Supervisor 生成并管理：

- `supervisor`：子 Agent 管理器
- `spawn`：生成子 Agent
- 独立上下文，不影响主会话

#### Memory（记忆系统）

跨会话记忆持久化，通过 JSON 文件存储在 `~/.coder/memory/`：

- `store`：记忆存储
- `retrieve`：关键字检索
- `autodream`：AutoDream 后台整合

#### Storage（持久化存储）

基于 libsql（SQLite）的数据持久化：

- `sqlite`：SQLite 数据库
- `migrate`：数据库迁移
- `db`：数据库操作

#### LSP（语言服务）

Language Server Protocol 客户端，提供代码智能：

- 自动补全（Completion）
- 悬停信息（Hover）
- 跳转定义（Go-to-definition）
- 诊断信息（Diagnostics）

#### MCP（模型上下文协议）

Model Context Protocol 实现：

- `client`：连接 MCP 服务器
- `server`：暴露 Coder 工具给 MCP
- `context7`：Context7 文档集成

### Phase 2 功能

#### Server（HTTP API）

基于 Axum 的 HTTP API 服务，支持：

- SSE 流式响应
- WebSocket 通信
- 会话管理
- 工具调用

```bash
# 启动 HTTP 服务
cargo run --features server
```

#### Permission（权限控制）

基于策略的工具访问控制：

- `allow`：允许
- `deny`：拒绝
- `ask`：询问用户

#### Sync（云同步）

跨设备数据同步：

- 配置同步
- 会话同步
- 记忆同步

#### Voice（语音）

音频输入支持：

- 基于 cpal 的音频捕获
- 语音转文字

#### OAuth

OAuth 2.0 授权码流程，用于第三方服务集成。

#### Analytics（分析）

使用追踪和遥测：

- 命令使用统计
- 性能指标

#### Computer（桌面自动化）

桌面自动化操作：

- 截图
- 鼠标控制
- 键盘输入

#### Worktree

Git Worktree 管理：

- 创建隔离的工作目录
- 并行分支开发

### Phase 3 功能

#### Adapters（平台适配器）

将 Coder 集成到即时通讯平台：

- `Telegram`：通过 Telegram Bot 使用 Coder
- `Feishu`：飞书集成（计划中）
- `Slack`：Slack 集成（计划中）

---

## 11. 会话管理

### 自动保存

会话默认每 60 秒自动保存到 `~/.coder/sessions/`，可通过 `session.auto_save_interval` 配置。

### 恢复会话

```bash
# 列出已有会话（查看 sessions 目录）
ls ~/.coder/sessions/

# 恢复指定会话
coder -s <session-id>
```

### 消息压缩

当消息数量达到 `session.max_messages_before_compact`（默认 100）时，自动触发压缩。也可通过 `/compact` 手动压缩（保留最近 10 条消息）。

### 手动保存

除自动保存外，每次发送消息后自动触发保存。

---

## 12. 数据目录结构

```
~/.coder/
├── config.toml          # 用户配置文件
├── sessions/            # 会话文件（JSON 格式）
│   ├── <session-id>.json
│   └── ...
└── memory/              # 记忆存储
    ├── *.json
    └── ...
```

---

## 13. 实用场景示例

### 场景一：日常编码辅助

```bash
# 进入项目目录，启动 TUI
cd my-project
coder

# 在 TUI 中输入
# > 为这个项目添加一个配置文件解析模块
```

### 场景二：代码审查

```bash
# 修改代码后运行审查
/review
# 查看变更
/diff
# AI 自动审查变更代码
```

### 场景三：快速搜索和修复

```bash
# 搜索代码中的问题
/search unwrap\()
# 定位后分析
/explain src/main.rs
```

### 场景四：Git 工作流集成

```bash
# 查看当前状态
/status
# 查看差异
/diff
# 提交
/commit Add user authentication
# 创建 PR
/pr
```

### 场景五：网络搜索辅助开发

```bash
# 搜索网络技术方案
/web_search Rust web framework comparison 2026
# 获取文档
/fetch https://docs.rs/tokio
```

### 场景六：使用不同 Agent 类型

在对话中通过 @ 提及切换 Agent：

```
> @agent:research 研究一下当前 Rust 生态中最流行的 Web 框架
> @agent:debug 帮我分析这个崩溃日志
> @agent:plan 设计一个用户认证系统的架构
```

### 场景七：Headless 模式集成 CI

```bash
# 在 CI 脚本中使用 Print 模式进行代码审查
coder --print "审查以下变更: $(git diff)" --provider claude
```

### 场景八：自定义配置多提供商

```toml
[ai]
default_provider = "deepseek"

[ai.provider.deepseek]
type = "openai"
api_key = "${DEEPSEEK_API_KEY}"
base_url = "https://api.deepseek.com/v1"
model = "deepseek-chat"

[ai.provider.claude]
type = "anthropic"
api_key = "${ANTHROPIC_API_KEY}"
model = "claude-sonnet-4-6"
```

```bash
# 使用默认提供商（DeepSeek）
coder

# 临时切换到 Claude
coder --provider claude --model claude-sonnet-4-6
```

---

## 14. 常见问题

### 终端显示异常

确保终端支持 UTF-8 和真彩色（true color）。如界面显示异常，尝试调整终端设置或使用 `--headless` 模式。

### 配置文件找不到

检查配置路径：默认查找 `~/.coder/config.toml` 和 `./coder.toml`，也可通过 `-c` 参数指定。

### AI 响应报错

- 检查 API Key 是否有效（环境变量是否正确设置）
- 检查网络连接（是否需要代理）
- 查看详细日志：`coder -v`

### 工具执行超时

调整 `tools.timeout_seconds` 配置值（默认 300 秒）。

### 会话丢失

检查 `~/.coder/sessions/` 目录是否存在且可写。使用 `-s` 参数指定会话 ID 恢复。

### 如何卸载

```bash
# 如果通过 cargo 安装
cargo uninstall coder

# 删除配置和数据
rm -rf ~/.coder
```
