//! Footer rendering for the Dictating screen.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::tui::theme::{Colors, Theme};

/// Render footer hotkeys for Dictating view.
pub fn render_footer(f: &mut ratatui::Frame, area: Rect, hotkey_text: &str) {
    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Colors::border()))
        .style(Style::default().bg(Colors::bg_medium()));
    f.render_widget(footer_block, area);
    let mut inner = area;
    inner.x += 1;
    inner.y += 1;
    inner.width = inner.width.saturating_sub(2);
    inner.height = inner.height.saturating_sub(2);

    use crate::tui::widgets::hotkey::{HotkeySeg, hotkey_single};

    let segs = segments_for_footer();
    let mut spans: Vec<Span> = Vec::new();
    let mut prev_was_token = false;
    for seg in &segs {
        match seg {
            HotkeySeg::Text(t) => {
                if prev_was_token {
                    let bridged = if t.starts_with(' ') {
                        t.replacen(" ", "\u{00A0}", 1)
                    } else {
                        (*t).to_string()
                    };
                    spans.push(Span::styled(bridged, Theme::text_primary()));
                } else {
                    spans.push(Span::styled((*t).to_string(), Theme::text_primary()));
                }
                prev_was_token = false;
            }
            HotkeySeg::Single(k) => {
                let style = match *k {
                    "esc" => Style::default()
                        .fg(Colors::vad_right())
                        .add_modifier(Modifier::BOLD),
                    "q" => Style::default()
                        .fg(Colors::device_accent())
                        .add_modifier(Modifier::BOLD),
                    _ => Theme::text_primary(),
                };
                spans.extend(hotkey_single(k, style));
                prev_was_token = true;
            }
            HotkeySeg::Combo(_, _) => {}
        }
    }

    // Append dynamic hotkey segment: [h] hotkey: {hotkey_text}
    {
        use crate::tui::widgets::hotkey::hotkey_single;
        spans.push(Span::raw("  "));
        let token_style = Theme::text_primary().add_modifier(Modifier::BOLD);
        spans.extend(hotkey_single("h", token_style));
        spans.push(Span::raw(" hotkey: "));
        spans.push(Span::styled(hotkey_text.to_string(), Theme::value_accent()));
    }

    let footer = Paragraph::new(ratatui::text::Line::from(spans))
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(footer, inner);
}

/// Return the logical hotkey segments used by the Dictating footer.
pub fn segments_for_footer() -> Vec<crate::tui::widgets::hotkey::HotkeySeg> {
    use crate::tui::widgets::hotkey::HotkeySeg::*;
    vec![
        Single("esc"),
        Text(" back to config  "),
        Single("q"),
        Text(" quit"),
    ]
}
