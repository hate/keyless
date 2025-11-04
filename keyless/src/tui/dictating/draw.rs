//! Main draw function for the Dictating screen.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::tui::state::{AppState, SinkChoice};
use crate::tui::theme::{Colors, Theme};

use super::{eq, footer, status};

/// Draw the dictating screen with live visualizer and controls.
pub fn draw_dictating(f: &mut ratatui::Frame, app: &AppState, sink_label: &str) {
    let full = f.area();

    // Compute footer height from hotkey segments and terminal width to prevent wrapping.
    // This ensures the footer fits exactly and avoids layout jitter across frames.
    let segs = footer::segments_for_footer();
    let footer_lines = crate::tui::widgets::hotkey::measure_lines_needed(full.width, &segs);

    // Top framed content + footer, mirroring config view layout structure.
    // Reserve at least 10 rows for content; footer uses exact computed length.
    let frame_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(footer_lines)])
        .split(full);

    // Outer frame with black background
    let main_outer = frame_layout[0];
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Colors::border()))
            .style(Style::default().bg(Colors::bg_dark())),
        main_outer,
    );
    let mut content = main_outer;
    // Shrink to inner content area (1-cell border on all sides).
    // Use saturating ops to avoid underflow on very small terminals.
    content.x += 1;
    content.y += 1;
    content.width = content.width.saturating_sub(2);
    content.height = content.height.saturating_sub(2);

    // Inner layout: input card + status + waveform + logs.
    // Fixed heights for card/status; percentage for waveform; Min(6) ensures logs are readable.
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),      // input info card
            Constraint::Length(3),      // status indicator
            Constraint::Percentage(40), // waveform
            Constraint::Min(6),         // logs
        ])
        .split(content);

    // Brand overlay: render title intersecting the top frame border.
    // Lifts the brand one cell above to overlap the outer border.
    {
        use crate::tui::widgets::brand::{BrandOptions, render_brand};
        let brand_area = Rect {
            x: main_outer.x,
            // `saturating_sub` prevents underflow when y == 0.
            y: main_outer.y.saturating_sub(1),
            width: main_outer.width,
            height: 1,
        };
        render_brand(
            f,
            brand_area,
            BrandOptions {
                title: "keyless",
                blinking_text: Some("█"),
            },
        );
    }

    // Input info card
    render_input_card(f, app, sink_label, layout[0]);

    // Status block
    status::render_status(
        f,
        layout[1],
        app.feedback.hold_active,
        app.feedback.vad_open,
    );

    // EQ bars with preview text
    eq::render_eq(
        f,
        layout[2],
        app.feedback.hold_active,
        app.feedback.vad_open,
        &app.feedback.spectrum_bars,
        &app.feedback.preview_text,
        &app.hotkey,
    );

    // Logs
    crate::tui::widgets::logs::render_logs(f, layout[3], &app.feedback.logs);

    // Footer
    footer::render_footer(f, frame_layout[1], &app.hotkey);
}

/// Render the top input card with mic/sink/model/language.
fn render_input_card(f: &mut ratatui::Frame, app: &AppState, sink_label: &str, area: Rect) {
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Colors::border()))
        .title("ℹ️ info")
        .title_style(Style::default().fg(Colors::text_primary()));
    f.render_widget(input_block, area);
    let mut inner = area;
    // Inset by 2 cells horizontally (more than standard 1) for card aesthetics.
    // Vertical inset is 1 cell; saturating ops prevent underflow.
    inner.x += 2;
    inner.y += 1;
    inner.width = inner.width.saturating_sub(4);
    inner.height = inner.height.saturating_sub(2);

    // Four-row layout: blank padding, info row 1, info row 2, blank padding.
    // Blank rows provide visual breathing room inside the card.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // blank
            Constraint::Length(1), // mic | sink | time
            Constraint::Length(1), // model | language | words
            Constraint::Length(1), // blank
        ])
        .split(inner);

    // mic | sink | time (three columns)
    let cols1 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
        .split(rows[1]);
    // Borrow mic_name from app state; no clone needed since we're just rendering.
    let row1_left = Paragraph::new(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled("mic: ", Theme::label()),
        ratatui::text::Span::styled(
            &app.feedback.mic_name,
            Style::default().fg(Colors::device_accent()),
        ),
    ]));
    f.render_widget(row1_left, cols1[0]);
    // Match sink color to choice enum for visual consistency with config screen.
    let sink_color = match app.selections.sink_choice {
        SinkChoice::Paste => Colors::sink_paste(),
        SinkChoice::Clipboard => Colors::sink_clipboard(),
        SinkChoice::File => Colors::sink_file(),
    };
    let row1_mid = Paragraph::new(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled("sink: ", Theme::label()),
        ratatui::text::Span::styled(sink_label, Style::default().fg(sink_color)),
    ]));
    f.render_widget(row1_mid, cols1[1]);
    // Time value converted from milliseconds to HH:MM:SS string; assumes monotonic counter.
    let row1_right = Paragraph::new(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled("time: ", Theme::label()),
        ratatui::text::Span::styled(
            keyless_core::utils::fmt_hms(app.feedback.session_talk_ms),
            Style::default().fg(Colors::text_primary()),
        ),
    ]));
    f.render_widget(row1_right, cols1[2]);

    // model | language | words
    // Lookup model name from runtime list; fallback to "unknown" if index is out of bounds.
    // Assumes upstream logic keeps model_idx valid, but defensive fallback prevents panic.
    let model_name = app
        .models
        .runtime_list
        .get(app.selections.model_idx)
        .map(|s| s.as_str())
        .unwrap_or("unknown");
    // Language label logic: index 0 is "auto" (multilingual models), indices 1+ map to
    // LANG_CODES with offset -1. Fallback to "en" if index is out of bounds.
    let lang_label = if app.selections.language_idx == 0 {
        "★ auto".to_string()
    } else {
        // Subtract 1 because index 0 is reserved for "auto"; LANG_CODES starts at index 0.
        let code = keyless_core::options::LANG_CODES
            .get(app.selections.language_idx - 1)
            .copied()
            .unwrap_or("en");
        keyless_core::options::lang_name(code).to_string()
    };
    let cols2 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
        .split(rows[2]);
    let row2_left = Paragraph::new(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled("model: ", Theme::label()),
        ratatui::text::Span::styled(model_name, Style::default().fg(Colors::vad_left())),
    ]));
    f.render_widget(row2_left, cols2[0]);
    let row2_mid = Paragraph::new(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled("language: ", Theme::label()),
        ratatui::text::Span::styled(lang_label, Style::default().fg(Colors::error_pink())),
    ]));
    f.render_widget(row2_mid, cols2[1]);
    // Format word count as string; allocates each frame but acceptable for TUI budget.
    let row2_right = Paragraph::new(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled("words: ", Theme::label()),
        ratatui::text::Span::styled(
            format!("{}", app.feedback.session_words),
            Style::default().fg(Colors::text_primary()),
        ),
    ]));
    f.render_widget(row2_right, cols2[2]);
}
