use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::tui::commands::command_palette_matches;
use crate::tui::state::{App, InputMode, PaletteEntry, SetupStep, UiState};
use crate::tui::utils::clipped_input;

pub const INPUT_HEIGHT: u16 = 3;
pub const FOOTER_HEIGHT: u16 = 1;
pub const HEADER_HEIGHT: u16 = 6;
pub const HEADER_WIDTH: u16 = 62;
/// Dynamic palette limit: show up to 20 entries (scrollable).
pub const MAX_PALETTE_LINES: usize = 20;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn draw_ui(frame: &mut ratatui::Frame, app: &App) {
    let size = frame.size();

    // Setup mode logic
    if app.is_in_setup() {
        match app.ui_state {
            UiState::Setup(SetupStep::Welcome) => {
                render_centered_screen(
                    frame,
                    vec![
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
                            "Press [ENTER] to start  ·  [Esc] to quit",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::SLOW_BLINK),
                        )),
                    ],
                );
                return;
            }
            UiState::Setup(SetupStep::ConfirmReset) => {
                render_centered_screen(
                    frame,
                    vec![
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
                            "Type N or press Esc to cancel.",
                            Style::default().fg(Color::DarkGray),
                        )),
                    ],
                );
                return;
            }
            UiState::Setup(SetupStep::SelectSchema) => {
                let area = frame.size();
                let title = Line::from(Span::styled(
                    "Select Schema (Up/Down + Enter · Esc to go back)",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));

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
                let s = SPINNER_FRAMES[app.spinner_idx % SPINNER_FRAMES.len()];
                render_centered_screen(
                    frame,
                    vec![
                        Line::from(Span::styled(
                            format!("{} Introspecting database...", s),
                            Style::default().fg(Color::Yellow),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            app.messages.last().cloned().unwrap_or_default(),
                            Style::default().fg(Color::DarkGray),
                        )),
                    ],
                );
                return;
            }
            _ => {}
        }

        // Other setup steps: body + input bar
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(INPUT_HEIGHT)])
            .split(size);
        let body = render_body(app, layout[0].height as usize);
        frame.render_widget(body, layout[0]);
        let (footer, cursor) = render_input_bar(app, layout[1]);
        frame.render_widget(footer, layout[1]);
        if let Some((x, y)) = cursor {
            frame.set_cursor(x, y);
        }
        return;
    }

    // --- Normal mode ---
    let header_height = if app.show_header() { HEADER_HEIGHT } else { 0 };

    let palette = command_palette_matches(app, &app.input);
    // Dynamic palette sizing: use available space, capped at MAX_PALETTE_LINES
    let max_palette = MAX_PALETTE_LINES.min(
        size.height
            .saturating_sub(header_height + INPUT_HEIGHT + FOOTER_HEIGHT + 3) as usize,
    );
    let palette_height = palette.len().min(max_palette) as u16;
    let bottom_reserved = INPUT_HEIGHT + FOOTER_HEIGHT + palette_height + 1;

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
            Constraint::Length(1), // spacer
            Constraint::Length(INPUT_HEIGHT),
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

    // Scrollbar indicator in spacer area
    if app.messages.len() > body_height as usize {
        let scrollbar = render_scrollbar_hint(app, body_height as usize);
        frame.render_widget(scrollbar, layout[2]);
    }

    let (input_area, cursor) = render_input_bar(app, layout[3]);
    frame.render_widget(input_area, layout[3]);

    let status_line = render_status_line(app);
    frame.render_widget(status_line, layout[4]);

    if palette_height > 0 {
        let palette_view = render_palette(&palette, app.palette_select, max_palette);
        frame.render_widget(palette_view, layout[5]);
    }
    if let Some((x, y)) = cursor {
        frame.set_cursor(x, y);
    }
}

// ---------------------------------------------------------------------------
// Shared rendering helpers
// ---------------------------------------------------------------------------

fn render_centered_screen(frame: &mut ratatui::Frame, lines: Vec<Line<'static>>) {
    let area = frame.size();
    let vertical_pad = area.height.saturating_sub(lines.len() as u16) / 2;
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(vertical_pad), Constraint::Min(0)])
        .split(area);
    let p = Paragraph::new(lines)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(p, layout[1]);
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
        Line::from(""),
        line_model,
        line_dir,
    ]))
    .block(block)
}

fn render_body(app: &App, height: usize) -> Paragraph<'static> {
    let total_lines = app.messages.len();
    if total_lines == 0 {
        return Paragraph::new("");
    }

    let view_end = total_lines.saturating_sub(app.scroll_offset as usize);
    let view_start = view_end.saturating_sub(height);

    let lines: Vec<Line<'static>> = app.messages[view_start..view_end]
        .iter()
        .map(|line| {
            if line.starts_with('►') {
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
            } else if line.starts_with("[") && line.len() > 10 && line.chars().nth(9) == Some(']') {
                // Timestamp-prefixed line: color the timestamp
                let (ts, rest) = line.split_at(10.min(line.len()));
                Line::from(vec![
                    Span::styled(ts.to_string(), Style::default().fg(Color::DarkGray)),
                    Span::raw(rest.to_string()),
                ])
            } else {
                Line::from(Span::raw(line.clone()))
            }
        })
        .collect();

    Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false })
}

