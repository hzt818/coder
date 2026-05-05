//! Input widget — bottom command bar
//!
//! Renders a 4-line bottom bar anchored to the bottom of the screen:
//!
//! ──────────────────────────── context ──     ← separator with title
//! ❯ user text█                                  ← input prompt
//! ────────────────────────────────────────     ← separator
//!  @tag1 @tag2 · shortcut · status             ← hint/status line

use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};
use super::theme::AppTheme;
use super::app::App;

/// Prepare input text for display, replacing newlines with visual indicators.
/// Returns (display_text, display_cursor_position)
fn prepare_input_display(input: &str, cursor_pos: usize) -> (String, usize) {
    let before_cursor = &input[..cursor_pos];
    let after_cursor = &input[cursor_pos..];

    let display_before = before_cursor.replace('\n', "⏎");
    let display_after = after_cursor.replace('\n', "⏎");
    let display_cursor = display_before.len();
    let display_input = format!("{}{}", display_before, display_after);

    (display_input, display_cursor)
}

/// Render the bottom command bar (4 lines)
pub fn render_input(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    mode_hint: &str,
    theme: &AppTheme,
) {
    if area.height < 4 {
        return;
    }

    let [sep1_line, input_line, sep2_line, hint_line] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(area);

    let width = usize::from(area.width);

    // ── Line 1: Separator with right-aligned context title ──
    let ctx_title = format!(" {} ", context_title(app));
    let dashes = "─".repeat(width.saturating_sub(ctx_title.len()));
    let sep1 = Line::from(Span::styled(
        format!("{}{}", dashes, ctx_title),
        Style::default().fg(theme.dim),
    ));
    frame.render_widget(Paragraph::new(sep1), sep1_line);

    // ── Line 2: Input prompt ──
    let prompt_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled("❯ ", prompt_style));

    let input = &app.input;
    let cursor_pos = app.cursor_pos;

    // Prepare display version with newlines shown as ⏎
    let (display_input, display_cursor) = prepare_input_display(input, cursor_pos);

    if display_input.is_empty() {
        spans.push(Span::styled(
            "Type a message or /help for commands",
            Style::default()
                .fg(theme.dim)
                .add_modifier(Modifier::ITALIC),
        ));
        spans.push(Span::styled(
            " █",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::SLOW_BLINK),
        ));
    } else {
        let before = &display_input[..display_cursor];
        spans.push(Span::styled(before.to_string(), Style::default().fg(theme.fg)));

        if let Some(c) = display_input[display_cursor..].chars().next() {
            spans.push(Span::styled(
                c.to_string(),
                Style::default().fg(theme.selection_fg).bg(theme.selection_bg),
            ));
            let c_len = c.len_utf8();
            let after = &display_input[display_cursor + c_len..];
            spans.push(Span::styled(after.to_string(), Style::default().fg(theme.fg)));
        } else {
            spans.push(Span::styled(
                "█",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::SLOW_BLINK),
            ));
        }
    }

    let input_para = Paragraph::new(Line::from(spans))
        .wrap(Wrap { trim: false });
    frame.render_widget(input_para, input_line);

    // ── Line 3: Plain separator ──
    let sep2 = Line::from(Span::styled(
        "─".repeat(width),
        Style::default().fg(theme.dim),
    ));
    frame.render_widget(Paragraph::new(sep2), sep2_line);

    // ── Line 4: Hint/status line with tags ──
    let hint_line_content = build_hint_line(app, mode_hint, theme, width);
    frame.render_widget(Paragraph::new(hint_line_content), hint_line);
}

/// Derive a short context title from the working directory
fn context_title(app: &App) -> String {
    // Use the last component of working_dir, or fallback to "coder"
    let dir = app.working_dir.trim_end_matches(&['/', '\\'][..]);
    dir.rsplit_once(&['/', '\\'][..])
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| "coder".to_string())
}

/// Build the hint line content with styled tags
fn build_hint_line<'a>(app: &App, mode_text: &'a str, theme: &AppTheme, width: usize) -> Line<'a> {
    let mut spans: Vec<Span<'a>> = Vec::new();

    // Prefix spacing
    spans.push(Span::styled("  ", Style::default()));

    // Dynamic hint based on mode
    match app.mode {
        super::app::AppMode::Input => {
            if app.input.is_empty() {
                // Show tag-style hints only when input is empty
                spans.push(Span::styled(
                    "@mention",
                    Style::default().fg(theme.user),
                ));
                spans.push(Span::styled("  ", Style::default()));

                spans.push(Span::styled(
                    "!shell",
                    Style::default().fg(theme.tool),
                ));
                spans.push(Span::styled("  ", Style::default()));

                spans.push(Span::styled(
                    "?help",
                    Style::default().fg(theme.accent),
                ));
                spans.push(Span::styled("  ", Style::default()));

                spans.push(Span::styled(
                    "/cmd",
                    Style::default().fg(theme.warning),
                ));
                spans.push(Span::styled("  ·  ", Style::default().fg(theme.dim)));
                spans.push(Span::styled(
                    "ctrl+o detail",
                    Style::default().fg(theme.dim),
                ));
            } else {
                // Show mode-specific hint
                spans.push(Span::styled(mode_text, Style::default().fg(theme.dim)));
            }
        }
        super::app::AppMode::Streaming => {
            spans.push(Span::styled(
                "◉ ",
                Style::default().fg(theme.success).add_modifier(Modifier::SLOW_BLINK),
            ));
            spans.push(Span::styled(
                "AI responding",
                Style::default().fg(theme.success),
            ));
            spans.push(Span::styled(" · ", Style::default().fg(theme.dim)));
            spans.push(Span::styled(
                mode_text,
                Style::default().fg(theme.dim),
            ));
        }
        _ => {
            spans.push(Span::styled(mode_text, Style::default().fg(theme.dim)));
        }
    }

    // Right side: mode text
    let mode_str = match app.mode {
        super::app::AppMode::Input => "input",
        super::app::AppMode::Normal => "normal",
        super::app::AppMode::Streaming => "streaming",
        super::app::AppMode::Detail => "detail",
        super::app::AppMode::Confirm { .. } => "confirm",
    };

    let right_text = format!(" {} ", mode_str);
    let right_style = Style::default().fg(theme.dim);

    // Calculate padding to right-align mode text
    let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
    let right_len = right_text.len();
    if left_len + right_len + 3 < width {
        let pad = width - left_len - right_len;
        spans.push(Span::styled(" ".repeat(pad), Style::default()));
    } else {
        spans.push(Span::styled("  ", Style::default()));
    }
    spans.push(Span::styled(right_text, right_style));

    Line::from(spans)
}
