//! Mention autocomplete popup - shown when user types @

use super::app::InputSubmode;
use super::theme::AppTheme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

/// Render the @ mention autocomplete popup
pub fn render_mention_popup(
    frame: &mut Frame,
    area: Rect,
    submode: &InputSubmode,
    theme: &AppTheme,
) {
    let (items, selected) = match submode {
        InputSubmode::Mention {
            ref items,
            selected,
            ..
        } => (items, *selected),
        _ => return,
    };

    if items.is_empty() {
        return;
    }

    // Calculate popup dimensions
    let popup_width = std::cmp::min(40, area.width.saturating_sub(10));
    let max_height = std::cmp::min(
        items.len() as u16 + 2,
        std::cmp::min(12, area.height.saturating_sub(6)),
    );

    // Position the popup above the input area (bottom of screen)
    let popup_x = area.x + 2; // Offset from left to align with input text
    let popup_y = area.bottom().saturating_sub(3 + max_height); // Above input area

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: max_height,
    };

    // Clear behind popup
    frame.render_widget(Clear, popup_area);

    // Block border
    let block = Block::default()
        .title(" Suggestions ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(popup_area);
    frame.render_widget(&block, popup_area);

    // List of suggestions
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let prefix = if i == selected { "→ " } else { "  " };
            let style = if i == selected {
                Style::default()
                    .fg(theme.selection_fg)
                    .bg(theme.selection_bg)
            } else {
                Style::default().fg(theme.fg)
            };
            ListItem::new(format!("{}{}", prefix, item)).style(style)
        })
        .collect();

    let list = List::new(list_items);

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    frame.render_stateful_widget(list, inner, &mut list_state);
}
