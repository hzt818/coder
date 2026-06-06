# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
# Default build (OpenAI + Anthropic + OpenCode providers)
cargo build --release

# Build with workspace flag (used in CI)
cargo build --workspace

# Run all tests
cargo test
cargo test --workspace

# Run a specific test (use -- to pass filter to test binary)
cargo test -- test_registry_default
cargo test --lib -- registry::tests::test_registry_default

# Build with all features (includes `security`)
cargo build --release --features "ai-openai,ai-anthropic,ai-google,ai-opencode,tools-git,tools-docker,tools-db,tools-oauth,team,skill,subagent,memory,storage,server,mcp,lsp,sync,voice,oauth,analytics,permission,security,computer,worktree"

# Minimal build (no default features)
cargo build --no-default-features --features "ai-openai"

# Run lints (Clippy) — CI uses -D warnings
cargo clippy --all-targets -- -D warnings

# Format check
cargo fmt --check

# Security audit (requires cargo-audit)
cargo audit --deny warnings

# Run verbose with warning/error output visible
cargo test 2>&1 | grep -E "warning:|error:|test result"

# Linux system dependencies (needed for full builds)
# sudo apt-get install -y pkg-config libssl-dev libdbus-1-dev libfontconfig-dev
```

## Project Architecture

Coder is a Rust (2021 edition, Tokio async) terminal-native AI coding companion. It integrates patterns from Codex and OpenCode.

### Core Runtime (`src/core/`)
- `pricing` — Token counting and cost estimation
- `compaction` — Context window compaction to manage token limits (default: keep latest 10 messages when approaching limit)
- `checkpoint` — Session checkpointing
- `audit` — Audit logging for all tool executions
- `capacity` — Output truncation and capacity routing for large tool results
- `snapshot` — Session snapshot/restore
- `automation` — Background automation manager
- `hooks` — Pre/post tool execution hook dispatcher
- `lsp_hooks` — LSP-driven auto-formatting on file writes (feature-gated)
- `task_manager` — Concurrent task tracking (background shells, agents)
- `features` — Feature management
- `bridge/` — Module continuity bridges: `agent_memory`, `session_storage`, `tool_skill`, `team_subagent` (`init_bridges()` called at startup)

### Context Storage (`src/context/`)
Pluggable message history backends via the `ContextStore` trait:
- `store.rs` — In-memory `Context` (default, backed by `RwLock<Vec<String>>`)
- `sqlite.rs` — SQLite-backed `SqliteContext` (requires `storage` feature)

Distinct from `agent/context.rs` which manages the agent's conversation context (system prompts, tool definitions, message formatting for the LLM).

### AI Provider Layer (`src/ai/`)
Each provider implements the `Provider` trait (`fn complete()`, `fn complete_stream()`):
- `openai` — OpenAI-compatible APIs (also covers DeepSeek, Ollama, Groq, MiniMax)
- `anthropic` — Anthropic Codex API with extended thinking support
- `google` — Google Gemini API
- `opencode` — OpenCode free tier (wraps Anthropic via opencode.ai)
- `custom` — User-defined HTTP API with custom request/response templates
- `provider.rs` — Factory function `create_provider()` that routes by provider_type string

### Tool System (`src/tool/`)
All tools implement the `Tool` trait (`name()`, `description()`, `schema()`, `execute()`):
- **Core:** bash, file_read, file_write, file_edit, glob, grep, web_fetch, web_search
- **Dev workflow:** plan, task, checklist, apply_patch, fim_edit, list_dir, snapshot, diagnostics, run_tests, review, ci, github, pr_attempt, recall
- **Infrastructure:** lsp (feature-gated), worktree, docker, db_query, oauth
- **Automation:** automation_tool, task_gate, rlm, cron, task_shell, monitor, notification, schedule, remote_trigger
- **Other:** docs, finance, validate_data, web_run

Tool registration is in `registry.rs` — the `Default` impl registers all available tools.

### Agent Engine (`src/agent/`)
The core ReAct loop (`loop.rs`): Think → Act → Observe cycle. Default max 50 tool rounds (override with `CODER_MAX_TOOL_ROUNDS`).
- `Agent` struct holds a `Provider` and `ToolRegistry`
- `context.rs` — Manages conversation context (messages, system prompts, tool definitions) for the LLM
- `dispatch.rs` — Sub-agent dispatch for parallel task execution
- `types.rs` — `AgentType` (Coding/Research/Debug/Plan/Review — each with specialized system prompts), `InteractionMode` (Plan/Agent/YOLO), `ReasoningEffort` (Off/Low/High/Max/Auto)
- `auto_reasoning.rs` — Automatic reasoning effort adjustment based on prompt complexity
- `coordinator.rs` — Multi-agent coordination

### Presentation Layer (`src/tui/`)
Built with Ratatui + Crossterm:
- `app.rs` — App state machine (Normal/Input/Streaming/Detail/Confirm modes), includes setup wizard flow on first launch
- `ui.rs` — Main event loop and rendering
- `input.rs` — Input handling with @ mentions, !commands, /slash-commands, ?help
- `vim.rs` — Vim-like keybindings
- `setup_wizard.rs` — First-run setup dialog (OpenCode free tier, OAuth, or manual API key entry)
- `chat_panel.rs`, `status_bar.rs`, `help.rs`, `command_palette.rs`, `mention_popup.rs`, `dialog_provider_setup.rs`, `detail_popup.rs`
- `syntax.rs` — Syntax highlighting via Syntect
- `theme.rs` — TUI theme definitions

### Feature Systems (all feature-gated)
- `team/` — Multi-agent team coordination with communication and task management
- `skill/` — Plug-in capabilities (brainstorm, code_review, debug, plan built-in skills) loaded via `loader.rs` and indexed in `registry.rs`
- `subagent/` — Spawn focused sub-agents for parallel tasks with supervisor support
- `memory/` — Cross-session persistence with keyword retrieval and auto-dreaming
- `lsp/` — LSP client via tower-lsp for code intelligence
- `mcp/` — Model Context Protocol client/server and Context7 integration
- `server/` — Axum HTTP + WebSocket API server
- `sync/` — Cloud sync for sessions and configuration
- `voice/` — Audio input/output via cpal + hound
- `computer/` — Computer use (keyboard, mouse, screenshot) via enigo + screenshots crate
- `storage/` — SQLite/libSQL persistence layer
- `oauth/` — OAuth 2.0 flow support
- `permission/` — Permission policy evaluation
- `analytics/` — Usage analytics
- `worktree/` — Git worktree management for isolated development

### Other Modules
- `config/` — TOML-based hierarchical config (CLI args > env vars > project config > user config > defaults); `${ENV_VAR}` references auto-resolved at load
- `session/` — Session persistence, history search, load/save
- `commands/` — Slash command parsing and dispatch (`/help`, `/git`, `/plan`, etc.)
- `execpolicy/` — Layered permission rulesets (deny > builtin > agent > user) with arity-aware bash command matching
- `sandbox/` — Sandboxed execution (local/remote isolation)
- `security/` — Encryption, keychain, input sanitizer (separate from permission)
- `util/` — Formatting, path, template utilities
- `adapters/` — External platform adapters (Telegram, Feishu)

### Key Design Patterns

- **Immutable data by default** — functions return new objects rather than mutating inputs
- **Trait-based polymorphism** — `Provider`, `Tool`, `Skill`, and `ContextStore` are all trait interfaces
- **Feature-gated modules** — Each Phase 1/2 feature is behind a Cargo feature flag; always wrap module declarations and usages in `#[cfg(feature = "...")]`
- **Hierarchical config** — Config is resolved at startup: CLI args → env vars → project `coder.toml` → user `~/.coder/config.toml` → built-in defaults
- **ToolResult envelope** — Every tool returns `ToolResult { success, output, error, metadata, truncated, estimated_tokens, original_size }`
- **Agent ReAct loop** — Think → Act → Observe cycle with a configurable max round limit (default 50)