fn render_scrollbar_hint(app: &App, body_height: usize) -> Paragraph<'static> {
    let total = app.messages.len();
    let visible_end = total.saturating_sub(app.scroll_offset as usize);
    let at_bottom = app.scroll_offset == 0;

    if at_bottom {
        Paragraph::new("")
    } else {
        let hidden_below = app.scroll_offset as usize;
        let hint = format!(
            "▲ {} more above · ▼ {} below · PageUp/Down to scroll",
            total
                .saturating_sub(visible_end)
                .saturating_sub(body_height),
            hidden_below
        );
        Paragraph::new(Line::from(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray),
        )))
    }
}

fn render_input_label_setup(step: &SetupStep) -> String {
    match step {
        SetupStep::ConfirmWorkspace => "Create workspace here? (y/n)  [Esc=back]".to_string(),
        SetupStep::ProfileName => "Enter profile name (e.g. dev):  [Esc=back]".to_string(),
        SetupStep::ConnectionString => "Enter Postgres connection string:  [Esc=back]".to_string(),
        SetupStep::DbSession => "Session connection (not saved):  [Esc=back]".to_string(),
        SetupStep::DbChange => "Update session connection:  [Esc=back]".to_string(),
        SetupStep::SelectSchema => "Select a schema (UP/DOWN + ENTER):  [Esc=back]".to_string(),
        SetupStep::Prompt(ctx) => {
            if let Some(label) = ctx.current_prompt() {
                format!("{}  [Esc=back]", label)
            } else {
                "[Esc=back]".to_string()
            }
        }
        _ => String::new(),
    }
}

fn render_input_bar(app: &App, area: Rect) -> (Paragraph<'static>, Option<(u16, u16)>) {
    // Override for setup
    if let UiState::Setup(step) = &app.ui_state {
        let prefix = "> ";
        let prefix_len = prefix.len();
        let (visible, cursor_x) =
            clipped_input(&app.input, area.width as usize, prefix_len, app.cursor_pos);

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
            let prefix = "> ";
            let prefix_len = prefix.len();
            let (visible, cursor_x) =
                clipped_input(&app.input, area.width as usize, prefix_len, app.cursor_pos);

            let content = if app.input.is_empty() {
                vec![
                    Span::styled(prefix, Style::default().fg(Color::Cyan)),
                    Span::styled(
                        "Type / for commands or describe a task...",
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

            let paragraph = Paragraph::new(vec![padding_line.clone(), content_line, padding_line])
                .style(Style::default().bg(Color::Rgb(30, 30, 30)));

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
            let paragraph = Paragraph::new(vec![Line::from(""), line, Line::from("")]);
            (paragraph, None)
        }
    }
}

fn render_status_line(app: &App) -> Paragraph<'static> {
    // Show active task spinner if present
    if let Some(task) = &app.active_task {
        let s = SPINNER_FRAMES[app.spinner_idx % SPINNER_FRAMES.len()];
        return Paragraph::new(Line::from(vec![Span::styled(
            format!("{} {} ", s, task.label),
            Style::default().fg(Color::Yellow),
        )]));
    }

    match &app.mode {
        InputMode::Command => {
            let left = if app.show_header() {
                "Tip: / for commands · Tab autocomplete · Esc back"
            } else {
                "Setup Mode · Esc to go back"
            };

            let status = format!(
                "mode: {} · profile: {}",
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

fn render_footer(app: &App, area: Rect) -> (Paragraph<'static>, Option<(u16, u16)>) {
    render_input_bar(app, area)
}

fn render_palette(
    entries: &[PaletteEntry],
    selected_idx: usize,
    max_lines: usize,
) -> Paragraph<'static> {
    let total_cnt = entries.len();

    // Windowing: keep selected visible
    let start_idx = if selected_idx >= max_lines {
        selected_idx - max_lines + 1
    } else {
        0
    };
    let end_idx = (start_idx + max_lines).min(total_cnt);

    let mut lines: Vec<Line<'static>> = entries[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(|(offset, entry)| {
            let actual_idx = start_idx + offset;
            let is_selected = actual_idx == selected_idx;
            let raw_str = format!("{:<22}  {}", entry.command, entry.description);

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

    // Show scroll indicators if there are hidden entries
    if start_idx > 0 {
        if let Some(first) = lines.first_mut() {
            *first = Line::from(vec![
                Span::styled("▲ ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!(
                        "{:<22}  {}",
                        entries[start_idx].command, entries[start_idx].description
                    ),
                    if start_idx == selected_idx {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ]);
        }
    }
    if end_idx < total_cnt {
        if let Some(last) = lines.last_mut() {
            let last_idx = end_idx - 1;
            *last = Line::from(vec![
                Span::styled("▼ ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!(
                        "{:<22}  {}",
                        entries[last_idx].command, entries[last_idx].description
                    ),
                    if last_idx == selected_idx {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ]);
        }
    }

    Paragraph::new(Text::from(lines))
}
