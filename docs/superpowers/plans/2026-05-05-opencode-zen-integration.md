# OpenCode Zen API Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate OpenCode's Zen API as a built-in AI provider in coder, supporting both anonymous (free tier) and API-key-authenticated usage.

**Architecture:** New `opencode` provider wraps OpenAI-compatible HTTP calls to `opencode.ai/zen/v1`. A TUI setup dialog on startup detects missing API keys and offers the user: free tier (anonymous), OAuth key acquisition, or manual key input.

**Tech Stack:** Rust, tokio, reqwest, ratatui, oauth2 crate, serde

---

## File Structure

### New files:
| File | Responsibility |
|------|---------------|
| `src/ai/opencode.rs` | OpenCode provider — anonymous/key modes, model fetching, chat_stream |
| `src/tui/dialog_provider_setup.rs` | TUI dialog for initial provider selection |
| `src/oauth/opencode.rs` | OpenCode OAuth flow (browser-based) |
| `src/ai/opencode_test.rs` | Tests for OpenCode provider |

### Modified files:
| File | Changes |
|------|---------|
| `Cargo.toml` | Add `ai-opencode` feature flag, ensure `oauth2` is a default dep |
| `src/ai/mod.rs` | Add `opencode` module and match arm in `create_provider()` |
| `src/config/settings.rs` | Add OpenCode as default provider when no API key found |
| `src/main.rs` | Add startup API key check, invoke setup dialog |
| `src/tui/mod.rs` or `src/tui/app.rs` | Export new dialog module |

---

### Task 1: Add `ai-opencode` feature flag

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add feature flag and dependency**

Edit `Cargo.toml`:

Add to `[features]`:
```toml
ai-opencode = []
```

Change `default` to include it:
```toml
default = ["tui", "ai-openai", "ai-anthropic", "ai-opencode", "tools-core"]
```

Ensure `oauth2` is an unconditional dependency (remove feature gate if any):
```toml
oauth2 = "5.0"   # already present, verify it's unconditional
```

- [ ] **Step 2: Verify cargo check**

Run: `cd D:\Coder\coder && cargo check`
Expected: No errors, new feature shows up.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: add ai-opencode feature flag"
```

---

### Task 2: Create `src/ai/opencode.rs` — OpenCode provider (core logic)

**Files:**
- Create: `src/ai/opencode.rs`
- Test: (Task 3)

- [ ] **Step 1: Create file with struct definition and constructor**

Create `src/ai/opencode.rs`:

```rust
//! OpenCode Zen API provider
//!
//! Provides access to OpenCode's free AI models via their Zen API proxy.
//! Supports two modes:
//! - Anonymous (no API key): IP-based rate limiting, limited model set
//! - Authenticated (with API key): workspace-based access, full model set

use async_trait::async_trait;
use serde::Deserialize;
use super::*;
use super::provider::{Provider, StreamHandler};

/// Response from GET /zen/v1/models
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
    #[allow(dead_code)]
    #[serde(default)]
    object: String,
}

/// OpenCode Zen API provider
#[derive(Debug)]
pub struct OpenCodeProvider {
    /// API key (None = anonymous/free tier mode)
    api_key: Option<String>,
    /// Base URL for Zen API (default: https://opencode.ai/zen/v1)
    base_url: String,
    /// Current model name
    model: String,
    /// Available models fetched from /zen/v1/models
    available_models: Vec<String>,
}

impl OpenCodeProvider {
    /// Create a new OpenCode provider.
    ///
    /// `api_key`:
    /// - `Some(key)` → authenticated mode (Bearer auth)
    /// - `None` → anonymous/free tier mode
    pub fn new(api_key: Option<String>, base_url: Option<String>, model: String) -> Self {
        let base_url = base_url
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| "https://opencode.ai/zen/v1".to_string())
            .trim_end_matches('/')
            .to_string();

