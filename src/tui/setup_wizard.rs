//! First-time setup wizard — guides users through configuring all features.
//!
//! Integrated into the main TUI state machine as `AppMode::Setup`.
//! Also invocable via `/setup` slash command at any time.

use super::theme::AppTheme;
use crate::config::Settings;
use crate::core::features::{self, FeatureStatus};
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};

// ── Step & Wizard State ──────────────────────────────────────────────────

/// Steps in the setup wizard, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum SetupStep {
    Welcome,
    Provider,
    UiTheme,
    UiPreferences,
    ToolSettings,
    SessionSettings,
    FeatureToggles,
    Review,
}

impl SetupStep {
    /// Total number of steps.
    pub fn count() -> usize {
        8
    }

    /// 1-based index of this step.
    pub fn index(&self) -> usize {
        match self {
            SetupStep::Welcome => 1,
            SetupStep::Provider => 2,
            SetupStep::UiTheme => 3,
            SetupStep::UiPreferences => 4,
            SetupStep::ToolSettings => 5,
            SetupStep::SessionSettings => 6,
            SetupStep::FeatureToggles => 7,
            SetupStep::Review => 8,
        }
    }

    /// Human-readable title.
    pub fn title(&self) -> &str {
        match self {
            SetupStep::Welcome => "Welcome",
            SetupStep::Provider => "AI Provider",
            SetupStep::UiTheme => "UI Theme",
            SetupStep::UiPreferences => "UI Preferences",
            SetupStep::ToolSettings => "Tool Settings",
            SetupStep::SessionSettings => "Session",
            SetupStep::FeatureToggles => "Features",
            SetupStep::Review => "Review & Save",
        }
    }

    /// Move to the next step; stays on Review if already there.
    pub fn next(&self) -> SetupStep {
        match self {
            SetupStep::Welcome => SetupStep::Provider,
            SetupStep::Provider => SetupStep::UiTheme,
            SetupStep::UiTheme => SetupStep::UiPreferences,
            SetupStep::UiPreferences => SetupStep::ToolSettings,
            SetupStep::ToolSettings => SetupStep::SessionSettings,
            SetupStep::SessionSettings => SetupStep::FeatureToggles,
            SetupStep::FeatureToggles => SetupStep::Review,
            SetupStep::Review => SetupStep::Review,
        }
    }

    /// Move to the previous step; stays on Welcome if already there.
    pub fn prev(&self) -> SetupStep {
        match self {
            SetupStep::Welcome => SetupStep::Welcome,
            SetupStep::Provider => SetupStep::Welcome,
            SetupStep::UiTheme => SetupStep::Provider,
            SetupStep::UiPreferences => SetupStep::UiTheme,
            SetupStep::ToolSettings => SetupStep::UiPreferences,
            SetupStep::SessionSettings => SetupStep::ToolSettings,
            SetupStep::FeatureToggles => SetupStep::SessionSettings,
            SetupStep::Review => SetupStep::FeatureToggles,
        }
    }
}

/// Which text field is currently being edited (for text-input steps).
#[derive(Debug, Clone, PartialEq)]
pub enum EditingField {
    None,
    ApiKey,
    BaseUrl,
    Model,
    Timeout,
    MaxOutput,
    AutoSave,
    MaxMessages,
}

/// A feature toggle item shown in the wizard.
#[derive(Debug, Clone)]
pub struct FeatureToggle {
    pub name: String,
    pub compile_enabled: bool,
    pub user_wants: bool,
    pub description: String,
}

impl FeatureToggle {
    fn from_status(fs: &FeatureStatus) -> Self {
        Self {
            name: fs.name.clone(),
            compile_enabled: fs.enabled,
            user_wants: fs.enabled, // default: on if compiled in
            description: fs.description.clone(),
        }
    }
}

/// Full wizard state — all collected configuration.
#[derive(Debug, Clone)]
pub struct SetupWizard {
    pub step: SetupStep,

    // ── Provider ──
    pub provider_index: usize,
    pub provider_types: Vec<String>,
    pub api_key: String,
    pub base_url: String,
    pub model: String,

    // ── UI ──
    pub theme_index: usize,
    pub themes: Vec<String>,
    pub show_line_numbers: bool,
    pub syntax_highlight: bool,
    pub mouse_support: bool,

    // ── Tools ──
    pub confirm_before_exec: bool,
    pub timeout_seconds: String,
    pub max_output_mb: String,

    // ── Session ──
    pub auto_save_interval: String,
    pub max_messages_before_compact: String,

