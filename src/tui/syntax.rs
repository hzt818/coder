//! Syntax highlighting for code blocks using syntect

use ratatui::prelude::*;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use std::sync::OnceLock;

use super::theme::AppTheme;

/// Get the global syntax set (loaded once)
fn syntax_set() -> &'static SyntaxSet {
    static SS: OnceLock<SyntaxSet> = OnceLock::new();
    SS.get_or_init(|| SyntaxSet::load_defaults_newlines())
}

/// Get the global theme set (loaded once)
fn theme_set() -> &'static ThemeSet {
    static TS: OnceLock<ThemeSet> = OnceLock::new();
    TS.get_or_init(|| ThemeSet::load_defaults())
}

/// Highlight a fenced code block and return styled Lines.
///
/// `language` is the language identifier from the fence (e.g. "rust", "py", "").
/// `code` is the raw code text (without the fence markers or language tag).
pub fn highlight_code_block(
    language: &str,
    code: &str,
    _app_theme: &AppTheme,
) -> Vec<Line<'static>> {
    let ss = syntax_set();
    let ts = theme_set();

    // Find syntax: try token match first, then extension, then name, fallback to plain text
    let syntax = ss
        .find_syntax_by_token(language)
        .or_else(|| ss.find_syntax_by_extension(language))
        .or_else(|| ss.find_syntax_by_name(language))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    // Use a consistent dark theme for code highlighting
    let theme = &ts.themes["base16-ocean.dark"];

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines: Vec<Line<'static>> = Vec::new();

    for line in code.lines() {
        let ranges = match highlighter.highlight_line(line, ss) {
            Ok(r) => r,
            Err(_) => {
                // Fallback: render the raw line
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default(),
                )));
                continue;
            }
        };

        let styled_spans: Vec<Span<'static>> = ranges
            .iter()
            .map(|(style, text)| {
                let fg = style.foreground;
                let mut s = Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b));
                if style
                    .font_style
                    .contains(syntect::highlighting::FontStyle::BOLD)
                {
                    s = s.add_modifier(Modifier::BOLD);
                }
                if style
                    .font_style
                    .contains(syntect::highlighting::FontStyle::ITALIC)
                {
                    s = s.add_modifier(Modifier::ITALIC);
                }
                Span::styled(text.to_string(), s)
            })
            .collect();

        lines.push(Line::from(styled_spans));
    }

    lines
}