        Self {
            api_key,
            base_url,
            model,
            available_models: Vec::new(),
        }
    }

    /// Fetch available models from the Zen API.
    pub async fn fetch_models(&self) -> anyhow::Result<Vec<String>> {
        let client = reqwest::Client::new();
        let mut req = client.get(format!("{}/models", self.base_url));

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenCode models API error ({}): {}", status, body);
        }

        let data: ModelsResponse = resp.json().await?;
        Ok(data.data.into_iter().map(|m| m.id).collect())
    }

    /// Update available models list (called after fetch).
    pub fn set_available_models(&mut self, models: Vec<String>) {
        self.available_models = models;
    }

    /// Get the list of available models.
    pub fn available_models(&self) -> &[String] {
        &self.available_models
    }

    /// Check if the provider is in anonymous mode.
    pub fn is_anonymous(&self) -> bool {
        self.api_key.is_none()
    }

    /// Build the OpenAI-compatible request body.
    fn build_request(&self, messages: &[Message], tools: &[ToolDef], config: &GenerateConfig) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages_to_openai(messages),
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "stream": true,
        });

        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools.iter().map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    }
                })
            }).collect::<Vec<_>>());
        }

        body
    }
}

#[async_trait]
impl Provider for OpenCodeProvider {
    fn name(&self) -> &str {
        "OpenCode"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        config: &GenerateConfig,
    ) -> anyhow::Result<StreamHandler> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        let client = reqwest::Client::new();
        let request_body = self.build_request(messages, tools, config);

        let mut request = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body);

        // Add auth header only when API key is present
        if let Some(ref key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // Provide user-friendly error messages
            let msg = match status {
                429 => format!(
                    "OpenCode API rate limit exceeded. {}",
                    if self.is_anonymous() {
                        "Try again later or get a free API key at https://opencode.ai/zen"
                    } else {
                        "Try again later."
                    }
                ),
                401 | 403 => "Invalid or expired API key. Check your key at https://opencode.ai/zen".to_string(),
                404 => format!("Model '{}' not available via OpenCode. Run /model to see available models.", self.model),
                _ => format!("OpenCode API error ({}): {}", status, body),
            };
            anyhow::bail!("{}", msg);
        }

        tokio::spawn(async move {
            // Reuse the SSE parser from the openai module
            // Since openai.rs is in the same crate, we can call its public functions
            // We need to make parse_sse_stream and messages_to_openai public in openai.rs
            if let Err(e) = super::openai::parse_sse_stream_public(response, tx.clone()).await {
                tracing::error!("OpenCode SSE parse error: {}", e);
                let _ = tx.send(StreamEvent::Error(e.to_string())).await;
            }
        });

        Ok(rx)
    }
}

