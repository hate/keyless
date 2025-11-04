//! Reusable hotkey token rendering helpers for footers and hints.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use crate::tui::theme::Theme;

/// Logical segment used for both rendering and layout estimation.
pub enum HotkeySeg {
    /// Plain text between tokens (e.g., " start  ")
    Text(&'static str),
    /// Single-key token like \[q] or \[esc]
    Single(&'static str),
    /// Combo token like [←|→] or [n|m]
    Combo(&'static str, &'static str),
}

/// Render a single hotkey token like: \[q]
pub fn hotkey_single(label: &str, key_style: Style) -> Vec<Span<'static>> {
    // Clone label to owned String for Span::styled (needs 'static lifetime).
    vec![
        Span::styled("[", Theme::key_bracket()),
        Span::styled(label.to_string(), key_style),
        Span::styled("]", Theme::key_bracket()),
    ]
}

/// Render a combo hotkey like: [←|→] or [n|m]
pub fn hotkey_combo(left: &str, right: &str, key_style: Style) -> Vec<Span<'static>> {
    // Clone both labels to owned Strings for Span::styled (needs 'static lifetime).
    vec![
        Span::styled("[", Theme::key_bracket()),
        Span::styled(left.to_string(), key_style),
        Span::styled("|", Theme::key_bracket()),
        Span::styled(right.to_string(), key_style),
        Span::styled("]", Theme::key_bracket()),
    ]
}

/// Render the footer content off-screen and measure exact wrapped line count.
/// Returns total footer height including borders.
pub fn measure_lines_needed(full_width: u16, segments: &[HotkeySeg]) -> u16 {
    // Inner width inside the rounded border box (subtract 2 for left+right borders).
    let inner_width = full_width.saturating_sub(2);
    // Early return for degenerate case: too narrow to render anything meaningful.
    if inner_width <= 1 {
        return 3; // minimal height (1 content line + 2 borders)
    }

    // Build spans exactly like footer rendering (styling does not affect width).
    // Must match footer.rs logic to get accurate measurements.
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut prev_was_token = false;
    for seg in segments {
        match seg {
            HotkeySeg::Text(s) => {
                // If previous segment was a token, replace first space with non-breaking space
                // to prevent awkward line breaks between token and text.
                if prev_was_token {
                    let bridged = if s.starts_with(' ') {
                        // Replace first space only (replacen with count 1) to prevent wrapping.
                        s.replacen(" ", "\u{00A0}", 1)
                    } else {
                        (*s).to_string()
                    };
                    spans.push(Span::raw(bridged));
                } else {
                    spans.push(Span::raw(*s));
                }
                prev_was_token = false;
            }
            HotkeySeg::Single(k) => {
                spans.extend(hotkey_single(k, Style::default()));
                prev_was_token = true;
            }
            HotkeySeg::Combo(l, r) => {
                spans.extend(hotkey_combo(l, r, Style::default()));
                prev_was_token = true;
            }
        }
    }

    // Render into an off-screen buffer with conservative max height (8 lines is plenty).
    // Actual height may be less; we'll count used rows below.
    let max_h: u16 = 8;
    let area = Rect {
        x: 0,
        y: 0,
        width: inner_width,
        height: max_h,
    };
    let mut buf = Buffer::empty(area);
    // Wrap with trim=true to remove trailing spaces; center alignment matches footer.
    let paragraph = Paragraph::new(Line::from(spans))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);
    paragraph.render(area, &mut buf);

    // Count used rows by scanning bottom-up: find last row with any non-space content.
    // Early break per row (if any char found, mark row as used and continue to next).
    let mut used_rows: u16 = 0;
    for y in 0..max_h {
        let mut any = false;
        for x in 0..inner_width {
            let sym = buf[(x, y)].symbol();
            // Check for non-empty and non-space; single space check is sufficient (trim handles trailing).
            if !sym.is_empty() && sym != " " {
                any = true;
                break;
            }
        }
        if any {
            // y is 0-indexed; used_rows is 1-indexed count (so y+1).
            used_rows = y + 1;
        }
    }
    // Ensure at least 1 content line even if buffer is empty (defensive).
    let content_lines = used_rows.max(1);
    content_lines + 2 // add borders (top + bottom)
}
