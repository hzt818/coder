//! TUI application state machine

use crate::agent::Agent;
use crate::agent::r#loop::AgentEvent;
use crate::agent::{InteractionMode, ReasoningEffort};
use crate::session::manager::SessionManager;
use crate::tui::help;
use crate::tui::vim::VimState;

/// Application modes
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Normal mode - viewing conversation
    Normal,
    /// Input mode - typing a message
    Input,
    /// Streaming mode - AI generating response
    Streaming,
    /// Detail popup open
    Detail,
    /// Confirm dialog
    Confirm { message: String },
}

/// Input sub-mode for special input handling
#[derive(Debug, Clone, PartialEq)]
pub enum InputSubmode {
    /// Normal text input
    Normal,
    /// @ mention autocomplete active
    Mention {
        query: String,
        items: Vec<String>,
        selected: usize,
    },
}

/// Type of special command detected from input prefix
#[derive(Debug, Clone, PartialEq)]
pub enum InputCommand {
    /// Normal AI chat message
    Chat,
    /// !command - direct shell execution
    Shell(String),
    /// ?query - help/question
    Help(String),
    /// /command - slash command
    Slash(String),
}

/// Application state
pub struct App {
    /// Current mode
    pub mode: AppMode,
    /// Input buffer
    pub input: String,
    /// Cursor position in the input buffer
    pub cursor_pos: usize,
    /// Input sub-mode (mention autocomplete, etc.)
    pub input_submode: InputSubmode,
    /// Chat messages for display
    pub messages: Vec<ChatMessage>,
    /// The agent driving the conversation
    pub agent: Agent,
    /// Status message
    pub status: String,
    /// Detail panel content
    pub detail_content: String,
    /// Show detail panel
    pub show_detail: bool,
    /// Scroll offset for chat panel
    pub scroll_offset: usize,
    /// Input history
    pub input_history: Vec<String>,
    pub history_position: Option<usize>,
    /// Token usage
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    /// Cost tracking
    pub total_cost: f64,
    pub session_cost: f64,
    /// Welcome screen info
    pub model_name: String,
    pub provider_name: String,
    pub working_dir: String,
    /// Session persistence manager
    pub session_manager: SessionManager,
    /// Current interaction mode (Plan/Agent/YOLO)
    pub interaction_mode: InteractionMode,
    /// Current reasoning effort
    pub reasoning_effort: ReasoningEffort,
    /// Cached status text (interior mutability for &self access)
    cached_status: std::cell::RefCell<String>,
    /// Whether the status cache is dirty
    cached_status_dirty: std::cell::Cell<bool>,
    /// Cached mention candidates (built once to avoid per-frame allocation)
    mention_all_candidates: Vec<String>,
    /// Vim modal editing state
    pub vim_state: VimState,
}

/// A message rendered in the chat panel
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub tool_calls: Vec<ToolCallDisplay>,
}

#[derive(Debug, Clone)]
pub struct ToolCallDisplay {
    pub tool_name: String,
    pub status: String, // "running", "success", "error"
    pub output: String,
}

impl App {
    /// Create a new app state
    pub fn new(
        agent: Agent,
        model_name: String,
        provider_name: String,
        working_dir: String,
    ) -> Self {
        let mention_all_candidates = Self::build_mention_candidates(&agent);
        Self {
            mode: AppMode::Input,
            input: String::new(),
            cursor_pos: 0,
            input_submode: InputSubmode::Normal,
            messages: Vec::new(),
            agent,
            status: "Ready".to_string(),
            detail_content: String::new(),
            show_detail: false,
            scroll_offset: 0,
            input_history: Vec::new(),
            history_position: None,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost: 0.0,
            session_cost: 0.0,
            model_name,
            provider_name,
            working_dir,
            session_manager: SessionManager::new(),
            interaction_mode: InteractionMode::default(),
            reasoning_effort: ReasoningEffort::default(),
            cached_status: std::cell::RefCell::new(String::new()),
            cached_status_dirty: std::cell::Cell::new(true),
            mention_all_candidates,
            vim_state: VimState::new(),
        }
    }