/// Convert internal messages to OpenAI format (re-export from openai.rs)
fn messages_to_openai(messages: &[Message]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|msg| {
            let role = msg.role.to_string();
            let mut json_msg = serde_json::json!({
                "role": role,
                "content": msg.text(),
            });

            if let Some(tcid) = &msg.tool_call_id {
                json_msg["tool_call_id"] = serde_json::json!(tcid);
            }

            json_msg
        })
        .collect()
}
```

- [ ] **Step 2: Make SSE parser functions public in `src/ai/openai.rs`**

Add `pub` to the SSE parser function so OpenCode provider can reuse it:

In `src/ai/openai.rs`, change:
```rust
async fn parse_sse_stream(
```
to:
```rust
pub async fn parse_sse_stream_public(
```
And rename all internal calls from `parse_sse_stream` to `parse_sse_stream_public`.

Also add `pub` to `process_sse_data`:
```rust
pub async fn process_sse_data(
```

Update the call site in `openai.rs` at line ~95-97:
```rust
// Before:
if let Err(e) = parse_sse_stream(response, tx.clone()).await {
// After:
if let Err(e) = parse_sse_stream_public(response, tx.clone()).await {
```

- [ ] **Step 3: Verify it compiles**

Run: `cd D:\Coder\coder && cargo check --features ai-opencode`
Expected: Compilation succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/ai/opencode.rs src/ai/openai.rs
git commit -m "feat: add OpenCode provider with anonymous and key-based modes"
```

---

### Task 3: Write tests for OpenCode provider

**Files:**
- Create: `src/ai/opencode_test.rs` (or inline tests in `opencode.rs`)

- [ ] **Step 1: Add unit tests for request building**

Add at the bottom of `src/ai/opencode.rs` (before any closed bracket):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_default_url() {
        let p = OpenCodeProvider::new(None, None, "claude-sonnet-4-6".to_string());
        assert_eq!(p.base_url, "https://opencode.ai/zen/v1");
        assert!(p.is_anonymous());
        assert_eq!(p.model(), "claude-sonnet-4-6");
    }

    #[test]
    fn test_new_with_custom_url() {
        let p = OpenCodeProvider::new(
            Some("opk_test".to_string()),
            Some("https://custom.zen.url/v1".to_string()),
            "gpt-4o".to_string(),
        );
        assert_eq!(p.base_url, "https://custom.zen.url/v1");
        assert!(!p.is_anonymous());
    }

    #[test]
    fn test_new_trims_trailing_slash() {
        let p = OpenCodeProvider::new(None, Some("https://opencode.ai/zen/v1/".to_string()), "m".to_string());
        assert_eq!(p.base_url, "https://opencode.ai/zen/v1");
    }

    #[test]
    fn test_build_request_basic() {
        let p = OpenCodeProvider::new(None, None, "claude-sonnet-4-6".to_string());
        let msgs = vec![Message::user("hello")];
        let tools = vec![];
        let config = GenerateConfig::default();
        let body = p.build_request(&msgs, &tools, &config);

        assert_eq!(body["model"], "claude-sonnet-4-6");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "hello");
        assert_eq!(body["stream"], true);
    }

    #[test]
    fn test_build_request_with_tools() {
        let p = OpenCodeProvider::new(None, None, "m".to_string());
        let msgs = vec![Message::user("list files")];
        let tools = vec![ToolDef {
            name: "bash".to_string(),
            description: "Run shell".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        }];
        let config = GenerateConfig::default();
        let body = p.build_request(&msgs, &tools, &config);

        assert!(body["tools"].is_array());
        assert_eq!(body["tools"][0]["function"]["name"], "bash");
    }

    #[test]
    fn test_available_models() {
        let mut p = OpenCodeProvider::new(None, None, "m".to_string());
        assert!(p.available_models().is_empty());

        p.set_available_models(vec!["m1".to_string(), "m2".to_string()]);
        assert_eq!(p.available_models().len(), 2);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd D:\Coder\coder && cargo test --features ai-opencode -p coder ai::opencode::tests`
Expected: All tests PASS.

- [ ] **Step 3: Commit**

```bash
git add src/ai/opencode.rs
git commit -m "test: add OpenCode provider unit tests"
```

---

### Task 4: Register opencode module in `src/ai/mod.rs`

**Files:**
- Modify: `src/ai/mod.rs`

- [ ] **Step 1: Add module declaration and provider support**

In `src/ai/mod.rs`, add after the existing modules:

```rust
#[cfg(feature = "ai-opencode")]
pub mod opencode;
```

In `create_provider()` function, add a new match arm after the "custom" arm:

```rust
"opencode" => {
    #[cfg(feature = "ai-opencode")]
    {
        let provider = opencode::OpenCodeProvider::new(
            config.api_key,
            config.base_url,
            model,
        );
        Ok(Box::new(provider))
    }
    #[cfg(not(feature = "ai-opencode"))]
    anyhow::bail!("OpenCode provider requires 'ai-opencode' feature")
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd D:\Coder\coder && cargo check`
Expected: Compilation succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/ai/mod.rs
git commit -m "feat: register opencode provider in AI module"
```

---

### Task 5: Add default OpenCode config to settings

**Files:**
- Modify: `src/config/settings.rs`

- [ ] **Step 1: Add OpenCode default provider in AiSettings::default()**

In `src/config/settings.rs`, modify `AiSettings::default()` to include opencode:

```rust
impl Default for AiSettings {
    fn default() -> Self {
        let mut providers = HashMap::new();

        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                provider_type: "openai".to_string(),
                api_key: Some("${OPENAI_API_KEY}".to_string()),
                base_url: Some("https://api.openai.com/v1".to_string()),
                model: Some("gpt-4o".to_string()),
                ..Default::default()
            },
        );

        providers.insert(
            "opencode".to_string(),
            ProviderConfig {
                provider_type: "opencode".to_string(),
                api_key: None,  // None = anonymous/free tier
                base_url: Some("https://opencode.ai/zen/v1".to_string()),
                model: Some("claude-sonnet-4-6".to_string()),
                ..Default::default()
            },
        );

        Self {
            default_provider: "opencode".to_string(),
            providers,
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd D:\Coder\coder && cargo check`
Expected: Compilation succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/config/settings.rs
git commit -m "feat: add opencode as default provider in config"
```

---

### Task 6: Create startup provider setup dialog (TUI)

**Files:**
- Create: `src/tui/dialog_provider_setup.rs`

- [ ] **Step 1: Create the dialog component**

Create `src/tui/dialog_provider_setup.rs`:

```rust
//! Provider Setup Dialog
//!
//! TUI dialog shown at startup when no API key is configured.
//! Offers the user:
//! 1. Use OpenCode Free Tier (anonymous)
//! 2. Get API Key via OAuth
//! 3. Enter API Key manually
//! 4. Skip (configure later)

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Options presented in the setup dialog
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProviderSetupChoice {
    /// Use OpenCode free tier (anonymous, no key needed)
    FreeTier,
    /// Get API key via OAuth browser flow
    OAuth,
    /// Enter API key manually
    Manual,
    /// Skip setup, user will configure later
    Skip,
}

impl ProviderSetupChoice {
    pub fn label(&self) -> &'static str {
        match self {
            Self::FreeTier => "Use OpenCode Free Tier (anonymous)",
            Self::OAuth => "Get Free API Key (OAuth)",
            Self::Manual => "Enter API Key Manually",
            Self::Skip => "Skip — I'll configure later",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::FreeTier => "Start immediately with IP-based rate limiting",
            Self::OAuth => "Sign in via browser, automatically get an API key",
            Self::Manual => "Paste your API key from opencode.ai/zen",
            Self::Skip => "Use /config command to set up a provider later",
        }
    }

    pub fn all() -> &'static [ProviderSetupChoice] {
        &[
            Self::FreeTier,
            Self::OAuth,
            Self::Manual,
            Self::Skip,
        ]
    }
}

/// Result from the provider setup dialog
#[derive(Debug)]
pub enum ProviderSetupResult {
    /// User chose to use OpenCode free tier
    FreeTier,
    /// User wants OAuth flow
    OAuth,
    /// User entered an API key manually
    ManualKey(String),
    /// User skipped setup
    Skipped,
    /// User quit the application
    Quit,
}

/// Run the provider setup dialog.
///
/// Returns the user's choice. This function blocks until a selection is made
/// or the user quits.
pub fn run_provider_setup_dialog<B: Backend>(frame: &mut Frame<B>) -> ProviderSetupResult {
    let mut selected = 0usize;
    let options = ProviderSetupChoice::all();
    let mut manual_key_input = String::new();
    let mut show_manual_input = false;

    loop {
        // Render dialog
        if show_manual_input {
            render_manual_key_dialog(frame, &manual_key_input);
        } else {
            render_provider_dialog(frame, options, selected);
        }

        // Handle input
        match event::read() {
            Ok(Event::Key(key)) => match key.code {
                KeyCode::Up | KeyCode::Char('k') if !show_manual_input => {
                    selected = selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') if !show_manual_input => {
                    selected = (selected + 1).min(options.len() - 1);
                }
                KeyCode::Enter => {
                    if show_manual_input {
                        return if manual_key_input.trim().is_empty() {
                            ProviderSetupResult::Skipped
                        } else {
                            ProviderSetupResult::ManualKey(manual_key_input.trim().to_string())
                        };
                    }
                    match options[selected] {
                        ProviderSetupChoice::FreeTier => return ProviderSetupResult::FreeTier,
                        ProviderSetupChoice::OAuth => return ProviderSetupResult::OAuth,
                        ProviderSetupChoice::Manual => {
                            show_manual_input = true;
                        }
                        ProviderSetupChoice::Skip => return ProviderSetupResult::Skipped,
                    }
                }
                KeyCode::Char(c) if show_manual_input => {
                    manual_key_input.push(c);
                }
                KeyCode::Backspace if show_manual_input => {
                    manual_key_input.pop();
                }
                KeyCode::Esc if show_manual_input => {
                    show_manual_input = false;
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    return ProviderSetupResult::Quit;
                }
                _ => {}
            },
            Ok(Event::Resize(_, _)) => {}
            _ => {}
        }
    }
}

fn render_provider_dialog<B: Backend>(
    frame: &mut Frame<B>,
    options: &[ProviderSetupChoice],
    selected: usize,
) {
    let area = centered_rect(60, 50, frame.size());

    // Clear area
    frame.render_widget(Clear, area);

    // Title
    let title_block = Block::default()
        .borders(Borders::ALL)
        .title(" 🔑 AI Provider Setup ")
        .style(Style::default().fg(Color::Cyan));

    // Options list
    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let prefix = if i == selected { " ▶ " } else { "   " };
            let content = vec![
                Line::from(Span::styled(
                    format!("{}{}", prefix, opt.label()),
                    if i == selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                )),
                Line::from(Span::styled(
                    format!("     {}", opt.description()),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(String::new()),
            ];
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items).block(title_block).highlight_style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_widget(list, area);

    // Instructions at bottom
    let instructions = Paragraph::new(Text::from(
        "\n ↑/↓ or j/k to navigate  •  Enter to select  •  q to quit",
    ))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::DarkGray));

    let instr_area = Rect::new(
        area.x,
        area.y + area.height - 3,
        area.width,
        3,
    );
    frame.render_widget(Clear, instr_area);
    frame.render_widget(instructions, instr_area);
}

fn render_manual_key_dialog<B: Backend>(frame: &mut Frame<B>, input: &str) {
    let area = centered_rect(60, 30, frame.size());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 🔑 Enter API Key ")
        .style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Instructions
    let instructions = Paragraph::new(Text::from(vec![
        Line::from("Get your free API key at:"),
        Line::from(Span::styled(
            "  https://opencode.ai/zen",
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED),
        )),
        Line::from(String::new()),
        Line::from("Paste your API key below (key will not be shown):"),
        Line::from(String::new()),
        Line::from(Span::styled(
            format!("  {}█", "*".repeat(input.len().saturating_sub(1))),
            Style::default().fg(Color::Green),
        )),
    ]))
    .style(Style::default().fg(Color::White));

    let text_area = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height,
    );
    frame.render_widget(instructions, text_area);
}

/// Helper to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height * (100 - percent_y)) / 200),
            Constraint::Length((r.height * percent_y) / 100),
            Constraint::Length((r.height * (100 - percent_y)) / 200),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width * (100 - percent_x)) / 200),
            Constraint::Length((r.width * percent_x) / 100),
            Constraint::Length((r.width * (100 - percent_x)) / 200),
        ])
        .split(popup_layout[1])[1]
}
```

- [ ] **Step 2: Export the dialog module**

Add to `src/tui/mod.rs` (or `src/tui/app.rs` — check the existing module structure):

If `src/tui/mod.rs` exists, add:
```rust
pub mod dialog_provider_setup;
```

If `src/tui/app.rs` is the module root through `src/tui/mod.rs`, add the pub mod there.

Check existing `src/tui/mod.rs`:

```rust
// Add to existing mod declarations
pub mod dialog_provider_setup;
```

- [ ] **Step 3: Verify it compiles**

Run: `cd D:\Coder\coder && cargo check`
Expected: Compilation succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/tui/dialog_provider_setup.rs src/tui/mod.rs
git commit -m "feat: add startup provider setup TUI dialog"
```

