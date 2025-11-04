//! Shared logs panel rendering.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};

use crate::tui::theme::{Colors, Theme};

/// Render a rolling logs list with syntax highlighting.
pub fn render_logs(f: &mut ratatui::Frame, area: Rect, logs: &std::collections::VecDeque<String>) {
    let block = Theme::block("📋 logs");
    f.render_widget(block, area);
    // Calculate inner area by subtracting border insets (1 char on each side).
    let mut inner = area;
    inner.x += 1;
    inner.y += 1;
    inner.width = inner.width.saturating_sub(2);
    inner.height = inner.height.saturating_sub(2);
    let maxw = inner.width as usize;
    let capacity = inner.height as usize;
    let total = logs.len();
    // Take only the last N lines that fit; skip older lines that don't fit.
    let take_n = capacity.min(total);
    let skip_n = total.saturating_sub(take_n);

    // Pre-allocate for capacity to avoid reallocs during extend.
    let mut items: Vec<ListItem> = Vec::with_capacity(capacity);
    // Pad with blank lines at top if fewer logs than available rows (visual alignment).
    let pad = capacity.saturating_sub(take_n);
    for _ in 0..pad {
        items.push(ListItem::new(Line::raw("")));
    }

    items.extend(logs.iter().skip(skip_n).map(|line| {
        // Borrow string slice for pattern matching; will borrow substrings below.
        let mut content = line.as_str();
        // Parse log level prefix for color-coding; check longest prefixes first (WARNING before WARN).
        let (level_style, prefix) = if content.starts_with("ERROR") {
            (Style::default().fg(Colors::error_pink()), "ERROR")
        } else if content.starts_with("DEBUG") {
            (Theme::text_muted(), "DEBUG")
        } else if content.starts_with("INFO") {
            (Style::default().fg(Colors::primary()), "INFO")
        } else if content.starts_with("WARN") || content.starts_with("WARNING") {
            (Style::default().fg(Colors::amber()), "WARN")
        } else if content.starts_with("TRACE") {
            (Style::default().fg(Colors::vad_left()), "TRACE")
        } else if content.starts_with("Final") || content.contains("Final:") {
            (Style::default().fg(Colors::success()), "Final")
        } else {
            (Theme::text_secondary(), "")
        };

        let mut spans: Vec<Span> = Vec::new();
        if !prefix.is_empty() {
            spans.push(Span::styled(prefix, level_style));
            // Strip level prefix: find first space, skip it (+1) to get message content.
            if let Some(idx) = content.find(' ') {
                content = &content[idx + 1..];
            } else {
                // No space found: entire line is prefix, no message content.
                content = "";
            }
            spans.push(Span::raw(" "));
        }

        // Tokenize remaining content by whitespace; add spacing between tokens.
        let mut first = true;
        for tok in content.split_whitespace() {
            if !first {
                spans.push(Span::raw(" "));
            }
            first = false;
            // Syntax highlighting: URLs → blue, paths → yellow, numbers → teal, else default.
            // Clone token string for Span::styled (needs owned String).
            let styled = if tok.starts_with("http://") || tok.starts_with("https://") {
                Span::styled(tok.to_string(), Style::default().fg(Colors::vad_left()))
            } else if tok.contains('/') || tok.contains('\\') {
                // Path-like tokens (forward/backslash); handles both Unix and Windows paths.
                Span::styled(tok.to_string(), Style::default().fg(Colors::soft_yellow()))
            } else if tok.parse::<f64>().is_ok() || tok.chars().all(|c| c.is_ascii_digit()) {
                // Numbers: try f64 parse first (handles decimals), else check all ASCII digits.
                Span::styled(tok.to_string(), Style::default().fg(Colors::teal_focus()))
            } else {
                Span::styled(tok.to_string(), Theme::text_secondary())
            };
            spans.push(styled);
        }

        // Truncate line if it exceeds available width; add ellipsis if space allows.
        let mut line = Line::from(spans);
        if line.width() > maxw {
            let mut s = line.to_string();
            if s.len() > maxw && maxw > 1 {
                // Truncate to maxw-1 to leave room for ellipsis; only if width > 1.
                s.truncate(maxw - 1);
                s.push('…');
            } else if maxw == 0 {
                // Zero-width area: clear content to avoid rendering issues.
                s.clear();
            }
            line = Line::from(Span::styled(s, Theme::text_secondary()));
        }
        ListItem::new(line)
    }));

    let list = List::new(items);
    f.render_widget(list, inner);
}
