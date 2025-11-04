//! Input devices list rendering.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem};

use crate::tui::state::AppState;
use crate::tui::theme::{Colors, Theme};

/// Render the input devices list.
pub fn render_devices(f: &mut ratatui::Frame, area: Rect, app: &AppState) {
    // Recompute list items each frame; incurs small allocs for text styling/formatting.
    // Acceptable given the short list size and TUI frame budget.
    let device_items: Vec<ListItem> = app
        .selections
        .device_names
        .iter()
        .enumerate()
        .map(|(idx, d)| {
            // Selection is driven by `app.selections.device_idx` (external state).
            // Assumes the index is in-bounds of `device_names`; upstream logic must enforce it.
            let is_selected = idx == app.selections.device_idx;
            let style = if is_selected {
                Style::default()
                    .fg(Colors::device_accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Theme::text_muted()
            };
            let text = if is_selected {
                // Selected row shows the raw device name; the active indicator will be provided by
                // the widget's `highlight_symbol` below to avoid duplicate indicators.
                d.clone()
            } else {
                // Non-selected rows are visually de-emphasized and receive an inactive indicator.
                format!("{} {}", Theme::inactive_indicator(), d)
            };
            ListItem::new(Span::styled(text, style))
        })
        .collect();

    // Local list state is ephemeral; selection is persisted in `app` not this state object.
    let mut device_state = ratatui::widgets::ListState::default();
    // No bounds guard here; relies on upstream to keep `device_idx < device_names.len()`.
    // Ratatui tolerates out-of-range select without panicking but may render no highlight.
    device_state.select(Some(app.selections.device_idx));

    // Trailing space ensures padding between the indicator glyph and the text.
    let indicator_symbol = format!("{} ", Theme::active_indicator());

    // Visual container: rounded border and accent-colored title for discoverability.
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Colors::border()))
        .title_style(Style::default().fg(Colors::device_accent()))
        .title("🎤 input devices");

    // Highlight uses accent fg + selected bg for strong contrast; bold improves readability.
    let list = List::new(device_items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Colors::device_accent())
                .bg(Colors::bg_selected())
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(&indicator_symbol);

    // Render with stateful API so the selected row gets the highlight/indicator this frame.
    f.render_stateful_widget(list, area, &mut device_state);
}