---

### Task 7: Add startup API key check in `main.rs`

This task modifies the main entry point to detect when no API key is configured and show the setup dialog.

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add startup check logic**

Change `run_tui_mode` signature to take owned `Settings` (since we may need to modify it), and add the API key check dialog:

```rust
async fn run_tui_mode(
    mut config: coder::config::Settings,
    cli: &Cli,
) -> anyhow::Result<()> {
    // Check if any provider has an API key configured
    let has_api_key = config.ai.providers.values().any(|p| p.api_key.is_some());

    if !has_api_key {
        use coder::tui::dialog_provider_setup::{run_provider_setup_dialog, ProviderSetupResult};

        let mut terminal = coder::tui::init_terminal()?;
        let result = {
            let mut frame = terminal.get_frame();
            run_provider_setup_dialog(&mut frame)
        };
        coder::tui::restore_terminal()?;

        match result {
            ProviderSetupResult::FreeTier => {
                tracing::info!("User selected OpenCode free tier (anonymous)");
                // Ensure default provider is opencode with no key
                config.ai.default_provider = "opencode".to_string();
            }
            ProviderSetupResult::OAuth => {
                tracing::info!("User selected OAuth flow");
                match coder::oauth::opencode::run_oauth_flow().await {
                    coder::oauth::opencode::OAuthResult::Success(key) => {
                        save_opencode_config(&mut config, &key)?;
                    }
                    coder::oauth::opencode::OAuthResult::Cancelled => {
                        anyhow::bail!("OAuth cancelled.");
                    }
                    coder::oauth::opencode::OAuthResult::Error(e) => {
                        anyhow::bail!("OAuth failed: {}", e);
                    }
                }
            }
            ProviderSetupResult::ManualKey(key) => {
                tracing::info!("User entered API key manually");
                save_opencode_config(&mut config, &key)?;
            }
            ProviderSetupResult::Skipped | ProviderSetupResult::Quit => {
                anyhow::bail!("No AI provider configured. Run with --help for options.");
            }
        }
    }

    let provider = create_provider(&config, cli)?;
    let tools = coder::tool::ToolRegistry::default();
    let agent = coder::agent::Agent::new(provider, tools);

    // Extract model & provider info for the welcome screen
    let provider_name = cli
        .provider
        .clone()
        .unwrap_or_else(|| config.ai.default_provider.clone());
    let model_name = cli
        .model
        .clone()
        .or_else(|| {
            config
                .ai
                .providers
                .get(&provider_name)
                .and_then(|p| p.model.clone())
        })
        .unwrap_or_else(|| "unknown".to_string());
    let working_dir = std::fs::canonicalize(&cli.directory)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| cli.directory.clone());

    let mut terminal = coder::tui::init_terminal()?;
    let mut app = coder::tui::App::new(agent, model_name, provider_name, working_dir);
    let result = coder::tui::ui::run_app(&mut app, &mut terminal, &config.ui).await;
    coder::tui::restore_terminal()?;
    result
}
```

