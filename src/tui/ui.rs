//! TUI layout and event handling

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Paragraph};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use super::app::{App, AppMode, InputSubmode};
use super::chat_panel;
use super::input;
use super::mention_popup;
use super::vim::Action;
use super::detail_popup;
use super::status_bar;
use super::theme::AppTheme;
use crate::agent::r#loop::AgentEvent;

/// Input action returned from key handlers
enum InputAction {
    None,
    SendMessage(String),
    Interrupt,
}

/// Run the TUI application loop
pub async fn run_app(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ui_config: &crate::config::UiSettings,
) -> anyhow::Result<()> {
    let theme = AppTheme::from_config(ui_config);

    // Channel for receiving streaming events
    let (stream_tx, mut stream_rx) = tokio::sync::mpsc::channel::<AgentEvent>(256);

    // Render throttling state
    let mut last_render = std::time::Instant::now();
    let mut _frame_count = 0u64;

    loop {
        // Adaptive frame interval: ~30 FPS during streaming, ~20 FPS when idle
        let now = std::time::Instant::now();
        let min_frame_interval = if app.mode == AppMode::Streaming {
            std::time::Duration::from_millis(33) // ~30 FPS during streaming
        } else {
            std::time::Duration::from_millis(50) // ~20 FPS when idle
        };

        // Only render if enough time has passed since last render
        if now.duration_since(last_render) >= min_frame_interval {
            // Update mention filter before rendering
            if let InputSubmode::Mention { .. } = app.input_submode {
                app.update_mention_filter();
            }

            // Render
            terminal.draw(|frame| {
                render(frame, app, &theme);
            })?;
            last_render = now;
            _frame_count += 1;
        }

        // Process streaming events
        if app.mode == AppMode::Streaming {
            match stream_rx.try_recv() {
                Ok(event) => {
                    app.handle_event(event);
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {}
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    app.mode = AppMode::Input;
                    app.status = "Ready".to_string();
                    app.mark_status_dirty();
                }
            }
        }

        // Handle keyboard/event input with adaptive polling
        let poll_timeout = if app.mode == AppMode::Streaming {
            std::time::Duration::from_millis(10) // faster polling during streaming
        } else {
            std::time::Duration::from_millis(50) // relaxed when idle
        };

        if event::poll(poll_timeout)? {
            let event = event::read()?;

            // Handle resize events for responsive layout
            if let Event::Resize(width, height) = event {
                app.status = format!("{}x{}", width, height);
                app.mark_status_dirty();
                // Clear frame for clean resize
                terminal.clear()?;
                continue;
            }

            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    let action = match app.mode {
                        AppMode::Input => handle_input_mode(app, key),
                        AppMode::Streaming => handle_streaming_mode(app, key),
                        AppMode::Normal | AppMode::Detail => handle_normal_mode(app, key),
                        AppMode::Confirm { .. } => handle_confirm_mode(app, key),
                    };

                    match action {
                        InputAction::SendMessage(input) => {
                            app.send_message();

                            // Check if we should start streaming (normal chat mode)
                            let cmd = App::detect_command(&input);
                            if matches!(cmd, super::app::InputCommand::Chat) {
                                // Start streaming via agent
                                let mut stream = app.agent.run_stream(&input).await;
                                // Forward events from agent stream to our render stream
                                let tx = stream_tx.clone();
                                tokio::spawn(async move {
                                    while let Some(event) = stream.recv().await {
                                        if tx.send(event).await.is_err() {
                                            break;
                                        }
                                    }
                                });

                                app.mode = AppMode::Streaming;
                            }
                        }
                        InputAction::Interrupt => {
                            app.mode = AppMode::Input;
                            app.status = "Interrupted".to_string();
                            app.mark_status_dirty();
                        }
                        InputAction::None => {}
                    }
                }
            }
        }

        // Auto-scroll to bottom when streaming
        if app.mode == AppMode::Streaming {
            app.scroll_offset = 0;
        }
    }
}

