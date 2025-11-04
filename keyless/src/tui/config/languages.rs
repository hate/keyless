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
    // Determine if the selected model is English-only (.en suffix); defaults to false if
    // model_idx is out of bounds. Assumes upstream logic keeps model_idx valid.
    let en_only = models
        .get(app_model_idx)
        .map(|m| is_english_only_model(m))
        .unwrap_or(false);

    // Pre-allocate capacity: LANG_CODES + 1 slot for "auto" (if not English-only).
    // This avoids reallocation during iteration when building the list.
    let mut items: Vec<ListItem> = Vec::with_capacity(LANG_CODES.len() + 1);
    if !en_only {
        // Virtual "auto" entry at UI index 0 for multilingual models only.
        // English-only models skip this entry since auto-detection isn't applicable.
        let is_selected = app_language_idx == 0;
        let style = if is_selected {
            Theme::text_primary()
        } else {
            Theme::text_muted()
        };
        let label = "★ auto"; // lowercase per style
        let text = if is_selected {
            // Selected row shows raw label; widget's highlight_symbol adds the indicator.
            label.to_string()
        } else {
            // Non-selected rows receive an inactive indicator prefix.
            format!("{} {}", Theme::inactive_indicator(), label)
        };
        items.push(ListItem::new(Span::styled(text, style)));
    }

    for (idx, code) in LANG_CODES.iter().enumerate() {
        let name = lang_name(code);
        // Map LANG_CODES index to UI list index. Critical for selection matching:
        // - Multilingual: "auto" occupies 0, so languages start at 1 (ui_idx = idx + 1).
        // - English-only: no "auto", so languages start at 0 (ui_idx = idx).
        // This offset ensures `app_language_idx` matches the correct rendered position.
        let ui_idx = if en_only { idx } else { idx + 1 };

        if en_only && *code != "en" {
            // English-only models: show non-English languages as disabled (crossed-out)
            // but still visible. Only "en" is selectable; others are grayed out.
            items.push(ListItem::new(Span::styled(
                name,
                Theme::text_muted().add_modifier(ratatui::style::Modifier::CROSSED_OUT),
            )));
        } else {
            // Normal rendering: check selection against computed UI index.
            let is_selected = app_language_idx == ui_idx;
            let style = if is_selected {
                Theme::text_primary()
            } else {
                Theme::text_muted()
            };
            let text = if is_selected {
                // Selected row omits inactive indicator; widget's highlight_symbol provides it.
                name.to_string()
            } else {
                // Non-selected rows show inactive indicator for visual de-emphasis.
                format!("{} {}", Theme::inactive_indicator(), name)
            };
            items.push(ListItem::new(Span::styled(text, style)));
        }
    }

    // Ephemeral list state; selection persists in `app` not this local state object.
    let mut lang_state = ratatui::widgets::ListState::default();
    // No bounds guard here; relies on upstream to keep `app_language_idx` valid for the
    // current model (respecting the "auto" offset). Ratatui tolerates out-of-range select.
    lang_state.select(Some(app_language_idx));
    // Trailing space ensures padding between the indicator glyph and the text.
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
    // Render with stateful API so the selected row gets the highlight/indicator this frame.
    f.render_stateful_widget(list, area, &mut lang_state);
}