### First-Run Flow

On first launch, if no API key is configured, a setup wizard (`src/tui/setup_wizard.rs`) offers:
1. Use OpenCode Free Tier (no API key needed)
2. Authenticate via OAuth in browser
3. Enter API key manually

## Modular Rewrite (in progress)

The codebase is being decomposed into independent crates under `rewrite/`:

```
rewrite/
├── coder-core/     — ReAct loop, agent engine
├── coder-cli/      — CLI argument parsing, runtime entry points
├── coder-tui/      — Terminal UI (Ratatui + Crossterm)
├── coder-ai/       — AI provider abstraction
├── coder-tools/    — Tool implementations
├── coder-storage/  — Persistence layer (SQLite, libSQL)
└── coder-context/  — Context storage abstraction
```

```bash
# Build the rewrite workspace
cargo build --workspace --manifest-path rewrite/Cargo.toml
```

The original `src/` tree remains the primary build target during the transition.

## Environment Variables

| Variable | Controls |
|----------|----------|
| `CODER_PROVIDER` | Default AI provider name |
| `CODER_MODEL` | Model name override |
| `CODER_CONFIG` | Config file path |
| `CODER_MAX_TOOL_ROUNDS` | Max ReAct loop iterations (default: 50) |

CLI flags (`--provider`, `--model`, `-c`) take precedence over these. The config file also supports `${ENV_VAR}`-style references (e.g. `${OPENAI_API_KEY}`) for API keys.

## CLI Flags (Notable)

| Flag | Description |
|------|-------------|
| `--serve` | Start the HTTP API server (Axum, requires `server` feature) |
| `--headless` | Run without TUI (stdin/stdout REPL) |
| `--print <QUERY>` | One-shot query, print result, exit |
| `-s, --session <ID>` | Resume a previous session |
| `-v, --verbose` | Enable debug logging |

## Code Quality

- `main.rs`: `#![deny(unused)]` + `#![warn(clippy::all, clippy::pedantic)]`
- CI enforces `cargo clippy -- -D warnings` and `cargo fmt --check`
- CI runs `cargo audit --deny warnings` for vulnerability scanning

## Feature Flags

Feature groups from Cargo.toml:
- **AI Providers:** `ai-openai`, `ai-anthropic`, `ai-google`, `ai-opencode`
- **Extra Tools:** `tools-git`, `tools-docker`, `tools-db`, `tools-oauth`
- **Phase 1:** `team`, `skill`, `subagent`, `memory`, `storage`, `lsp`, `mcp`
- **Phase 2:** `server`, `permission`, `sync`, `voice`, `oauth`, `analytics`, `security`, `computer`, `worktree`

Default features: `ai-openai`, `ai-anthropic`, `ai-opencode`
