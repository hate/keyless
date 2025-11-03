//! Theme helpers for consistent TUI styling with a dark grey base and light blue accents.

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

/// Refined color palette - dark greys with light blue accent
pub struct Colors;

impl Colors {
    /// Primary accent - light cyan-blue (#5eb3f6)
    #[inline]
    pub fn primary() -> Color {
        Color::Rgb(94, 179, 246) // #5eb3f6
    }

    /// Brighter accent for hover/active emphasis (#7dc4ff)
    #[inline]
    pub fn accent_bright() -> Color {
        Color::Rgb(125, 196, 255) // #7dc4ff
    }

    /// Dimmer accent (#4a8fc4)
    #[inline]
    pub fn accent_dim() -> Color {
        Color::Rgb(74, 143, 196) // #4a8fc4
    }

    /// Default border - dark grey (#2a2a2a)
    #[inline]
    pub fn border() -> Color {
        Color::Rgb(42, 42, 42) // #2a2a2a
    }

    // Focused/active border uses accent where needed; no dedicated function required

    /// Light grey primary text (#e5e5e5)
    #[inline]
    pub fn text_primary() -> Color {
        Color::Rgb(229, 229, 229) // #e5e5e5
    }

    /// Medium grey secondary text (#a0a0a0)
    #[inline]
    pub fn text_secondary() -> Color {
        Color::Rgb(160, 160, 160) // #a0a0a0
    }

    /// Dim grey muted/disabled (#5a5a5a)
    #[inline]
    pub fn text_muted() -> Color {
        Color::Rgb(90, 90, 90) // #5a5a5a
    }

    /// Label grey (#8a8a8a)
    #[inline]
    pub fn label() -> Color {
        Color::Rgb(138, 138, 138) // #8a8a8a
    }

    // (cards use bg_selected or explicit backgrounds)

    /// Main background - pure black (#000000)
    #[inline]
    pub fn bg_dark() -> Color {
        Color::Rgb(0, 0, 0) // #000000
    }

    /// Hint/info/footers background - slightly lighter (#0f0f0f)
    #[inline]
    pub fn bg_medium() -> Color {
        Color::Rgb(15, 15, 15) // #0f0f0f
    }

    /// Subtle selected background (neutral grey #1a1a1a)
    #[inline]
    pub fn bg_selected() -> Color {
        Color::Rgb(26, 26, 26) // #1a1a1a
    }

    // Domain-specific accents (centralize hardcoded colors)
    #[inline]
    pub fn sink_paste() -> Color {
        Color::Green
    }

    #[inline]
    pub fn sink_clipboard() -> Color {
        Color::Yellow
    }

    #[inline]
    pub fn sink_file() -> Color {
        Color::Blue
    }

    #[inline]
    pub fn device_accent() -> Color {
        Color::LightRed
    }

    #[inline]
    pub fn vad_left() -> Color {
        Color::Rgb(187, 154, 247)
    } // purple

    #[inline]
    pub fn vad_right() -> Color {
        Color::Rgb(255, 165, 0)
    } // orange

    /// Teal focus accent (#4fd6be)
    #[inline]
    pub fn teal_focus() -> Color {
        Color::Rgb(79, 214, 190)
    }

    /// Soft green success (#73daca)
    #[inline]
    pub fn success() -> Color {
        Color::Rgb(115, 218, 202)
    }

    /// Download icon (#06f51a)
    #[inline]
    pub fn downloaded() -> Color {
        Color::Rgb(6, 245, 26) // #06f51a
    }

    /// Soft yellow metadata (#c0a36e)
    #[inline]
    pub fn soft_yellow() -> Color {
        Color::Rgb(192, 163, 110)
    }

    /// Pink error (#f7768e)
    #[inline]
    pub fn error_pink() -> Color {
        Color::Rgb(247, 118, 142)
    }

    /// Amber/orange warning (#e0af68)
    #[inline]
    pub fn amber() -> Color {
        Color::Rgb(224, 175, 104)
    }

    // Listening status - green (#66ff66)
    #[inline]
    pub fn listening() -> Color {
        Color::Rgb(115, 218, 117)
    }

    // Audio meter gradient colors
    /// Audio meter green - bottom zone (#50FA7B)
    #[inline]
    pub fn meter_green() -> Color {
        Color::Rgb(80, 250, 123)
    }

    /// Audio meter yellow - middle zone (#F1FA8C)
    #[inline]
    pub fn meter_yellow() -> Color {
        Color::Rgb(241, 250, 140)
    }

    /// Audio meter red - peak zone (#FF5555)
    #[inline]
    pub fn meter_red() -> Color {
        Color::Rgb(255, 85, 85)
    }
}

/// Predefined style palette for the TUI.
pub struct Theme;

impl Theme {
    /// Label text - dedicated label grey, bold
    #[inline]
    pub fn label() -> Style {
        Style::default()
            .fg(Colors::label())
            .add_modifier(Modifier::BOLD)
    }

    /// Value accent - primary accent color (light blue)
    #[inline]
    pub fn value_accent() -> Style {
        Style::default().fg(Colors::primary())
    }

    /// Warning/attention - use brighter accent for emphasis
    #[inline]
    pub fn warn() -> Style {
        Style::default()
            .fg(Colors::accent_bright())
            .add_modifier(Modifier::BOLD)
    }

    /// Selected item - neutral light text on subtle grey background
    #[inline]
    pub fn selected() -> Style {
        Style::default()
            .fg(Colors::text_primary())
            .bg(Colors::bg_selected())
            .add_modifier(Modifier::BOLD)
    }

    /// Primary text - bright, for headers
    #[inline]
    pub fn text_primary() -> Style {
        Style::default().fg(Colors::text_primary())
    }

    /// Secondary text - medium brightness
    #[inline]
    pub fn text_secondary() -> Style {
        Style::default().fg(Colors::text_secondary())
    }

    /// Muted text - dim, for disabled items
    #[inline]
    pub fn text_muted() -> Style {
        Style::default()
            .fg(Colors::text_muted())
            .add_modifier(Modifier::DIM)
    }

    // (no separate success style; use value_accent instead)

    /// Standard rounded border block with a title - refined border color
    #[inline]
    pub fn block<'a>(title: &'a str) -> Block<'a> {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Colors::border()))
            .title_style(Style::default().fg(Colors::text_primary()))
            .title(title)
    }

    /// Key bracket style for hotkey hints
    #[inline]
    pub fn key_bracket() -> Style {
        Style::default()
            .fg(Colors::text_muted())
            .add_modifier(Modifier::BOLD)
    }

    /// Active indicator symbol
    #[inline]
    pub fn active_indicator() -> &'static str {
        "▶"
    }

    /// Inactive indicator symbol
    #[inline]
    pub fn inactive_indicator() -> &'static str {
        "○"
    }
}