    // ── Features ──
    pub features: Vec<FeatureToggle>,
    pub feature_selected: usize,

    // ── Editing state ──
    pub editing: EditingField,
    pub input_buf: String,
    pub cursor: usize,

    // ── Messaging ──
    pub error: Option<String>,
    pub show_exit_confirm: bool,
    pub saved: bool,
}

impl SetupWizard {
    /// Create a new wizard with sensible defaults.
    pub fn new() -> Self {
        let provider_types = vec![
            "opencode".to_string(),
            "openai".to_string(),
            "anthropic".to_string(),
            "google".to_string(),
        ];
        Self {
            step: SetupStep::Welcome,
            provider_index: 0,
            provider_types,
            api_key: String::new(),
            base_url: String::new(),
            model: String::new(),
            theme_index: 0,
            themes: vec![
                "coder-dark".to_string(),
                "high-contrast".to_string(),
                "light".to_string(),
            ],
            show_line_numbers: true,
            syntax_highlight: true,
            mouse_support: true,
            confirm_before_exec: false,
            timeout_seconds: String::from("300"),
            max_output_mb: String::from("1"),
            auto_save_interval: String::from("60"),
            max_messages_before_compact: String::from("100"),
            features: features::get_all_features()
                .iter()
                .map(FeatureToggle::from_status)
                .collect(),
            feature_selected: 0,
            editing: EditingField::None,
            input_buf: String::new(),
            cursor: 0,
            error: None,
            show_exit_confirm: false,
            saved: false,
        }
    }

    /// Populate from existing settings (for `/setup` reconfiguration).
    pub fn from_settings(s: &Settings) -> Self {
        let mut w = Self::new();

        // AI provider defaults
        if let Some(p) = s.ai.providers.get(&s.ai.default_provider) {
            w.provider_index = w
                .provider_types
                .iter()
                .position(|t| t == &s.ai.default_provider)
                .unwrap_or(0);
            w.api_key = p.api_key.clone().unwrap_or_default();
            w.base_url = p.base_url.clone().unwrap_or_default();
            w.model = p.model.clone().unwrap_or_default();
        }

        w.theme_index = w
            .themes
            .iter()
            .position(|t| t == &s.ui.theme)
            .unwrap_or(0);
        w.show_line_numbers = s.ui.show_line_numbers;
        w.syntax_highlight = s.ui.syntax_highlight;
        w.mouse_support = s.ui.mouse_support;
        w.confirm_before_exec = s.tools.confirm_before_exec;
        w.timeout_seconds = s.tools.timeout_seconds.to_string();
        w.max_output_mb = (s.tools.max_output_bytes / 1_000_000).to_string();
        w.auto_save_interval = s.session.auto_save_interval.to_string();
        w.max_messages_before_compact = s.session.max_messages_before_compact.to_string();

        w
    }

    // ── Navigation ──

    pub fn next_step(&mut self) {
        self.error = None;
        self.editing = EditingField::None;
        self.step = self.step.next();
    }

    pub fn prev_step(&mut self) {
        self.error = None;
        self.editing = EditingField::None;
        self.step = self.step.prev();
    }

    // ── Selection movement (for list/checkbox steps) ──

    pub fn select_next(&mut self) {
        match self.step {
            SetupStep::Provider => {
                let len = self.provider_types.len();
                self.provider_index = (self.provider_index + 1).min(len.saturating_sub(1));
                // Pre-fill defaults when provider changes
                self.apply_provider_defaults();
            }
            SetupStep::UiTheme => {
                let len = self.themes.len();
                self.theme_index = (self.theme_index + 1).min(len.saturating_sub(1));
            }
            SetupStep::FeatureToggles => {
                let len = self.features.len();
                self.feature_selected = (self.feature_selected + 1).min(len.saturating_sub(1));
            }
            _ => {}
        }
    }

    pub fn select_prev(&mut self) {
        match self.step {
            SetupStep::Provider => {
                self.provider_index = self.provider_index.saturating_sub(1);
                self.apply_provider_defaults();
            }
            SetupStep::UiTheme => {
                self.theme_index = self.theme_index.saturating_sub(1);
            }
            SetupStep::FeatureToggles => {
                self.feature_selected = self.feature_selected.saturating_sub(1);
            }
            _ => {}
        }
    }

    /// Toggle the currently selected feature on/off.
    pub fn toggle_feature(&mut self) {
        if let Some(f) = self.features.get_mut(self.feature_selected) {
            f.user_wants = !f.user_wants;
        }
    }

