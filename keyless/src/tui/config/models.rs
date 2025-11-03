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
    // Size is filled later by metadata; show em dash for now.
    let rows: Vec<Row> = models
        .iter()
        .map(|m| {
            let id = *m;
            let mut parts = id.split('/');
            let by = parts.next().unwrap_or("");
            let name = parts.next().unwrap_or(id); // Just the model name without author/
            let lang = if id.ends_with(".en") { "en" } else { "multi" };
            let size_cell = if let Some(map) = sizes {
                if let Some(bytes) = map.get(id) {
                    Cell::from(human_size(*bytes))
                } else {
                    Cell::from("—")
                }
            } else {
                Cell::from("—")
            };
            let installed_cell = if let Some(set) = installed {
                if set.contains(id) {
                    Cell::from("    ✓").style(Style::default().fg(Colors::downloaded()))
                } else {
                    Cell::from("")
                }
            } else {
                Cell::from("")
            };
            Row::new(vec![
                Cell::from(name.to_string()),
                Cell::from(by.to_string()),
                size_cell,
                Cell::from(lang.to_string()),
                installed_cell,
            ])
        })
        .collect();

    let mut state = TableState::default();
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

    // Column widths: name grows; others are tight
    let widths = [
        Constraint::Percentage(50),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(8),
        Constraint::Percentage(10),
    ];

    let indicator_symbol = format!("{} ", Theme::active_indicator());
    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .highlight_symbol(indicator_symbol)
        .row_highlight_style(Theme::selected())
        .column_spacing(1);

    f.render_stateful_widget(table, area, &mut state);
}
