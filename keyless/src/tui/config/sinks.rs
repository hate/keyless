//! Sinks row rendering for the Config screen.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::tui::state::AppState;
use crate::tui::theme::{Colors, Theme};

/// Render the sinks row (paste/clipboard/file).
pub fn render_sinks(f: &mut ratatui::Frame, area: Rect, app: &AppState, sinks: &[&str]) {
    let sink_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    for (i, rect) in sink_row.iter().enumerate() {
        let (icon, color) = match i {
            0 => ("💻", Colors::sink_paste()),
            1 => ("📋", Colors::sink_clipboard()),
            _ => ("📄", Colors::sink_file()),
        };
        let is_selected = app.selections.sink_choice.to_index() == i;
        let border_style = if is_selected {
            Style::default().fg(color)
        } else {
            Style::default()
                .fg(Colors::border())
                .add_modifier(Modifier::DIM)
        };
        let bg_style = if is_selected {
            Style::default().bg(Colors::bg_selected())
        } else {
            Style::default()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .style(bg_style);
        f.render_widget(block.clone(), *rect);

        let mut inner = *rect;
        inner.x += 1;
        inner.y += 1;
        inner.width = inner.width.saturating_sub(2);
        inner.height = inner.height.saturating_sub(2);

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(inner.height.saturating_sub(1) / 2),
                Constraint::Length(1),
                Constraint::Min(inner.height / 2),
            ])
            .split(inner);
        let style = if is_selected {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Theme::text_muted()
        };
        let label = Paragraph::new(format!("{} {}", icon, sinks[i]))
            .style(style)
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(label, v[1]);
    }
}