/// Render the full TUI layout
fn render(frame: &mut Frame, app: &App, theme: &AppTheme) {
    let area = frame.area();

    if area.width < 20 || area.height < 6 {
        let msg = Paragraph::new("Terminal too small")
            .style(Style::default().fg(theme.error));
        frame.render_widget(msg, area);
        return;
    }

    // Show welcome screen when no messages yet
    if app.messages.is_empty() {
        render_welcome(frame, area, app, theme);
        return;
    }

    // Normal chat layout
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),     // Title bar
            Constraint::Min(3),         // Chat panel
            Constraint::Min(4),         // Input area (sep, prompt, sep, hints)
            Constraint::Length(1),      // Status bar
        ])
        .split(area);

    // ── Responsive title bar ──
    let title = format!(" 🦀 Coder");
    let title_span = Span::styled(
        &title,
        Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
    );

    // Responsive: show more detail when terminal is wide enough
    let title_bar = if area.width > 60 {
        let detail = format!(
            "  v{}  ·  {} tools  ·  {} msgs",
            env!("CARGO_PKG_VERSION"),
            app.agent.tools().len(),
            app.messages.len(),
        );
        ratatui::widgets::Paragraph::new(Line::from(vec![
            title_span,
            Span::styled(detail, Style::default().fg(theme.dim)),
        ]))
        .block(Block::default())
    } else {
        ratatui::widgets::Paragraph::new(Line::from(vec![title_span]))
            .block(Block::default())
    };

    frame.render_widget(title_bar, layout[0]);

    // Chat panel
    chat_panel::render_chat(
        frame,
        layout[1],
        &app.messages,
        app.scroll_offset,
        &app.working_dir,
        theme,
    );

    // Determine mode hint based on input content and submode
    let mode_hint: &str = if matches!(app.input_submode, InputSubmode::Mention { .. }) {
        "Tab/↑↓ Select · Enter Confirm · Esc Cancel"
    } else if app.mode == AppMode::Streaming {
        "Interrupt · Ctrl+C"
    } else {
        match app.mode {
            AppMode::Input => {
                if app.input.starts_with('!') {
                    "Enter to execute shell"
                } else if app.input.starts_with('?') {
                    "Enter for help"
                } else if app.input.starts_with('/') {
                    "/help for all commands · Enter to execute"
                } else {
                    "Ctrl+O for detail"
                }
            }
            AppMode::Detail => "Esc to close detail",
            AppMode::Normal => "i for input · / for commands",
            AppMode::Confirm { .. } => "y/n confirm · esc cancel",
            _ => "",
        }
    };

    input::render_input(frame, layout[2], app, mode_hint, theme);

    // @ mention popup
    if matches!(app.input_submode, InputSubmode::Mention { .. }) {
        mention_popup::render_mention_popup(frame, layout[2], &app.input_submode, theme);
    }

    // Status bar
    status_bar::render_status_bar(frame, layout[3], &app.status_text(), theme);

    // Detail popup
    if app.show_detail {
        let content = if app.detail_content.is_empty() {
            format!(
                "Session: {}\nMessages: {}\nInput Tokens: {}\nOutput Tokens: {}\n\nEsc to close.",
                app.agent.session().id,
                app.messages.len(),
                app.total_input_tokens,
                app.total_output_tokens,
            )
        } else {
            app.detail_content.clone()
        };
        detail_popup::render_detail_popup(frame, area, &content, theme);
    }
}

