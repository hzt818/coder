//! TUI layout and event handling

use super::app::{App, AppMode, InputSubmode};
use super::chat_panel;
use super::detail_popup;
use super::input;
use super::mention_popup;
use super::status_bar;
use super::theme::AppTheme;
use super::vim::Action;
use crate::agent::r#loop::AgentEvent;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use std::sync::atomic::Ordering;

/// Input action returned from key handlers
enum InputAction {
    None,
    SendMessage(String),
    Interrupt,
}

/// Run the TUI application loop.
pub async fn run_app(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ui_config: &crate::config::UiSettings,
    _shutdown: std::sync::Arc<tokio::sync::Notify>,
) -> anyhow::Result<()> {
    let theme = AppTheme::from_config(ui_config);

    let (stream_tx, mut stream_rx) = tokio::sync::mpsc::channel::<AgentEvent>(256);
    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut last_render = std::time::Instant::now();
    let mut _frame_count = 0u64;

    loop {
        if crate::SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
            tracing::info!("TUI loop: shutdown requested, exiting");
            break;
        }

        let now = std::time::Instant::now();
        let min_frame_interval = if app.mode == AppMode::Streaming {
            std::time::Duration::from_millis(33)
        } else {
            std::time::Duration::from_millis(50)
        };

        if now.duration_since(last_render) >= min_frame_interval {
            if let InputSubmode::Mention { .. } = app.input_submode {
                app.update_mention_filter();
            }
            terminal.draw(|frame| render(frame, app, &theme))?;
            last_render = now;
            _frame_count += 1;
        }

        if app.mode == AppMode::Streaming {
            match stream_rx.try_recv() {
                Ok(event) => {
                    app.handle_event(event);
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {}
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    app.mode = AppMode::Input;
                    app.status = "Stream ended".to_string();
                    app.mark_status_dirty();
                }
            }
        }

        let poll_timeout = if app.mode == AppMode::Streaming {
            std::time::Duration::from_millis(10)
        } else {
            std::time::Duration::from_millis(50)
        };

        if event::poll(poll_timeout)? {
            let event = event::read()?;
            if let Event::Resize(width, height) = event {
                app.status = format!("{}x{}", width, height);
                app.mark_status_dirty();
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
                            let cmd = App::detect_command(&input);
                            if matches!(cmd, super::app::InputCommand::Chat) {
                                let mut stream = app.agent.run_stream(&input).await;
                                let tx = stream_tx.clone();
                                let cancel = cancel_flag.clone();
                                cancel_flag.store(false, std::sync::atomic::Ordering::SeqCst);
                                tokio::spawn(async move {
                                    loop {
                                        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
                                            break;
                                        }
                                        tokio::select! {
                                            event = stream.recv() => {
                                                match event {
                                                    Some(ev) => { if tx.send(ev).await.is_err() { break; } }
                                                    None => break,
                                                }
                                            }
                                        }
                                    }
                                });
                                app.mode = AppMode::Streaming;
                            }
                        }
                        InputAction::Interrupt => {
                            cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                            app.mode = AppMode::Input;
                            app.status = "Interrupted".to_string();
                            app.mark_status_dirty();
                        }
                        InputAction::None => {}
                    }
                }
            }
        }

        if app.mode == AppMode::Streaming {
            app.scroll_offset = 0;
        }
    }
    Ok(())
}

/// Render the full TUI layout
fn render(frame: &mut Frame, app: &App, theme: &AppTheme) {
    let area = frame.area();
    if area.width < 20 || area.height < 6 {
        frame.render_widget(
            Paragraph::new("Terminal too small").style(Style::default().fg(theme.error)),
            area,
        );
        return;
    }
    if app.messages.is_empty() {
        render_welcome(frame, area, app, theme);
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Min(4),
            Constraint::Length(1),
        ])
        .split(area);

    let title = " 🦀 Coder".to_string();
    let title_span = Span::styled(
        &title,
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    );
    let title_bar = if area.width > 60 {
        let detail = format!(
            "  v{}  ·  {} tools  ·  {} msgs",
            env!("CARGO_PKG_VERSION"),
            app.agent.tools().len(),
            app.messages.len()
        );
        Paragraph::new(Line::from(vec![
            title_span,
            Span::styled(detail, Style::default().fg(theme.dim)),
        ]))
        .block(Block::default())
    } else {
        Paragraph::new(Line::from(vec![title_span])).block(Block::default())
    };
    frame.render_widget(title_bar, layout[0]);

    chat_panel::render_chat(
        frame,
        layout[1],
        &app.messages,
        app.scroll_offset,
        &app.working_dir,
        app.git_branch.as_deref(),
        theme,
    );

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
                    "/help · Enter"
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
    if matches!(app.input_submode, InputSubmode::Mention { .. }) {
        mention_popup::render_mention_popup(frame, layout[2], &app.input_submode, theme);
    }
    status_bar::render_status_bar(frame, layout[3], &app.status_text(), theme);

    if app.show_detail {
        let content = if app.detail_content.is_empty() {
            format!(
                "Session: {}\nMessages: {}\nInput: {}\nOutput: {}\n\nEsc to close.",
                app.agent.session().id,
                app.messages.len(),
                app.total_input_tokens,
                app.total_output_tokens
            )
        } else {
            app.detail_content.clone()
        };
        detail_popup::render_detail_popup(frame, area, &content, theme);
    }
}