    /// Toggle a boolean setting (for UI prefs, tool toggles).
    pub fn toggle_bool(&mut self, field: &str) {
        match field {
            "line_numbers" => self.show_line_numbers = !self.show_line_numbers,
            "syntax_highlight" => self.syntax_highlight = !self.syntax_highlight,
            "mouse_support" => self.mouse_support = !self.mouse_support,
            "confirm_before_exec" => self.confirm_before_exec = !self.confirm_before_exec,
            _ => {}
        }
    }

    fn apply_provider_defaults(&mut self) {
        let name = self
            .provider_types
            .get(self.provider_index)
            .map(|s| s.as_str())
            .unwrap_or("opencode");
        match name {
            "openai" => {
                if self.base_url.is_empty() {
                    self.base_url = "https://api.openai.com/v1".into();
                }
                if self.model.is_empty() {
                    self.model = "gpt-4o".into();
                }
            }
            "anthropic" => {
                if self.base_url.is_empty() {
                    self.base_url = "https://api.anthropic.com".into();
                }
                if self.model.is_empty() {
                    self.model = "claude-sonnet-4-6".into();
                }
            }
            "google" => {
                if self.base_url.is_empty() {
                    self.base_url =
                        "https://generativelanguage.googleapis.com/v1beta".into();
                }
                if self.model.is_empty() {
                    self.model = "gemini-2.5-flash".into();
                }
            }
            "opencode" | _ => {
                if self.base_url.is_empty() {
                    self.base_url = "https://opencode.ai/zen/v1".into();
                }
                if self.model.is_empty() {
                    self.model = "claude-sonnet-4-6".into();
                }
            }
        }
    }

    // ── Text editing ──

    pub fn start_editing(&mut self, field: EditingField) {
        self.input_buf = match &field {
            EditingField::ApiKey => self.api_key.clone(),
            EditingField::BaseUrl => self.base_url.clone(),
            EditingField::Model => self.model.clone(),
            EditingField::Timeout => self.timeout_seconds.clone(),
            EditingField::MaxOutput => self.max_output_mb.clone(),
            EditingField::AutoSave => self.auto_save_interval.clone(),
            EditingField::MaxMessages => self.max_messages_before_compact.clone(),
            EditingField::None => String::new(),
        };
        self.cursor = self.input_buf.len();
        self.editing = field;
        self.error = None;
    }

    pub fn commit_editing(&mut self) {
        let value = std::mem::take(&mut self.input_buf);
        match &self.editing {
            EditingField::ApiKey => self.api_key = value,
            EditingField::BaseUrl => self.base_url = value,
            EditingField::Model => self.model = value,
            EditingField::Timeout => self.timeout_seconds = value,
            EditingField::MaxOutput => self.max_output_mb = value,
            EditingField::AutoSave => self.auto_save_interval = value,
            EditingField::MaxMessages => self.max_messages_before_compact = value,
            EditingField::None => {}
        }
        self.editing = EditingField::None;
    }

    pub fn cancel_editing(&mut self) {
        self.input_buf.clear();
        self.cursor = 0;
        self.editing = EditingField::None;
        self.error = None;
    }

    pub fn editing_insert(&mut self, c: char) {
        self.input_buf.insert(self.cursor, c);
        self.cursor += 1;
    }

    pub fn editing_backspace(&mut self) {
        if self.cursor > 0 {
            self.input_buf.remove(self.cursor - 1);
            self.cursor -= 1;
        }
    }

    pub fn editing_delete(&mut self) {
        if self.cursor < self.input_buf.len() {
            self.input_buf.remove(self.cursor);
        }
    }

    pub fn editing_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn editing_right(&mut self) {
        if self.cursor < self.input_buf.len() {
            self.cursor += 1;
        }
    }

    pub fn editing_home(&mut self) {
        self.cursor = 0;
    }

    pub fn editing_end(&mut self) {
        self.cursor = self.input_buf.len();
    }

    // ── Validation ──

