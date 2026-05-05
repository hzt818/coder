<div align="center">

# 🦀 Coder

**你的 AI 终端编程搭档**

*融合 Claude Code 与 OpenCode 的精粹，用 Rust 重新定义 AI 开发体验*

![Rust](https://img.shields.io/badge/Rust-2021-edition?logo=rust&style=flat-square)
![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)
![Version](https://img.shields.io/badge/version-0.1.0-orange?style=flat-square)

[🇬🇧 English](README.md) · [🇨🇳 中文](README_CN.md)

---

> **Coder** 不只是一个 AI 编程工具 —— 它是你的**终端原生 AI 开发环境**。从头到底用 Rust 构建，融合了 Claude Code 的对话能力和 OpenCode 的开放灵活，全都包裹在一个又快又酷的 TUI 里。🚀

</div>

---

## 🎬 看看它的样子

<video src="intro.mp4" controls width="100%" style="max-width: 800px; border-radius: 8px;"></video>

---

## ✨ 为什么选择 Coder？

### 🎯 **AI 自由，想用啥用啥**

Coder 不把你锁死在某个 AI 厂商。**带上你自己的模型** —— 或者直接用免费版，十秒上手：

| AI 提供商 | 支持模型 | 说明 |
|-----------|---------|------|
| **OpenCode** (免费) | Claude Sonnet 4.6, Claude Haiku 4.5 | 有免费额度，不需要信用卡 |
| **OpenAI** | GPT-4o, GPT-4, o1, o3 | 同时也兼容 DeepSeek、Ollama、Groq、MiniMax |
| **Anthropic** | Claude Opus 4.6, Sonnet 4.6, Haiku 4.5 | 完整支持扩展思考 |
| **Google** | Gemini 2.0 Flash, Gemini 1.5 Pro | — |
| **自定义** | 任意 HTTP API | 自己定义请求和响应的格式 |

> 👋 **新来的？** 直接敲 `coder` — 如果没有检测到 API Key，会弹出一个友好的设置对话框。选"使用 OpenCode 免费版"，10 秒后就可以开始写代码了。不需要注册，不需要信用卡，不折腾。

### 💻 **漂亮的终端界面**

基于 [Ratatui](https://github.com/ratatui-org/ratatui) 构建，Coder 的 TUI 为终端重度用户量身打造：

```
┌──────────────────────────────────────────────────────────┐
│  🦀 Coder  v0.1.0  ·  claude-sonnet-4-6  ·  8 个工具   │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─ 消息 ────────────────────────────────────────────┐  │
│  │  你: 用 Rust 写一个二分查找                        │  │
│  │  AI:  好的，这是一个简洁的实现...                    │  │
│  │                                                     │  │
│  │  ┌─ ⏳ file_write: src/binary_search.rs ─────────┐  │  │
│  │  │  ✅ 文件写入成功（420 字节）                   │  │  │
│  │  └───────────────────────────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                          │
├──────────────────────────────────────────────────────────┤
│ > 用 Rust 写一个二分查找                 [Enter 发送]    │
├──────────────────────────────────────────────────────────┤
│ 🦀 12 个工具 | 会话: 3 | tokens: 1.2k/128k | 输入模式    │
└──────────────────────────────────────────────────────────┘
```

### 🛠️ **真正的工具箱，不只是聊天**

Coder 内置了一整套工具，让 AI **真的能干实事**，而不只是动嘴皮子：

| 工具 | 它能干啥 |
|------|---------|
| `bash` | 在你的终端里跑命令 |
| `file_read` / `file_write` / `file_edit` | 读文件、写文件、改文件 |
| `glob` / `grep` | 找文件、搜代码 |
| `web_fetch` / `web_search` | 实时上网搜资料 |
| `git` | 暂存、提交、对比、推送、创建 PR |
| `docker` | 管理容器 |
| `db_query` | 查数据库 |
| `docs` | 查文档 |
| 还有更多... | 任务管理、写计划、代码审查、CI |

### 🧠 **三种交互模式，适配你的工作方式**

| 模式 | Shell | 文件修改 | 最适合 |
|------|-------|---------|--------|
| 🔍 **Plan**（计划） | ❌ 只读 | ❌ 只读 | 架构设计、代码审查 |
| 🤖 **Agent**（代理） | ✅ 先问 | ✅ 先问 | 日常编码 — 安全稳妥 |
| ⚡ **YOLO**（冲） | ✅ 自动 | ✅ 自动 | 自动化脚本、CI 任务 |

### 👥 **多 Agent 与团队协作**

从单打独斗到团队合作，Coder 都能胜任：

- **多种 Agent 类型**：编程、研究、调试、规划、审查 —— 每种都有专门的提示词和工具权限
- **技能系统**：可复用的能力模块，如头脑风暴、代码审查、项目规划
- **子 Agent 系统**：派生子 Agent 并行处理独立任务
- **团队模式**：多个 Agent 协同完成复杂工作流
- **记忆系统**：跨会话记忆持久化，支持关键词检索

### 🔌 **生来就为扩展**

Coder 从第一天起就为扩展性而设计：

- **MCP 支持**（模型上下文协议）：连接外部 MCP 服务器，或把 Coder 的工具暴露给其他人用
- **LSP 集成**：通过语言服务器协议获得代码智能
- **自定义 Provider**：用请求/响应模板定义你自己的 AI 提供商
- **API 服务**：HTTP + WebSocket 接口，远程也能用
- **特性开关**：通过 Cargo 特性按需编译，不需要的就不编译

---

## 🚀 快速上手

### 安装

```bash
# 从源码构建
git clone https://github.com/hzt818/coder
cd coder
cargo build --release

# 安装到系统
cargo install --path .
```

> **💡 小提示：** 如果 `~/.cargo/bin` 还没在 `$PATH` 里，记得加进去。

### 第一次运行

```bash
# 直接跑 — 设置向导会引导你
coder
```

第一次启动时，Coder 会：
1. 检测到没有配置 API Key
2. 弹出一个设置对话框，提供三个选项：
   - **使用 OpenCode 免费版** → 直接开始，不需要 API Key
   - **获取免费 API Key（OAuth）** → 浏览器认证
   - **手动输入 API Key** → 粘贴你的密钥
3. 选完就能用了！

### 配置文件

Coder 使用 TOML 格式的配置文件，按下面这个顺序查找：

1. 命令行参数（`--config`、`--model`、`--provider`）
2. 环境变量（`CODER_PROVIDER`、`CODER_MODEL`）
3. 项目配置（`./coder.toml`）
4. 用户配置（`~/.coder/config.toml`）
5. 代码里的默认值

```toml
# ~/.coder/config.toml
[ai]
default_provider = "openai"

[ai.providers.openai]
type = "openai"
api_key = "${OPENAI_API_KEY}"    # 引用环境变量，安全又方便
base_url = "https://api.openai.com/v1"
model = "gpt-4o"

[ui]
theme = "coder-dark"
syntax_highlight = true
mouse_support = true
```

---

## 📖 使用指南

### 命令行参数

```bash
🦀 Coder - AI 驱动的开发工具

用法: coder [选项]

选项:
  -c, --config <文件>      配置文件路径
  -d, --directory <目录>   工作目录 [默认: .]
      --headless           无 TUI 模式（标准输入输出）
      --model <模型>       指定 AI 模型
      --print <查询>       一次性查询，打印结果后退出
      --provider <名称>    指定 AI 提供商
  -s, --session <ID>       恢复之前的会话
  -v, --verbose            输出调试日志
  -V, --version            显示版本号
```

### 运行模式

```bash
# 🌟 默认 TUI 模式 — 全功能体验
coder

# 🖥️ Headless 模式 — 适合 SSH 或 CI 场景
coder --headless

# 📄 Print 模式 — 查完就走，脚本神器
coder --print "解释 Rust 的借用检查器"
coder --print "用 Python 写一个快排" > quicksort.py

# 🔄 恢复之前的对话
coder -s <session-id>

# 🌐 启动 HTTP API 服务
coder --serve
```

### 斜杠命令（TUI 中使用）

| 分类 | 命令 |
|------|------|
| **信息** | `/help`、`/tools`、`/model`、`/context` |
| **Git** | `/status`、`/diff`、`/commit`、`/pr` |
| **搜索** | `/search`、`/web_search`、`/fetch` |
| **代码** | `/review`、`/plan`、`/test`、`/lint`、`/fix`、`/explain`、`/doc` |
| **会话** | `/clear`、`/compact`、`/summarize`、`/memory`、`/quit` |
| **配置** | `/config`、`/init` |

### @ 提及

输入 `@` 触发自动补全：

| 类型 | 例子 |
|------|------|
| **工具** | `@bash`、`@grep`、`@file_read`、`@web_search` |
| **Agent 类型** | `@agent:coding`、`@agent:research`、`@agent:debug` |
| **技能** | `@skill:brainstorm`、`@skill:code-review`、`@skill:plan` |

### 输入模式

| 前缀 | 模式 | 例子 |
|------|------|------|
| (直接打字) | AI 对话 | `用 Rust 写一个斐波那契函数` |
| `!` | Shell 命令 | `!git status` |
| `?` | 帮助 | `?git` |
| `/` | 斜杠命令 | `/help` |

---

## 🏗️ 架构速览

```
┌─────────────────────────────────────────────────────────────┐
│                   命令行层 (main.rs)                          │
│         TUI · Headless · Print · API Server                  │
├─────────────────────────────────────────────────────────────┤
│                   Agent (ReAct 循环)                         │
│       ┌──────────┐  ┌──────────┐  ┌──────────────────┐      │
│       │  Context  │  │ Provider │  │  ToolRegistry    │      │
│       └──────────┘  └──────────┘  └──────────────────┘      │
├─────────────────────────────────────────────────────────────┤
│                   AI 提供商层                                  │
│   OpenAI  ·  Anthropic  ·  Google  ·  自定义 (你说了算)       │
├─────────────────────────────────────────────────────────────┤
│                      工具层                                    │
│   Bash · 文件操作 · Glob · Grep · 网页 · Git · Docker · 数据库  │
├─────────────────────────────────────────────────────────────┤
│          扩展系统 (团队、技能、子 Agent 等)                    │
├─────────────────────────────────────────────────────────────┤
│                    存储层 (SQLite)                             │
└─────────────────────────────────────────────────────────────┘
```

核心是 **Agent ReAct 循环** —— 它思考、行动、观察，循环往复：

1. **思考** → 把上下文发给 AI
2. **行动** → AI 决定调用哪个工具（或者直接回答问题）
3. **观察** → 工具执行结果回到上下文中
4. **循环** → 直到任务完成（最多 10 轮）

---

## 🔧 从源码构建

### 特性开关

Coder 用 Cargo 特性来控制编译模块。以下是一些常见的构建配置：

```bash
# 最小构建 — 只要核心 + OpenAI
cargo build --no-default-features --features "ai-openai"

# 默认构建 — TUI + OpenAI + Anthropic + OpenCode
cargo build --release

# 全功能构建 — 要啥有啥
cargo build --release --features "ai-openai,ai-anthropic,ai-google,ai-opencode,tools-git,tools-docker,tools-db,tools-oauth,team,skill,subagent,memory,storage,server,mcp,lsp,sync,voice,oauth,analytics,permission,computer,worktree"
```

**特性分组：**

| 分组 | 特性 | 说明 |
|------|------|------|
| **AI 提供商** | `ai-openai`, `ai-anthropic`, `ai-google`, `ai-opencode` | 想支持哪些 AI 后端 |
| **Phase 1** | `team`, `skill`, `subagent`, `memory`, `storage`, `lsp`, `mcp` | 扩展系统：团队、技能、子 Agent |
| **Phase 2** | `server`, `permission`, `sync`, `voice`, `oauth`, `analytics`, `computer`, `worktree` | 高级功能 |
| **额外工具** | `tools-git`, `tools-docker`, `tools-db`, `tools-oauth` | 可选工具集成 |

### 发布版构建优化

```toml
[profile.release]
opt-level = 3        # 最大速度优化
lto = true           # 链接时优化
codegen-units = 1    # 更好的内联
strip = true         # 更小的二进制体积
```

---

## 🎯 使用场景

### 🧑‍💻 日常开发

```bash
cd your-project
coder
# → "给数据库模块加上错误处理"
# → "找出并修复项目中所有的 unwrap() 调用"
# → "给认证中间件写测试"
```

### 🔍 代码审查

```bash
coder --print "审查这些变更：$(git diff)"
# 或者在 TUI 中：
# /review
```

### 🤖 自动化 & CI

```bash
# 在脚本中一次生成代码
RESULT=$(coder --print "生成一个 Rust 应用的 Dockerfile")

# Headless 模式跑长任务
coder --headless
```

### 🧪 调试

```bash
# 在 TUI 中使用调试 Agent
@agent:debug 帮我看看这个测试为啥失败
```

### 🌐 API 服务

```bash
coder --serve
# 然后就通过 HTTP/WebSocket 在 http://localhost:3000 交互了
```

---

## 🗺️ 项目路线图

| 阶段 | 功能 | 状态 |
|------|------|------|
| **核心** | TUI、AI 提供商、工具、Agent 循环 | ✅ 已完成 |
| **Phase 1** | 团队、技能、子 Agent、记忆、存储、LSP、MCP | ✅ 已完成 |
| **Phase 2** | 服务、权限、同步、语音、OAuth、桌面操控、Worktree | ✅ 已完成 |
| **Phase 3** | 适配器（Telegram、飞书、Slack）、多模态、插件 | 🚧 计划中 |

---

## 🤝 参与贡献

Coder 是开源项目，欢迎各种贡献！参与方式：

1. **Fork** 这个仓库
2. **创建** 特性分支（`git checkout -b feature/amazing`）
3. **提交** 你的修改（`git commit -m 'feat: 添加了超酷功能'`）
4. **推送** 到分支（`git push origin feature/amazing`）
5. **创建** Pull Request

参与前请：
- 阅读[架构文档](docs/architecture.md)了解设计思路
- 查看现有 [issues](https://github.com/hzt818/coder/issues) 了解讨论
- 遵循现有代码风格（默认不可变、小而专注的文件）

---

## 📚 文档

| 资源 | 说明 |
|------|------|
| [文档中心](docs/README.md) | 全部文档索引 |
| [架构文档](docs/architecture.md) | 深入理解系统设计 |
| [使用指南](docs/usage.md) | 完整的使用文档 |
| [API 参考](docs/api.md) | HTTP API 接口文档 |
| [配置示例](config.example.toml) | 示例配置文件 |

---

## 📄 许可证

MIT 许可证 — 详见 [LICENSE](LICENSE) 文件。

---

<div align="center">

**🦀 由热爱终端的开发者打造**

*Coder —— 因为最好的代码编辑器就在你的终端里。*

</div>
