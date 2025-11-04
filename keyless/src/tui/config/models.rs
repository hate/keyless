//! Models table rendering for the Config screen.

use keyless_core::utils::human_size;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState};

use crate::tui::theme::{Colors, Theme};

/// Render the models table with install status and sizes.
pub fn render_models(
    f: &mut ratatui::Frame,
    area: Rect,
    app_model_idx: usize,
    models: &[&str],
    sizes: Option<&std::collections::HashMap<String, u64>>,
    installed: Option<&std::collections::HashSet<String>>,
) {
    // Build rows: Name | By | Size | Lang | ✓
    // Recompute each frame; allocates Vec<Row> and clones strings per model. Acceptable given
    // the small number of models and TUI frame budget.
    let rows: Vec<Row> = models
        .iter()
        .map(|m| {
            let id = *m;
            // Parse "author/model" format; fallback to empty author if no '/' found.
            // Assumes model IDs are either "model" or "author/model" (no trailing slashes).
            let mut parts = id.split('/');
            let by = parts.next().unwrap_or("");
            // Extract model name (second part) or use full ID if no separator exists.
            // This handles both formats gracefully without panicking.
            let name = parts.next().unwrap_or(id);
            // Detect language from filename suffix; ".en" indicates English-only model.
            // All other models are assumed multilingual (supports auto-detection).
            let lang = if id.ends_with(".en") { "en" } else { "multi" };
            let size_cell = if let Some(map) = sizes {
                // Lookup size from metadata cache; show em dash (—) if not yet loaded.
                // Cache is populated asynchronously during model discovery.
                if let Some(bytes) = map.get(id) {
                    Cell::from(human_size(*bytes))
                } else {
                    Cell::from("—")
                }
            } else {
                // No size metadata available; show placeholder until cache is populated.
                Cell::from("—")
            };
            let installed_cell = if let Some(set) = installed {
                // Check membership in installed set (O(1) lookup); show green checkmark if found.
                // Set is built from discovered models at startup.
                if set.contains(id) {
                    // Leading spaces align the checkmark; color signals local availability.
                    Cell::from("    ✓").style(Style::default().fg(Colors::downloaded()))
                } else {
                    Cell::from("")
                }
            } else {
                // No installed set provided; hide checkmark column entirely.
                Cell::from("")
            };
            // Clone strings here; Cells need owned data. Minor alloc per row is acceptable.
            Row::new(vec![
                Cell::from(name.to_string()),
                Cell::from(by.to_string()),
                size_cell,
                Cell::from(lang.to_string()),
                installed_cell,
            ])
        })
        .collect();

    // Ephemeral table state; selection persists in `app` not this local state object.
    let mut state = TableState::default();
    // No bounds guard here; relies on upstream to keep `app_model_idx < models.len()`.
    // Ratatui tolerates out-of-range select without panicking but may render no highlight.
    state.select(Some(app_model_idx));

    let header = Row::new(vec!["name", "by", "size", "lang", "installed"]) // lowercase to match style
        .style(Theme::text_primary())
        .bottom_margin(0);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Colors::border()))
        .title_style(Style::default().fg(Colors::text_primary()))
        .title("🧠 models");

    // Column widths: name gets 50% (longest content), others are compact for metadata.
    // Percentages sum to 98%; remaining 2% provides natural spacing via column_spacing.
    let widths = [
        Constraint::Percentage(50),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(8),
        Constraint::Percentage(10),
    ];

    // Trailing space ensures padding between the indicator glyph and the text.
    let indicator_symbol = format!("{} ", Theme::active_indicator());
    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .highlight_symbol(indicator_symbol)
        .row_highlight_style(Theme::selected())
        .column_spacing(1);

    // Render with stateful API so the selected row gets the highlight/indicator this frame.
    f.render_stateful_widget(table, area, &mut state);
}