    /// Validate current step before advancing. Returns Some(error) on failure.
    pub fn validate_step(&self) -> Option<String> {
        match self.step {
            SetupStep::Provider => {
                if self.api_key.trim().is_empty()
                    && self.provider_types
                        .get(self.provider_index)
                        .map(|s| s.as_str())
                        != Some("opencode")
                {
                    return Some("API key is required for this provider.".into());
                }
                if !self.base_url.trim().is_empty() {
                    if let Err(e) = url::Url::parse(self.base_url.trim()) {
                        return Some(format!("Invalid base URL: {e}"));
                    }
                }
                None
            }
            SetupStep::ToolSettings => {
                if let Err(e) = self.timeout_seconds.trim().parse::<u64>() {
                    return Some(format!("Timeout must be a number: {e}"));
                }
                let t: u64 = self.timeout_seconds.trim().parse().unwrap_or(0);
                if t < 10 || t > 600 {
                    return Some("Timeout must be 10–600 seconds.".into());
                }
                if let Err(e) = self.max_output_mb.trim().parse::<u64>() {
                    return Some(format!("Max output must be a number: {e}"));
                }
                None
            }
            SetupStep::SessionSettings => {
                if let Err(e) = self.auto_save_interval.trim().parse::<u64>() {
                    return Some(format!("Auto-save interval must be a number: {e}"));
                }
                let a: u64 = self.auto_save_interval.trim().parse().unwrap_or(0);
                if a < 5 || a > 3600 {
                    return Some("Auto-save must be 5–3600 seconds.".into());
                }
                if let Err(e) = self.max_messages_before_compact.trim().parse::<usize>() {
                    return Some(format!("Max messages must be a number: {e}"));
                }
                None
            }
            _ => None,
        }
    }

    // ── Config generation ──

    /// Serialise the wizard state as a TOML config string.
    pub fn to_config_toml(&self) -> String {
        let provider_name = self
            .provider_types
            .get(self.provider_index)
            .cloned()
            .unwrap_or_else(|| "opencode".to_string());
        let themes = [
            ("coder-dark", "coder-dark"),
            ("high-contrast", "high-contrast"),
            ("light", "light"),
        ];
        let theme_name = themes
            .get(self.theme_index)
            .map(|(_, v)| *v)
            .unwrap_or("coder-dark");

        let to = self.timeout_seconds.trim().parse::<u64>().unwrap_or(300);
        let mo = self.max_output_mb.trim().parse::<u64>().unwrap_or(1) * 1_000_000;
        let as_ = self.auto_save_interval.trim().parse::<u64>().unwrap_or(60);
        let mc = self
            .max_messages_before_compact
            .trim()
            .parse::<usize>()
            .unwrap_or(100);

        let mut buf = String::new();
        buf.push_str("# Coder configuration — generated by setup wizard\n\n");

        // [ai]
        buf.push_str("[ai]\n");
        buf.push_str(&format!("default_provider = \"{provider_name}\"\n\n"));
        buf.push_str(&format!("[ai.providers.{provider_name}]\n"));
        buf.push_str(&format!("provider_type = \"{provider_name}\"\n"));
        match provider_name.as_str() {
            "opencode" if self.api_key.trim().is_empty() => {
                // anonymous — omit api_key
            }
            _ => {
                buf.push_str(&format!("api_key = \"{}\"\n", self.api_key.trim()));
            }
        }
        if !self.base_url.trim().is_empty() {
            buf.push_str(&format!("base_url = \"{}\"\n", self.base_url.trim()));
        }
        if !self.model.trim().is_empty() {
            buf.push_str(&format!("model = \"{}\"\n", self.model.trim()));
        }
        buf.push('\n');

        // [ui]
        buf.push_str("[ui]\n");
        buf.push_str(&format!("theme = \"{theme_name}\"\n"));
        buf.push_str(&format!("show_line_numbers = {}\n", self.show_line_numbers));
        buf.push_str(&format!("syntax_highlight = {}\n", self.syntax_highlight));
        buf.push_str(&format!("mouse_support = {}\n", self.mouse_support));
        buf.push('\n');

        // [tools]
        buf.push_str("[tools]\n");
        buf.push_str(&format!(
            "confirm_before_exec = {}\n",
            self.confirm_before_exec
        ));
        buf.push_str(&format!("timeout_seconds = {to}\n"));
        buf.push_str(&format!("max_output_bytes = {mo}\n"));
        buf.push('\n');

        // [session]
        buf.push_str("[session]\n");
        buf.push_str(&format!("auto_save_interval = {as_}\n"));
        buf.push_str(&format!("max_messages_before_compact = {mc}\n"));

        buf
    }

    /// Write config to `~/.coder/config.toml`, creating a backup if one exists.
    pub fn save_config(&self) -> Result<(), String> {
        use std::io::Write;

        let config_path = crate::util::path::coder_dir().join("config.toml");

        // Create parent directory
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {e}"))?;
        }

        // Backup existing config
        if config_path.exists() {
            let bak = crate::util::path::coder_dir().join("config.toml.bak");
            std::fs::copy(&config_path, &bak)
                .map_err(|e| format!("Failed to backup config: {e}"))?;
        }

