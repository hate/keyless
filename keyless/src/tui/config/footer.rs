//! Footer rendering for the Config screen.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::tui::theme::{Colors, Theme};

/// Render the footer with hotkey hints for the Config screen.
pub fn render_footer(
    f: &mut ratatui::Frame,
    area: Rect,
    expert_mode: bool,
    expert_confirm: bool,
    hotkey_text: &str,
) {
    // Outer framed footer box with darker background
    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Colors::border()))
        .style(Style::default().bg(Colors::bg_medium()));
    f.render_widget(footer_block, area);

    // Inner content area
    let mut inner = area;
    inner.x += 1;
    inner.y += 1;
    inner.width = inner.width.saturating_sub(2);
    inner.height = inner.height.saturating_sub(2);

    use crate::tui::widgets::hotkey::{HotkeySeg, hotkey_combo, hotkey_single};

    let segs = if expert_confirm {
        segments_for_footer_confirm()
    } else if expert_mode {
        segments_for_footer_expert()
    } else {
        segments_for_footer()
    };
    let mut spans: Vec<Span> = Vec::new();
    let mut prev_was_token = false;
    for seg in &segs {
        match seg {
            HotkeySeg::Text(t) => {
                if prev_was_token {
                    // Replace only the first leading space with NBSP to keep token+label together
                    let bridged = if t.starts_with(' ') {
                        t.replacen(" ", "\u{00A0}", 1)
                    } else {
                        (*t).to_string()
                    };
                    spans.push(Span::raw(bridged));
                } else {
                    spans.push(Span::raw(*t));
                }
                prev_was_token = false;
            }
            HotkeySeg::Single(k) => {
                let style = if expert_confirm {
                    match *k {
                        "esc" => Style::default()
                            .fg(Colors::accent_bright())
                            .add_modifier(Modifier::BOLD),
                        "y" => Style::default()
                            .fg(Colors::success())
                            .add_modifier(Modifier::BOLD),
                        "n" => Style::default()
                            .fg(Colors::error_pink())
                            .add_modifier(Modifier::BOLD),
                        _ => Style::default()
                            .fg(Colors::text_primary())
                            .add_modifier(Modifier::BOLD),
                    }
                } else {
                    match *k {
                        "enter" => Style::default()
                            .fg(Colors::text_primary())
                            .add_modifier(Modifier::BOLD),
                        "q" => Theme::text_secondary().add_modifier(Modifier::BOLD),
                        "n" | "m" => Style::default()
                            .fg(Colors::device_accent())
                            .add_modifier(Modifier::BOLD),
                        // Expert-mode: emphasize important keys
                        "esc" => Style::default()
                            .fg(Colors::accent_bright())
                            .add_modifier(Modifier::BOLD),
                        "r" => Style::default()
                            .fg(Colors::amber())
                            .add_modifier(Modifier::BOLD),
                        "o" => Style::default()
                            .fg(Colors::primary())
                            .add_modifier(Modifier::BOLD),
                        _ => Style::default()
                            .fg(Colors::text_primary())
                            .add_modifier(Modifier::BOLD),
                    }
                };
                spans.extend(hotkey_single(k, style));
                prev_was_token = true;
            }
            HotkeySeg::Combo(l, r) => {
                let style = match (*l, *r) {
                    ("←", "→") | ("↑", "↓") | ("[", "]") => Style::default()
                        .fg(Colors::text_primary())
                        .add_modifier(Modifier::BOLD),
                    ("-", "_") | ("1", "2") => Style::default()
                        .fg(Colors::vad_left())
                        .add_modifier(Modifier::BOLD),
                    ("=", "+") | ("3", "4") => Style::default()
                        .fg(Colors::vad_right())
                        .add_modifier(Modifier::BOLD),
                    ("n", "m") => Style::default()
                        .fg(Colors::device_accent())
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default()
                        .fg(Colors::text_primary())
                        .add_modifier(Modifier::BOLD),
                };
                spans.extend(hotkey_combo(l, r, style));
                prev_was_token = true;
            }
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

    let hints = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(hints, inner);
}

/// Return the logical hotkey segments used by the Config footer.
pub fn segments_for_footer() -> Vec<crate::tui::widgets::hotkey::HotkeySeg> {
    use crate::tui::widgets::hotkey::HotkeySeg::*;
    vec![
        Single("enter"),
        Text(" start  "),
        Single("q"),
        Text(" quit  "),
        Combo("←", "→"),
        Text(" sink  "),
        Combo("↑", "↓"),
        Text(" model  "),
        Single("backspace"),
        Text(" delete model  "),
        Combo("[", "]"),
        Text(" language  "),
        Combo("n", "m"),
        Text(" device  "),
        Combo("-", "_"),
        Text(" start dB  "),
        Combo("1", "2"),
        Text(" min  "),
        Combo("=", "+"),
        Text(" stop dB  "),
        Combo("3", "4"),
        Text(" silence  "),
        Single("x"),
        Text(" expert mode  "),
    ]
}

/// Hotkeys for Expert Mode overlay.
pub fn segments_for_footer_expert() -> Vec<crate::tui::widgets::hotkey::HotkeySeg> {
    use crate::tui::widgets::hotkey::HotkeySeg::*;
    vec![
        Single("esc"),
        Text(" close  "),
        Single("r"),
        Text(" reset defaults  "),
        Single("d"),
        Text(" delete models  "),
        Combo("↑", "↓"),
        Text(" select  "),
        Combo("←", "→"),
        Text(" adjust "),
        Text("(shift = coarse)  "),
        Single("o"),
        Text(" open config  "),
    ]
}

/// Hotkeys for Expert Mode confirm-reset overlay.
pub fn segments_for_footer_confirm() -> Vec<crate::tui::widgets::hotkey::HotkeySeg> {
    use crate::tui::widgets::hotkey::HotkeySeg::*;
    vec![
        Single("esc"),
        Text(" cancel  "),
        Single("y"),
        Text(" confirm  "),
        Single("n"),
        Text(" cancel  "),
    ]
}
