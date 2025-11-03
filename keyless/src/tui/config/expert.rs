//! Expert mode overlay for EQ tuning and config management.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use crate::tui::state::AppState;
use crate::tui::theme::{Colors, Theme};

/// EQ parameters that can be adjusted in Expert mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EqParameter {
    Bands = 0,
    NoiseReduction = 1,
    WindowDb = 2,
    Gamma = 3,
    Attack = 4,
    Decay = 5,
}

impl EqParameter {
    /// Total number of EQ parameters.
    pub const COUNT: u8 = 6;

    /// Get the display name for this parameter.
    pub fn name(self) -> &'static str {
        match self {
            Self::Bands => "bands",
            Self::NoiseReduction => "noise reduction",
            Self::WindowDb => "window dB",
            Self::Gamma => "gamma",
            Self::Attack => "attack",
            Self::Decay => "decay",
        }
    }

    /// Convert from u8 index.
    pub fn from_index(index: u8) -> Self {
        match index % Self::COUNT {
            0 => Self::Bands,
            1 => Self::NoiseReduction,
            2 => Self::WindowDb,
            3 => Self::Gamma,
            4 => Self::Attack,
            5 => Self::Decay,
            _ => unreachable!("index % 6 is always 0-5"),
        }
    }

    /// Convert to u8 index.
    pub fn to_index(self) -> u8 {
        self as u8
    }

    /// Move to the next parameter (wraps around).
    pub fn next(self) -> Self {
        Self::from_index((self as u8 + 1) % Self::COUNT)
    }

    /// Move to the previous parameter (wraps around).
    pub fn prev(self) -> Self {
        Self::from_index((self as u8 + Self::COUNT - 1) % Self::COUNT)
    }
}

pub fn render_expert(f: &mut ratatui::Frame, full: Rect, app: &AppState) {
    let Some(expert) = app.overlays.expert.as_ref() else {
        return;
    };

    // Centered panel dimensions
    let w = full.width.saturating_sub(10).min(80);
    let h = 20u16;
    let x = full.x + (full.width.saturating_sub(w)) / 2;
    let y = full.y + (full.height.saturating_sub(h)) / 2;
    let area = Rect {
        x,
        y,
        width: w,
        height: h,
    };

    // Clear underlay and draw panel
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(Span::styled(
            "⚙ expert mode",
            Style::default().fg(Colors::text_primary()),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Colors::border()))
        .title_style(
            Style::default()
                .fg(Colors::accent_bright())
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Colors::bg_medium()));
    f.render_widget(block, area);

    let mut inner = area;
    inner.x += 2;
    inner.y += 1;
    inner.width = inner.width.saturating_sub(4);
    inner.height = inner.height.saturating_sub(2);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // eq frame (6 lines + 2 border)
            Constraint::Min(0),
        ])
        .split(inner);

    let label = |name: &str| Span::styled(name.to_string(), Theme::label());
    let val = |s: String, selected: bool| {
        let base = if selected {
            Colors::success()
        } else {
            Colors::text_primary()
        };
        let style = if selected {
            Style::default().fg(base).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(base)
        };
        Span::styled(s, style)
    };

    // EQ framed section
    let selected_label = expert.selected.name();
    let eq_frame = Block::default()
        .title(Span::styled(
            format!("eq tuning — selected: {}", selected_label),
            Style::default()
                .fg(Colors::accent_dim())
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Colors::border()));
    f.render_widget(eq_frame, sections[0]);

    let mut eq_inner = sections[0];
    eq_inner.x += 1;
    eq_inner.y += 1;
    eq_inner.width = eq_inner.width.saturating_sub(2);
    eq_inner.height = eq_inner.height.saturating_sub(2);

    let eq_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(eq_inner);

    let lines: [Line; 6] = [
        Line::from(vec![
            label("bands: "),
            val(
                format!("{}", app.eq_tuning.bands),
                expert.selected == EqParameter::Bands,
            ),
        ]),
        Line::from(vec![
            label("noise reduction: "),
            val(
                format!("{:.2}", app.eq_tuning.noise_reduction),
                expert.selected == EqParameter::NoiseReduction,
            ),
        ]),
        Line::from(vec![
            label("window dB: "),
            val(
                format!("{:.1}", app.eq_tuning.window_db),
                expert.selected == EqParameter::WindowDb,
            ),
        ]),
        Line::from(vec![
            label("gamma: "),
            val(
                format!("{:.2}", app.eq_tuning.gamma),
                expert.selected == EqParameter::Gamma,
            ),
        ]),
        Line::from(vec![
            label("attack: "),
            val(
                format!("{:.2}", app.eq_tuning.attack),
                expert.selected == EqParameter::Attack,
            ),
        ]),
        Line::from(vec![
            label("decay: "),
            val(
                format!("{:.2}", app.eq_tuning.decay),
                expert.selected == EqParameter::Decay,
            ),
        ]),
    ];

    for (i, line) in lines.into_iter().enumerate() {
        let p = Paragraph::new(line).alignment(Alignment::Left);
        f.render_widget(p, eq_rows[i]);
    }

    // Confirmation overlay
    if expert.confirm_reset {
        let pw = 48u16;
        let ph = 5u16;
        let px = full.x + (full.width.saturating_sub(pw)) / 2;
        let py = full.y + (full.height.saturating_sub(ph)) / 2;
        let prompt = Rect {
            x: px,
            y: py,
            width: pw,
            height: ph,
        };
        f.render_widget(Clear, prompt);
        let pblock = Block::default()
            .title(Span::styled(
                "confirm reset",
                Style::default()
                    .fg(Colors::amber())
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Colors::border()))
            .style(Style::default().bg(Colors::bg_medium()));
        f.render_widget(pblock, prompt);

        let mut pin = prompt;
        pin.x += 2;
        pin.y += 2;
        pin.width = pin.width.saturating_sub(4);
        pin.height = pin.height.saturating_sub(4);

        let msg = Paragraph::new(Line::from(vec![
            Span::styled("reset config to defaults? ", Theme::text_primary()),
            Span::styled(
                "y",
                Style::default()
                    .fg(Colors::success())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" / "),
            Span::styled(
                "n",
                Style::default()
                    .fg(Colors::error_pink())
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .alignment(Alignment::Center);
        f.render_widget(msg, pin);
    }

    // Delete models confirmation
    if expert.confirm_purge {
        let pw = 56u16;
        let ph = 5u16;
        let px = full.x + (full.width.saturating_sub(pw)) / 2;
        let py = full.y + (full.height.saturating_sub(ph)) / 2;
        let prompt = Rect {
            x: px,
            y: py,
            width: pw,
            height: ph,
        };
        f.render_widget(Clear, prompt);
        let pblock = Block::default()
            .title(Span::styled(
                "delete all downloaded models",
                Style::default()
                    .fg(Colors::amber())
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Colors::border()))
            .style(Style::default().bg(Colors::bg_medium()));
        f.render_widget(pblock, prompt);
        let mut pin = prompt;
        pin.x += 2;
        pin.y += 2;
        pin.width = pin.width.saturating_sub(4);
        pin.height = pin.height.saturating_sub(4);
        let msg = Paragraph::new(Line::from(vec![
            Span::styled("this cannot be undone — ", Theme::text_primary()),
            Span::styled(
                "y",
                Style::default()
                    .fg(Colors::success())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" / "),
            Span::styled(
                "n",
                Style::default()
                    .fg(Colors::error_pink())
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .alignment(Alignment::Center);
        f.render_widget(msg, pin);
    }
}