        // Write atomically: write to .tmp then rename
        let tmp = crate::util::path::coder_dir().join("config.toml.tmp");
        {
            let mut f =
                std::fs::File::create(&tmp).map_err(|e| format!("Failed to create config: {e}"))?;
            f.write_all(self.to_config_toml().as_bytes())
                .map_err(|e| format!("Failed to write config: {e}"))?;
            f.flush().map_err(|e| format!("Failed to flush config: {e}"))?;
        }
        std::fs::rename(&tmp, &config_path)
            .map_err(|e| format!("Failed to finalise config: {e}"))?;

        Ok(())
    }
}

impl Default for SetupWizard {
    fn default() -> Self {
        Self::new()
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────

/// Compute a centered rectangle for the wizard popup.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    let x = r.x + (r.width.saturating_sub(popup_width)) / 2;
    let y = r.y + (r.height.saturating_sub(popup_height)) / 2;
    Rect::new(x, y, popup_width, popup_height)
}

/// Render the setup wizard overlay.
pub fn render_setup_wizard(frame: &mut Frame, area: Rect, wizard: &SetupWizard, theme: &AppTheme) {
    // Render a dimming backdrop
    let backdrop = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(backdrop, area);

    let popup = centered_rect(76, 82, area);

    // Ensure minimum dimensions
    if popup.width < 30 || popup.height < 10 {
        frame.render_widget(
            Paragraph::new("Terminal too small for setup wizard")
                .style(Style::default().fg(theme.error)),
            area,
        );
        return;
    }

    // Clear behind the popup
    frame.render_widget(ratatui::widgets::Clear, popup);

    // Step indicator in title
    let title = format!(
        " Setup Wizard — Step {}/{}: {} ",
        wizard.step.index(),
        SetupStep::count(),
        wizard.step.title(),
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .title(title)
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        );
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Split inner area: content + footer hints
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(2)])
        .split(inner);

    // Render step-specific content
    match wizard.step {
        SetupStep::Welcome => render_welcome_step(frame, layout[0], wizard, theme),
        SetupStep::Provider => render_provider_step(frame, layout[0], wizard, theme),
        SetupStep::UiTheme => render_theme_step(frame, layout[0], wizard, theme),
        SetupStep::UiPreferences => render_ui_prefs_step(frame, layout[0], wizard, theme),
        SetupStep::ToolSettings => render_tool_step(frame, layout[0], wizard, theme),
        SetupStep::SessionSettings => render_session_step(frame, layout[0], wizard, theme),
        SetupStep::FeatureToggles => render_features_step(frame, layout[0], wizard, theme),
        SetupStep::Review => render_review_step(frame, layout[0], wizard, theme),
    }

    // Footer: navigation hints
    let hint = if wizard.show_exit_confirm {
        Span::styled(
            "Discard changes?  y / n",
            Style::default().fg(theme.warning),
        )
    } else if wizard.editing != EditingField::None {
        Span::styled(
            "Enter to confirm · Esc to cancel · ← → to move cursor",
            Style::default().fg(theme.dim),
        )
    } else if wizard.step == SetupStep::Welcome {
        Span::styled("Enter to begin · Esc to skip", Style::default().fg(theme.dim))
    } else if wizard.step == SetupStep::Review {
        Span::styled(
            "Enter to save & finish · Esc to go back",
            Style::default().fg(theme.dim),
        )
    } else {
        Span::styled(
            "Enter to continue · Esc to go back · ↑↓ to select",
            Style::default().fg(theme.dim),
        )
    };

    // Error message if present
    let footer = if let Some(ref err) = wizard.error {
        Line::from(vec![
            Span::styled(format!("⚠ {err}"), Style::default().fg(theme.error)),
            Span::raw("  |  "),
            hint,
        ])
    } else {
        Line::from(hint)
    };

    frame.render_widget(
        Paragraph::new(footer).alignment(Alignment::Center),
        layout[1],
    );
}

// ── Step Renderers ────────────────────────────────────────────────────────

