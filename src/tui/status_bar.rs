//! Status bar widget

use super::theme::AppTheme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Render the status bar
pub fn render_status_bar(frame: &mut Frame, area: Rect, status_text: &str, theme: &AppTheme) {
    let block = Block::default()
        .border_style(Style::default().fg(theme.dim))
        .borders(Borders::TOP);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = Paragraph::new(Span::styled(
        status_text,
        Style::default().fg(theme.status_bar),
    ));

    frame.render_widget(text, inner);
}
