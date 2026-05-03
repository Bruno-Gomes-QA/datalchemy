use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::tui::commands::command_palette_matches;
use crate::tui::state::{App, InputMode, PaletteEntry, SetupStep, UiState};
use crate::tui::utils::clipped_input;

pub const INPUT_HEIGHT: u16 = 3;
pub const FOOTER_HEIGHT: u16 = 1; // context/status line
pub const HEADER_HEIGHT: u16 = 6;
pub const HEADER_WIDTH: u16 = 62;
pub const MAX_PALETTE_LINES: usize = 8;

pub fn draw_ui(frame: &mut ratatui::Frame, app: &App) {
    let size = frame.size();

    // Setup mode logic (keep simple layout)
    if app.is_in_setup() {
        // Full screen for Welcome/Introspecting
        match app.ui_state {
            UiState::Setup(SetupStep::Welcome) => {
                let area = frame.size();
                let block = Block::default()
                    .borders(Borders::NONE)
                    .style(Style::default().bg(Color::Reset));
                let text = vec![
                    Line::from(Span::styled(
                        "✨ Datalchemy ✨",
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Turn your database schema into gold.",
                        Style::default().fg(Color::Gray),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "We will setup your workspace and inspect your database.",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press [ENTER] to start",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::SLOW_BLINK),
                    )),
                ];

                let vertical_pad = area.height.saturating_sub(text.len() as u16) / 2;
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(vertical_pad), Constraint::Min(0)])
                    .split(area);

                let p = Paragraph::new(text)
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(block);
                frame.render_widget(p, layout[1]);
                return;
            }
            UiState::Setup(SetupStep::ConfirmReset) => {
                let area = frame.size();
                let text = vec![
                    Line::from(Span::styled(
                        "RESET WORKSPACE",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("This will delete: {}", app.paths.root.display()),
                        Style::default().fg(Color::Gray),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Type Y and press Enter to confirm.",
                        Style::default().fg(Color::Yellow),
                    )),
                    Line::from(Span::styled(
                        "Type N and press Enter to cancel.",
                        Style::default().fg(Color::DarkGray),
                    )),
                ];

                let vertical_pad = area.height.saturating_sub(text.len() as u16) / 2;
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(vertical_pad), Constraint::Min(0)])
                    .split(area);

                let p = Paragraph::new(text)
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(Block::default().borders(Borders::NONE));
                frame.render_widget(p, layout[1]);
                return;
            }
            UiState::Setup(SetupStep::SelectSchema) => {
                let area = frame.size();
                let title = Line::from(Span::styled(
                    "Select Schema",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));

                // Render schemas list
                let list_items: Vec<Line> = app
                    .available_schemas
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        if i == app.schema_picker_idx {
                            Line::from(vec![
                                Span::styled(" ► ", Style::default().fg(Color::Green)),
                                Span::styled(
                                    s,
                                    Style::default()
                                        .fg(Color::White)
                                        .add_modifier(Modifier::BOLD),
                                ),
                            ])
                        } else {
                            Line::from(vec![
                                Span::raw("   "),
                                Span::styled(s, Style::default().fg(Color::Gray)),
                            ])
                        }
                    })
                    .collect();

                let total_height = (list_items.len() + 4) as u16;
                let vertical_pad = area.height.saturating_sub(total_height) / 2;

                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(vertical_pad),
                        Constraint::Length(total_height),
                        Constraint::Min(0),
                    ])
                    .split(area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .style(Style::default().bg(Color::Reset));

                let p = Paragraph::new(list_items).block(block);
                frame.render_widget(p, layout[1]);
                return;
            }
            UiState::Setup(SetupStep::Introspecting) => {
                let spinner = vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                let s = spinner[app.spinner_idx % spinner.len()];

                let area = frame.size();
                let text = vec![
                    Line::from(Span::styled(
                        format!("{} Introspecting database...", s),
                        Style::default().fg(Color::Yellow),
                    )),
                    Line::from(""),
                    // Show last few messages as "logs"
                    Line::from(Span::styled(
                        app.messages.last().cloned().unwrap_or_default(),
                        Style::default().fg(Color::DarkGray),
                    )),
                ];

                let vertical_pad = area.height.saturating_sub(text.len() as u16) / 2;
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(vertical_pad), Constraint::Min(0)])
                    .split(area);

                let p = Paragraph::new(text).alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(p, layout[1]);
                return;
            }
            _ => {}
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(INPUT_HEIGHT)])
            .split(size);
        let body = render_body(app, layout[0].height as usize);
        frame.render_widget(body, layout[0]);
        let (footer, cursor) = render_footer(app, layout[1]);
        frame.render_widget(footer, layout[1]);
        if let Some((x, y)) = cursor {
            frame.set_cursor(x, y);
        }
        return;
    }

    let header_height = if app.show_header() { HEADER_HEIGHT } else { 0 };

    let palette = command_palette_matches(app, &app.input);
    let palette_height = palette.len().min(MAX_PALETTE_LINES) as u16;
    let bottom_reserved = INPUT_HEIGHT + FOOTER_HEIGHT + palette_height + 1; // +1 for spacer

    let body_height = size
        .height
        .saturating_sub(header_height)
        .saturating_sub(bottom_reserved)
        .max(1);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Length(body_height),
            Constraint::Length(1),            // Spacer
            Constraint::Length(INPUT_HEIGHT), // Input (taller)
            Constraint::Length(FOOTER_HEIGHT),
            Constraint::Length(palette_height),
        ])
        .split(size);

    if app.show_header() {
        let header_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(HEADER_WIDTH), Constraint::Min(1)])
            .split(layout[0]);

        let header = render_header(app);
        frame.render_widget(header, header_layout[0]);
    }

    let body = render_body(app, layout[1].height as usize);
    frame.render_widget(body, layout[1]);

    // Layout[2] is spacer, leave empty

    let (input_area, cursor) = render_input_bar(app, layout[3]);
    frame.render_widget(input_area, layout[3]);

    let status_line = render_status_line(app);
    frame.render_widget(status_line, layout[4]);

    if palette_height > 0 {
        let palette_view = render_palette(&palette, app.palette_select);
        frame.render_widget(palette_view, layout[5]);
    }
    if let Some((x, y)) = cursor {
        frame.set_cursor(x, y);
    }
}

