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
    // Inset by 1 cell on all sides to account for border; saturating ops prevent underflow.
    inner.x += 1;
    inner.y += 1;
    inner.width = inner.width.saturating_sub(2);
    inner.height = inner.height.saturating_sub(2);

    // Center VAD content vertically (3 lines total: 2 for thresholds, 1 for timing).
    // Min(0) constraints above/below allow flexible padding while keeping content centered.
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
    // Split the VAD area into two rows: thresholds and timing.
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(1)])
        .split(vad_area);

    // Split threshold row: left column (start), divider, right column (stop).
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(layout[0]);

    // Render a divider between the start and stop thresholds.
    let divider = Block::default()
        .style(Style::default().fg(Colors::border()))
        .borders(Borders::LEFT);
    f.render_widget(divider, cols[1]);

    // start label + gauge
    // Hint categories: lower threshold = more sensitive = earlier activation.
    // Boundaries: <= -60 dB (very sensitive), <= -40 dB (normal), > -40 dB (less sensitive).
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
    // Hint categories: higher threshold = less sensitive = earlier deactivation.
    // Boundaries: >= -40 dB (early stop), >= -60 dB (normal), < -60 dB (late stop).
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

    // Timing hint categories: shorter duration = faster response.
    // Boundaries: < 200 ms (fast), < 500 ms (normal), >= 500 ms (slow).
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

    // Silence hint categories: shorter silence tolerance = faster stop detection.
    // Boundaries: < 300 ms (fast), < 1000 ms (normal), >= 1000 ms (slow).
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
    // Gauge range: -80 dB (minimum, quiet) to 0 dB (maximum, loud).
    // These bounds represent typical audio levels for VAD; values outside are clamped.
    const MIN_DB: f64 = -80.0;
    const MAX_DB: f64 = 0.0;
    // Normalize dB value to [0, 1] ratio: (value - min) / (max - min).
    // Clamp prevents out-of-range values from producing invalid ratios or UB.
    let ratio = ((value_db as f64 - MIN_DB) / (MAX_DB - MIN_DB)).clamp(0.0, 1.0);
    // Darken RGB colors by multiplying channels by 0.4 (60% reduction) for unfilled portion.
    // Named colors are converted to fixed RGB values for consistency.
    let darken_color = |c: Color| -> Color {
        match c {
            Color::Rgb(r, g, b) => Color::Rgb(
                // Cast through f32 to preserve precision during multiplication.
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
    // Special case: primary color uses accent_dim for unfilled; others use darken_color.
    let dimmed_color = if color == Colors::primary() {
        Colors::accent_dim()
    } else {
        darken_color(color)
    };
    // Apply horizontal padding to avoid gauge touching borders.
    let padding = 2;
    let mut padded_area = area;
    // Only apply padding if area is wide enough; prevents negative width on tiny terminals.
    if padded_area.width > padding * 2 {
        padded_area.x += padding;
        padded_area.width -= padding * 2;
    }
    // Calculate filled width from ratio: multiply padded width by ratio, cast to u16.
    // Truncation is acceptable for visual gauge; remaining pixels go to unfilled.
    let filled_width = (padded_area.width as f64 * ratio) as u16;
    // Saturating_sub prevents underflow if filled_width exceeds padded_area.width (shouldn't happen
    // but protects against rounding edge cases).
    let unfilled_width = padded_area.width.saturating_sub(filled_width);
    if filled_width > 0 {
        // Render filled portion: left side of gauge with full brightness color.
        let mut filled_area = padded_area;
        filled_area.width = filled_width;
        // Allocate string of block characters; acceptable alloc for small gauge widths.
        let fill_text = "█".repeat(filled_width as usize);
        let filled_paragraph = Paragraph::new(fill_text).style(Style::default().fg(color));
        f.render_widget(filled_paragraph, filled_area);
    }
    if unfilled_width > 0 {
        // Render unfilled portion: right side of gauge with dimmed color.
        // Position starts after filled_width to create contiguous visual bar.
        let mut unfilled_area = padded_area;
        unfilled_area.x += filled_width;
        unfilled_area.width = unfilled_width;
        let fill_text = "█".repeat(unfilled_width as usize);
        let dimmed_paragraph = Paragraph::new(fill_text).style(Style::default().fg(dimmed_color));
        f.render_widget(dimmed_paragraph, unfilled_area);
    }
}
