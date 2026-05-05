//! Chat panel - renders the conversation history

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use super::app::ChatMessage;
use super::theme::AppTheme;
use super::syntax::highlight_code_block;

/// Parse content and yield either plain lines or code block lines
fn parse_content<'a>(content: &'a str) -> Vec<ContentLine<'a>> {
    let mut result = Vec::new();
    let mut in_code_block = false;
    let mut code_language = String::new();
    let mut code_lines = String::new();

    for line in content.lines() {
        if line.starts_with("```") {
            if !in_code_block {
                // Start of code block
                in_code_block = true;
                code_language = line.trim_start_matches("```").trim().to_string();
                code_lines.clear();
            } else {
                // End of code block - highlight and add
                let highlighted = highlight_code_block(&code_language, &code_lines, &AppTheme::default());
                for hline in highlighted {
                    result.push(ContentLine::Highlighted(hline));
                }
                in_code_block = false;
                code_language.clear();
                code_lines.clear();
            }
        } else if in_code_block {
            if !code_lines.is_empty() {
                code_lines.push('\n');
            }
            code_lines.push_str(line);
        } else {
            result.push(ContentLine::Plain(line.to_string()));
        }
    }

    // Handle unclosed code block
    if in_code_block && !code_lines.is_empty() {
        let highlighted = highlight_code_block(&code_language, &code_lines, &AppTheme::default());
        for hline in highlighted {
            result.push(ContentLine::Highlighted(hline));
        }
    }

    result
}

/// A line that is either plain text or a highlighted code block
enum ContentLine<'a> {
    Plain(String),
    Highlighted(Line<'a>),
}

/// Render the chat panel
pub fn render_chat(frame: &mut Frame, area: Rect, messages: &[ChatMessage], offset: usize, working_dir: &str, git_branch: Option<&str>, theme: &AppTheme) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in messages {
        // Header line
        let (role_label, role_color) = match msg.role.as_str() {
            "user" => (" User", theme.user),
            "assistant" => ("◉ Assistant", theme.assistant),
            "system" => ("◆ System", theme.warning),
            _ => ("●", theme.fg),
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} · {} ", role_label, msg.timestamp),
                Style::default().fg(role_color).add_modifier(Modifier::BOLD),
            ),
        ]));

        // Content - parse for code blocks and render accordingly
        let content_lines = parse_content(&msg.content);
        for cl in content_lines {
            match cl {
                ContentLine::Plain(text) => {
                    let style = match msg.role.as_str() {
                        "user" => Style::default().fg(theme.user),
                        "assistant" => Style::default().fg(theme.assistant),
                        "system" => Style::default().fg(theme.warning),
                        _ => Style::default().fg(theme.fg),
                    };
                    lines.push(Line::from(Span::styled(text, style)));
                }
                ContentLine::Highlighted(hl) => {
                    lines.push(hl);
                }
            }
        }

        // Tool calls
        for tc in &msg.tool_calls {
            let status_icon = match tc.status.as_str() {
                "running" => "⏳",
                "success" => "✔",
                "error" => "✘",
                _ => "?",
            };
            let status_color = match tc.status.as_str() {
                "running" => theme.warning,
                "success" => theme.success,
                "error" => theme.error,
                _ => theme.fg,
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" ┌─ {} {} ", status_icon, tc.tool_name),
                    Style::default().fg(theme.tool).add_modifier(Modifier::BOLD),
                ),
            ]));
            for out_line in tc.output.lines() {
                lines.push(Line::from(Span::styled(
                    format!(" │ {}", out_line),
                    Style::default().fg(theme.dim),
                )));
            }
            lines.push(Line::from(Span::styled(
                " └──",
                Style::default().fg(status_color),
            )));
        }

        // Spacing between messages
        lines.push(Line::from(""));
    }

    if lines.is_empty() {
        // Welcome title
        lines.push(Line::from(vec![
            Span::styled(
                format!(" 🦀 Coder v{}", env!("CARGO_PKG_VERSION")),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        // Working directory
        lines.push(Line::from(vec![
            Span::styled(" Working directory: ", Style::default().fg(theme.dim)),
            Span::styled(working_dir, Style::default().fg(theme.fg)),
        ]));

        // Git branch (cached at startup)
        if let Some(branch) = git_branch {
            lines.push(Line::from(vec![
                Span::styled(" Git branch: ", Style::default().fg(theme.dim)),
                Span::styled(branch, Style::default().fg(theme.success)),
                Span::styled(" ✓", Style::default().fg(theme.success)),
            ]));
        }

        lines.push(Line::from(""));

        // Quick actions header
        lines.push(Line::from(vec![
            Span::styled(
                " Quick actions:",
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   /help     ", Style::default().fg(theme.tool)),
            Span::styled("Show all commands     ", Style::default().fg(theme.dim)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   /tools    ", Style::default().fg(theme.tool)),
            Span::styled("List available tools  ", Style::default().fg(theme.dim)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   @mention  ", Style::default().fg(theme.tool)),
            Span::styled("Autocomplete via @    ", Style::default().fg(theme.dim)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   !command  ", Style::default().fg(theme.tool)),
            Span::styled("Run shell commands   ", Style::default().fg(theme.dim)),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                " Type a message to start coding.",
                Style::default().fg(theme.dim),
            ),
        ]));
    }

    let chat_paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false })
        .scroll((offset as u16, 0));

    let mut chat_area = area;
    chat_area.x += 1;
    chat_area.y += 1;
    chat_area.width = chat_area.width.saturating_sub(2);
    chat_area.height = chat_area.height.saturating_sub(2);

    frame.render_widget(chat_paragraph, chat_area);
}