Also update the call site in `main.rs` from `run_tui_mode(&config, &cli)` to `run_tui_mode(config, &cli)`.

Add the helper function at the bottom of `main.rs` before the closing:

```rust
/// Save OpenCode API key to config file and reload settings.
fn save_opencode_config(config: &mut coder::config::Settings, key: &str) -> anyhow::Result<()> {
    let config_path = coder::util::path::coder_dir().join("config.toml");

    let opencode_config = coder::config::ProviderConfig {
        provider_type: "opencode".to_string(),
        api_key: Some(key.to_string()),
        base_url: Some("https://opencode.ai/zen/v1".to_string()),
        ..Default::default()
    };

    config.ai.providers.insert("opencode".to_string(), opencode_config);
    config.ai.default_provider = "opencode".to_string();

    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let toml_str = toml::to_string(&config)
        .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;
    std::fs::write(&config_path, toml_str)
        .map_err(|e| anyhow::anyhow!("Failed to write config: {}", e))?;
    tracing::info!("OpenCode API key saved to {:?}", config_path);

    // Reload from the saved file so all env vars are resolved
    *config = coder::config::Settings::load(Some(config_path.to_str().unwrap()))?;
    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd D:\Doder\coder && cargo check`
Expected: Compilation succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs src/config/settings.rs
git commit -m "feat: add startup API key check with provider setup dialog"
```

---

### Task 8: Implement OpenCode OAuth flow

**Files:**
- Create: `src/oauth/opencode.rs`

- [ ] **Step 1: Create OAuth module**

Create `src/oauth/opencode.rs`:

```rust
//! OpenCode OAuth flow
//!
//! Handles the browser-based OAuth flow to get a free OpenCode API key.
//! Flow:
//! 1. Start local HTTP server on a random port
//! 2. Open browser to https://opencode.ai/oauth/authorize
//! 3. User authorizes, callback arrives at local server
//! 4. Exchange code for API key
//! 5. Save key to config

