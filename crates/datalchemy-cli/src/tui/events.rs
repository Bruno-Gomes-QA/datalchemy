use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sqlx::postgres::PgPoolOptions;

use crate::CliError;
use crate::tui::commands::{command_palette_matches, execute_command, sanitize_command_for_log};
use crate::tui::state::{App, AppEvent, InputMode, SetupStep, UiState};
use crate::workspace::{DbProfile, LlmProvider, WriteIntent, save_profiles, save_settings};
use datalchemy_core::validate_schema;
use datalchemy_introspect::{IntrospectOptions, introspect_postgres_with_options};
// removed unused imports

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<(), CliError> {
    match app.mode.clone() {
        InputMode::Command => handle_command_key(app, key),
        InputMode::Approval { intent, command } => handle_approval_key(app, intent, command, key),
    }
}

fn handle_command_key(app: &mut App, key: KeyEvent) -> Result<(), CliError> {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::PageUp => {
            app.scroll_offset = app.scroll_offset.saturating_add(5);
        }
        KeyCode::PageDown => {
            app.scroll_offset = app.scroll_offset.saturating_sub(5);
        }
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
        KeyCode::Enter => {
            if app.input.starts_with('/') {
                let matches = command_palette_matches(app, &app.input);
                if !matches.is_empty()
                    && app.palette_select < matches.len()
                    && app.input.trim() != matches[app.palette_select].command
                {
                    app.input = matches[app.palette_select].command.to_string();
                    app.palette_select = 0;
                    // Intentionally fall through to execution instead of returning
                    // return Ok(());
                }
            }

            // Special handling for Schema Selection (arrow keys)
            if let UiState::Setup(SetupStep::SelectSchema) = app.ui_state {
                // Selection logic handled below in Up/Down keys
                // This block handles other char input or if we want to support typing filters
            }

            let input = app.input.drain(..).collect::<String>();
            let input = input.trim();

            // Allow empty input for Schema selection (selecting with Enter)
            if let UiState::Setup(SetupStep::SelectSchema) = app.ui_state {
                handle_setup_input(app, "")?;
                return Ok(());
            }

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
        KeyCode::Backspace => {
            app.input.pop();
            app.palette_select = 0;
        }
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(());
            }
            app.input.push(ch);
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
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('c')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.mode = InputMode::Command;
            app.push_message("approval denied.");
        }
        _ => {}
    }
    Ok(())
}

fn handle_setup_input(app: &mut App, input: &str) -> Result<(), CliError> {
    match app.ui_state.clone() {
        UiState::Setup(SetupStep::Welcome) => {
            // Any input moves to next step
            app.ui_state = UiState::Setup(SetupStep::ConfirmWorkspace);
            app.messages.clear();
            app.push_message("Welcome to Datalchemy.");
            app.push_message("");
        }
        UiState::Setup(SetupStep::ConfirmReset) => {
            if matches!(input, "s" | "S" | "y" | "Y") {
                if app.paths.root.exists() {
                    std::fs::remove_dir_all(&app.paths.root)?;
                }
                execute_command(app, "/init", true)?;
                app.messages.clear();
                app.input.clear();
                app.session_conn = None;
                app.last_out_id = None;
                app.setup_profile_name = None;
                app.push_message("Workspace reset.");
                app.push_message("");
                app.push_message("Enter a name for your profile (e.g. 'dev'):");
                app.ui_state = UiState::Setup(SetupStep::ProfileName);
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
                app.push_message("");
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
                app.push_message("");
                app.push_message("Enter your Postgres connection string:");
                app.ui_state = UiState::Setup(SetupStep::ConnectionString);
            }
        }
        UiState::Setup(SetupStep::ConnectionString) => {
            let conn_str = input.trim();
            if conn_str.is_empty() {
                app.push_message("Connection string cannot be empty.");
                return Ok(());
            }

            // Basic validation
            if !conn_str.starts_with("postgres://") && !conn_str.starts_with("postgresql://") {
                app.push_message("Only PostgreSQL is supported at the moment. Support for other databases is coming soon!");
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

            // Spawn schema fetch task
            let tx = app.tx.clone();
            let conn_string = conn_str.to_string();

            app.runtime.spawn(async move {
                tx.send(AppEvent::Log("Connecting to database...".into())).ok();
                match PgPoolOptions::new().connect(&conn_string).await {
                    Ok(pool) => {
                        tx.send(AppEvent::Log("Connected! Fetching schemas...".into())).ok();

                        // Fetch schemas
                        let schemas_result = sqlx::query!(
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
                                let schemas: Vec<String> = rows
                                    .into_iter()
                                    .filter_map(|r| r.schema_name)
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
                            "Connection failed: {}",
                            e
                        ))))
                        .ok();
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
            if !conn_str.starts_with("postgres://") && !conn_str.starts_with("postgresql://") {
                app.push_message("Only PostgreSQL is supported at the moment.");
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
            if !conn_str.starts_with("postgres://") && !conn_str.starts_with("postgresql://") {
                app.push_message("Only PostgreSQL is supported at the moment.");
                return Ok(());
            }
            app.session_conn = Some(conn_str.to_string());
            app.ui_state = UiState::Normal;
            app.push_message("session connection updated for this run.");
        }
        UiState::Setup(SetupStep::SelectSchema) => {
            // Handled by key navigation mostly, but if enter is pressed:
            let selected_schema = if app.available_schemas.is_empty() {
                None
            } else {
                Some(vec![app.available_schemas[app.schema_picker_idx].clone()])
            };

            // Now Spawn Introspection
            app.ui_state = UiState::Setup(SetupStep::Introspecting);
            app.messages.clear(); // Clear schema list log if any

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

            app.runtime.spawn(async move {
                tx.send(AppEvent::Log("Starting introspection...".into()))
                    .ok();
                match PgPoolOptions::new().connect(&conn_string).await {
                    Ok(pool) => match introspect_postgres_with_options(&pool, options).await {
                        Ok(schema) => {
                            tx.send(AppEvent::Log("Introspection successful.".into()))
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
            });
        }
        UiState::Setup(SetupStep::Introspecting) => {
            // Ignore input while introspecting
        }
        UiState::Setup(SetupStep::LlmEnable) => {
            if matches!(input, "s" | "S" | "y" | "Y") {
                app.settings.llm_enabled = true;
                app.settings.llm_provider = LlmProvider::Gemini;
                let model = app
                    .llm_models
                    .models
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "gemini-1.5-flash".to_string());
                app.settings.llm_model = Some(model);
                save_settings(&app.paths, &app.settings)?;
            }
            app.ui_state = UiState::Normal;
            app.push_message("");
            app.push_message("Setup complete. Type /help to see commands.");
        }
        _ => {}
    }
    Ok(())
}
