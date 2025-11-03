//! Input devices list rendering.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem};

use crate::tui::state::AppState;
use crate::tui::theme::{Colors, Theme};

/// Render the input devices list.
pub fn render_devices(f: &mut ratatui::Frame, area: Rect, app: &AppState) {
    let device_items: Vec<ListItem> = app
        .selections
        .device_names
        .iter()
        .enumerate()
        .map(|(idx, d)| {
            let is_selected = idx == app.selections.device_idx;
            let style = if is_selected {
                Style::default()
                    .fg(Colors::device_accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Theme::text_muted()
            };
            let text = if is_selected {
                d.clone()
            } else {
                format!("{} {}", Theme::inactive_indicator(), d)
            };
            ListItem::new(Span::styled(text, style))
        })
        .collect();
    let mut device_state = ratatui::widgets::ListState::default();
    device_state.select(Some(app.selections.device_idx));
    let indicator_symbol = format!("{} ", Theme::active_indicator());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Colors::border()))
        .title_style(Style::default().fg(Colors::device_accent()))
        .title("🎤 input devices");
    let list = List::new(device_items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Colors::device_accent())
                .bg(Colors::bg_selected())
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(&indicator_symbol);
    f.render_stateful_widget(list, area, &mut device_state);
}