use oauth2::{
    AuthorizationCode, AuthUrl, ClientId, CsrfToken,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
    basic::BasicClient,
    StandardTokenResponse,
    EmptyExtraTokenFields,
    StandardErrorResponse,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tiny_http::{Server, Response};

/// Result of the OAuth flow
#[derive(Debug)]
pub enum OAuthResult {
    Success(String),  // The API key
    Cancelled,
    Error(String),
}

/// Run the OAuth flow to get an OpenCode API key.
///
/// Starts a local HTTP server, opens the browser, waits for the callback.
pub async fn run_oauth_flow() -> OAuthResult {
    // Find a free port
    let listener = match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => return OAuthResult::Error(format!("Failed to bind port: {}", e)),
    };
    let port = listener.local_addr().unwrap().port();
    drop(listener); // release so tiny_http can bind

    let redirect_url = format!("http://127.0.0.1:{}/callback", port);

    let client = BasicClient::new(
        ClientId::new("coder-desktop".to_string()),
        None, // client secret - not needed for PKCE
        AuthUrl::new("https://opencode.ai/oauth/authorize".to_string())
            .expect("Invalid auth URL"),
        Some(TokenUrl::new("https://opencode.ai/oauth/token".to_string())
            .expect("Invalid token URL")),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url.clone()).expect("Invalid redirect URL"));

    // Generate PKCE challenge + authorization URL
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("zen:read".to_string()))
        .add_scope(Scope::new("zen:write".to_string()))
        .url();

    // Start local server
    let server = match Server::http(format!("127.0.0.1:{}", port)) {
        Ok(s) => Arc::new(s),
        Err(e) => return OAuthResult::Error(format!("Failed to start HTTP server: {}", e)),
    };

    let done = Arc::new(AtomicBool::new(false));
    let result = Arc::new(std::sync::Mutex::new(None::<String>));
    let error = Arc::new(std::sync::Mutex::new(None::<String>));

    // Open browser
    if let Err(e) = open::that(auth_url.to_string()) {
        return OAuthResult::Error(format!("Failed to open browser: {}", e));
    }

    // Wait for callback
    let server_clone = server.clone();
    let done_clone = done.clone();
    let result_clone = result.clone();
    let error_clone = error.clone();

    tokio::task::spawn_blocking(move || {
        for request in server_clone.incoming_requests() {
            let url = request.url().to_string();

            // Handle callback
            if url.starts_with("/callback") {
                let query = url.split('?').nth(1).unwrap_or("");
                let params: std::collections::HashMap<String, String> = query
                    .split('&')
                    .filter_map(|pair| {
                        let mut parts = pair.splitn(2, '=');
                        let key = parts.next()?.to_string();
                        let value = parts.next()?.to_string();
                        Some((key, value))
                    })
                    .collect();

                if let Some(code) = params.get("code") {
                    // Exchange code for API key
                    // For now, store the code — in production we'd exchange it server-side
                    let mut res = result_clone.lock().unwrap();
                    *res = Some(code.clone());
                    let _ = request.respond(Response::from_string(
                        "Authorization successful! You can close this window."
                    ));
                } else if let Some(err) = params.get("error") {
                    let mut e = error_clone.lock().unwrap();
                    *e = Some(format!("OAuth error: {}", err));
                    let _ = request.respond(Response::from_string(
                        format!("Authorization failed: {}. You can close this window.", err)
                    ));
                }

                done_clone.store(true, Ordering::SeqCst);
            } else {
                let _ = request.respond(Response::from_string("OpenCode OAuth callback server"));
            }

            if done_clone.load(Ordering::SeqCst) {
                break;
            }
        }
    });

    // Wait for completion with timeout
    let mut waited = 0;
    const TIMEOUT_SECS: u64 = 120;
    while !done.load(Ordering::SeqCst) && waited < TIMEOUT_SECS {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        waited += 1;
    }

    if !done.load(Ordering::SeqCst) {
        return OAuthResult::Error("OAuth timed out. Try again or enter API key manually.".to_string());
    }

    if let Some(err) = error.lock().unwrap().take() {
        return OAuthResult::Error(err);
    }

    if let Some(code) = result.lock().unwrap().take() {
        // In a full implementation, exchange code for API key via POST
        OAuthResult::Success(code)
    } else {
        OAuthResult::Cancelled
    }
}
```

- [ ] **Step 2: Add `tiny_http` and `open` dependencies to Cargo.toml**

```toml
# OAuth
oauth2 = "5.0"
tiny_http = "0.12"
open = "5"
```

- [ ] **Step 3: Add module to `src/oauth/mod.rs`**

```rust
#[cfg(feature = "ai-opencode")]
pub mod opencode;
```

- [ ] **Step 4: Wire OAuth into `main.rs`**

In Task 7's `ProviderSetupResult::OAuth` arm, replace the fallback:

```rust
ProviderSetupResult::OAuth => {
    tracing::info!("User selected OAuth flow");
    match coder::oauth::opencode::run_oauth_flow().await {
        coder::oauth::opencode::OAuthResult::Success(key) => {
            tracing::info!("OAuth successful, got API key");
            // Save key and reload config (same logic as ManualKey)
            save_opencode_config(&mut config, &key)?;
            // Re-create provider with the key
            // Note: config is reloaded inside save_opencode_config
        }
        coder::oauth::opencode::OAuthResult::Cancelled => {
            tracing::info!("OAuth cancelled by user");
            anyhow::bail!("OAuth cancelled.");
        }
        coder::oauth::opencode::OAuthResult::Error(e) => {
            tracing::error!("OAuth error: {}", e);
            anyhow::bail!("OAuth failed: {}", e);
        }
    }
}
```

(Helper function `save_opencode_config` is defined in Task 7 above — no duplicate needed here.)

- [ ] **Step 5: Verify it compiles**

Run: `cd D:\Coder\coder && cargo check`
Expected: Compilation succeeds.

- [ ] **Step 6: Commit**

```bash
git add src/oauth/opencode.rs src/oauth/mod.rs Cargo.toml src/main.rs
git commit -m "feat: add OpenCode OAuth flow for API key acquisition"
```

---

### Task 9: Add `/model` command support for OpenCode model switching

**Files:**
- Modify: `src/agent/mod.rs` or relevant agent command handler

- [ ] **Step 1: Add model listing support**

When the user runs `/model` and the current provider is OpenCode, show the fetched model list:

In the agent's command handler (in `src/agent/mod.rs` or wherever `/model` is processed), add:

```rust
// Check if provider is OpenCode and has model list
if let Some(opencode_provider) = provider.downcast_ref::<opencode::OpenCodeProvider>() {
    let models = opencode_provider.available_models();
    if !models.is_empty() {
        response.push_str("Available OpenCode models:\n");
        for (i, m) in models.iter().enumerate() {
            response.push_str(&format!("  {}. {}\n", i + 1, m));
        }
        response.push_str("\nUse /model <name> to switch.\n");
    }
}
```

- [ ] **Step 2: Commit**

```bash
git commit -m "feat: support model listing for OpenCode provider"
```

---

### Task 10: Integration — wire everything together

**Files:**
- Modify: `src/main.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Ensure `util::path` is accessible from main.rs checks**