/// Render the welcome screen (shown when no messages yet)
fn render_welcome(frame: &mut Frame, area: Rect, app: &App, theme: &AppTheme) {
    // ── Outer border with title ──
    let border_style = Style::default().fg(theme.welcome_border);
    let block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Line::from(Span::styled(
            " coder ",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        )))
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // ── Split inner: content area + input area ──
    let [content_area, input_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Min(4)])
        .areas(inner);

    // ── Build centered welcome content ──
    let mut lines: Vec<Line> = Vec::new();

    // Crab ASCII art — blue glow gradient
    let crab_art: [(&str, Color); 3] = [
        ("      ▐▛█ █▜▌", theme.crab_glow),
        ("      ▝▜█▄█▛▘", theme.crab_core),
        ("        ▘▘ ▝▝", theme.crab),
    ];
    for (art, color) in &crab_art {
        lines.push(
            Line::from(Span::styled(*art, Style::default().fg(*color)))
                .alignment(Alignment::Center),
        );
    }

    // Decorative separator above title
    lines.push(Line::from("").alignment(Alignment::Center));
    lines.push(
        Line::from(Span::styled(
            "──────────────────",
            Style::default().fg(theme.dim),
        ))
        .alignment(Alignment::Center),
    );

    // Welcome title
    lines.push(
        Line::from(Span::styled(
            "Welcome back!",
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
    );

    // Decorative separator below title
    lines.push(
        Line::from(Span::styled(
            "──────────────────",
            Style::default().fg(theme.dim),
        ))
        .alignment(Alignment::Center),
    );

    // Separator
    lines.push(Line::from("").alignment(Alignment::Center));

    // Model name (bold)
    let model_display = format!("{} ({})", app.model_name, app.provider_name);
    lines.push(
        Line::from(Span::styled(
            &model_display,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
    );

    // Working directory
    lines.push(
        Line::from(Span::styled(
            &app.working_dir,
            Style::default().fg(theme.dim),
        ))
        .alignment(Alignment::Center),
    );

    // Git branch (if in a git repo)
    let git_branch = std::process::Command::new("git")
        .args(["-C", &app.working_dir, "branch", "--show-current"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        });
    if let Some(ref branch) = git_branch {
        lines.push(
            Line::from(vec![
                Span::styled("branch: ", Style::default().fg(theme.dim)),
                Span::styled(branch, Style::default().fg(theme.success)),
                Span::styled(" ✓", Style::default().fg(theme.success)),
            ])
            .alignment(Alignment::Center),
        );
    }

    // Session info
    let session_info = format!(
        "session: {} | tools: {}",
        app.agent.session().id,
        app.agent.tools().len(),
    );
    lines.push(
        Line::from(Span::styled(&session_info, Style::default().fg(theme.dim)))
            .alignment(Alignment::Center),
    );

    // Bottom hint
    lines.push(Line::from("").alignment(Alignment::Center));
    lines.push(
        Line::from(Span::styled(
            "Type a message to start coding",
            Style::default().fg(theme.dim),
        ))
        .alignment(Alignment::Center),
    );

    // Quick action hints (shown when terminal is wide enough)
    if area.width > 50 {
        lines.push(Line::from("").alignment(Alignment::Center));
        lines.push(
            Line::from(vec![
                Span::styled("/help", Style::default().fg(theme.tool)),
                Span::styled("  ", Style::default().fg(theme.dim)),
                Span::styled("/tools", Style::default().fg(theme.tool)),
                Span::styled("  ", Style::default().fg(theme.dim)),
                Span::styled("@mention", Style::default().fg(theme.tool)),
                Span::styled("  ", Style::default().fg(theme.dim)),
                Span::styled("!command", Style::default().fg(theme.tool)),
            ])
            .alignment(Alignment::Center),
        );
    }

    // Center content vertically: calculate padding
    let content_line_count = lines.len() + 1; // +1 for some breathing room
    let extra_top = (content_area.height as usize).saturating_sub(content_line_count) / 2;

    let mut padded = Vec::new();
    for _ in 0..extra_top {
        padded.push(Line::from("").alignment(Alignment::Center));
    }
    padded.extend(lines);

    let welcome_para = Paragraph::new(padded)
        .alignment(Alignment::Center);
    frame.render_widget(welcome_para, content_area);

    // ── Input area at bottom ──
    let mode_hint: &str = if app.input.starts_with('!') {
        "Enter to execute shell"
    } else if app.input.starts_with('?') {
        "Enter for help"
    } else if app.input.starts_with('/') {
        "/help for all commands"
    } else {
        "Enter to send · @ mention · ! shell · / cmd"
    };
    input::render_input(frame, input_area, app, mode_hint, theme);

    // @ mention popup
    if matches!(app.input_submode, InputSubmode::Mention { .. }) {
        mention_popup::render_mention_popup(frame, input_area, &app.input_submode, theme);
    }
}

/// Handle key events in input mode
fn handle_input_mode(app: &mut App, key: crossterm::event::KeyEvent) -> InputAction {
    // If mention mode is active, handle mention keys first
    if matches!(app.input_submode, InputSubmode::Mention { .. }) {
        match key.code {
            KeyCode::Tab | KeyCode::Down => {
                app.mention_next();
                return InputAction::None;
            }
            KeyCode::Up => {
                app.mention_prev();
                return InputAction::None;
            }
            KeyCode::Enter => {
                app.confirm_mention();
                return InputAction::None;
            }
            KeyCode::Esc => {
                app.input_submode = InputSubmode::Normal;
                app.mark_status_dirty();
                return InputAction::None;
            }
            _ => {}
        }
    }

    // ── Vim normal mode key handling ──
    if !app.vim_state.is_insert() {
        let actions = app.vim_state.handle_normal_key(key);
        for action in actions {
            match action {
                Action::NoAction => {}
                Action::MoveLeft => app.cursor_left(),
                Action::MoveRight => app.cursor_right(),
                Action::MoveUp => app.history_back(),
                Action::MoveDown => app.history_forward(),
                Action::MoveHome => app.cursor_home(),
                Action::MoveEnd => app.cursor_end(),
                Action::DeleteChar => app.delete_char(),
                Action::Backspace => app.backspace(),
                Action::DeleteToEnd => {
                    if app.cursor_pos < app.input.len() {
                        app.input.truncate(app.cursor_pos);
                    }
                }
                Action::DeleteLine => {
                    app.input.clear();
                    app.cursor_pos = 0;
                }
                Action::InsertChar(c) => app.insert_char(c),
                Action::EnterInsertMode => {
                    app.vim_state.enter_insert_mode();
                }
                Action::EnterNormalMode => {
                    app.mode = AppMode::Normal;
                }
                Action::Submit => {
                    let input = app.input.trim().to_string();
                    if !input.is_empty() {
                        app.input.clear();
                        app.cursor_pos = 0;
                        app.mark_status_dirty();
                        return InputAction::SendMessage(input);
                    }
                }
            }
        }
        // Skip regular input handling when in vim normal mode
        app.mark_status_dirty();
        return InputAction::None;
    }

    // Ctrl+ combinations
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => {
                app.input.clear();
                app.cursor_pos = 0;
                return InputAction::None;
            }
            KeyCode::Char('o') => {
                app.toggle_detail();
                if app.show_detail {
                    app.mode = AppMode::Detail;
                    app.mark_status_dirty();
                }
                return InputAction::None;
            }
            KeyCode::Char('w') => {
                app.delete_word_back();
                return InputAction::None;
            }
            KeyCode::Char('u') => {
                app.input.clear();
                app.cursor_pos = 0;
                return InputAction::None;
            }
            KeyCode::Char('a') | KeyCode::Home => {
                app.cursor_home();
                return InputAction::None;
            }
            KeyCode::Char('e') | KeyCode::End => {
                app.cursor_end();
                return InputAction::None;
            }
            KeyCode::Char('b') | KeyCode::Left => {
                app.cursor_left();
                return InputAction::None;
            }
            KeyCode::Char('f') | KeyCode::Right => {
                app.cursor_right();
                return InputAction::None;
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                app.delete_char();
                return InputAction::None;
            }
            _ => {}
        }
    }

    // Normal key handling
    match key.code {
        KeyCode::Enter => {
            if key.modifiers.contains(KeyModifiers::ALT) {
                // Alt+Enter: insert newline instead of sending
                app.insert_char('\n');
            } else {
                // Plain Enter: send the message
                let input = app.input.trim().to_string();
                if !input.is_empty() {
                    app.input.clear();
                    app.cursor_pos = 0;
                    return InputAction::SendMessage(input);
                }
            }
        }
        KeyCode::Char(c) => {
            app.insert_char(c);
        }
        KeyCode::Backspace => {
            app.backspace();
        }
        KeyCode::Delete => {
            app.delete_char();
        }
        KeyCode::Left => {
            app.cursor_left();
        }
        KeyCode::Right => {
            app.cursor_right();
        }
        KeyCode::Home => {
            app.cursor_home();
        }
        KeyCode::End => {
            app.cursor_end();
        }
        KeyCode::Up => {
            // If at first line, navigate history
            app.history_back();
        }
        KeyCode::Down => {
            app.history_forward();
        }
        KeyCode::Tab => {
            // If input has @, activate mention
            if app.input.contains('@') {
                let items = app.mention_candidates("");
                if !items.is_empty() {
                    app.input_submode = InputSubmode::Mention {
                        query: String::new(),
                        items,
                        selected: 0,
                    };
                    app.mark_status_dirty();
                }
            }
        }
        KeyCode::Esc => {
            // Toggle to vim normal mode (stay in AppMode::Input for navigation)
            app.vim_state.enter_normal_mode();
            app.mark_status_dirty();
        }
        _ => {}
    }

    InputAction::None
}

/// Handle key events in streaming mode
fn handle_streaming_mode(app: &mut App, key: crossterm::event::KeyEvent) -> InputAction {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return InputAction::Interrupt;
    }

    if key.code == KeyCode::Char('o') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.toggle_detail();
    }

    InputAction::None
}

