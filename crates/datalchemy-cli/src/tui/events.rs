use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sqlx::postgres::PgPoolOptions;

use crate::CliError;
use crate::tui::commands::{command_palette_matches, execute_command, sanitize_command_for_log};
use crate::tui::conn::is_supported_connection;
use crate::tui::state::{App, AppEvent, InputMode, SetupStep, UiState};
use crate::workspace::{DbProfile, WriteIntent, save_profiles, save_settings};
use datalchemy_core::validate_schema;
use datalchemy_introspect::{
    IntrospectOptions, introspect_postgres_with_options, introspect_sqlite_with_options,
};

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<(), CliError> {
    match app.mode.clone() {
        InputMode::Command => handle_command_key(app, key),
        InputMode::Approval { intent, command } => handle_approval_key(app, intent, command, key),
    }
}

fn handle_command_key(app: &mut App, key: KeyEvent) -> Result<(), CliError> {
    match key.code {
        // -- quit --
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // -- scroll --
        KeyCode::PageUp => {
            app.scroll_offset = app.scroll_offset.saturating_add(5);
        }
        KeyCode::PageDown => {
            app.scroll_offset = app.scroll_offset.saturating_sub(5);
        }

        // -- Esc: go back in setup or clear input --
        KeyCode::Esc => {
            if app.is_in_setup() {
                handle_setup_back(app);
            } else if !app.input.is_empty() {
                app.input_clear();
                app.palette_select = 0;
            }
        }

        // -- arrow down --
        KeyCode::Down => {
            if let UiState::Setup(SetupStep::SelectSchema) = app.ui_state {
                if !app.available_schemas.is_empty() {
                    app.schema_picker_idx = (app.schema_picker_idx + 1)
                        .min(app.available_schemas.len().saturating_sub(1));
                }
            } else if app.input.starts_with('/') {
                let matches = command_palette_matches(app, &app.input);
                if !matches.is_empty() {
                    app.palette_select =
                        (app.palette_select + 1).min(matches.len().saturating_sub(1));
                }
            }
        }

        // -- arrow up --
        KeyCode::Up => {
            if let UiState::Setup(SetupStep::SelectSchema) = app.ui_state {
                app.schema_picker_idx = app.schema_picker_idx.saturating_sub(1);
            } else if app.input.starts_with('/') {
                let matches = command_palette_matches(app, &app.input);
                if !matches.is_empty() {
                    app.palette_select = app.palette_select.saturating_sub(1);
                }
            }
        }

        // -- cursor movement --
        KeyCode::Left => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                app.input_delete_word_back();
            } else {
                app.input_move_left();
            }
        }
        KeyCode::Right => {
            app.input_move_right();
        }
        KeyCode::Home => {
            app.input_move_home();
        }
        KeyCode::End => {
            app.input_move_end();
        }
        KeyCode::Delete => {
            app.input_delete_forward();
        }

        // -- Ctrl+A = Home, Ctrl+E = End, Ctrl+W = delete word --
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.input_move_home();
        }
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.input_move_end();
        }
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.input_delete_word_back();
            app.palette_select = 0;
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.input_clear();
            app.palette_select = 0;
        }

        // -- Tab: autocomplete from palette --
        KeyCode::Tab => {
            if app.input.starts_with('/') {
                let matches = command_palette_matches(app, &app.input);
                if !matches.is_empty() && app.palette_select < matches.len() {
                    let cmd = matches[app.palette_select].command.to_string();
                    app.input_set(cmd);
                    app.palette_select = 0;
                }
            }
        }

        // -- Enter --
        KeyCode::Enter => {
            // Autocomplete from palette if partial match
            if app.input.starts_with('/') {
                let matches = command_palette_matches(app, &app.input);
                if !matches.is_empty()
                    && app.palette_select < matches.len()
                    && app.input.trim() != matches[app.palette_select].command
                {
                    let cmd = matches[app.palette_select].command.to_string();
                    app.input_set(cmd);
                    app.palette_select = 0;
                }
            }

            // Schema selection: handle Enter without input
            if let UiState::Setup(SetupStep::SelectSchema) = app.ui_state {
                app.input_clear();
                handle_setup_input(app, "")?;
                return Ok(());
            }

            let input = app.input_take();
            let input = input.trim();

            if !input.is_empty() {
                if app.is_in_setup() {
                    if let Err(err) = handle_setup_input(app, input) {
                        app.push_message(format!("error: {err}"));
                    }
                } else {
                    let sanitized = sanitize_command_for_log(input);
                    app.record_command(&sanitized);
                    if let Err(err) = execute_command(app, input, false) {
                        app.push_message(format!("error: {err}"));
                    }
                }
                app.scroll_offset = 0;
                app.palette_select = 0;
            }
        }

        // -- Backspace --
        KeyCode::Backspace => {
            app.input_delete_back();
            app.palette_select = 0;
        }

        // -- regular char --
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(());
            }
            app.input_insert_char(ch);
            app.palette_select = 0;
        }
        _ => {}
    }
    Ok(())
}