/// Render the welcome screen
fn render_welcome(frame: &mut Frame, area: Rect, app: &App, theme: &AppTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.welcome_border))
        .title(Line::from(Span::styled(
            " coder ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [content_area, input_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Min(4)])
        .areas(inner);

    let mut lines: Vec<Line> = Vec::new();
    for (art, color) in &[
        ("      ▐▛█ █▜▌", theme.crab_glow),
        ("      ▝▜█▄█▛▘", theme.crab_core),
        ("        ▘▘ ▝▝", theme.crab),
    ] {
        lines.push(
            Line::from(Span::styled(*art, Style::default().fg(*color)))
                .alignment(Alignment::Center),
        );
    }
    lines.push(Line::from("").alignment(Alignment::Center));
    lines.push(
        Line::from(Span::styled("─".repeat(18), Style::default().fg(theme.dim)))
            .alignment(Alignment::Center),
    );
    lines.push(
        Line::from(Span::styled(
            "Welcome back!",
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
    );
    lines.push(
        Line::from(Span::styled("─".repeat(18), Style::default().fg(theme.dim)))
            .alignment(Alignment::Center),
    );
    lines.push(Line::from("").alignment(Alignment::Center));
    lines.push(
        Line::from(Span::styled(
            format!("{} ({})", app.model_name, app.provider_name),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
    );
    lines.push(
        Line::from(Span::styled(
            &app.working_dir,
            Style::default().fg(theme.dim),
        ))
        .alignment(Alignment::Center),
    );
    if let Some(ref branch) = app.git_branch {
        lines.push(
            Line::from(vec![
                Span::styled("branch: ", Style::default().fg(theme.dim)),
                Span::styled(branch, Style::default().fg(theme.success)),
                Span::styled(" ✓", Style::default().fg(theme.success)),
            ])
            .alignment(Alignment::Center),
        );
    }
    lines.push(
        Line::from(Span::styled(
            format!(
                "session: {} | tools: {}",
                app.agent.session().id,
                app.agent.tools().len()
            ),
            Style::default().fg(theme.dim),
        ))
        .alignment(Alignment::Center),
    );
    lines.push(Line::from("").alignment(Alignment::Center));
    lines.push(
        Line::from(Span::styled(
            "Type a message to start coding",
            Style::default().fg(theme.dim),
        ))
        .alignment(Alignment::Center),
    );
    if area.width > 50 {
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
    let extra_top = (content_area.height as usize).saturating_sub(lines.len() + 1) / 2;
    let mut padded = vec![Line::from("").alignment(Alignment::Center); extra_top];
    padded.extend(lines);
    frame.render_widget(
        Paragraph::new(padded).alignment(Alignment::Center),
        content_area,
    );

    let hint = if app.input.starts_with('!') {
        "Enter to execute shell"
    } else if app.input.starts_with('?') {
        "Enter for help"
    } else if app.input.starts_with('/') {
        "/help for all commands"
    } else {
        "Enter to send · @ mention · ! shell · / cmd"
    };
    input::render_input(frame, input_area, app, hint, theme);
    if matches!(app.input_submode, InputSubmode::Mention { .. }) {
        mention_popup::render_mention_popup(frame, input_area, &app.input_submode, theme);
    }
}

/// Handle key events in input mode
fn handle_input_mode(app: &mut App, key: crossterm::event::KeyEvent) -> InputAction {
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
        app.mark_status_dirty();
        return InputAction::None;
    }

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

    match key.code {
        KeyCode::Enter => {
            if key.modifiers.contains(KeyModifiers::ALT) {
                app.insert_char('\n');
            } else {
                let input2 = app.input.trim().to_string();
                if !input2.is_empty() {
                    app.input.clear();
                    app.cursor_pos = 0;
                    return InputAction::SendMessage(input2);
                }
            }
        }
        KeyCode::Char(c) => app.insert_char(c),
        KeyCode::Backspace => app.backspace(),
        KeyCode::Delete => app.delete_char(),
        KeyCode::Left => app.cursor_left(),
        KeyCode::Right => app.cursor_right(),
        KeyCode::Home => app.cursor_home(),
        KeyCode::End => app.cursor_end(),
        KeyCode::Up => app.history_back(),
        KeyCode::Down => app.history_forward(),
        KeyCode::Tab if app.input.contains('@') => {
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
        KeyCode::Esc => {
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
        KeyCode::Up | KeyCode::PageUp => app.scroll_up(),
        KeyCode::Down | KeyCode::PageDown => app.scroll_down(),
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