fn render_welcome_step(frame: &mut Frame, area: Rect, _w: &SetupWizard, theme: &AppTheme) {
    let lines = vec![
        Line::from(Span::styled(
            "🦀 Welcome to Coder!",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("This wizard will help you configure:"),
        Line::from(""),
        Line::from(Span::styled(
            "  1. AI Provider  — choose your LLM backend",
            Style::default().fg(theme.fg),
        )),
        Line::from(Span::styled(
            "  2. UI Theme     — pick colors that suit you",
            Style::default().fg(theme.fg),
        )),
        Line::from(Span::styled(
            "  3. UI Preferences — toggles for line numbers, highlighting, mouse",
            Style::default().fg(theme.fg),
        )),
        Line::from(Span::styled(
            "  4. Tool Settings — execution timeouts, confirmations",
            Style::default().fg(theme.fg),
        )),
        Line::from(Span::styled(
            "  5. Session      — auto-save, compaction thresholds",
            Style::default().fg(theme.fg),
        )),
        Line::from(Span::styled(
            "  6. Features     — enable/disable optional capabilities",
            Style::default().fg(theme.fg),
        )),
        Line::from(Span::styled(
            "  7. Review       — confirm and save",
            Style::default().fg(theme.fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "You can run /setup at any time to reconfigure.",
            Style::default().fg(theme.dim),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to start, or Esc to skip and use defaults.",
            Style::default().fg(theme.tool),
        )),
    ];

    let extra_top = (area.height as usize)
        .saturating_sub(lines.len() + 2)
        / 2;
    let mut padded = vec![Line::from(""); extra_top];
    padded.extend(lines);

    frame.render_widget(Paragraph::new(padded).alignment(Alignment::Center), area);
}

fn render_provider_step(frame: &mut Frame, area: Rect, w: &SetupWizard, theme: &AppTheme) {
    // If editing a field, show text input
    if w.editing != EditingField::None {
        render_editing_dialog(frame, area, w, theme);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // provider type selector
            Constraint::Length(1),  // spacer
            Constraint::Length(5),  // details
        ])
        .split(area);

    // Provider type selector
    let items: Vec<ListItem> = w
        .provider_types
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let prefix = if i == w.provider_index { "▶ " } else { "  " };
            let desc = provider_desc(name);
            let label = format!("{prefix}{name}  — {desc}");
            if i == w.provider_index {
                ListItem::new(label).style(
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(label).style(Style::default().fg(theme.fg))
            }
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::NONE)
        .title(" Provider Type ");
    frame.render_widget(List::new(items).block(list_block), chunks[0]);

    // Current provider details
    let provider_name = w
        .provider_types
        .get(w.provider_index)
        .map(|s| s.as_str())
        .unwrap_or("opencode");

    // Mask API key for display
    let key_display = if w.api_key.is_empty() {
        "(not set — press e to edit)".to_string()
    } else {
        mask_secret(&w.api_key)
    };

    let details = vec![
        Line::from(Span::styled(
            format!("Provider: {provider_name}"),
            Style::default().fg(theme.fg),
        )),
        Line::from(vec![
            Span::styled("API Key:  ", Style::default().fg(theme.dim)),
            Span::styled(&key_display, Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("Base URL: ", Style::default().fg(theme.dim)),
            Span::styled(
                if w.base_url.is_empty() {
                    "(default)"
                } else {
                    &w.base_url
                },
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("Model:    ", Style::default().fg(theme.dim)),
            Span::styled(
                if w.model.is_empty() {
                    "(default)"
                } else {
                    &w.model
                },
                Style::default().fg(theme.fg),
            ),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(details).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.dim)),
        ),
        chunks[2],
    );

    // Hint for editing
    frame.render_widget(
        Paragraph::new("e — edit API key  |  u — edit URL  |  m — edit model")
            .style(Style::default().fg(theme.dim))
            .alignment(Alignment::Center),
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area)[0],
    );
}

fn render_theme_step(frame: &mut Frame, area: Rect, w: &SetupWizard, theme: &AppTheme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(1)])
        .split(area);

    let items: Vec<ListItem> = w
        .themes
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let prefix = if i == w.theme_index { "▶ " } else { "  " };
            let desc = theme_desc(name);
            if i == w.theme_index {
                ListItem::new(format!("{prefix}{name}  — {desc}")).style(
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(format!("{prefix}{name}  — {desc}"))
                    .style(Style::default().fg(theme.fg))
            }
        })
        .collect();

    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::NONE).title(" Theme ")),
        chunks[0],
    );

    // Mini preview
    let preview = theme_preview(w.themes.get(w.theme_index).map(|s| s.as_str()));
    frame.render_widget(
        Paragraph::new(preview).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.dim))
                .title(" Preview "),
        ),
        chunks[1],
    );
}