    /// Detect the type of command from the input
    pub fn detect_command(input: &str) -> InputCommand {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return InputCommand::Chat;
        }
        match trimmed.chars().next() {
            Some('!') => InputCommand::Shell(trimmed[1..].trim().to_string()),
            Some('?') => InputCommand::Help(trimmed[1..].trim().to_string()),
            Some('/') => InputCommand::Slash(trimmed[1..].trim().to_string()),
            _ => InputCommand::Chat,
        }
    }

    // ── Cursor operations ──

    /// Move cursor left by one character
    pub fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Move cursor right by one character
    pub fn cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
        }
    }

    /// Move cursor to the beginning
    pub fn cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    /// Move cursor to the end
    pub fn cursor_end(&mut self) {
        self.cursor_pos = self.input.len();
    }

    /// Insert a character at the cursor position
    pub fn insert_char(&mut self, c: char) {
        // If mention mode is active and user types `@`, close old one
        if self.input_submode != InputSubmode::Normal {
            if c != ' ' && c != '\t' {
                // Update mention query
                if let InputSubmode::Mention { ref mut query, .. } = self.input_submode {
                    query.push(c);
                }
            } else {
                self.input_submode = InputSubmode::Normal;
                self.mark_status_dirty();
            }
        }

        // Check for @ to start mention mode
        if c == '@' {
            let mention_items = self.mention_candidates("");
            if !mention_items.is_empty() {
                self.input_submode = InputSubmode::Mention {
                    query: String::new(),
                    items: mention_items,
                    selected: 0,
                };
                self.mark_status_dirty();
            }
        }

        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
        self.clear_mention_on_space(c);
    }

    /// Delete character before cursor (Backspace)
    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            let was_at = self.cursor_pos - 1;
            self.input.remove(was_at);
            self.cursor_pos -= 1;

            // If we backspace into or past @, close mention
            if self.cursor_pos > 0 {
                let prev = self.input[..self.cursor_pos].chars().last();
                if prev != Some('@') && self.input_submode != InputSubmode::Normal {
                    self.input_submode = InputSubmode::Normal;
                    self.mark_status_dirty();
                }
            }
        }
    }

    /// Delete character at cursor (Delete)
    pub fn delete_char(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.input.remove(self.cursor_pos);
        }
    }

    /// Delete word before cursor (Ctrl+W)
    pub fn delete_word_back(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let before = &self.input[..self.cursor_pos];
        if let Some(pos) = before.rfind(|c: char| c == ' ' || c == '/' || c == '!' || c == '@') {
            let delete_len = self.cursor_pos - pos;
            for _ in 0..delete_len {
                self.input.remove(pos);
                self.cursor_pos -= 1;
            }
        } else {
            self.input.clear();
            self.cursor_pos = 0;
        }
    }

    fn clear_mention_on_space(&mut self, c: char) {
        if c == ' ' && self.input_submode != InputSubmode::Normal {
            self.input_submode = InputSubmode::Normal;
            self.mark_status_dirty();
        }
    }

    // ── Mention ( @ ) system ──

    /// Build the static mention candidates list (called once during construction)
    fn build_mention_candidates(agent: &crate::agent::Agent) -> Vec<String> {
        let mut items: Vec<String> = agent
            .tools()
            .tool_names()
            .into_iter()
            .map(|n| format!("🔧 {}", n))
            .collect();

        // Agent types
        for t in &["coding", "research", "debug", "plan", "review"] {
            items.push(format!("🤖 agent:{}", t));
        }

        // Common skills
        for s in &["brainstorm", "code-review", "plan", "debug", "tdd"] {
            items.push(format!("⚡ skill:{}", s));
        }

        items
    }

    /// Get candidate items for @ mention autocomplete (uses cached list)
    pub fn mention_candidates(&self, query: &str) -> Vec<String> {
        if query.is_empty() {
            return self.mention_all_candidates.clone();
        }
        let q = query.to_lowercase();
        self.mention_all_candidates
            .iter()
            .filter(|item| item.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }

    /// Select next mention item
    pub fn mention_next(&mut self) {
        if let InputSubmode::Mention {
            ref mut selected,
            ref items,
            ..
        } = self.input_submode
        {
            if *selected + 1 < items.len() {
                *selected += 1;
            } else {
                *selected = 0;
            }
        }
    }

    /// Select previous mention item
    pub fn mention_prev(&mut self) {
        if let InputSubmode::Mention {
            ref mut selected,
            ref items,
            ..
        } = self.input_submode
        {
            if *selected > 0 {
                *selected -= 1;
            } else {
                *selected = items.len().saturating_sub(1);
            }
        }
    }

    /// Confirm the currently selected mention item.
    /// Replaces the `@query` in the input with `@item_name `
    pub fn confirm_mention(&mut self) {
        if let InputSubmode::Mention {
            query: _,
            ref items,
            selected,
            ..
        } = self.input_submode.clone()
        {
            if items.is_empty() {
                self.input_submode = InputSubmode::Normal;
                self.mark_status_dirty();
                return;
            }

            let selected_item = &items[selected];
            // Extract just the name part after the icon
            let name = selected_item.split_once(' ').map(|(_, n)| n).unwrap_or(selected_item);

            // Find the @ position
            if let Some(at_pos) = self.input[..self.cursor_pos].rfind('@') {
                // Replace from @ to cursor with the selected name
                let after_cursor = self.input[self.cursor_pos..].to_string();
                self.input.truncate(at_pos);
                self.input.push_str(name);
                self.input.push(' ');
                self.input.push_str(&after_cursor);
                self.cursor_pos = at_pos + name.len() + 1;
            }

            self.input_submode = InputSubmode::Normal;
            self.mark_status_dirty();
        }
    }

    /// Update mention filter as user types
    pub fn update_mention_filter(&mut self) {
        // Extract query first (avoid simultaneous borrows)
        let extracted = if let InputSubmode::Mention { .. } = self.input_submode {
            self.input[..self.cursor_pos].rfind('@').map(|at_pos| {
                self.input[at_pos + 1..self.cursor_pos].to_string()
            })
        } else {
            None
        };

        match extracted {
            Some(extracted_query) => {
                let candidates = self.mention_candidates(&extracted_query);
                if let InputSubmode::Mention { ref mut query, ref mut items, ref mut selected } = self.input_submode {
                    *query = extracted_query;
                    *items = candidates;
                    *selected = 0;
                }
            }
            None => {
                if matches!(self.input_submode, InputSubmode::Mention { .. }) {
                    self.input_submode = InputSubmode::Normal;
                    self.mark_status_dirty();
                }
            }
        }
    }

    // ── Process special commands ──

    /// Process a detected command and return the action result.
    /// Returns (processed: bool, response_text: Option<String>)
    pub fn process_command(&mut self, cmd: &InputCommand) -> (bool, Option<String>) {
        match cmd {
            InputCommand::Chat => (false, None), // normal AI processing

            InputCommand::Shell(cmd) => {
                // Execute shell command directly
                let output = self.execute_shell(cmd);
                (true, Some(output))
            }

            InputCommand::Help(topic) => {
                let help = if topic.is_empty() {
                    help::format_all()
                } else if let Some(detailed) = help::get_help(&topic) {
                    detailed
                } else {
                    help::search(&topic)
                };
                (true, Some(help))
            }

            InputCommand::Slash(cmd) => {
                let (handled, response) = self.handle_slash(cmd);
                (handled, response)
            }
        }
    }

    /// Execute a direct shell command (!)
    fn execute_shell(&self, cmd: &str) -> String {
        let output = std::process::Command::new(if cfg!(target_os = "windows") { "cmd" } else { "sh" })
            .arg(if cfg!(target_os = "windows") { "/C" } else { "-c" })
            .arg(cmd)
            .output();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);
                let mut result = format!("$ {}\n\n{}", cmd, stdout);
                if !stderr.is_empty() {
                    result.push_str(&format!("\nSTDERR:\n{}", stderr));
                }
                result
            }
            Err(e) => format!("$ {}\n\nError: {}", cmd, e),
        }
    }

    /// Handle slash command
    fn handle_slash(&mut self, cmd: &str) -> (bool, Option<String>) {
        // Split command name from arguments
        let parts: Vec<&str> = cmd.splitn(2, char::is_whitespace).collect();
        let command = parts[0].to_lowercase();
        let args = parts.get(1).copied().unwrap_or("").trim();

        match command.as_str() {
            // ── Info commands ──
            "help" | "h" => {
                if args.is_empty() {
                    (true, Some(help::format_all()))
                } else if let Some(detailed) = help::get_help(args) {
                    (true, Some(detailed))
                } else {
                    (true, Some(help::search(args)))
                }
            }
            "tools" | "t" => {
                let mut result = "── Available Tools ──\n".to_string();
                for name in self.agent.tools().tool_names() {
                    if let Some(tool) = self.agent.tools().get(&name) {
                        result.push_str(&format!("  🔧 {} - {}\n", name, tool.description()));
                    }
                }
                (true, Some(result))
            }
            "model" | "m" => {
                let mut info = format!(
                    "Provider: {}\nModel: {}\nAgent: {}\nTools: {}\nMessages: {}\nTokens: {}/{}",
                    self.provider_name,
                    self.model_name,
                    self.agent.agent_type(),
                    self.agent.tools().len(),
                    self.agent.session().message_count(),
                    crate::util::format::format_tokens(self.total_input_tokens + self.total_output_tokens),
                    crate::util::format::format_tokens(128_000),
                );
                if self.provider_name == "opencode" {
                    info.push_str("\n\n── OpenCode Models ──\n");
                    info.push_str("To fetch available models, visit https://opencode.ai/zen\n");
                    info.push_str("Use /model <name> to switch (e.g., /model gpt-4o)");
                }
                (true, Some(info))
            }
            "context" | "ctx" => {
                let ctx = format!(
                    "Messages: {}\nInput tokens: {}\nOutput tokens: {}\nTotal: {}",
                    self.messages.len(),
                    self.total_input_tokens,
                    self.total_output_tokens,
                    crate::util::format::format_tokens(self.total_input_tokens + self.total_output_tokens),
                );
                (true, Some(ctx))
            }

            // ── Git commands ──
            "status" | "st" => {
                let result = self.execute_shell("git status --short --branch");
                (true, Some(result))
            }
            "diff" => {
                let flag = if args == "--staged" || args == "--cached" { " --staged" } else { "" };
                let result = self.execute_shell(&format!("git diff{}", flag));
                (true, Some(result))
            }
            "commit" => {
                if args.is_empty() {
                    (true, Some("Usage: /commit <message>".to_string()))
                } else {
                    let result = self.execute_shell(&format!("git commit -m \"{}\"", args.replace('"', "\\\"")));
                    (true, Some(result))
                }
            }
            "pr" => {
                let result = self.execute_shell("gh pr create --fill");
                (true, Some(result))
            }

            // ── Search commands ──
            "search" | "s" => {
                if args.is_empty() {
                    (true, Some("Usage: /search <pattern> [path]".to_string()))
                } else {
                    let result = self.execute_shell(&format!("rg --line-number --color never '{}'", args.replace('\'', "'\\''")));
                    (true, Some(result))
                }
            }
            "web_search" | "ws" => {
                if args.is_empty() {
                    (true, Some("Usage: /web_search <query>".to_string()))
                } else {
                    // Use DuckDuckGo lite for simple web search
                    let query = args.replace(' ', "+");
                    let result = self.execute_shell(&format!("curl -s \"https://lite.duckduckgo.com/lite/?q={}\" | sed -n 's/.*<a[^>]*href=\"\\([^\"]*\\)\"[^>]*>\\([^<]*\\)<\\/a>.*/\\1 - \\2/p' | head -20", query));
                    (true, Some(result))
                }
            }
            "fetch" | "f" => {
                if args.is_empty() {
                    (true, Some("Usage: /fetch <url>".to_string()))
                } else {
                    let result = self.execute_shell(&format!("curl -sL '{}' | head -200", args));
                    (true, Some(result))
                }
            }

            // ── Action commands ──
            "clear" | "c" => {
                self.messages.clear();
                (true, Some("Conversation cleared.".to_string()))
            }
            "compact" => {
                // Use the intelligent compaction module
                if self.messages.is_empty() {
                    (true, Some("No messages to compact.".to_string()))
                } else {
                    // Convert ChatMessages to AI Messages for compaction
                    let ai_msgs: Vec<crate::ai::Message> = self.messages.iter()
                        .filter(|m| m.role == "user" || m.role == "assistant")
                        .map(|m| {
                            if m.role == "user" {
                                crate::ai::Message::user(&m.content)
                            } else {
                                crate::ai::Message::assistant(&m.content)
                            }
                        })
                        .collect();

                    let config = crate::core::compaction::CompactionConfig::default();
                    let result = crate::core::compaction::compact_messages(&ai_msgs, &config);

                    if result.summary_added {
                        // Replace display messages with compacted version
                        let recent_count = result.compacted_messages.saturating_sub(1);
                        if recent_count < self.messages.len() {
                            let split_at = self.messages.len().saturating_sub(recent_count);
                            self.messages.drain(..split_at);
                        }
                        (true, Some(crate::core::compaction::format_compaction_result(&result)))
                    } else {
                        (true, Some("No compaction needed.".to_string()))
                    }
                }
            }
            "summarize" => {
                let summary = format!(
                    "Conversation summary:\n- Messages: {}\n- Tokens: {}\n- Tools used: {}",
                    self.messages.len(),
                    crate::util::format::format_tokens(self.total_input_tokens + self.total_output_tokens),
                    self.agent.tools().len(),
                );
                (true, Some(summary))
            }
            "review" | "r" => {
                let result = self.execute_shell("git diff --stat");
                (true, Some(format!("── Code Review ──\n\nChanges:\n{}\n\nUse /diff to see full diff.", result)))
            }
            "plan" => {
                let plan = format!(
                    "── Implementation Plan ──\n\n\
                     Goal: {}\n\n\
                     1. Analysis - Understand requirements\n\
                     2. Design - Plan approach\n\
                     3. Implementation - Build incrementally\n\
                     4. Testing - Verify correctness\n\
                     5. Review - Quality check\n\n\
                     Start with step 1.",
                    if args.is_empty() { "Not specified" } else { args }
                );
                (true, Some(plan))
            }
            "test" => {
                let result = self.execute_shell("cargo test 2>&1");
                (true, Some(format!("── Test Results ──\n\n{}", result)))
            }
            "lint" => {
                let result = self.execute_shell("cargo clippy 2>&1");
                (true, Some(format!("── Lint Results ──\n\n{}", result)))
            }
            "fix" => {
                let result = self.execute_shell("cargo fix --allow-dirty 2>&1");
                (true, Some(format!("── Fix Results ──\n\n{}", result)))
            }
            "explain" => {
                if args.is_empty() {
                    (true, Some("Usage: /explain <code_or_file_path>".to_string()))
                } else {
                    (true, Some(format!("── Explanation for: {} ──\n\n(Explain mode: AI-powered explanation coming soon)\n\n{}", args, args)))
                }
            }
            "doc" => {
                if args.is_empty() {
                    (true, Some("Usage: /doc <target>\nExamples: /doc src/main.rs, /doc README.md".to_string()))
                } else {
                    (true, Some(format!("── Documentation for: {} ──\n\n(Doc generation coming soon)", args)))
                }
            }

            // ── Config commands ──
            "config" => {
                let config_path = crate::util::path::coder_dir().join("config.toml");
                let exists = if config_path.exists() { "Yes" } else { "No (using defaults)" };
                let info = format!(
                    "── Configuration ──\n\n\
                     Config file: {}\nExists: {}\n\n\
                     To set: /config key=value\n\
                     Edit: notepad {}",
                    config_path.display(),
                    exists,
                    config_path.display(),
                );
                (true, Some(info))
            }
            "init" => {
                let config_path = crate::util::path::coder_dir().join("config.toml");
                let msg = format!(
                    "── Initialize Configuration ──\n\n\
                     To initialize coder, create a configuration file at:\n  {}\n\n\
                     Example config:\n\
                     ```toml\n\
                     [ai]\n\
                     default_provider = \"openai\"\n\n\
                     [ai.providers.openai]\n\
                     provider_type = \"openai\"\n\
                     api_key = \"${{OPENAI_API_KEY}}\"\n\
                     model = \"gpt-4o\"\n\
                     ```",
                    config_path.display(),
                );
                (true, Some(msg))
            }
            "memory" => {
                let session_path = crate::util::path::sessions_dir();
                let count = if session_path.exists() {
                    std::fs::read_dir(&session_path).map(|e| e.count()).unwrap_or(0)
                } else {
                    0
                };
                let mem_path = crate::util::path::memory_dir();
                let mem_count = if mem_path.exists() {
                    std::fs::read_dir(&mem_path).map(|e| e.count()).unwrap_or(0)
                } else {
                    0
                };
                let info = format!(
                    "── Memory & Sessions ──\n\n\
                     Active session: {}\nMessages: {}\n\n\
                     Saved sessions: {}\nSession dir: {}\nMemory entries: {}\nMemory dir: {}",
                    self.agent.session().id,
                    self.agent.session().message_count(),
                    count,
                    session_path.display(),
                    mem_count,
                    mem_path.display(),
                );
                (true, Some(info))
            }
            "quit" | "q" | "exit" => {
                std::process::exit(0);
            }

            // ── Cost commands ──
            "cost" => {
                let cost_str = format!(
                    "── Cost Report ──\n\n\
                     Input tokens: {}\nOutput tokens: {}\nTotal tokens: {}\n\
                     Session cost: ${:.6}\nPer-1K input: ~{:.4}\n\n\
                     Costs are estimates based on model pricing.",
                    crate::util::format::format_tokens(self.total_input_tokens),
                    crate::util::format::format_tokens(self.total_output_tokens),
                    crate::util::format::format_tokens(self.total_input_tokens + self.total_output_tokens),
                    self.session_cost,
                    if self.total_input_tokens > 0 {
                        self.session_cost / (self.total_input_tokens as f64 / 1000.0)
                    } else {
                        0.0
                    },
                );
                (true, Some(cost_str))
            }
            "checkpoint" => {
                let info = crate::core::checkpoint::format_checkpoint_info();
                (true, Some(info))
            }

            // ── Mode commands ──
            "mode" => {
                let lower = args.to_lowercase();
                if lower == "plan" || lower == "p" {
                    self.interaction_mode = InteractionMode::Plan;
                    (true, Some("Switched to Plan mode (read-only, planning focus)".to_string()))
                } else if lower == "agent" || lower == "a" {
                    self.interaction_mode = InteractionMode::Agent;
                    (true, Some("Switched to Agent mode (interactive with approval)".to_string()))
                } else if lower == "yolo" || lower == "y" {
                    self.interaction_mode = InteractionMode::Yolo;
                    (true, Some("Switched to YOLO mode (auto-approve all tools)".to_string()))
                } else {
                    let msg = format!(
                        "Current mode: {} {}\n\nAvailable modes:\n  /mode plan  - Read-only investigation\n  /mode agent - Interactive with approvals (default)\n  /mode yolo  - Auto-approve all tools\n\nShortcut: Tab cycles through modes",
                        self.interaction_mode.indicator(),
                        self.interaction_mode.display_name(),
                    );
                    (true, Some(msg))
                }
            }
            "effort" | "e" => {
                let lower = args.to_lowercase();
                if lower == "off" {
                    self.reasoning_effort = ReasoningEffort::Off;
                    (true, Some("Reasoning effort set to: off".to_string()))
                } else if lower == "low" || lower == "l" {
                    self.reasoning_effort = ReasoningEffort::Low;
                    (true, Some("Reasoning effort set to: low".to_string()))
                } else if lower == "high" || lower == "h" {
                    self.reasoning_effort = ReasoningEffort::High;
                    (true, Some("Reasoning effort set to: high".to_string()))
                } else if lower == "max" || lower == "m" {
                    self.reasoning_effort = ReasoningEffort::Max;
                    (true, Some("Reasoning effort set to: max".to_string()))
                } else if lower == "auto" {
                    self.reasoning_effort = ReasoningEffort::Auto;
                    (true, Some("Reasoning effort set to: auto (automatic selection based on prompt)".to_string()))
                } else {
                    let msg = format!(
                        "Current reasoning effort: {}\n\nAvailable:\n  /effort off   - No extended thinking\n  /effort low    - Light reasoning\n  /effort high   - Standard reasoning (default)\n  /effort max    - Maximum reasoning\n  /effort auto   - Automatic selection\n\nShortcut: Shift+Tab cycles levels",
                        self.reasoning_effort.display_name(),
                    );
                    (true, Some(msg))
                }
            }

            _ => (true, Some(format!("Unknown command: /{}\nTry: /help for all commands", command))),
        }
    }

    // ── Message sending ──

    /// Send a message and start streaming
    pub fn send_message(&mut self) {
        let input = self.input.trim().to_string();
        if input.is_empty() {
            return;
        }

        // Add to history
        self.input_history.push(input.clone());
        self.history_position = None;

        // Detect command type
        let command = Self::detect_command(&input);

        // Add user message to display
        let display_content = match &command {
            InputCommand::Shell(cmd) => format!("!{}", cmd),
            InputCommand::Help(topic) => {
                if topic.is_empty() { "?".to_string() } else { format!("?{}", topic) }
            }
            InputCommand::Slash(cmd) => format!("/{}", cmd),
            InputCommand::Chat => input.clone(),
        };

        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content: display_content,
            timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
            tool_calls: Vec::new(),
        });

        // Clear input and reset cursor
        self.input.clear();
        self.cursor_pos = 0;
        self.input_submode = InputSubmode::Normal;

        // Process command or start AI streaming
        match command {
            InputCommand::Chat => {
                self.mode = AppMode::Streaming;
                // Streaming is handled by the TUI event loop
            }
            InputCommand::Shell(_) | InputCommand::Help(_) | InputCommand::Slash(_) => {
                let (_, response) = self.process_command(&command);
                if let Some(text) = response {
                    self.messages.push(ChatMessage {
                        role: "system".to_string(),
                        content: text,
                        timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
                        tool_calls: Vec::new(),
                    });
                }
                self.mode = AppMode::Input;
            }
        }

        // Persist session after every message
        self.auto_save();
        self.mark_status_dirty();
    }

    // ── Event processing ──

    /// Process a new agent event
    pub fn handle_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::ThinkingStart { provider, .. } => {
                self.status = format!("💭 Thinking ({})...", provider);
                self.messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: String::new(),
                    timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
                    tool_calls: Vec::new(),
                });
            }
            AgentEvent::TextChunk(chunk) => {
                if let Some(last) = self.messages.last_mut() {
                    if last.role == "assistant" {
                        last.content.push_str(&chunk);
                    }
                }
            }
            AgentEvent::ToolCallStart { name, .. } => {
                if let Some(last) = self.messages.last_mut() {
                    if last.role == "assistant" {
                        last.tool_calls.push(ToolCallDisplay {
                            tool_name: name,
                            status: "running".to_string(),
                            output: String::new(),
                        });
                    }
                }
                self.status = format!("🔧 Running tool...");
            }
            AgentEvent::ToolResult { tool_name, result } => {
                if let Some(last) = self.messages.last_mut() {
                    for tc in last.tool_calls.iter_mut() {
                        if tc.tool_name == tool_name {
                            tc.status = if result.success { "success".into() } else { "error".into() };
                            tc.output = result.output.clone();
                        }
                    }
                }
                self.status = format!("🔧 {}: {}", tool_name, if result.success { "OK" } else { "Failed" });
            }
            AgentEvent::Done { stop_reason, usage } => {
                if let Some(usage) = usage {
                    self.total_input_tokens += usage.input_tokens;
                    self.total_output_tokens += usage.output_tokens;
                }
                self.status = format!("Done ({})", stop_reason);
                self.mode = AppMode::Input;
            }
            AgentEvent::ReasoningChunk(chunk) => {
                if let Some(last) = self.messages.last_mut() {
                    if last.role == "assistant" {
                        last.content.push_str(&chunk);
                    }
                }
            }
            AgentEvent::ModeChanged { mode } => {
                self.interaction_mode = mode;
                self.status = format!("Mode changed to: {}", mode.display_name());
            }
            AgentEvent::ReasoningEffortChanged { effort } => {
                self.reasoning_effort = effort;
                self.status = format!("Reasoning effort: {}", effort.display_name());
            }
            AgentEvent::CostEstimate { estimate } => {
                self.total_input_tokens += estimate.input_tokens;
                self.total_output_tokens += estimate.output_tokens;
                self.total_cost += estimate.total_cost;
                self.session_cost += estimate.total_cost;
            }
            AgentEvent::Compaction { result } => {
                let msg = format!(
                    "Context compacted: {:.1}% reduction ({} → {} messages)",
                    result.reduction_pct(),
                    result.original_messages,
                    result.compacted_messages
                );
                self.status = msg;
                self.messages.push(ChatMessage {
                    role: "system".to_string(),
                    content: crate::core::compaction::format_compaction_result(&result),
                    timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
                    tool_calls: Vec::new(),
                });
            }
            AgentEvent::Error(e) => {
                self.status = format!("Error: {}", e);
                self.mode = AppMode::Input;
                self.messages.push(ChatMessage {
                    role: "system".to_string(),
                    content: format!("Error: {}", e),
                    timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
                    tool_calls: Vec::new(),
                });
            }
        }
        self.mark_status_dirty();
    }

    /// Auto-save the current session to disk
    pub fn auto_save(&self) {
        if let Err(e) = self.session_manager.save(self.agent.session()) {
            eprintln!("Warning: Failed to auto-save session: {}", e);
        }
    }

    // ── Navigation ──

    /// Scroll chat up
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
        self.mark_status_dirty();
    }

    /// Scroll chat down
    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        self.mark_status_dirty();
    }

    /// Toggle detail panel
    pub fn toggle_detail(&mut self) {
        self.show_detail = !self.show_detail;
    }

    /// Navigate input history back
    pub fn history_back(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        let pos = self.history_position.unwrap_or(self.input_history.len());
        if pos > 0 {
            let new_pos = pos - 1;
            self.input = self.input_history[new_pos].clone();
            self.cursor_pos = self.input.len();
            self.history_position = Some(new_pos);
        }
    }

    /// Navigate input history forward
    pub fn history_forward(&mut self) {
        if let Some(pos) = self.history_position {
            if pos + 1 < self.input_history.len() {
                self.input = self.input_history[pos + 1].clone();
                self.cursor_pos = self.input.len();
                self.history_position = Some(pos + 1);
            } else {
                self.input.clear();
                self.cursor_pos = 0;
                self.history_position = None;
            }
        }
    }

    /// Mark the status text cache as dirty (needs recomputation)
    pub fn mark_status_dirty(&self) {
        self.cached_status_dirty.set(true);
    }

    /// Cycle to the next interaction mode (Plan → Agent → YOLO → Plan)
    pub fn cycle_mode(&mut self) -> InteractionMode {
        let new_mode = self.interaction_mode.cycle();
        self.interaction_mode = new_mode;
        self.agent.cycle_mode();
        self.status = format!("Mode: {}", new_mode.display_name());
        self.mark_status_dirty();
        new_mode
    }

    /// Cycle to the next reasoning effort
    pub fn cycle_reasoning_effort(&mut self) -> ReasoningEffort {
        let new_effort = self.reasoning_effort.cycle();
        self.reasoning_effort = new_effort;
        self.agent.cycle_reasoning_effort();
        self.status = format!("Reasoning: {}", new_effort.display_name());
        self.mark_status_dirty();
        new_effort
    }

    /// Get current status line text (uses cached value when possible)
    pub fn status_text(&self) -> String {
        if !self.cached_status_dirty.get() {
            return self.cached_status.borrow().clone();
        }
        let computed = self.compute_status_text();
        *self.cached_status.borrow_mut() = computed.clone();
        self.cached_status_dirty.set(false);
        computed
    }

    /// Compute the status text from current state (internal, no caching)
    fn compute_status_text(&self) -> String {
        let mut status = format!(
            "🦀 tools:{} | session:{} | tokens:{}/{} | {} {} | {}",
            self.agent.tools().len(),
            self.agent.session().message_count(),
            crate::util::format::format_tokens(self.total_input_tokens + self.total_output_tokens),
            crate::util::format::format_tokens(128_000),
            self.interaction_mode.indicator(),
            self.interaction_mode.display_name(),
            self.status,
        );

        if self.scroll_offset > 0 {
            status.push_str(&format!(" | ↑ {} lines", self.scroll_offset));
        }

        status
    }
}