/// Handle key events in normal mode
fn handle_normal_mode(app: &mut App, key: crossterm::event::KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Char('i') => {
            app.mode = AppMode::Input;
            app.mark_status_dirty();
        }
        KeyCode::Char('/') => {
            app.input.push('/');
            app.cursor_pos = 1;
            app.mode = AppMode::Input;
            app.mark_status_dirty();
        }
        KeyCode::Char('!') => {
            app.input.push('!');
            app.cursor_pos = 1;
            app.mode = AppMode::Input;
            app.mark_status_dirty();
        }
        KeyCode::Char('?') => {
            app.input.push('?');
            app.cursor_pos = 1;
            app.mode = AppMode::Input;
            app.mark_status_dirty();
        }
        KeyCode::Esc => {
            if app.show_detail {
                app.show_detail = false;
                app.mode = AppMode::Normal;
                app.mark_status_dirty();
            } else {
                let _ = super::restore_terminal();
                std::process::exit(0);
            }
        }
        KeyCode::Up | KeyCode::PageUp => {
            app.scroll_up();
        }
        KeyCode::Down | KeyCode::PageDown => {
            app.scroll_down();
        }
        _ => {}
    }

    if key.code == KeyCode::Char('o') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.toggle_detail();
        if app.show_detail {
            app.mode = AppMode::Detail;
            app.mark_status_dirty();
        }
    }

    InputAction::None
}

/// Handle key events in confirm mode
fn handle_confirm_mode(app: &mut App, key: crossterm::event::KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.mode = AppMode::Input;
            app.status = "Confirmed".to_string();
            app.mark_status_dirty();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Input;
            app.status = "Cancelled".to_string();
            app.mark_status_dirty();
        }
        _ => {}
    }

    InputAction::None
}
