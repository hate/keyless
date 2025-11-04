//! Status indicator rendering for the Dictating screen.

use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::tui::theme::Colors;

/// Render the status block (listening/idle and gate state).
pub fn render_status(f: &mut ratatui::Frame, area: Rect, hold_active: bool, vad_open: bool) {
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Colors::border()));
    f.render_widget(status_block, area);
    let mut inner = area;
    // Inset by 1 cell on all sides to account for border; saturating ops prevent underflow.
    inner.x += 1;
    inner.y += 1;
    inner.width = inner.width.saturating_sub(2);
    inner.height = inner.height.saturating_sub(2);

    // Select status text based on PTT state: "LISTENING" when held, "IDLE" otherwise.
    let text = if hold_active { "LISTENING" } else { "IDLE" };
    // Color: bright listening color when active, muted grey when idle for visual de-emphasis.
    let color = if hold_active {
        Colors::listening()
    } else {
        Colors::text_muted()
    };

    // Build status line:
    // - When listening (PTT held): show LISTENING · VAD: {OPEN|CLOSED}
    // - When idle: show only IDLE (hide VAD state entirely since it's not relevant)
    let mut spans: Vec<Span> = vec![Span::styled(
        text,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )];

    if hold_active {
        // Render VAD state only when listening; separator "·" provides visual spacing.
        spans.push(Span::raw("   ·   "));
        spans.push(Span::styled(
            "VAD:",
            Style::default()
                .fg(Colors::text_primary())
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
        if vad_open {
            // VAD open: green (success) indicates speech is being detected and processed.
            spans.push(Span::styled(
                "OPEN",
                Style::default()
                    .fg(Colors::meter_green())
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            // VAD closed: red indicates waiting for speech (below threshold).
            spans.push(Span::styled(
                "CLOSED",
                Style::default()
                    .fg(Colors::meter_red())
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }

    let line = ratatui::text::Line::from(spans);
    // Center-align status text for visual balance within the status block.
    let p = Paragraph::new(line).alignment(Alignment::Center);
    f.render_widget(p, inner);
}
