//! VAD thresholds and timing panel rendering for the Config screen.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::tui::state::AppState;
use crate::tui::theme::{Colors, Theme};

/// Render the VAD thresholds and timing panel with live gauge.
pub fn render_vad(f: &mut ratatui::Frame, area: Rect, app: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Colors::border()));
    f.render_widget(block, area);
    let mut inner = area;
    inner.x += 1;
    inner.y += 1;
    inner.width = inner.width.saturating_sub(2);
    inner.height = inner.height.saturating_sub(2);

    // Center VAD content vertically (3 lines total: 2 for thresholds, 1 for timing)
    let vad_content_height = 3;
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),                     // Top padding
            Constraint::Length(vad_content_height), // VAD content
            Constraint::Min(0),                     // Bottom padding
        ])
        .split(inner);

    let vad_area = vertical_layout[1];
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(1)])
        .split(vad_area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(layout[0]);

    let divider = Block::default()
        .style(Style::default().fg(Colors::border()))
        .borders(Borders::LEFT);
    f.render_widget(divider, cols[1]);

    // start label + gauge
    let start_hint = if app.vad.start_db <= -60.0 {
        "early"
    } else if app.vad.start_db <= -40.0 {
        "normal"
    } else {
        "late"
    };
    let start_col = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(cols[0]);
    let start_label = Paragraph::new(Line::from(vec![
        Span::styled(
            "start threshold",
            Style::default()
                .fg(Colors::vad_left())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": "),
        Span::styled(
            format!("{:.0} dB", app.vad.start_db),
            Style::default()
                .fg(Colors::text_primary())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(format!("({})", start_hint), Theme::text_secondary()),
    ]))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(start_label, start_col[0]);
    render_vad_gauge(f, start_col[1], Colors::vad_left(), app.vad.start_db);

    // stop label + gauge
    let stop_hint = if app.vad.stop_db >= -40.0 {
        "early"
    } else if app.vad.stop_db >= -60.0 {
        "normal"
    } else {
        "late"
    };
    let stop_col = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(cols[2]);
    let stop_label = Paragraph::new(Line::from(vec![
        Span::styled(
            "stop threshold",
            Style::default()
                .fg(Colors::vad_right())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": "),
        Span::styled(
            format!("{:.0} dB", app.vad.stop_db),
            Style::default()
                .fg(Colors::text_primary())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(format!("({})", stop_hint), Theme::text_secondary()),
    ]))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(stop_label, stop_col[0]);
    render_vad_gauge(f, stop_col[1], Colors::vad_right(), app.vad.stop_db);

    // timing row
    let timing_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(layout[1]);
    let timing_divider = Block::default()
        .style(Style::default().fg(Colors::border()))
        .borders(Borders::LEFT);
    f.render_widget(timing_divider, timing_cols[1]);

    let min_hint = match app.vad.min_duration_ms {
        ms if ms < 200 => "fast",
        ms if ms < 500 => "normal",
        _ => "slow",
    };
    let min_text = Paragraph::new(Line::from(vec![
        Span::styled(
            "min speech",
            Style::default()
                .fg(Colors::vad_left())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": "),
        Span::styled(
            format!("{} ms", app.vad.min_duration_ms),
            Theme::text_primary().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(format!("({})", min_hint), Theme::text_secondary()),
    ]))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(min_text, timing_cols[0]);

    let max_hint = match app.vad.max_silence_ms {
        ms if ms < 300 => "fast",
        ms if ms < 1000 => "normal",
        _ => "slow",
    };
    let max_text = Paragraph::new(Line::from(vec![
        Span::styled(
            "max silence",
            Style::default()
                .fg(Colors::vad_right())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": "),
        Span::styled(
            format!("{} ms", app.vad.max_silence_ms),
            Theme::text_primary().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(format!("({})", max_hint), Theme::text_secondary()),
    ]))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(max_text, timing_cols[2]);
}

/// Render a compact horizontal VAD gauge for the current dB level.
fn render_vad_gauge(f: &mut ratatui::Frame, area: Rect, color: Color, value_db: f32) {
    const MIN_DB: f64 = -80.0;
    const MAX_DB: f64 = 0.0;
    let ratio = ((value_db as f64 - MIN_DB) / (MAX_DB - MIN_DB)).clamp(0.0, 1.0);
    let darken_color = |c: Color| -> Color {
        match c {
            Color::Rgb(r, g, b) => Color::Rgb(
                (r as f32 * 0.4) as u8,
                (g as f32 * 0.4) as u8,
                (b as f32 * 0.4) as u8,
            ),
            Color::Green => Color::Rgb(0, 100, 0),
            Color::Cyan => Color::Rgb(0, 100, 100),
            Color::Blue => Color::Rgb(0, 0, 100),
            Color::Yellow => Color::Rgb(100, 100, 0),
            _ => Colors::text_muted(),
        }
    };
    let dimmed_color = if color == Colors::primary() {
        Colors::accent_dim()
    } else {
        darken_color(color)
    };
    let padding = 2;
    let mut padded_area = area;
    if padded_area.width > padding * 2 {
        padded_area.x += padding;
        padded_area.width -= padding * 2;
    }
    let filled_width = (padded_area.width as f64 * ratio) as u16;
    let unfilled_width = padded_area.width.saturating_sub(filled_width);
    if filled_width > 0 {
        let mut filled_area = padded_area;
        filled_area.width = filled_width;
        let fill_text = "█".repeat(filled_width as usize);
        let filled_paragraph = Paragraph::new(fill_text).style(Style::default().fg(color));
        f.render_widget(filled_paragraph, filled_area);
    }
    if unfilled_width > 0 {
        let mut unfilled_area = padded_area;
        unfilled_area.x += filled_width;
        unfilled_area.width = unfilled_width;
        let fill_text = "█".repeat(unfilled_width as usize);
        let dimmed_paragraph = Paragraph::new(fill_text).style(Style::default().fg(dimmed_color));
        f.render_widget(dimmed_paragraph, unfilled_area);
    }
}
