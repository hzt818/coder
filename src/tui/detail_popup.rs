//! Detail popup - shown when user presses Ctrl+O

use super::theme::AppTheme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

/// Render the detail popup overlay
pub fn render_detail_popup(frame: &mut Frame, area: Rect, content: &str, theme: &AppTheme) {
    // Create a centered popup area
    let popup_width = std::cmp::min(60, area.width.saturating_sub(10));
    let popup_height = std::cmp::min(20, area.height.saturating_sub(4));
    let popup_x = (area.width - popup_width) / 2;
    let popup_y = (area.height - popup_height) / 2;

    let popup_area = Rect {
        x: area.x + popup_x,
        y: area.y + popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Details ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(Color::Reset));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let text = Paragraph::new(Span::styled(content, Style::default().fg(theme.fg)))
        .wrap(Wrap { trim: false });

    frame.render_widget(text, inner);
}
