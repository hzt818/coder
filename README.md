<div align="center">

# 🦀 Coder

**Your AI-Powered Terminal Coding Companion**

*Integrating the best of Claude Code and OpenCode — reimagined in Rust.*

![Rust](https://img.shields.io/badge/Rust-2021-edition?logo=rust&style=flat-square)
![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)
![Version](https://img.shields.io/badge/version-0.1.0-orange?style=flat-square)

[🇬🇧 English](README.md) · [🇨🇳 中文](README_CN.md)

---

> **Coder** isn't just another AI coding tool — it's your **terminal-native AI development environment**. Built from the ground up in Rust, it brings together the conversational power of Claude Code with the open flexibility of OpenCode, all wrapped in a blazing-fast, beautiful TUI. 🚀

</div>

---

## 🎬 See It in Action

<video src="intro.mp4" controls width="100%" style="max-width: 800px; border-radius: 8px;"></video>

---

## ✨ What Makes Coder Special?

### 🎯 **AI, Your Way**

Coder doesn't lock you into one AI provider. **Bring your own model** — or use the free tier to get started in seconds:

| Provider | Models | Notes |
|----------|--------|-------|
| **OpenCode** (Free) | Claude Sonnet 4.6, Claude Haiku 4.5 | Free tier available, no credit card needed |
| **OpenAI** | GPT-4o, GPT-4, o1, o3 | Also works with DeepSeek, Ollama, Groq, MiniMax |
| **Anthropic** | Claude Opus 4.6, Sonnet 4.6, Haiku 4.5 | Full extended thinking support |
| **Google** | Gemini 2.0 Flash, Gemini 1.5 Pro | — |
| **Custom** | Any HTTP API | Define your own request/response templates |

> 👋 **New here?** Just run `coder` — if no API key is found, a friendly setup dialog will appear. Pick "Use OpenCode Free Tier" and you're coding in 10 seconds flat. No signup, no credit card, no fuss.

### 💻 **Beautiful Terminal UI**

Built with [Ratatui](https://github.com/ratatui-org/ratatui), Coder's TUI is designed for developers who live in the terminal:

```
┌──────────────────────────────────────────────────────────┐
│  🦀 Coder  v0.1.0  ·  claude-sonnet-4-6  ·  8 tools    │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─ Message ──────────────────────────────────────────┐  │
│  │  You:  Write a binary search in Rust               │  │
│  │  AI:    Here's a clean implementation...            │  │
│  │                                                     │  │
│  │  ┌─ ⏳ file_write: src/binary_search.rs ─────────┐  │  │
│  │  │  ✅ File written successfully (420 bytes)     │  │  │
│  │  └───────────────────────────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                          │
├──────────────────────────────────────────────────────────┤
│ > Write a binary search in Rust       [Enter to send]    │
├──────────────────────────────────────────────────────────┤
│ 🦀 tools:12 | session:3 | tokens:1.2k/128k | mode:input  │
└──────────────────────────────────────────────────────────┘
```

### 🛠️ **A Toolbox That Actually Helps**

Coder comes packed with tools that let AI **do** things, not just talk:

| Tool | What It Does |
|------|-------------|
| `bash` | Run commands in your terminal |
| `file_read` / `file_write` / `file_edit` | Read, create, and modify files |
| `glob` / `grep` | Find files and search code |
| `web_fetch` / `web_search` | Browse the web in real-time |
| `git` | Stage, commit, diff, push, create PRs |
| `docker` | Manage containers |
| `db_query` | Run database queries |
| `docs` | Look up documentation |
| And more... | Task management, planning, code review, CI |

### 🧠 **Smart Interaction Modes**

Coder adapts to how you want to work:

| Mode | Shell | File Edit | Best For |
|------|-------|-----------|----------|
| 🔍 **Plan** | ❌ Read-only | ❌ Read-only | Architecture, design, code review |
| 🤖 **Agent** | ✅ Ask first | ✅ Ask first | Daily coding — safe defaults |
| ⚡ **YOLO** | ✅ Auto | ✅ Auto | Trusted automation, CI scripts |

### 👥 **Multi-Agent & Team Collaboration**

Coder scales from solo work to full team collaboration:

- **Multiple Agent Types**: Coding, Research, Debug, Plan, Review — each with specialized prompts and tool access
- **Skill System**: Reusable capabilities like brainstorming, code review, and planning
- **Subagent System**: Spawn focused sub-agents for parallel tasks
- **Team Mode**: Multiple agents coordinating on complex workflows
- **Memory System**: Cross-session memory persistence with keyword retrieval

### 🔌 **Extensible by Design**

Coder is built from day one for extensibility:

- **MCP Support** (Model Context Protocol): Connect to external MCP servers or expose Coder's tools to others
- **LSP Integration**: Language Server Protocol for code intelligence
- **Custom Providers**: Define your own AI provider with request/response templates
- **API Server**: HTTP + WebSocket API for remote access
- **Feature Flags**: Compile only what you need with Cargo features

---

## 🚀 Quick Start

### Installation

```bash
# Build from source
git clone https://github.com/hzt818/coder
cd coder
cargo build --release

# Install globally
cargo install --path .
```

> **💡 Pro tip:** Add `~/.cargo/bin` to your `$PATH` if it isn't already there.

### First Run

```bash
# Just run it — the setup dialog will guide you
coder
```

On first launch, Coder will:
1. Detect that no API key is configured
2. Show a setup dialog with options:
   - **Use OpenCode Free Tier** → Start coding immediately, no API key needed
   - **Get Free API Key (OAuth)** → Authenticate via browser
   - **Enter API Key Manually** → Paste your key
3. That's it — you're in!

### Configuration

Coder uses a simple TOML config file. It checks these locations in order:

1. CLI arguments (`--config`, `--model`, `--provider`)
2. Environment variables (`CODER_PROVIDER`, `CODER_MODEL`)
3. Project config (`./coder.toml`)
4. User config (`~/.coder/config.toml`)
5. Built-in defaults

```toml
# ~/.coder/config.toml
[ai]
default_provider = "openai"

[ai.providers.openai]
type = "openai"
api_key = "${OPENAI_API_KEY}"    # Environment variable reference
base_url = "https://api.openai.com/v1"
model = "gpt-4o"

[ui]
theme = "coder-dark"
syntax_highlight = true
mouse_support = true
```

---

## 📖 Usage Guide

### Command-Line Options

```bash
🦀 Coder - AI-powered development tool

Usage: coder [OPTIONS]

Options:
  -c, --config <FILE>      Config file path
  -d, --directory <DIR>    Working directory [default: .]
      --headless           Run without TUI (stdin/stdout)
      --model <MODEL>      AI model to use
      --print <QUERY>      One-shot query, print result, exit
      --provider <NAME>    AI provider to use
  -s, --session <ID>       Resume a previous session
  -v, --verbose            Enable debug logging
  -V, --version            Show version
```

### Run Modes

```bash
# 🌟 Full TUI experience (default)
coder

# 🖥️ Headless — great for SSH or CI
coder --headless

# 📄 One-shot query — perfect for scripting
coder --print "Explain Rust's borrow checker"
coder --print "Generate a quicksort in Python" > quicksort.py

# 🔄 Resume a session
coder -s <session-id>

# 🌐 Start the HTTP API server
coder --serve
```

### Slash Commands (in TUI)

| Category | Commands |
|----------|----------|
| **Info** | `/help`, `/tools`, `/model`, `/context` |
| **Git** | `/status`, `/diff`, `/commit`, `/pr` |
| **Search** | `/search`, `/web_search`, `/fetch` |
| **Code** | `/review`, `/plan`, `/test`, `/lint`, `/fix`, `/explain`, `/doc` |
| **Session** | `/clear`, `/compact`, `/summarize`, `/memory`, `/quit` |
| **Config** | `/config`, `/init` |

### @ Mentions

Type `@` in the input to trigger autocomplete:

| Type | Examples |
|------|----------|
| **Tools** | `@bash`, `@grep`, `@file_read`, `@web_search` |
| **Agent types** | `@agent:coding`, `@agent:research`, `@agent:debug` |
| **Skills** | `@skill:brainstorm`, `@skill:code-review`, `@skill:plan` |

### Input Modes

| Prefix | Mode | Example |
|--------|------|---------|
| (text) | AI Chat | `Write a Fibonacci function in Rust` |
| `!` | Shell | `!git status` |
| `?` | Help | `?git` |
| `/` | Slash | `/help` |

---

## 🏗️ Architecture at a Glance

```
┌─────────────────────────────────────────────────────────────┐
│                     CLI Layer (main.rs)                       │
│         TUI · Headless · Print · API Server                  │
├─────────────────────────────────────────────────────────────┤
│                    Agent (ReAct Loop)                        │
│       ┌──────────┐  ┌──────────┐  ┌──────────────────┐      │
│       │  Context  │  │ Provider │  │  ToolRegistry    │      │
│       └──────────┘  └──────────┘  └──────────────────┘      │
├─────────────────────────────────────────────────────────────┤
│                   AI Provider Layer                           │
│   OpenAI  ·  Anthropic  ·  Google  ·  Custom (User-Defined)  │
├─────────────────────────────────────────────────────────────┤
│                      Tool Layer                               │
│   Bash · File I/O · Glob · Grep · Web · Git · Docker · DB   │
├─────────────────────────────────────────────────────────────┤
│          Feature Systems (Team, Skill, Subagent, etc.)        │
├─────────────────────────────────────────────────────────────┤
│                    Storage Layer (SQLite)                     │
└─────────────────────────────────────────────────────────────┘
```

The core is the **Agent ReAct loop** — it thinks, acts, and observes in cycles:

1. **Think** → Send context to the AI provider
2. **Act** → The AI decides to use a tool (or respond directly)
3. **Observe** → Tool results feed back into context
4. **Repeat** → Until the task is complete (max 10 rounds)

---

## 🔧 Building from Source

### Feature Flags

Coder uses Cargo features for modular compilation. Here are the common build configurations:

```bash
# Minimal — just core + OpenAI
cargo build --no-default-features --features "ai-openai"

# Default — TUI + OpenAI + Anthropic + OpenCode
cargo build --release

# Full — everything
cargo build --release --features "ai-openai,ai-anthropic,ai-google,ai-opencode,tools-git,tools-docker,tools-db,tools-oauth,team,skill,subagent,memory,storage,server,mcp,lsp,sync,voice,oauth,analytics,permission,computer,worktree"
```

**Feature groups:**

| Group | Features | Description |
|-------|----------|-------------|
| **AI Providers** | `ai-openai`, `ai-anthropic`, `ai-google`, `ai-opencode` | Which AI backends to support |
| **Phase 1** | `team`, `skill`, `subagent`, `memory`, `storage`, `lsp`, `mcp` | Extensions: teams, skills, subagents |
| **Phase 2** | `server`, `permission`, `sync`, `voice`, `oauth`, `analytics`, `computer`, `worktree` | Advanced features |
| **Extra Tools** | `tools-git`, `tools-docker`, `tools-db`, `tools-oauth` | Optional tool integrations |

### Release Build Optimizations

```toml
[profile.release]
opt-level = 3        # Maximum speed
lto = true           # Link-time optimization
codegen-units = 1    # Better inlining
strip = true         # Smaller binary
```

---

## 🎯 Use Cases

### 🧑‍💻 Daily Development

```bash
cd your-project
coder
# → "Add error handling to the database module"
# → "Find and fix all unwrap() calls in this project"
# → "Write tests for the auth middleware"
```

### 🔍 Code Review

```bash
coder --print "Review these changes: $(git diff)"
# Or in TUI:
# /review
```

### 🤖 Automation & CI

```bash
# One-shot code generation in scripts
RESULT=$(coder --print "Generate a Dockerfile for a Rust app")

# Headless mode for long-running sessions
coder --headless
```

### 🧪 Debugging

```bash
# In TUI, use the debug agent
@agent:debug Help me understand why this test is failing
```

### 🌐 API Server

```bash
coder --serve
# Now you can interact via HTTP/WebSocket at http://localhost:3000
```

---

## 🗺️ Project Roadmap

| Phase | Features | Status |
|-------|----------|--------|
| **Core** | TUI, AI Providers, Tools, Agent Loop | ✅ Complete |
| **Phase 1** | Team, Skill, Subagent, Memory, Storage, LSP, MCP | ✅ Complete |
| **Phase 2** | Server, Permission, Sync, Voice, OAuth, Computer, Worktree | ✅ Complete |
| **Phase 3** | Adapters (Telegram, Feishu, Slack), Multi-modal, Plugins | 🚧 Planned |

---

## 🤝 Contributing

Coder is open-source and we'd love your contributions! Here's how to get started:

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing`)
3. **Commit** your changes (`git commit -m 'feat: add amazing feature'`)
4. **Push** to the branch (`git push origin feature/amazing`)
5. **Open** a Pull Request

Before contributing, please:
- Read the [architecture docs](docs/architecture.md) to understand the design
- Check existing [issues](https://github.com/hzt818/coder/issues) for discussions
- Follow the existing code style (immutable by default, small focused files)

---

## 📚 Documentation

| Resource | Description |
|----------|-------------|
| [Documentation Hub](docs/README.md) | Central documentation index |
| [Architecture](docs/architecture.md) | Deep dive into the system design |
| [Usage Guide](docs/usage.md) | Complete usage documentation |
| [API Reference](docs/api.md) | HTTP API endpoint documentation |
| [Config Example](config.example.toml) | Example configuration file |

---

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.

---

<div align="center">

**Made with 🦀 by developers who love the terminal**

*Coder — because the best code editor is the one in your terminal.*

</div>
