//! Shared brand/header widget: renders the app title and an optional blinking character

use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui::theme::Theme;

/// Options for rendering the brand header.
pub struct BrandOptions<'a> {
    /// The main title text, e.g., "keyless".
    /// If `blinking_text` is provided, it blinks immediately after the title
    /// (use "█" to achieve a fat blinking caret effect).
    pub title: &'a str,
    /// Optional inline blinking text rendered immediately after the caret.
    /// When the blink state is OFF, the same number of spaces are rendered
    /// to keep the layout stable.
    pub blinking_text: Option<&'a str>,
}

/// Render the brand header inside the given area (single line), centered.
pub fn render_brand(f: &mut Frame, area: Rect, opts: BrandOptions<'_>) {
    let mut lines: Vec<Line> = Vec::with_capacity(2);
    // Blink phase (used only when blinking_text is provided)
    let caret_on = (keyless_core::utils::now_millis() / 500).is_multiple_of(2);
    let mut title_line: Vec<Span> = vec![Span::styled(
        opts.title,
        Theme::text_primary().add_modifier(Modifier::BOLD),
    )];
    if let Some(txt) = opts.blinking_text {
        let display = if caret_on {
            txt.to_string()
        } else {
            " ".repeat(txt.chars().count())
        };
        title_line.push(Span::styled(display, Theme::text_primary()));
    }
    lines.push(Line::from(title_line));
    // No second line; header is intentionally minimal.
    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, area);
}