fn handle_approval_key(
    app: &mut App,
    intent: WriteIntent,
    command: String,
    key: KeyEvent,
) -> Result<(), CliError> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.mode = InputMode::Command;
            app.push_message(format!(
                "approved: {} ({} paths)",
                intent.reason,
                intent.paths.len()
            ));
            if let Err(err) = execute_command(app, &command, true) {
                app.push_message(format!("error: {err}"));
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = InputMode::Command;
            app.push_message("approval denied.");
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.mode = InputMode::Command;
            app.push_message("approval denied.");
        }
        _ => {}
    }
    Ok(())
}

/// Go back one step in the setup wizard.
fn handle_setup_back(app: &mut App) {
    match &app.ui_state {
        UiState::Setup(step) => {
            let prev = match step {
                SetupStep::Welcome => {
                    app.should_quit = true;
                    return;
                }
                SetupStep::ConfirmWorkspace => SetupStep::Welcome,
                SetupStep::ConfirmReset => {
                    app.ui_state = UiState::Normal;
                    app.push_message("Reset canceled.");
                    return;
                }
                SetupStep::ProfileName => SetupStep::ConfirmWorkspace,
                SetupStep::ConnectionString => SetupStep::ProfileName,
                SetupStep::SelectSchema => SetupStep::ConnectionString,
                SetupStep::Introspecting => return, // can't cancel mid-introspect
                SetupStep::DbSession | SetupStep::DbChange => {
                    app.ui_state = UiState::Normal;
                    return;
                }
                SetupStep::Prompt(ctx) => {
                    if ctx.collected.is_empty() {
                        // First step: go back to Normal
                        app.ui_state = UiState::Normal;
                        app.push_message("Canceled.");
                        return;
                    } else {
                        // Go back one prompt step
                        let mut new_ctx = ctx.clone();
                        new_ctx.collected.pop();
                        if let Some(label) = new_ctx.current_prompt() {
                            app.push_message(label.to_string());
                        }
                        app.input_clear();
                        app.ui_state = UiState::Setup(SetupStep::Prompt(new_ctx));
                        return;
                    }
                }
            };
            app.input_clear();
            app.ui_state = UiState::Setup(prev);
        }
        _ => {}
    }
}

