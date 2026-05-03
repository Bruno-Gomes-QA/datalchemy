pub mod commands;
pub mod conn;
pub mod events;
pub mod secrets;
pub mod state;
pub mod ui;
pub mod utils;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::{
    event, execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::CliError;
use events::handle_key;
use state::App;
use ui::draw_ui;

use state::AppEvent;

pub fn run(runtime: tokio::runtime::Handle, workspace_root: PathBuf) -> Result<(), CliError> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut app = App::new(runtime, workspace_root, tx)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app, &mut rx);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
) -> Result<(), CliError> {
    while !app.should_quit {
        terminal.draw(|frame| draw_ui(frame, app))?;

        // Check for async events
        while let Ok(event) = rx.try_recv() {
            match event {
                AppEvent::Log(msg) => {
                    app.push_message(msg);
                    if let state::UiState::Setup(state::SetupStep::Introspecting) = &app.ui_state {
                        if app.messages.len() > 10 {
                            app.messages.remove(0);
                        }
                    }
                }
                AppEvent::SchemasLoaded(res) => match res {
                    Ok(schemas) => {
                        app.available_schemas = schemas;
                        if app.available_schemas.is_empty() {
                            app.available_schemas.push("public".to_string());
                        }
                        app.schema_picker_idx = 0;
                        app.ui_state = state::UiState::Setup(state::SetupStep::SelectSchema);
                    }
                    Err(e) => {
                        app.push_message(format!("Error: {}", e));
                        app.push_message("Please check connection string and try again.");
                        app.ui_state = state::UiState::Setup(state::SetupStep::ConnectionString);
                    }
                },
                AppEvent::IntrospectionDone(res) => match res {
                    Ok(_) => {
                        app.push_raw("");
                        app.push_message("Introspection successful!");
                        app.push_message("Setup complete. Type /help to see commands.");
                        app.ui_state = state::UiState::Normal;
                    }
                    Err(e) => {
                        app.push_message(format!("Error: {}", e));
                        app.push_message("Please enter connection string again:");
                        app.ui_state = state::UiState::Setup(state::SetupStep::ConnectionString);
                    }
                },
                AppEvent::AsyncTaskDone(msg) => {
                    app.finish_task();
                    if !msg.is_empty() {
                        app.push_message(msg);
                    }
                }
            }
        }

        // Tick for spinner
        app.spinner_idx = app.spinner_idx.wrapping_add(1);

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                event::Event::Key(key) => handle_key(app, key)?,
                event::Event::Mouse(mouse) => match mouse.kind {
                    event::MouseEventKind::ScrollDown => {
                        app.scroll_offset = app.scroll_offset.saturating_sub(1);
                    }
                    event::MouseEventKind::ScrollUp => {
                        app.scroll_offset = app.scroll_offset.saturating_add(1);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
    Ok(())
}