The startup dialog code in `main.rs` uses `coder::util::path::coder_dir()`. Verify it's exported via `src/lib.rs`:

```rust
pub mod util;
```

- [ ] **Step 2: Add model fetching in create_provider for opencode**

In `src/ai/mod.rs`, modify the opencode match arm to also fetch models:

```rust
"opencode" => {
    #[cfg(feature = "ai-opencode")]
    {
        let mut provider = opencode::OpenCodeProvider::new(
            config.api_key,
            config.base_url,
            model,
        );
        // Fetch available models (non-blocking, best-effort)
        let provider_clone = provider.api_key.clone();
        let base_url = provider.base_url.clone();
        // Fetch in background task - models can be loaded lazily
        tokio::spawn(async move {
            let temp_provider = opencode::OpenCodeProvider::new(
                provider_clone,
                Some(base_url),
                String::new(),
            );
            match temp_provider.fetch_models().await {
                Ok(models) => {
                    tracing::info!("Fetched {} OpenCode models", models.len());
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch OpenCode models: {}", e);
                }
            }
        });
        Ok(Box::new(provider))
    }
    #[cfg(not(feature = "ai-opencode"))]
    anyhow::bail!("OpenCode provider requires 'ai-opencode' feature")
}
```

Wait — this spawn task approach has a problem because the provider is moved. Let's simplify: make model-fetching lazy / on-demand when the user first requests it or calls /model. For now, just instantiate the provider without fetching:

```rust
"opencode" => {
    #[cfg(feature = "ai-opencode")]
    {
        Ok(Box::new(opencode::OpenCodeProvider::new(
            config.api_key,
            config.base_url,
            model,
        )))
    }
    #[cfg(not(feature = "ai-opencode"))]
    anyhow::bail!("OpenCode provider requires 'ai-opencode' feature")
}
```

Models can be fetched lazily when `/model` is invoked.

- [ ] **Step 3: Full compile check and test run**

Run: `cd D:\Coder\coder && cargo check && cargo test`
Expected: All tests pass, binary compiles.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/ai/mod.rs src/lib.rs
git commit -m "feat: integrate OpenCode provider with startup dialog"
```

---

### Task 11: Final cleanup and docs

**Files:**
- Various

- [ ] **Step 1: Add OpenCode configuration documentation to README**

Add a section to the project's README or a new doc:

```markdown
## OpenCode Free Tier

Coder comes with built-in support for OpenCode's free AI models.

### Quick Start

1. Run `coder` for the first time
2. Select "Use OpenCode Free Tier" from the setup dialog
3. Start coding immediately — no API key required!

### Getting a Free API Key

For higher rate limits and more models:
1. Go to https://opencode.ai/zen
2. Create an account and generate an API key
3. Run `coder` and select "Enter API Key Manually"

### Configuration

```toml
[ai]
default_provider = "opencode"

[ai.providers.opencode]
provider_type = "opencode"
# api_key = "opk_..."  # Optional — leave empty for anonymous free tier
model = "claude-sonnet-4-6"
```
```

- [ ] **Step 2: Final verification**

Run: `cd D:\Coder\coder && cargo build`
Expected: Binary builds successfully.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add OpenCode free tier documentation"
```
