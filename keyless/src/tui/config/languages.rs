//! Languages panel rendering for the Config screen.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem};

use crate::tui::theme::{Colors, Theme};
use keyless_core::options::{LANG_CODES, is_english_only_model, lang_name};

/// Render the languages panel, including an "auto" entry for multilingual models.
pub fn render_languages(
    f: &mut ratatui::Frame,
    area: Rect,
    models: &[&str],
    app_model_idx: usize,
    app_language_idx: usize,
) {
    let en_only = models
        .get(app_model_idx)
        .map(|m| is_english_only_model(m))
        .unwrap_or(false);

    // Build items with a virtual "★ auto" entry at index 0 (skip for .en models)
    let mut items: Vec<ListItem> = Vec::with_capacity(LANG_CODES.len() + 1);
    if !en_only {
        let is_selected = app_language_idx == 0;
        let style = if is_selected {
            Theme::text_primary()
        } else {
            Theme::text_muted()
        };
        let label = "★ auto"; // lowercase per style
        let text = if is_selected {
            label.to_string()
        } else {
            format!("{} {}", Theme::inactive_indicator(), label)
        };
        items.push(ListItem::new(Span::styled(text, style)));
    }

    for (idx, code) in LANG_CODES.iter().enumerate() {
        let name = lang_name(code);
        // Real index in the UI list depends on whether "auto" is present
        // - Multilingual: auto at 0, so languages offset by +1 (ui_idx = idx + 1)
        // - .en models: no auto, so languages start at 0 (ui_idx = idx)
        let ui_idx = if en_only { idx } else { idx + 1 };

        if en_only && *code != "en" {
            items.push(ListItem::new(Span::styled(
                name,
                Theme::text_muted().add_modifier(ratatui::style::Modifier::CROSSED_OUT),
            )));
        } else {
            let is_selected = app_language_idx == ui_idx;
            let style = if is_selected {
                Theme::text_primary()
            } else {
                Theme::text_muted()
            };
            let text = if is_selected {
                name.to_string()
            } else {
                format!("{} {}", Theme::inactive_indicator(), name)
            };
            items.push(ListItem::new(Span::styled(text, style)));
        }
    }

    let mut lang_state = ratatui::widgets::ListState::default();
    lang_state.select(Some(app_language_idx));
    let indicator_symbol = format!("{} ", Theme::active_indicator());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Colors::border()))
        .title_style(Style::default().fg(Colors::text_primary()))
        .title("🌐 languages");
    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected())
        .highlight_symbol(&indicator_symbol);
    f.render_stateful_widget(list, area, &mut lang_state);
}