fn render_ui_prefs_step(frame: &mut Frame, area: Rect, w: &SetupWizard, theme: &AppTheme) {
    let items = [
        ("Show line numbers in code blocks", w.show_line_numbers, '1'),
        ("Syntax highlighting", w.syntax_highlight, '2'),
        ("Mouse support", w.mouse_support, '3'),
    ];

    let lines: Vec<Line> = items
        .iter()
        .map(|(desc, on, key)| {
            let check = if *on { "✔" } else { "✘" };
            let color = if *on { theme.success } else { theme.dim };
            Line::from(vec![
                Span::styled(format!(" [{check}] "), Style::default().fg(color)),
                Span::styled(*desc, Style::default().fg(theme.fg)),
                Span::styled(format!("  (press {key})"), Style::default().fg(theme.dim)),
            ])
        })
        .collect();

    let extra = (area.height as usize).saturating_sub(lines.len() + 2) / 2;
    let mut padded = vec![Line::from(""); extra];
    padded.extend(lines);

    frame.render_widget(
        Paragraph::new(padded).alignment(Alignment::Left),
        area,
    );
}

fn render_tool_step(frame: &mut Frame, area: Rect, w: &SetupWizard, theme: &AppTheme) {
    if w.editing != EditingField::None {
        render_editing_dialog(frame, area, w, theme);
        return;
    }

    let items = vec![
        Line::from(vec![
            Span::styled(
                format!(
                    " [{check}] Confirm before executing commands  (press 1)",
                    check = if w.confirm_before_exec { "✔" } else { "✘" }
                ),
                Style::default().fg(if w.confirm_before_exec {
                    theme.success
                } else {
                    theme.dim
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Timeout (seconds): ", Style::default().fg(theme.fg)),
            Span::styled(
                &w.timeout_seconds,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (press e to edit)", Style::default().fg(theme.dim)),
        ]),
        Line::from(vec![
            Span::styled(" Max output (MB):    ", Style::default().fg(theme.fg)),
            Span::styled(
                &w.max_output_mb,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (press m to edit)", Style::default().fg(theme.dim)),
        ]),
    ];

    let extra = (area.height as usize).saturating_sub(items.len() + 2) / 2;
    let mut padded = vec![Line::from(""); extra];
    padded.extend(items);

    frame.render_widget(Paragraph::new(padded), area);
}

fn render_session_step(frame: &mut Frame, area: Rect, w: &SetupWizard, theme: &AppTheme) {
    if w.editing != EditingField::None {
        render_editing_dialog(frame, area, w, theme);
        return;
    }

    let items = vec![
        Line::from(vec![
            Span::styled(
                " Auto-save interval (seconds): ",
                Style::default().fg(theme.fg),
            ),
            Span::styled(
                &w.auto_save_interval,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (press e to edit)", Style::default().fg(theme.dim)),
        ]),
        Line::from(vec![
            Span::styled(
                " Max messages before compaction: ",
                Style::default().fg(theme.fg),
            ),
            Span::styled(
                &w.max_messages_before_compact,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  (press m to edit)", Style::default().fg(theme.dim)),
        ]),
    ];

    let extra = (area.height as usize).saturating_sub(items.len() + 2) / 2;
    let mut padded = vec![Line::from(""); extra];
    padded.extend(items);

    frame.render_widget(Paragraph::new(padded), area);
}

fn render_features_step(frame: &mut Frame, area: Rect, w: &SetupWizard, theme: &AppTheme) {
    // Show features in a scrollable list
    let items: Vec<ListItem> = w
        .features
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let prefix = if i == w.feature_selected { "▶" } else { " " };
            let check = if f.user_wants { "✔" } else { "✘" };
            let color = if f.compile_enabled {
                theme.success
            } else {
                theme.dim
            };
            let compile_mark = if f.compile_enabled { "" } else { " (not compiled)" };
            ListItem::new(format!(
                "{prefix} [{check}] {}{compile_mark} — {}",
                f.name, f.description
            ))
            .style(if i == w.feature_selected {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            })
        })
        .collect();

    let hint = Paragraph::new("Space to toggle · ↑↓ to navigate")
        .style(Style::default().fg(theme.dim))
        .alignment(Alignment::Center);

    let list_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::NONE)
                .title(" Feature Toggles "),
        ),
        list_area[0],
    );
    frame.render_widget(hint, list_area[1]);
}

fn render_review_step(frame: &mut Frame, area: Rect, w: &SetupWizard, theme: &AppTheme) {
    let provider_name = w
        .provider_types
        .get(w.provider_index)
        .cloned()
        .unwrap_or_else(|| "opencode".to_string());
    let theme_name = w
        .themes
        .get(w.theme_index)
        .cloned()
        .unwrap_or_else(|| "coder-dark".to_string());
    let enabled_features: Vec<&str> = w
        .features
        .iter()
        .filter(|f| f.user_wants)
        .map(|f| f.name.as_str())
        .collect();

    let masked_key = mask_secret(&w.api_key);
    let summary = vec![
        Line::from(Span::styled(
            "── Configuration Summary ──",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Provider:  ", Style::default().fg(theme.dim)),
            Span::styled(&provider_name, Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("API Key:   ", Style::default().fg(theme.dim)),
            Span::styled(&masked_key, Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("Model:     ", Style::default().fg(theme.dim)),
            Span::styled(&w.model, Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("Theme:     ", Style::default().fg(theme.dim)),
            Span::styled(theme_name, Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("Line nums: ", Style::default().fg(theme.dim)),
            Span::styled(
                if w.show_line_numbers { "on" } else { "off" },
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("Timeout:   ", Style::default().fg(theme.dim)),
            Span::styled(
                format!("{}s", w.timeout_seconds),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("Features:  ", Style::default().fg(theme.dim)),
            Span::styled(
                format!("{} enabled", enabled_features.len()),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to save and start using Coder!",
            Style::default().fg(theme.tool),
        )),
    ];

    frame.render_widget(
        Paragraph::new(summary).scroll((0, 0)),
        area,
    );
}

// ── Editing dialog (shared across provider/tool/session steps) ────

fn render_editing_dialog(frame: &mut Frame, area: Rect, w: &SetupWizard, theme: &AppTheme) {
    let field_label = match &w.editing {
        EditingField::ApiKey => "API Key",
        EditingField::BaseUrl => "Base URL",
        EditingField::Model => "Model Name",
        EditingField::Timeout => "Timeout (10–600 seconds)",
        EditingField::MaxOutput => "Max Output (MB)",
        EditingField::AutoSave => "Auto-Save Interval (5–3600 seconds)",
        EditingField::MaxMessages => "Max Messages Before Compaction",
        EditingField::None => "",
    };

    // Mask API key display
    let display_value = if matches!(w.editing, EditingField::ApiKey) {
        "*".repeat(w.input_buf.len())
    } else {
        w.input_buf.clone()
    };

    // Show cursor
    let display = if display_value.is_empty() {
        format!("▌")
    } else {
        let c = w.cursor.min(display_value.len());
        format!(
            "{}▌{}",
            &display_value[..c],
            &display_value[c..],
        )
    };

    let lines = vec![
        Line::from(Span::styled(
            format!("Editing: {field_label}"),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            display,
            Style::default().fg(theme.fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter → confirm | Esc → cancel | ← → Home End | Backspace",
            Style::default().fg(theme.dim),
        )),
    ];

    let extra = (area.height as usize).saturating_sub(lines.len() + 2) / 2;
    let mut padded = vec![Line::from(""); extra];
    padded.extend(lines);

    frame.render_widget(
        Paragraph::new(padded).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent)),
        ),
        area,
    );
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn provider_desc(name: &str) -> &str {
    match name {
        "opencode" => "free tier, anonymous",
        "openai" => "GPT-4o, o3, etc.",
        "anthropic" => "Claude Sonnet/Opus",
        "google" => "Gemini 2.5 Flash/Pro",
        _ => "",
    }
}

fn theme_desc(name: &str) -> &str {
    match name {
        "coder-dark" => "dark background, green accents",
        "high-contrast" => "maximum readability",
        "light" => "light background, dark text",
        _ => "",
    }
}

fn theme_preview(name: Option<&str>) -> Vec<Line<'static>> {
    match name {
        Some("light") => vec![
            Line::from("  user message          "),
            Line::from("  assistant response     "),
            Line::from("  ── code block ──      "),
            Line::from("  fn main() {            "),
            Line::from("      println!(\"hi\");  "),
            Line::from("  }                      "),
        ],
        Some("high-contrast") => vec![
            Line::from("  user message          "),
            Line::from("  assistant response     "),
            Line::from("  ── code block ──      "),
            Line::from("  fn main() {            "),
            Line::from("      println!(\"hi\");  "),
            Line::from("  }                      "),
        ],
        _ => vec![
            Line::from("  user message          "),
            Line::from("  assistant response     "),
            Line::from("  ── code block ──      "),
            Line::from("  fn main() {            "),
            Line::from("      println!(\"hi\");  "),
            Line::from("  }                      "),
        ],
    }
}

/// Show last 4 characters of a secret, masked otherwise.
pub fn mask_secret(s: &str) -> String {
    if s.is_empty() {
        return "(empty)".to_string();
    }
    if s.len() <= 4 {
        return "****".to_string();
    }
    format!("****{}", &s[s.len() - 4..])
}