fn handle_setup_input(app: &mut App, input: &str) -> Result<(), CliError> {
    match app.ui_state.clone() {
        UiState::Setup(SetupStep::Welcome) => {
            app.ui_state = UiState::Setup(SetupStep::ConfirmWorkspace);
            app.messages.clear();
            app.push_message("Welcome to Datalchemy.");
            app.push_raw("");
        }
        UiState::Setup(SetupStep::ConfirmReset) => {
            if matches!(input, "s" | "S" | "y" | "Y") {
                if app.paths.root.exists() {
                    std::fs::remove_dir_all(&app.paths.root)?;
                }
                app.messages.clear();
                app.input_clear();
                app.session_conn = None;
                app.last_out_id = None;
                app.setup_profile_name = None;
                app.settings = crate::workspace::WorkspaceSettings::default();
                app.profiles = crate::workspace::ProfilesConfig::default();
                app.ui_state = UiState::Setup(SetupStep::Welcome);
            } else if matches!(input, "n" | "N") {
                app.ui_state = UiState::Normal;
                app.push_message("Reset canceled.");
            } else {
                app.push_message("Please answer with y or n.");
            }
        }
        UiState::Setup(SetupStep::ConfirmWorkspace) => {
            if matches!(input, "s" | "S" | "y" | "Y") {
                execute_command(app, "/init", true)?;
                app.push_raw("");
                app.push_message(
                    "Great! Please enter a name for your profile (e.g. 'dev', 'prod'):",
                );
                app.ui_state = UiState::Setup(SetupStep::ProfileName);
            } else if matches!(input, "n" | "N") {
                app.push_message("Setup canceled.");
                app.should_quit = true;
            } else {
                app.push_message("Please answer with y or n.");
            }
        }
        UiState::Setup(SetupStep::ProfileName) => {
            if input.trim().is_empty() {
                app.push_message("Profile name cannot be empty.");
            } else {
                app.setup_profile_name = Some(input.trim().to_string());
                app.push_raw("");
                app.push_message("Enter your database connection string:");
                app.ui_state = UiState::Setup(SetupStep::ConnectionString);
            }
        }
        UiState::Setup(SetupStep::ConnectionString) => {
            let conn_str = input.trim();
            if conn_str.is_empty() {
                app.push_message("Connection string cannot be empty.");
                return Ok(());
            }
            if !is_supported_connection(conn_str) {
                app.push_message("Unsupported database. URL must start with postgres://, postgresql://, or sqlite://");
                return Ok(());
            }

            let profile_name = app
                .setup_profile_name
                .clone()
                .unwrap_or_else(|| "local".to_string());
            let profile = DbProfile::from_connection(conn_str);
            app.profiles.profiles.insert(profile_name.clone(), profile);
            app.settings.active_profile = Some(profile_name);
            save_profiles(&app.paths, &app.profiles)?;
            save_settings(&app.paths, &app.settings)?;
            app.session_conn = Some(conn_str.to_string());

            app.ui_state = UiState::Setup(SetupStep::Introspecting);
            app.messages.clear();

            let tx = app.tx.clone();
            let conn_string = conn_str.to_string();
            let is_sqlite = conn_str.starts_with("sqlite://");

            app.runtime.spawn(async move {
                tx.send(AppEvent::Log("Connecting to database...".into())).ok();
                if is_sqlite {
                    match sqlx::sqlite::SqlitePoolOptions::new()
                        .connect(&conn_string)
                        .await
                    {
                        Ok(_pool) => {
                            // SQLite has a single "main" schema, skip schema selection.
                            tx.send(AppEvent::SchemasLoaded(Ok(vec!["main".to_string()])))
                                .ok();
                        }
                        Err(e) => {
                            tx.send(AppEvent::SchemasLoaded(Err(format!(
                                "Connection failed: {}. Check the path and try again.",
                                e
                            ))))
                            .ok();
                        }
                    }
                } else {
                    match PgPoolOptions::new().connect(&conn_string).await {
                        Ok(pool) => {
                            tx.send(AppEvent::Log("Connected! Fetching schemas...".into())).ok();
                            let schemas_result: Result<Vec<sqlx::postgres::PgRow>, sqlx::Error> =
                                sqlx::query(
                                "SELECT schema_name FROM information_schema.schemata
                                 WHERE schema_name NOT IN ('information_schema', 'pg_catalog', 'pg_toast')
                                 AND schema_name NOT LIKE 'pg_temp_%'
                                 AND schema_name NOT LIKE 'pg_toast_temp_%'
                                 ORDER BY schema_name"
                            )
                            .fetch_all(&pool)
                            .await;

                            match schemas_result {
                                Ok(rows) => {
                                    use sqlx::Row as _;
                                    let schemas: Vec<String> = rows
                                        .into_iter()
                                        .filter_map(|r| r.try_get::<String, _>("schema_name").ok())
                                        .collect();
                                    tx.send(AppEvent::SchemasLoaded(Ok(schemas))).ok();
                                }
                                Err(e) => {
                                    tx.send(AppEvent::SchemasLoaded(Err(format!(
                                        "Failed to list schemas: {}",
                                        e
                                    ))))
                                    .ok();
                                }
                            }
                        }
                        Err(e) => {
                            tx.send(AppEvent::SchemasLoaded(Err(format!(
                                "Connection failed: {}. Check the URL and try again.",
                                e
                            ))))
                            .ok();
                        }
                    }
                }
            });
        }
        UiState::Setup(SetupStep::DbSession) => {
            let conn_str = input.trim();
            if conn_str.is_empty() {
                app.push_message("Connection string cannot be empty.");
                return Ok(());
            }
            if !is_supported_connection(conn_str) {
                app.push_message("Unsupported database.");
                return Ok(());
            }
            app.session_conn = Some(conn_str.to_string());
            app.ui_state = UiState::Normal;
            app.push_message("session connection updated (not saved).");
        }
        UiState::Setup(SetupStep::DbChange) => {
            let conn_str = input.trim();
            if conn_str.is_empty() {
                app.push_message("Connection string cannot be empty.");
                return Ok(());
            }
            if !is_supported_connection(conn_str) {
                app.push_message("Unsupported database.");
                return Ok(());
            }
            app.session_conn = Some(conn_str.to_string());
            app.ui_state = UiState::Normal;
            app.push_message("session connection updated for this run.");
        }
        UiState::Setup(SetupStep::SelectSchema) => {
            let selected_schema = if app.available_schemas.is_empty() {
                None
            } else {
                Some(vec![app.available_schemas[app.schema_picker_idx].clone()])
            };

            app.ui_state = UiState::Setup(SetupStep::Introspecting);
            app.messages.clear();

            let tx = app.tx.clone();
            let Some(conn_string) = app.session_conn.clone() else {
                app.push_message("missing connection string. please enter it again.");
                app.ui_state = UiState::Setup(SetupStep::ConnectionString);
                return Ok(());
            };

            let options = IntrospectOptions {
                include_system_schemas: false,
                include_views: true,
                include_materialized_views: true,
                include_foreign_tables: true,
                include_indexes: true,
                include_comments: true,
                schemas: selected_schema,
            };

            let is_sqlite = conn_string.starts_with("sqlite://");

            app.runtime.spawn(async move {
                tx.send(AppEvent::Log("Starting introspection...".into()))
                    .ok();
                if is_sqlite {
                    match sqlx::sqlite::SqlitePoolOptions::new()
                        .connect(&conn_string)
                        .await
                    {
                        Ok(pool) => match introspect_sqlite_with_options(&pool, options).await {
                            Ok(schema) => {
                                tx.send(AppEvent::Log("Introspection complete.".into()))
                                    .ok();
                                if let Err(e) = validate_schema(&schema) {
                                    tx.send(AppEvent::IntrospectionDone(Err(format!(
                                        "Schema validation failed: {}",
                                        e
                                    ))))
                                    .ok();
                                } else {
                                    tx.send(AppEvent::IntrospectionDone(Ok(()))).ok();
                                }
                            }
                            Err(e) => {
                                tx.send(AppEvent::IntrospectionDone(Err(format!(
                                    "Introspection error: {}",
                                    e
                                ))))
                                .ok();
                            }
                        },
                        Err(e) => {
                            tx.send(AppEvent::IntrospectionDone(Err(format!(
                                "Connection failed: {}",
                                e
                            ))))
                            .ok();
                        }
                    }
                } else {
                    match PgPoolOptions::new().connect(&conn_string).await {
                        Ok(pool) => match introspect_postgres_with_options(&pool, options).await {
                            Ok(schema) => {
                                tx.send(AppEvent::Log("Introspection complete.".into()))
                                    .ok();
                                if let Err(e) = validate_schema(&schema) {
                                    tx.send(AppEvent::IntrospectionDone(Err(format!(
                                        "Schema validation failed: {}",
                                        e
                                    ))))
                                    .ok();
                                } else {
                                    tx.send(AppEvent::IntrospectionDone(Ok(()))).ok();
                                }
                            }
                            Err(e) => {
                                tx.send(AppEvent::IntrospectionDone(Err(format!(
                                    "Introspection error: {}",
                                    e
                                ))))
                                .ok();
                            }
                        },
                        Err(e) => {
                            tx.send(AppEvent::IntrospectionDone(Err(format!(
                                "Connection failed: {}",
                                e
                            ))))
                            .ok();
                        }
                    }
                }
            });
        }
        UiState::Setup(SetupStep::Introspecting) => {
            // Ignore input while introspecting
        }
        UiState::Setup(SetupStep::Prompt(mut ctx)) => {
            let value = input.trim().to_string();

            // Validate per-command/step
            if value.is_empty() {
                app.push_message("Value cannot be empty.");
                return Ok(());
            }

            // Connection string validation for /profiles new step 2
            if ctx.command == "/profiles new" && ctx.collected.len() == 1 {
                if !is_supported_connection(&value) {
                    app.push_message("Unsupported database. URL must start with postgres://, postgresql://, or sqlite://");
                    return Ok(());
                }
            }

            ctx.push(value);

            if ctx.is_complete() {
                // Assemble and execute command
                app.ui_state = UiState::Normal;
                let full_cmd = format!("{} {}", ctx.command, ctx.collected.join(" "));
                let sanitized = crate::tui::commands::sanitize_command_for_log(&full_cmd);
                app.record_command(&sanitized);
                if let Err(err) = execute_command(app, &full_cmd, false) {
                    app.push_message(format!("error: {err}"));
                }
            } else {
                // Show next prompt
                if let Some(label) = ctx.current_prompt() {
                    app.push_message(label.to_string());
                }
                app.ui_state = UiState::Setup(SetupStep::Prompt(ctx));
            }
        }
        _ => {}
    }
    Ok(())
}