fn render_header(app: &App) -> Paragraph<'static> {
    let profile_display = app.profile_display();
    let model_display = app
        .settings
        .llm_model
        .clone()
        .unwrap_or_else(|| "none".to_string());

    let title = Line::from(vec![
        Span::styled(">_ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Datalchemy (v{})", env!("CARGO_PKG_VERSION")),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ]);

    // Codex style: keys in dark gray, values in white/color
    let line_model = Line::from(vec![
        Span::styled("model:     ", Style::default().fg(Color::DarkGray)),
        Span::styled(model_display, Style::default().fg(Color::Cyan)),
        Span::styled("  /llm set", Style::default().fg(Color::DarkGray)),
    ]);

    let line_dir = Line::from(vec![
        Span::styled("directory: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", app.paths.root.display()),
            Style::default().fg(Color::White),
        ),
    ]);

    let _line_profile = Line::from(vec![
        Span::styled("profile:   ", Style::default().fg(Color::DarkGray)),
        Span::styled(profile_display, Style::default().fg(Color::Yellow)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .border_type(BorderType::Rounded);

    Paragraph::new(Text::from(vec![
        title,
        Line::from(""), // spacer
        line_model,
        line_dir,
        // line_profile, // Optional, mimicking minimal codex look
    ]))
    .block(block)
}

fn render_body(app: &App, height: usize) -> Paragraph<'static> {
    let total_lines = app.messages.len();
    // If empty, return empty
    if total_lines == 0 {
        return Paragraph::new("");
    }

    let view_end = total_lines.saturating_sub(app.scroll_offset as usize);
    let view_start = view_end.saturating_sub(height);

    let lines: Vec<Line<'static>> = app.messages[view_start..view_end]
        .iter()
        .map(|line| {
            if line.starts_with("►") {
                let text = line.trim_start_matches(|c| c == '►' || c == ' ');
                Line::from(vec![
                    Span::styled("●", Style::default().fg(Color::Green)),
                    Span::raw(" "),
                    Span::styled(
                        text.to_string(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(Span::raw(line.clone()))
            }
        })
        .collect();

    Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false })
}

fn render_input_label_setup(step: &crate::tui::state::SetupStep) -> &'static str {
    match step {
        crate::tui::state::SetupStep::ConfirmWorkspace => "Create workspace here? (y/n)",
        crate::tui::state::SetupStep::ProfileName => "Enter profile name (e.g. dev):",
        crate::tui::state::SetupStep::ConnectionString => "Enter Postgres connection string:",
        crate::tui::state::SetupStep::DbSession => "Session connection (not saved):",
        crate::tui::state::SetupStep::DbChange => "Update session connection:",
        crate::tui::state::SetupStep::SelectSchema => "Select a schema (UP/DOWN + ENTER):",
        crate::tui::state::SetupStep::LlmEnable => "Enable LLM? (y/n)",
        _ => "",
    }
}

fn render_input_bar(app: &App, area: Rect) -> (Paragraph<'static>, Option<(u16, u16)>) {
    // Override for setup
    if let UiState::Setup(step) = &app.ui_state {
        let prefix = "> ";
        let prefix_len = prefix.len();
        let (visible, cursor_x) = clipped_input(&app.input, area.width as usize, prefix_len);

        let label = render_input_label_setup(step);

        let content = if app.input.is_empty() {
            vec![
                Span::styled(prefix, Style::default().fg(Color::Green)),
                Span::styled(label, Style::default().fg(Color::Gray)),
            ]
        } else {
            vec![
                Span::styled(prefix, Style::default().fg(Color::Green)),
                Span::raw(visible),
            ]
        };

        let padding_line = Line::from("");
        let content_line = Line::from(content);
        let paragraph = Paragraph::new(vec![padding_line.clone(), content_line, padding_line])
            .style(Style::default().bg(Color::Rgb(20, 20, 20)));

        let cursor = Some((area.x + cursor_x + prefix_len as u16, area.y + 1));
        return (paragraph, cursor);
    }

    match &app.mode {
        InputMode::Command => {
            // Codex style: >_ prompt, dark bg bar
            let prefix = "> ";
            let prefix_len = prefix.len();
            let (visible, cursor_x) = clipped_input(&app.input, area.width as usize, prefix_len);

            // Placeholder if empty
            let content = if app.input.is_empty() {
                vec![
                    Span::styled(prefix, Style::default().fg(Color::Cyan)),
                    Span::styled(
                        "Describe a task or query...",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]
            } else {
                vec![
                    Span::styled(prefix, Style::default().fg(Color::Cyan)),
                    Span::raw(visible),
                ]
            };

            let padding_line = Line::from("");
            let content_line = Line::from(content);

            // 3-line Layout: Padding, Content, Padding (centered)
            let paragraph = Paragraph::new(vec![padding_line.clone(), content_line, padding_line])
                .style(Style::default().bg(Color::Rgb(30, 30, 30))); // Dark gray background strip

            let cursor = Some((area.x + cursor_x + prefix_len as u16, area.y + 1));
            (paragraph, cursor)
        }
        InputMode::Approval { intent, .. } => {
            let line = Line::from(vec![
                Span::styled(
                    "! APPROVAL REQUIRED: ",
                    Style::default().fg(Color::Black).bg(Color::Yellow),
                ),
                Span::raw(" "),
                Span::raw(intent.reason.clone()),
            ]);
            // Centered-ish vertically if height is 3
            let paragraph = Paragraph::new(vec![Line::from(""), line, Line::from("")]);
            (paragraph, None)
        }
    }
}

fn render_status_line(app: &App) -> Paragraph<'static> {
    match &app.mode {
        InputMode::Command => {
            let left = if app.show_header() {
                "Tip: Use /help to list commands."
            } else {
                "Setup Mode"
            };

            let status = format!(
                "mode: {} . profile: {}",
                app.mode_display(),
                app.profile_display()
            );

            Paragraph::new(Line::from(vec![
                Span::styled(left, Style::default().fg(Color::DarkGray)),
                Span::raw("   "),
                Span::styled(status, Style::default().fg(Color::DarkGray)),
            ]))
        }
        InputMode::Approval { .. } => Paragraph::new(Line::from(vec![Span::styled(
            "press 'y' to confirm, 'n' to deny",
            Style::default().fg(Color::White),
        )])),
    }
}

// Deprecated old signature wrapper if needed, or we just rely on draw_ui calls.
// Since we changed draw_ui logic, we don't need render_footer as exposed before.
fn render_footer(app: &App, area: Rect) -> (Paragraph<'static>, Option<(u16, u16)>) {
    // This is for setup mode fallback
    render_input_bar(app, area)
}

fn render_palette(entries: &[PaletteEntry], selected_idx: usize) -> Paragraph<'static> {
    // Simple windowing logic
    let total_cnt = entries.len();
    let max_lines = MAX_PALETTE_LINES;

    // Determine start index to ensure selected_idx is visible
    // If selected is 0..7, start is 0.
    // If selected is 8, we want to see 8., so start must be at least 1 (showing 1..9).
    // Actually, just keep selected in the middle or bottom if scrolling down.

    // Simplest: if selected_idx >= max_lines, start = selected_idx - max_lines + 1
    // But be careful with bounds.
    let start_idx = if selected_idx >= max_lines {
        selected_idx - max_lines + 1
    } else {
        0
    };

    let end_idx = (start_idx + max_lines).min(total_cnt);

    let lines: Vec<Line<'static>> = entries[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(|(offset, entry)| {
            let actual_idx = start_idx + offset;
            let is_selected = actual_idx == selected_idx;
            let raw_str = format!("{:<20}  {}", entry.command, entry.description);

            if is_selected {
                Line::from(Span::styled(
                    raw_str,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(raw_str, Style::default().fg(Color::DarkGray)))
            }
        })
        .collect();
    Paragraph::new(Text::from(lines))
}
