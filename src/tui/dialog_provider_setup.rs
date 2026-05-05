//! Provider Setup Dialog
//!
//! TUI dialog shown at startup when no API key is configured.
//! Offers the user:
//! 1. Use OpenCode Free Tier (anonymous)
//! 2. Get API Key via OAuth
//! 3. Enter API Key manually
//! 4. Skip (configure later)

use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{self as term, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame, Terminal,
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
/// Sets up the terminal in raw mode, renders the dialog, and returns the user's choice.
/// Blocks until a selection is made or the user quits.
pub fn run_provider_setup_dialog() -> ProviderSetupResult {
    // Setup terminal
    let _ = term::enable_raw_mode();
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(stdout, EnterAlternateScreen);
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(_) => {
            let _ = term::disable_raw_mode();
            return ProviderSetupResult::Skipped;
        }
    };
    let _ = terminal.clear();

    let mut selected = 0usize;
    let options = ProviderSetupChoice::all();
    let mut manual_key_input = String::new();
    let mut show_manual_input = false;

    let result = loop {
        // Render dialog
        let _ = terminal.draw(|frame| {
            if show_manual_input {
                render_manual_key_dialog(frame, &manual_key_input);
            } else {
                render_provider_dialog(frame, options, selected);
            }
        });

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
                        break if manual_key_input.trim().is_empty() {
                            ProviderSetupResult::Skipped
                        } else {
                            ProviderSetupResult::ManualKey(manual_key_input.trim().to_string())
                        };
                    }
                    match options[selected] {
                        ProviderSetupChoice::FreeTier => break ProviderSetupResult::FreeTier,
                        ProviderSetupChoice::OAuth => break ProviderSetupResult::OAuth,
                        ProviderSetupChoice::Manual => {
                            show_manual_input = true;
                        }
                        ProviderSetupChoice::Skip => break ProviderSetupResult::Skipped,
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
                    manual_key_input.clear();
                }
                KeyCode::Char('q') | KeyCode::Char('Q') if !show_manual_input => {
                    break ProviderSetupResult::Quit;
                }
                _ => {}
            },
            Ok(Event::Resize(_, _)) => {}
            _ => {}
        }
    };

    // Restore terminal
    let _ = term::disable_raw_mode();
    let _ = crossterm::execute!(std::io::stdout(), LeaveAlternateScreen);
    result
}

fn render_provider_dialog(
    frame: &mut Frame,
    options: &[ProviderSetupChoice],
    selected: usize,
) {
    let area = centered_rect(60, 50, frame.area());

    // Clear area
    frame.render_widget(Clear, area);

    // Title border
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

fn render_manual_key_dialog(frame: &mut Frame, input: &str) {
    let area = centered_rect(60, 30, frame.area());
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

    frame.render_widget(instructions, inner);
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
