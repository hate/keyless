//! EQ spectrum rendering for the Dictating screen.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::tui::theme::Colors;

/// Interpolate between two colors based on a ratio (0.0 to 1.0).
fn lerp_color(c1: Color, c2: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (c1, c2) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let r = (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u8;
            let g = (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u8;
            let b = (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u8;
            Color::Rgb(r, g, b)
        }
        _ => c1,
    }
}

/// Compute a smooth gradient color based on row position (0.0 = bottom, 1.0 = top).
/// Uses standard audio metering colors: green → yellow → red.
fn gradient_color(position: f32) -> Color {
    let pos = position.clamp(0.0, 1.0);

    if pos < 0.5 {
        // Bottom 50%: green → yellow
        lerp_color(Colors::meter_green(), Colors::meter_yellow(), pos / 0.5)
    } else {
        // Top 50%: yellow → red
        lerp_color(
            Colors::meter_yellow(),
            Colors::meter_red(),
            (pos - 0.5) / 0.5,
        )
    }
}

/// Render the EQ spectrum with optional preview text.
pub fn render_eq(
    f: &mut ratatui::Frame,
    area: Rect,
    hold_active: bool,
    vad_open: bool,
    spectrum_bars: &[u16],
    preview_text: &str,
    hotkey_text: &str,
) {
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

    let width = inner.width as usize;

    // Reserve top line for preview text only while listening (PTT held)
    let text_rows = if inner.height > 1 && hold_active && !preview_text.is_empty() {
        1
    } else {
        0
    };

    // Render preview text at top if present and listening
    if text_rows > 0 && hold_active && !preview_text.is_empty() {
        let mut text_area = inner;
        text_area.height = 1;

        // Truncate text if it exceeds width
        let display_text = if preview_text.len() > width {
            &preview_text[..width]
        } else {
            preview_text
        };

        let text_line = Line::from(Span::styled(
            display_text,
            Style::default().fg(Colors::text_muted()),
        ));
        f.render_widget(Paragraph::new(text_line), text_area);
    }

    // Adjust visualizer area to start below text
    inner.y += text_rows;
    inner.height = inner.height.saturating_sub(text_rows);

    let rows = inner.height.max(1) as usize;

    if rows == 0 {
        return;
    }

    let bar_set = symbols::bar::NINE_LEVELS;
    let max_value = 100u16;

    // Render bars with 8-level precision
    // j iterates high to low: j=high renders at bottom, j=low renders at top
    for j in (0..rows).rev() {
        let mut row_area = inner;
        row_area.y += j as u16; // j high = bottom row, j low = top row
        row_area.height = 1;
        let mut spans: Vec<Span> = Vec::with_capacity(width);

        // Compute gradient position for this row (0.0 = bottom, 1.0 = top)
        let row_position = (rows - j - 1) as f32 / rows.max(1) as f32;

        let total = spectrum_bars.len().max(1);
        for x in 0..width {
            let idx = x * total / width;
            let v = *spectrum_bars.get(idx).unwrap_or(&0);

            // Map value (0-100) to row * 8 range for sub-row precision
            let scaled = v as u64 * rows as u64 * 8 / max_value as u64;
            let tmp = rows - j - 1; // Position from bottom (0 = bottom, rows-1 = top)
            let remainder = scaled.saturating_sub(tmp as u64 * 8);

            let symbol = if !hold_active {
                // When PTT not held (idle), show single grey line at bottom
                if j == rows - 1 {
                    bar_set.half
                } else {
                    bar_set.empty
                }
            } else if !vad_open {
                // PTT held but VAD closed (waiting for speech), show nothing
                bar_set.empty
            } else {
                // Active visualization with 8-level precision
                match remainder {
                    0 => bar_set.empty,
                    1 => bar_set.one_eighth,
                    2 => bar_set.one_quarter,
                    3 => bar_set.three_eighths,
                    4 => bar_set.half,
                    5 => bar_set.five_eighths,
                    6 => bar_set.three_quarters,
                    7 => bar_set.seven_eighths,
                    _ => bar_set.full,
                }
            };

            // Color: smooth gradient from bottom (blue) to top (bright cyan)
            let zone_color = if !hold_active || !vad_open {
                Colors::text_muted()
            } else {
                gradient_color(row_position)
            };

            spans.push(Span::styled(
                symbol.to_string(),
                Style::default().fg(zone_color),
            ));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), row_area);
    }

    // Idle hint overlay when not holding PTT (centered both horizontally and vertically)
    if !hold_active {
        let spans = vec![
            Span::styled(
                "press and hold ",
                Style::default().fg(Colors::text_secondary()),
            ),
            Span::styled(
                hotkey_text.to_string(),
                Style::default()
                    .fg(Colors::text_primary())
                    .bg(Colors::bg_selected())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to start speaking",
                Style::default().fg(Colors::text_secondary()),
            ),
        ];
        let line = Line::from(spans);
        let mut center = inner;
        center.y = center
            .y
            .saturating_add(center.height.saturating_div(2).saturating_sub(1));
        center.height = 1;
        let p = Paragraph::new(line).alignment(ratatui::layout::Alignment::Center);
        f.render_widget(p, center);
    }
}
