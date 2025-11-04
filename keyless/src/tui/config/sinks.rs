//! Sinks row rendering for the Config screen.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::tui::state::AppState;
use crate::tui::theme::{Colors, Theme};

/// Render the sinks row (paste/clipboard/file).
pub fn render_sinks(f: &mut ratatui::Frame, area: Rect, app: &AppState, sinks: &[&str]) {
    // Three-column layout: 33/34/33 split. Middle column gets +1% to absorb rounding
    // differences and ensure exactly 3 columns fit. Assumes exactly 3 sinks.
    let sink_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    for (i, rect) in sink_row.iter().enumerate() {
        // Map index to icon/color: paste (0), clipboard (1), file (2).
        // Assumes sinks array matches this order; `_` handles index 2+ defensively.
        let (icon, color) = match i {
            0 => ("💻", Colors::sink_paste()),
            1 => ("📋", Colors::sink_clipboard()),
            _ => ("📄", Colors::sink_file()),
        };
        // Match selection by converting enum to index; assumes sink_choice matches enum order.
        let is_selected = app.selections.sink_choice.to_index() == i;
        let border_style = if is_selected {
            // Selected sink: bright colored border for visibility.
            Style::default().fg(color)
        } else {
            // Unselected: dimmed default border to de-emphasize.
            Style::default()
                .fg(Colors::border())
                .add_modifier(Modifier::DIM)
        };
        let bg_style = if is_selected {
            // Selected sink gets highlighted background for contrast.
            Style::default().bg(Colors::bg_selected())
        } else {
            Style::default()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .style(bg_style);
        // Clone needed because Block doesn't implement Copy; small cost per iteration.
        f.render_widget(block.clone(), *rect);

        let mut inner = *rect;
        // Inset by 1 cell on all sides to account for border; saturating ops prevent underflow.
        inner.x += 1;
        inner.y += 1;
        inner.width = inner.width.saturating_sub(2);
        inner.height = inner.height.saturating_sub(2);

        // Vertical layout centers the label: equal Min constraints above/below a fixed 1-row
        // middle. Saturating_sub on top avoids underflow when height is very small.
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(inner.height.saturating_sub(1) / 2),
                Constraint::Length(1),
                Constraint::Min(inner.height / 2),
            ])
            .split(inner);

        let style = if is_selected {
            // Selected: color-matched bold text for strong visual feedback.
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Theme::text_muted()
        };

        // Format allocates a new string each frame; acceptable given small count and TUI budget.
        let label = Paragraph::new(format!("{} {}", icon, sinks[i]))
            .style(style)
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true });

        // Render label in the middle row (v[1]) for vertical centering.
        f.render_widget(label, v[1]);
    }
}
