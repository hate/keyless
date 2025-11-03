//! Main draw function for the Config screen.

use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph};

use crate::tui::state::{AppState, SinkChoice};
use crate::tui::theme::{Colors, Theme};

use super::{device, expert, footer, languages, models, sinks, vad};

/// Draw the configuration screen with all panels and overlays.
pub fn draw_config(
    f: &mut ratatui::Frame,
    app: &AppState,
    sinks_list: &[&str],
    models_list: &[&str],
) {
    let full = f.area();

    // Dynamic footer height based on the footer content and available width (generic estimator)
    let segs = if app
        .overlays
        .expert
        .as_ref()
        .map(|e| e.confirm_reset)
        .unwrap_or(false)
    {
        footer::segments_for_footer_confirm()
    } else if app.expert_mode() {
        footer::segments_for_footer_expert()
    } else {
        footer::segments_for_footer()
    };
    let lines_needed = crate::tui::widgets::hotkey::measure_lines_needed(full.width, &segs);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(lines_needed)])
        .split(full);

    // Outer frame
    let main_outer = layout[0];
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Colors::border()))
            .style(Style::default().bg(Colors::bg_dark())),
        main_outer,
    );
    let mut content = main_outer;
    content.x += 1;
    content.y += 1;
    content.width = content.width.saturating_sub(2);
    content.height = content.height.saturating_sub(2);

    let sink_height: u16 = ((sinks_list.len() as u16) + 2).min(6);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),                  // Top info (metrics + warnings)
            Constraint::Length(sink_height.max(5)), // Sinks
            Constraint::Min(10),                    // Models + (Languages/Devices)
            Constraint::Length(7),                  // VAD + Logs
        ])
        .split(content);
    // Brand overlay: render title intersecting the top frame border
    {
        use crate::tui::widgets::brand::{BrandOptions, render_brand};
        let brand_area = ratatui::layout::Rect {
            x: main_outer.x,
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

    // Lifetime metrics row + Warnings / file path (top info area)
    let extra = chunks[0];
    let extra_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(extra);

    // Lifetime counters (words + talk time)
    let metrics = Paragraph::new(Line::from(vec![
        Span::styled("[total]    words: ", Theme::label()),
        Span::styled(format!("{}", app.lifetime_words), Theme::text_primary()),
        Span::raw("    "),
        Span::styled("time: ", Theme::label()),
        Span::styled(
            keyless_core::utils::fmt_hms(app.lifetime_talk_ms),
            Theme::text_primary(),
        ),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(metrics, extra_rows[0]);

    // Warnings / file path (rendered below metrics)
    let mut warn_lines: Vec<Line> = Vec::new();
    if matches!(app.selections.sink_choice, SinkChoice::File) {
        let mut spans = vec![
            Span::styled("file: ", Theme::label()),
            Span::styled(app.selections.file_path.clone(), Colors::text_primary()),
        ];
        if app.selections.file_edit {
            spans.push(Span::styled(
                " [editing]",
                Style::default().fg(ratatui::style::Color::Yellow),
            ));
        }
        spans.push(Span::styled(
            if app.selections.file_edit {
                "   (esc to exit)"
            } else {
                "   (e to edit)"
            },
            Theme::text_muted(),
        ));
        warn_lines.push(Line::from(spans));
    } else {
        if let Some(err) = &app.overlays.error_message {
            warn_lines.push(Line::from(vec![
                Span::styled("error: ", Theme::warn()),
                Span::styled(err.to_string(), Theme::text_secondary()),
            ]));
        }
        if !app.perm_warnings.is_empty() {
            for w in &app.perm_warnings {
                warn_lines.push(Line::from(vec![
                    Span::styled("permission: ", Theme::warn()),
                    Span::styled(w.as_str(), Theme::text_secondary()),
                ]));
            }
        }
    }
    if !warn_lines.is_empty() {
        let warning_p = Paragraph::new(warn_lines);
        f.render_widget(warning_p, extra_rows[1]);
    }

    // Sinks row
    sinks::render_sinks(f, chunks[1], app, sinks_list);

    // Main content: Models (left) | Languages + Devices (right stacked)
    let main_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(chunks[2]);

    // Left: Models
    let installed_set: std::collections::HashSet<String> =
        app.models.discovered.iter().cloned().collect();
    models::render_models(
        f,
        main_cols[0],
        app.selections.model_idx,
        models_list,
        Some(&app.models.sizes),
        Some(&installed_set),
    );

    // Right: Languages (65%) + Devices (35%) stacked
    let right_stack = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(65), // Languages
            Constraint::Percentage(35), // Devices
        ])
        .split(main_cols[1]);

    languages::render_languages(
        f,
        right_stack[0],
        models_list,
        app.selections.model_idx,
        app.selections.language_idx,
    );
    device::render_devices(f, right_stack[1], app);

    // Bottom: VAD + Logs (side by side)
    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[3]);

    vad::render_vad(f, bottom_cols[0], app);
    crate::tui::widgets::logs::render_logs(f, bottom_cols[1], &app.feedback.logs);

    // Footer (shows confirm overlay hotkeys when confirm is active)
    footer::render_footer(
        f,
        layout[1],
        app.expert_mode(),
        app.overlays
            .expert
            .as_ref()
            .map(|e| e.confirm_reset)
            .unwrap_or(false),
        &app.hotkey,
    );

    // Expert overlay (centered)
    if app.expert_mode() {
        expert::render_expert(f, full, app);
    }

    // Download overlay (centered, modal)
    if app.is_downloading() {
        let is_loading = app
            .overlays
            .download
            .as_ref()
            .map(|d| d.model.as_str() == "loading")
            .unwrap_or(false);
        let overlay = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Colors::border()))
            .style(Style::default().bg(Colors::bg_selected()))
            .title(if is_loading {
                "loading"
            } else {
                "⬇ downloading model"
            })
            .title_style(Theme::text_primary())
            .title_bottom(if is_loading {
                Line::from(vec![
                    Span::styled("[", Theme::key_bracket()),
                    Span::styled("esc", Theme::text_primary()),
                    Span::styled("] ", Theme::key_bracket()),
                    Span::styled("cancel", Theme::text_muted()),
                ])
                .alignment(Alignment::Center)
            } else {
                Line::from(vec![
                    Span::styled("[", Theme::key_bracket()),
                    Span::styled("esc", Theme::text_primary()),
                    Span::styled("] ", Theme::key_bracket()),
                    Span::styled("pause", Theme::text_muted()),
                    Span::styled("  ", Theme::text_muted()),
                    Span::styled("[", Theme::key_bracket()),
                    Span::styled("backspace", Theme::text_primary()),
                    Span::styled("] ", Theme::key_bracket()),
                    Span::styled("cancel", Theme::text_muted()),
                ])
                .alignment(Alignment::Center)
            });
        // center a box for message + gauge
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45),
                Constraint::Length(4),
                Constraint::Percentage(45),
            ])
            .split(full)[1];
        let inner_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ])
            .split(outer);
        let area = inner_cols[1];
        // Clear the background to make overlay opaque
        f.render_widget(Clear, area);
        f.render_widget(overlay, area);
        let mut msg_area = area;
        msg_area.x += 2;
        msg_area.y += 1;
        msg_area.width = msg_area.width.saturating_sub(4);
        msg_area.height = 1;
        let text = app
            .overlays
            .download
            .as_ref()
            .map(|d| d.message.as_str())
            .unwrap_or("this may take a few minutes on first run...");
        // If text starts with percentage (e.g., "75% model..."), hide it from display
        let display_text = if let Some(pct_end) = text.find('%') {
            let before_pct = &text[..pct_end];
            if before_pct.trim().parse::<u16>().is_ok() {
                // Skip past "NN% " to hide the percentage from user
                text.get(pct_end + 1..).unwrap_or(text).trim_start()
            } else {
                text
            }
        } else {
            text
        };
        let is_stage = display_text.starts_with("plan:")
            || display_text.starts_with("downloading ")
            || display_text.starts_with("starting");
        let stage_style = if is_stage {
            Theme::text_primary().add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Theme::text_primary()
        };
        // render primary line
        let p = Paragraph::new(Line::from(vec![Span::styled(display_text, stage_style)]))
            .style(Style::default().bg(Colors::bg_selected()));
        f.render_widget(p, msg_area);
        // render gauge on second line if percent known
        let mut gauge_area = msg_area;
        gauge_area.y = gauge_area.y.saturating_add(1);
        if let Some(pct_end) = text.find('%') {
            // Extract the percentage number (should be at start of string now)
            if let Ok(val) = text[..pct_end].trim().parse::<u16>() {
                let g = Gauge::default()
                    .gauge_style(Theme::selected())
                    .style(Style::default().bg(Colors::bg_selected()))
                    .ratio({
                        let r = val as f64 / 100.0;
                        if val >= 99 && text.contains("ETA 0s") {
                            1.0
                        } else {
                            r.clamp(0.0, 1.0)
                        }
                    });
                f.render_widget(g, gauge_area);
            }
        }

        // Error toast (bottom-center)
        if let Some(err) = &app.overlays.error_message {
            let toast = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Colors::border()))
                .title("⚠ error");
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(85),
                    Constraint::Length(3),
                    Constraint::Percentage(15),
                ])
                .split(full)[1];
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(50),
                    Constraint::Percentage(25),
                ])
                .split(outer);
            let area = cols[1];
            f.render_widget(toast, area);
            let mut msg = area;
            msg.x += 2;
            msg.y += 1;
            msg.width = msg.width.saturating_sub(4);
            msg.height = 1;
            let p = Paragraph::new(Line::from(vec![Span::styled(
                err.to_string(),
                Theme::text_primary(),
            )]));
            f.render_widget(p, msg);
        }
    }
}
