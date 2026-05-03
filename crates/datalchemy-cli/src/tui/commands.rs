use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use serde_json::Value;

use datalchemy_core::{DatabaseSchema, redact_connection_string, validate_schema};
use datalchemy_eval::{EvaluateOptions, EvaluationEngine, collect_schema_metrics};
use datalchemy_generate::{GenerateOptions, GenerationEngine};
use datalchemy_introspect::{
    IntrospectOptions, introspect_postgres_with_options, introspect_sqlite_with_options,
};
use datalchemy_plan::{
    ColumnGeneratorRule, GeneratorRef, PLAN_VERSION, Plan, PlanGlobal, Rule, SchemaRef, Target,
    validate_plan, validate_plan_against_schema, validate_plan_json,
};

use crate::CliError;
use crate::tui::secrets::{VaultMeta, decrypt_from_file, encrypt_to_file, load_env_file};
use crate::tui::state::{App, AppEvent, PaletteEntry, PromptContext, SetupStep, UiState};
use crate::tui::utils::{
    append_line, command_with_id, csv_preview, extract_flag_value, list_dirs, list_preview_files,
    move_dir_contents, open_in_editor, read_head_lines, read_tail_lines, set_private_permissions,
};
use crate::workspace::{
    ApprovalPolicy, ArtifactStatus, DbProfile, DoctorLevel, LlmProvider, OutManifest, PlanMeta,
    PrivacyMode, RunManifest, RunOptions, WorkspaceMode, WorkspaceSettings, WriteIntent,
    load_or_create_llm_models, load_or_create_profiles, load_or_create_settings, new_artifact_id,
    run_doctor, save_profiles, save_settings, write_bytes_atomic, write_json_atomic,
};
use sqlx::{Row, postgres::PgPoolOptions};

use crate::tui::conn::{is_sqlite, is_supported_connection};

pub fn execute_command(app: &mut App, input: &str, bypass_approval: bool) -> Result<(), CliError> {
    let mut parts = input.split_whitespace();
    let command = match parts.next() {
        Some(cmd) => cmd,
        None => return Ok(()),
    };

    match command {
        "/help" => cmd_help(app),
        "/exit" => {
            app.should_quit = true;
            Ok(())
        }
        "/status" => cmd_status(app),
        "/init" => cmd_init(app, bypass_approval, input),
        "/reset" => cmd_reset(app),
        "/settings" => cmd_settings(app, parts.collect(), bypass_approval, input),
        "/profiles" => cmd_profiles(app, parts.collect(), bypass_approval, input),
        "/db" => cmd_db(app, parts.collect()),
        "/introspect" => cmd_introspect(app, parts.collect(), bypass_approval, input),
        "/runs" => cmd_runs(app, parts.collect(), bypass_approval, input),
        "/plans" => cmd_plans(app, parts.collect(), bypass_approval, input),
        "/plan" => cmd_plan(app, parts.collect(), bypass_approval, input),
        "/generate" => cmd_generate(app, parts.collect(), bypass_approval, input),
        "/out" => cmd_out(app, parts.collect()),
        "/eval" => cmd_eval(app, parts.collect(), bypass_approval, input),
        "/doctor" => cmd_doctor(app),
        "/logs" => cmd_logs(app, parts.collect()),
        "/open" => cmd_open(app, parts.collect()),
        "/secrets" => cmd_secrets(app, parts.collect(), bypass_approval, input),
        "/llm" => cmd_llm(app, parts.collect(), bypass_approval, input),
        _ => {
            app.push_message(format!("unknown command: {command}. type /help for list."));
            Ok(())
        }
    }
}

/// Start an interactive multi-step prompt flow.
fn start_prompt(app: &mut App, ctx: PromptContext) {
    if let Some(label) = ctx.current_prompt() {
        app.push_message(label.to_string());
    }
    app.input_clear();
    app.ui_state = UiState::Setup(SetupStep::Prompt(ctx));
}

pub fn cmd_help(app: &mut App) -> Result<(), CliError> {
    app.push_raw("COMMANDS");
    app.push_raw("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    app.push_raw("workspace:");
    if app.paths.root.exists() {
        app.push_raw("  /reset                  delete workspace and re-setup");
    } else {
        app.push_raw("  /init                   create workspace");
    }
    app.push_raw("  /status                 show current configuration");
    app.push_raw("  /doctor                 diagnose workspace issues");
    app.push_raw("  /logs [<run_id>]        show log tail");
    app.push_raw("  /open <path>            preview a file");
    app.push_raw("");
    app.push_raw("db + profiles:");
    app.push_raw("  /profiles list          list profiles");
    app.push_raw("  /profiles new <n> <url> create profile");
    app.push_raw("  /profiles set <name>    set active profile");
    app.push_raw("  /profiles delete <name> remove profile");
    app.push_raw("  /db session             set ephemeral connection");
    app.push_raw("  /db change              update active connection");
    app.push_raw("  /db show-current        show connection details");
    app.push_raw("  /db test                test connectivity");
    app.push_raw("  /db privileges          inspect user/db info");
    app.push_raw("");
    app.push_raw("pipeline:");
    app.push_raw("  /introspect             capture schema.json from DB");
    app.push_raw("  /runs list              list introspection runs");
    app.push_raw("  /runs set <id>          set active run");
    app.push_raw("  /runs inspect <id>      show run details");
    app.push_raw("  /runs delete <id>       delete run");
    app.push_raw("  /plan new               create plan from schema");
    app.push_raw("  /plan edit              edit plan.json in editor");
    app.push_raw("  /plan show              show current plan summary");
    app.push_raw("  /plan validate          validate plan vs schema");
    app.push_raw("  /plans list             list all plans");
    app.push_raw("  /plans set <id>         set active plan");
    app.push_raw("  /generate               generate CSV outputs");
    app.push_raw("  /out list               list generated outputs");
    app.push_raw("  /out preview <id>       preview CSV files");
    app.push_raw("  /eval [<out_id>]        evaluate last output");
    app.push_raw("");
    app.push_raw("settings:");
    app.push_raw("  /settings show          show all settings");
    app.push_raw("  /settings set <k> <v>   update a setting");
    app.push_raw("");
    app.push_raw("secrets:");
    app.push_raw("  /secrets status         vault status");
    app.push_raw("  /secrets import-env     load .env into session");
    app.push_raw("  /secrets store-session  store session (encrypted)");
    app.push_raw("  /secrets unlock <pass>  unlock vault");
    app.push_raw("  /secrets delete         delete vault");
    app.push_raw("");
    app.push_raw("navigation:");
    app.push_raw("  Tab         autocomplete command");
    app.push_raw("  Up/Down     navigate palette / schema list");
    app.push_raw("  Esc         go back / clear input");
    app.push_raw("  Ctrl+A/E    home / end of input");
    app.push_raw("  Ctrl+W      delete word backward");
    app.push_raw("  Ctrl+U      clear input line");
    app.push_raw("  PageUp/Down scroll messages");
    app.push_raw("  /exit       quit");
    Ok(())
}

fn cmd_status(app: &mut App) -> Result<(), CliError> {
    app.push_raw("");
    app.push_raw("WORKSPACE STATUS");
    app.push_raw("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    app.push_message(format!("Root:      {}", app.paths.root.display()));
    app.push_message(format!(
        "Profile:   {}",
        app.settings.active_profile.as_deref().unwrap_or("none")
    ));
    app.push_message(format!("Mode:      {}", app.mode_display()));

    let env_conn = std::env::var("DATABASE_URL").ok();
    let file_conn = if env_conn.is_none() {
        load_env_file(Path::new(".env"))
            .ok()
            .and_then(|values| values.get("DATABASE_URL").cloned())
    } else {
        None
    };
    let conn_status = if let Some(conn) = &app.session_conn {
        let safe = redact_connection_string(conn);
        format!("session ({})", safe.redacted)
    } else if let Some(conn) = env_conn {
        let safe = redact_connection_string(&conn);
        format!("env ({})", safe.redacted)
    } else if let Some(conn) = file_conn {
        let safe = redact_connection_string(&conn);
        format!(".env ({})", safe.redacted)
    } else if let Some(safe_str) = app.active_profile_redacted() {
        format!("profile metadata ({safe_str})")
    } else {
        "none".to_string()
    };
    app.push_message(format!("Connection: {conn_status}"));

    let run_count = app.iter_runs().count();
    let plan_count = app.iter_plans().count();
    let out_count = list_dirs(&app.paths.out_dir)?.len();
    app.push_raw("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    app.push_message(format!(
        "Runs: {}  |  Plans: {}  |  Outputs: {}",
        run_count, plan_count, out_count
    ));
    app.push_message(format!(
        "Active Run: {}",
        app.settings.active_run_id.as_deref().unwrap_or("none")
    ));
    app.push_message(format!(
        "Active Plan: {}",
        app.settings.active_plan_id.as_deref().unwrap_or("none")
    ));
    app.push_raw("");
    Ok(())
}

fn cmd_init(app: &mut App, bypass_approval: bool, raw: &str) -> Result<(), CliError> {
    if !bypass_approval && app.requires_approval() {
        let intent = WriteIntent::new(
            "initialize workspace",
            vec![
                app.paths.root.clone(),
                app.paths.settings_path(),
                app.paths.profiles_path(),
                app.paths.llm_models_path(),
                app.paths.cli_log_path(),
            ],
        );
        return app.request_approval(intent, raw);
    }

    app.paths.ensure_dirs()?;
    if !app.paths.settings_path().exists() {
        save_settings(&app.paths, &WorkspaceSettings::default())?;
    }
    if !app.paths.profiles_path().exists() {
        save_profiles(&app.paths, &crate::workspace::ProfilesConfig::default())?;
    }

    let vault_meta_path = app.paths.vault_meta_path();
    if !vault_meta_path.exists() {
        let meta = VaultMeta {
            status: "absent".to_string(),
            created_at: Some(Utc::now().to_rfc3339()),
        };
        write_json_atomic(&vault_meta_path, &meta)?;
        set_private_permissions(&vault_meta_path)?;
    }
    append_line(&app.paths.cli_log_path(), "workspace initialized")?;

    app.settings = load_or_create_settings(&app.paths)?;
    app.profiles = load_or_create_profiles(&app.paths)?;
    app.llm_models = load_or_create_llm_models(&app.paths)?;
    app.push_message("workspace initialized.");
    Ok(())
}

fn cmd_reset(app: &mut App) -> Result<(), CliError> {
    if !app.paths.root.exists() {
        app.push_message("workspace not found. run /init.");
        return Ok(());
    }
    app.messages.clear();
    app.input_clear();
    app.ui_state = UiState::Setup(SetupStep::ConfirmReset);
    Ok(())
}

fn cmd_settings(
    app: &mut App,
    args: Vec<&str>,
    bypass_approval: bool,
    raw: &str,
) -> Result<(), CliError> {
    if args.is_empty() {
        app.input_set("/settings ".to_string());
        return Ok(());
    }

    if args[0] == "show" {
        app.push_raw("SETTINGS");
        app.push_raw("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        app.push_message(format!(
            "approval_policy: {:?}",
            app.settings.approval_policy
        ));
        app.push_message(format!("mode:            {:?}", app.settings.mode));
        app.push_message(format!("privacy:         {:?}", app.settings.privacy));
        app.push_message(format!("llm_enabled:     {}", app.settings.llm_enabled));
        app.push_message(format!("llm_provider:    {:?}", app.settings.llm_provider));
        app.push_message(format!(
            "llm_model:       {}",
            app.settings.llm_model.as_deref().unwrap_or("none")
        ));
        app.push_message(format!(
            "active_profile:  {}",
            app.settings.active_profile.as_deref().unwrap_or("none")
        ));
        app.push_message(format!(
            "active_run_id:   {}",
            app.settings.active_run_id.as_deref().unwrap_or("none")
        ));
        app.push_message(format!(
            "active_plan_id:  {}",
            app.settings.active_plan_id.as_deref().unwrap_or("none")
        ));
        return Ok(());
    }

    if args.len() < 3 || args[0] != "set" {
        app.input_set("/settings set ".to_string());
        return Ok(());
    }

    if !bypass_approval && app.requires_approval() {
        let intent = WriteIntent::new("update settings", vec![app.paths.settings_path()]);
        return app.request_approval(intent, raw);
    }

    let key = args[1];
    let value = args[2];
    match key {
        "approval_policy" => {
            app.settings.approval_policy = parse_approval_policy(value)?;
        }
        "mode" => {
            app.settings.mode = parse_workspace_mode(value)?;
        }
        "privacy" => {
            app.settings.privacy = parse_privacy_mode(value)?;
        }
        "llm_enabled" => {
            app.settings.llm_enabled = value == "true";
        }
        "llm_provider" => {
            app.settings.llm_provider = parse_llm_provider(value)?;
        }
        "llm_model" => {
            app.settings.llm_model = Some(value.to_string());
        }
        _ => {
            app.push_message("unknown settings key");
            return Ok(());
        }
    }

    save_settings(&app.paths, &app.settings)?;
    app.push_message("settings updated.");
    Ok(())
}

fn cmd_profiles(
    app: &mut App,
    args: Vec<&str>,
    bypass_approval: bool,
    raw: &str,
) -> Result<(), CliError> {
    if args.is_empty() {
        app.input_set("/profiles ".to_string());
        return Ok(());
    }

    if args[0] == "list" {
        if app.profiles.profiles.is_empty() {
            app.push_message("no profiles configured.");
            return Ok(());
        }
        let mut names: Vec<String> = app.profiles.profiles.keys().cloned().collect();
        names.sort();
        for name in names {
            let active = app
                .settings
                .active_profile
                .as_deref()
                .map(|value| value == name)
                .unwrap_or(false);
            app.push_message(format!("{}{}", if active { "* " } else { "  " }, name));
        }
        return Ok(());
    }

    match args[0] {
        "new" => {
            if args.len() < 3 {
                start_prompt(
                    app,
                    PromptContext::new(
                        "/profiles new",
                        vec![
                            "Profile name:",
                            "Database connection string (postgres:// or sqlite://):",
                        ],
                    ),
                );
                return Ok(());
            }
            if !bypass_approval && app.requires_approval() {
                let intent = WriteIntent::new(
                    "create profile",
                    vec![app.paths.profiles_path(), app.paths.settings_path()],
                );
                return app.request_approval(intent, raw);
            }
            let name = args[1].to_string();
            let conn = args[2];
            let profile = DbProfile::from_connection(conn);
            app.profiles.profiles.insert(name.clone(), profile);
            app.settings.active_profile = Some(name);
            app.session_conn = Some(conn.to_string());
            save_profiles(&app.paths, &app.profiles)?;
            save_settings(&app.paths, &app.settings)?;
            app.push_message("profile created.");
        }
        "set" => {
            if args.len() < 2 {
                app.input_set("/profiles set ".to_string());
                return Ok(());
            }
            if !bypass_approval && app.requires_approval() {
                let intent =
                    WriteIntent::new("set active profile", vec![app.paths.settings_path()]);
                return app.request_approval(intent, raw);
            }
            let name = args[1];
            if !app.profiles.profiles.contains_key(name) {
                app.push_message("profile not found.");
                return Ok(());
            }
            app.settings.active_profile = Some(name.to_string());
            save_settings(&app.paths, &app.settings)?;
            app.push_message("active profile updated.");
        }
        "delete" => {
            if args.len() < 2 {
                app.input_set("/profiles delete ".to_string());
                return Ok(());
            }
            if !bypass_approval && app.requires_approval() {
                let intent = WriteIntent::new(
                    "delete profile",
                    vec![app.paths.profiles_path(), app.paths.settings_path()],
                );
                return app.request_approval(intent, raw);
            }
            let name = args[1];
            app.profiles.profiles.remove(name);
            if app.settings.active_profile.as_deref() == Some(name) {
                app.settings.active_profile = None;
            }
            save_profiles(&app.paths, &app.profiles)?;
            save_settings(&app.paths, &app.settings)?;
            app.push_message("profile deleted.");
        }
        _ => {
            app.input_set("/profiles ".to_string());
        }
    }
    Ok(())
}

fn cmd_db(app: &mut App, args: Vec<&str>) -> Result<(), CliError> {
    if args.is_empty() {
        app.input_set("/db ".to_string());
        return Ok(());
    }

    match args[0] {
        "session" => {
            app.push_message("Session connection (not saved). Paste database connection string:");
            app.ui_state = UiState::Setup(SetupStep::DbSession);
            app.input_clear();
        }
        "show-current" => {
            app.push_raw("");
            app.push_raw("DATABASE CONNECTION");
            app.push_raw("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            let env_conn = std::env::var("DATABASE_URL").ok();
            let file_conn = if env_conn.is_none() {
                load_env_file(Path::new(".env"))
                    .ok()
                    .and_then(|values| values.get("DATABASE_URL").cloned())
            } else {
                None
            };
            let conn_opt = app
                .session_conn
                .clone()
                .or(env_conn.clone())
                .or(file_conn.clone())
                .or_else(|| app.active_profile_redacted());

            if let Some(conn) = conn_opt {
                let redacted = redact_connection_string(&conn);
                app.push_message(format!("URL:      {}", redacted.redacted));
                if let Some(host) = redacted.host {
                    app.push_message(format!("Host:     {}", host));
                }
                if let Some(db) = redacted.database {
                    app.push_message(format!("Database: {}", db));
                }
                if let Some(user) = redacted.user {
                    app.push_message(format!("User:     {}", user));
                }
                if app.session_conn.is_some() {
                    app.push_message("Status:   Session (ephemeral)");
                } else if env_conn.is_some() {
                    app.push_message("Status:   Env (DATABASE_URL)");
                } else if file_conn.is_some() {
                    app.push_message("Status:   .env (DATABASE_URL)");
                } else {
                    app.push_message("Status:   Profile metadata only (use /db session or .env)");
                }
            } else {
                app.push_message("Status:   No connection configured");
            }
            app.push_raw("");
        }
        "change" => {
            if app.settings.active_profile.is_none() {
                app.push_message("missing active profile. use /profiles new or /profiles set.");
                return Ok(());
            }
            app.push_message("Update session connection. Paste database connection string:");
            app.ui_state = UiState::Setup(SetupStep::DbChange);
            app.input_clear();
        }
        "test" => {
            let conn = match app.resolve_connection_string() {
                Ok(c) => c,
                Err(e) => {
                    app.push_message(e);
                    return Ok(());
                }
            };
            app.push_message("Testing connectivity...");
            let tx = app.tx.clone();
            let is_sq = is_sqlite(&conn);
            app.runtime.spawn(async move {
                if is_sq {
                    match sqlx::sqlite::SqlitePoolOptions::new()
                        .acquire_timeout(std::time::Duration::from_secs(5))
                        .connect(&conn)
                        .await
                    {
                        Ok(_) => {
                            tx.send(AppEvent::Log("Connection successful!".into())).ok();
                        }
                        Err(e) => {
                            tx.send(AppEvent::Log(format!("Connection failed: {}", e)))
                                .ok();
                        }
                    }
                } else {
                    match PgPoolOptions::new()
                        .acquire_timeout(std::time::Duration::from_secs(5))
                        .connect(&conn)
                        .await
                    {
                        Ok(_) => {
                            tx.send(AppEvent::Log("Connection successful!".into())).ok();
                        }
                        Err(e) => {
                            tx.send(AppEvent::Log(format!("Connection failed: {}", e)))
                                .ok();
                        }
                    };
                }
            });
        }
        "privileges" => {
            let conn = match app.resolve_connection_string() {
                Ok(c) => c,
                Err(e) => {
                    app.push_message(e);
                    return Ok(());
                }
            };
            app.push_message("Fetching info...");
            let tx = app.tx.clone();
            let is_sq = is_sqlite(&conn);
            app.runtime.spawn(async move {
                if is_sq {
                    match sqlx::sqlite::SqlitePoolOptions::new()
                        .acquire_timeout(std::time::Duration::from_secs(5))
                        .connect(&conn)
                        .await
                    {
                        Ok(pool) => {
                            let q = sqlx::query("SELECT sqlite_version() AS ver");
                            match q.fetch_one(&pool).await {
                                Ok(row) => {
                                    let ver: String = row.try_get("ver").unwrap_or_default();
                                    tx.send(AppEvent::Log(format!("Engine: SQLite, Ver: {}", ver)))
                                        .ok();
                                }
                                Err(e) => {
                                    tx.send(AppEvent::Log(format!("Query failed: {}", e))).ok();
                                }
                            }
                        }
                        Err(e) => {
                            tx.send(AppEvent::Log(format!("Connection failed: {}", e)))
                                .ok();
                        }
                    }
                } else {
                    match PgPoolOptions::new()
                        .acquire_timeout(std::time::Duration::from_secs(5))
                        .connect(&conn)
                        .await
                    {
                        Ok(pool) => {
                            let q =
                                sqlx::query("SELECT current_user, current_database(), version()");
                            match q.fetch_one(&pool).await {
                                Ok(row) => {
                                    let user: String =
                                        row.try_get("current_user").unwrap_or_default();
                                    let db: String =
                                        row.try_get("current_database").unwrap_or_default();
                                    let ver: String = row.try_get("version").unwrap_or_default();
                                    let short_ver = ver.split_whitespace().next().unwrap_or("?");
                                    tx.send(AppEvent::Log(format!(
                                        "User: {}, DB: {}, Ver: {}",
                                        user, db, short_ver
                                    )))
                                    .ok();
                                }
                                Err(e) => {
                                    tx.send(AppEvent::Log(format!("Query failed: {}", e))).ok();
                                }
                            }
                        }
                        Err(e) => {
                            tx.send(AppEvent::Log(format!("Connection failed: {}", e)))
                                .ok();
                        }
                    };
                }
            });
        }
        _ => {
            app.input_set("/db ".to_string());
        }
    }
    Ok(())
}

fn cmd_introspect(
    app: &mut App,
    args: Vec<&str>,
    bypass_approval: bool,
    raw: &str,
) -> Result<(), CliError> {
    if !app.paths.root.exists() {
        app.push_message("workspace missing. run /init first.");
        return Ok(());
    }
    if app.settings.active_profile.is_none() {
        app.push_message("no active profile. run will be labeled as session.");
    }

    let run_id = extract_flag_value(&args, "--run-id").unwrap_or_else(|| new_artifact_id("run"));
    let conn = match app.resolve_connection_string() {
        Ok(value) => value,
        Err(message) => {
            app.push_message(message);
            return Ok(());
        }
    };

    if !bypass_approval && app.requires_approval() {
        let intent = WriteIntent::new(
            "create run artifacts",
            vec![app.paths.runs_dir.join(&run_id)],
        );
        return app.request_approval(intent, &command_with_id(raw, "--run-id", &run_id));
    }

    let options = parse_introspect_options(&args);
    let strict = args.iter().any(|arg| *arg == "--strict");
    let run_dir = app.paths.runs_dir.join(&run_id);
    std::fs::create_dir_all(&run_dir)?;

    let redacted = redact_connection_string(&conn);
    let config_path = run_dir.join("config.redacted.json");
    write_json_atomic(&config_path, &redacted)?;

    let manifest_path = run_dir.join("run_manifest.json");
    let manifest = RunManifest {
        run_id: run_id.clone(),
        status: ArtifactStatus::Running,
        db_profile: app
            .settings
            .active_profile
            .clone()
            .unwrap_or_else(|| "session".to_string()),
        introspect_options: RunOptions {
            include_system_schemas: options.include_system_schemas,
            include_views: options.include_views,
            include_materialized_views: options.include_materialized_views,
            include_foreign_tables: options.include_foreign_tables,
            include_indexes: options.include_indexes,
            include_comments: options.include_comments,
            schemas: options.schemas.clone(),
        },
        schema_fingerprint: None,
        artifact_version: crate::workspace::ARTIFACT_VERSION.to_string(),
        cli_version: crate::workspace::CLI_VERSION.to_string(),
        created_at: Utc::now().to_rfc3339(),
        finished_at: None,
    };
    write_json_atomic(&manifest_path, &manifest)?;

    let logs_path = run_dir.join("logs.ndjson");
    append_line(&logs_path, "{\"event\":\"run_started\"}")?;

    app.start_task("Introspecting database...");
    let is_sq = is_sqlite(&conn);
    let result = app.runtime.block_on(async {
        if is_sq {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(5)
                .acquire_timeout(Duration::from_secs(10))
                .connect(&conn)
                .await?;
            let schema = introspect_sqlite_with_options(&pool, options).await?;
            Ok::<DatabaseSchema, CliError>(schema)
        } else {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(5)
                .acquire_timeout(Duration::from_secs(10))
                .connect(&conn)
                .await?;
            let schema = introspect_postgres_with_options(&pool, options).await?;
            Ok::<DatabaseSchema, CliError>(schema)
        }
    });
    app.finish_task();

    match result {
        Ok(schema) => {
            validate_schema(&schema)?;
            let metrics = collect_schema_metrics(&schema);
            write_json_atomic(&run_dir.join("schema.json"), &schema)?;
            write_json_atomic(&run_dir.join("metrics.json"), &metrics)?;

            if strict && metrics.fk_graph.has_cycle {
                append_line(
                    &logs_path,
                    "{\"event\":\"run_finished\",\"status\":\"ERROR\"}",
                )?;
                let mut final_manifest = manifest.clone();
                final_manifest.status = ArtifactStatus::Error;
                final_manifest.finished_at = Some(Utc::now().to_rfc3339());
                final_manifest.schema_fingerprint = schema.schema_fingerprint.clone();
                write_json_atomic(&manifest_path, &final_manifest)?;
                app.push_message("introspect failed: foreign key cycles detected.");
                return Ok(());
            }

            append_line(&logs_path, "{\"event\":\"run_finished\",\"status\":\"OK\"}")?;

            let mut final_manifest = manifest.clone();
            final_manifest.status = ArtifactStatus::Ok;
            final_manifest.finished_at = Some(Utc::now().to_rfc3339());
            final_manifest.schema_fingerprint = schema.schema_fingerprint.clone();
            write_json_atomic(&manifest_path, &final_manifest)?;

            app.settings.active_run_id = Some(run_id);
            save_settings(&app.paths, &app.settings)?;
            app.push_message("introspect completed.");
        }
        Err(err) => {
            append_line(
                &logs_path,
                "{\"event\":\"run_finished\",\"status\":\"ERROR\"}",
            )?;
            let mut final_manifest = manifest;
            final_manifest.status = ArtifactStatus::Error;
            final_manifest.finished_at = Some(Utc::now().to_rfc3339());
            write_json_atomic(&manifest_path, &final_manifest)?;
            app.push_message(format!("introspect failed: {err}"));
        }
    }

    Ok(())
}

fn cmd_runs(
    app: &mut App,
    args: Vec<&str>,
    bypass_approval: bool,
    raw: &str,
) -> Result<(), CliError> {
    if args.is_empty() {
        app.input_set("/runs ".to_string());
        return Ok(());
    }

    match args[0] {
        "list" => {
            let runs = list_dirs(&app.paths.runs_dir)?;
            if runs.is_empty() {
                app.push_message("no runs found.");
                return Ok(());
            }
            app.push_raw("RUNS");
            app.push_raw("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            for run in runs {
                let active = app.settings.active_run_id.as_deref() == Some(run.as_str());
                let label = if active { "active" } else { " " };
                app.push_message(format!("{label:>6}  {run}"));
            }
            app.push_message(format!(
                "active run: {}",
                app.settings.active_run_id.as_deref().unwrap_or("none")
            ));
        }
        "set" => {
            if args.len() < 2 {
                app.input_set("/runs set ".to_string());
                return Ok(());
            }
            if !bypass_approval && app.requires_approval() {
                let intent = WriteIntent::new("set active run", vec![app.paths.settings_path()]);
                return app.request_approval(intent, raw);
            }
            let run_id = args[1].to_string();
            app.settings.active_run_id = Some(run_id);
            save_settings(&app.paths, &app.settings)?;
            app.push_message("active run updated.");
        }
        "inspect" => {
            if args.len() < 2 {
                app.input_set("/runs inspect ".to_string());
                return Ok(());
            }
            let run_id = args[1];
            let manifest_path = app.paths.runs_dir.join(run_id).join("run_manifest.json");
            if !manifest_path.exists() {
                app.push_message("run_manifest.json not found.");
                return Ok(());
            }
            let manifest: RunManifest =
                serde_json::from_str(&std::fs::read_to_string(manifest_path)?)?;
            app.push_raw("RUN DETAILS");
            app.push_raw("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            app.push_message(format!("run_id:           {}", manifest.run_id));
            app.push_message(format!("status:           {:?}", manifest.status));
            app.push_message(format!("db_profile:       {}", manifest.db_profile));
            app.push_message(format!(
                "schema_fingerprint: {}",
                manifest.schema_fingerprint.as_deref().unwrap_or("none")
            ));
            app.push_message(format!("artifact_version: {}", manifest.artifact_version));
            app.push_message(format!("cli_version:      {}", manifest.cli_version));
            app.push_message(format!("created_at:       {}", manifest.created_at));
            app.push_message(format!(
                "finished_at:      {}",
                manifest.finished_at.as_deref().unwrap_or("running")
            ));
            app.push_raw("options:");
            app.push_message(format!(
                "  schemas: {:?}",
                manifest
                    .introspect_options
                    .schemas
                    .as_deref()
                    .unwrap_or(&[])
            ));
            app.push_message(format!(
                "  include_system_schemas: {}",
                manifest.introspect_options.include_system_schemas
            ));
            app.push_message(format!(
                "  include_views: {}",
                manifest.introspect_options.include_views
            ));
            app.push_message(format!(
                "  include_materialized_views: {}",
                manifest.introspect_options.include_materialized_views
            ));
            app.push_message(format!(
                "  include_foreign_tables: {}",
                manifest.introspect_options.include_foreign_tables
            ));
            app.push_message(format!(
                "  include_indexes: {}",
                manifest.introspect_options.include_indexes
            ));
            app.push_message(format!(
                "  include_comments: {}",
                manifest.introspect_options.include_comments
            ));
        }
        "delete" => {
            if args.len() < 2 {
                app.input_set("/runs delete ".to_string());
                return Ok(());
            }
            if !bypass_approval && app.requires_approval() {
                let intent = WriteIntent::new("delete run", vec![app.paths.runs_dir.join(args[1])]);
                return app.request_approval(intent, raw);
            }
            let run_id = args[1];
            let run_dir = app.paths.runs_dir.join(run_id);
            if !run_dir.exists() {
                app.push_message("run not found.");
                return Ok(());
            }
            std::fs::remove_dir_all(&run_dir)?;
            if app.settings.active_run_id.as_deref() == Some(run_id) {
                app.settings.active_run_id = None;
                save_settings(&app.paths, &app.settings)?;
            }
            app.push_message("run deleted.");
        }
        _ => {
            app.input_set("/runs ".to_string());
        }
    }
    Ok(())
}

fn cmd_plans(
    app: &mut App,
    args: Vec<&str>,
    bypass_approval: bool,
    raw: &str,
) -> Result<(), CliError> {
    if args.is_empty() {
        app.input_set("/plans ".to_string());
        return Ok(());
    }

    if args[0] == "list" {
        let plans = list_dirs(&app.paths.plans_dir)?;
        if plans.is_empty() {
            app.push_message("no plans found.");
            return Ok(());
        }
        for plan in plans {
            let active = app.settings.active_plan_id.as_deref() == Some(plan.as_str());
            app.push_message(format!("{}{}", if active { "* " } else { "  " }, plan));
        }
        return Ok(());
    }

    if args[0] == "set" {
        if args.len() < 2 {
            app.input_set("/plans set ".to_string());
            return Ok(());
        }
        if !bypass_approval && app.requires_approval() {
            let intent = WriteIntent::new("set active plan", vec![app.paths.settings_path()]);
            return app.request_approval(intent, raw);
        }
        let plan_id = args[1].to_string();
        app.settings.active_plan_id = Some(plan_id);
        save_settings(&app.paths, &app.settings)?;
        app.push_message("active plan updated.");
        return Ok(());
    }

    app.input_set("/plans ".to_string());
    Ok(())
}

fn cmd_plan(
    app: &mut App,
    args: Vec<&str>,
    bypass_approval: bool,
    raw: &str,
) -> Result<(), CliError> {
    if args.is_empty() {
        app.input_set("/plan ".to_string());
        return Ok(());
    }
    match args[0] {
        "new" => cmd_plan_new(app, args.clone(), bypass_approval, raw),
        "edit" => cmd_plan_edit(app, bypass_approval, raw),
        "show" => cmd_plan_show(app),
        "validate" => cmd_plan_validate(app),
        _ => {
            app.input_set("/plan ".to_string());
            Ok(())
        }
    }
}

fn cmd_plan_new(
    app: &mut App,
    args: Vec<&str>,
    bypass_approval: bool,
    raw: &str,
) -> Result<(), CliError> {
    let run_id = match &app.settings.active_run_id {
        Some(id) => id.clone(),
        None => {
            app.push_message("missing active run. use /introspect.");
            return Ok(());
        }
    };

    let plan_id = extract_flag_value(&args, "--plan-id").unwrap_or_else(|| new_artifact_id("plan"));
    if !bypass_approval && app.requires_approval() {
        let intent = WriteIntent::new(
            "create plan artifacts",
            vec![app.paths.plans_dir.join(&plan_id)],
        );
        return app.request_approval(intent, &command_with_id(raw, "--plan-id", &plan_id));
    }

    let schema_path = app.paths.runs_dir.join(&run_id).join("schema.json");
    let schema = read_schema(&schema_path)?;

    let plan_dir = app.paths.plans_dir.join(&plan_id);
    std::fs::create_dir_all(&plan_dir)?;

    app.start_task("Generating smart plan...");
    let plan = smart_plan(&schema);
    app.finish_task();

    let plan_json = serde_json::to_vec_pretty(&plan)?;
    write_bytes_atomic(&plan_dir.join("plan.json"), &plan_json)?;

    let meta = PlanMeta {
        plan_id: plan_id.clone(),
        status: ArtifactStatus::Ok,
        schema_run_id: run_id,
        schema_fingerprint: schema.schema_fingerprint.clone(),
        provider: "heuristic".to_string(),
        model: "smart_plan".to_string(),
        mock: false,
        artifact_version: crate::workspace::ARTIFACT_VERSION.to_string(),
        cli_version: crate::workspace::CLI_VERSION.to_string(),
        created_at: Utc::now().to_rfc3339(),
        finished_at: Some(Utc::now().to_rfc3339()),
    };
    write_json_atomic(&plan_dir.join("plan.meta.json"), &meta)?;

    write_bytes_atomic(
        &plan_dir.join("prompt.txt"),
        b"smart plan generated by heuristic engine",
    )?;

    // Count assigned generators for feedback
    let gen_count = plan.rules.len();
    let table_count = plan.targets.len();

    app.settings.active_plan_id = Some(plan_id);
    save_settings(&app.paths, &app.settings)?;
    app.push_message(format!(
        "plan created: {table_count} tables, {gen_count} generator rules, {} rows total.",
        plan.targets.iter().map(|t| t.rows).sum::<u64>()
    ));
    Ok(())
}

fn cmd_plan_show(app: &mut App) -> Result<(), CliError> {
    let plan_id = match &app.settings.active_plan_id {
        Some(id) => id.clone(),
        None => {
            app.push_message("missing active plan. use /plan new.");
            return Ok(());
        }
    };

    let plan_path = app.paths.plans_dir.join(&plan_id).join("plan.json");
    if !plan_path.exists() {
        app.push_message("plan.json not found.");
        return Ok(());
    }

    let plan_json: Value = serde_json::from_str(&std::fs::read_to_string(&plan_path)?)?;
    let plan = parse_plan(&plan_json)?;

    app.push_raw("PLAN SUMMARY");
    app.push_raw("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    app.push_message(format!("plan_id:      {}", plan_id));
    app.push_message(format!("plan_version: {}", plan.plan_version));
    app.push_message(format!("seed:         {}", plan.seed));
    if let Some(global) = &plan.global {
        if let Some(locale) = &global.locale {
            app.push_message(format!("locale:       {}", locale));
        }
    }
    app.push_raw("");
    app.push_raw("TARGETS:");
    for target in &plan.targets {
        app.push_message(format!(
            "  {}.{}: {} rows",
            target.schema, target.table, target.rows
        ));
    }
    app.push_raw("");
    app.push_raw(format!("RULES: {} total", plan.rules.len()));
    for rule in &plan.rules {
        match rule {
            Rule::ColumnGenerator(cg) => {
                app.push_message(format!(
                    "  {}.{}.{} -> {}",
                    cg.schema,
                    cg.table,
                    cg.column,
                    cg.generator.id()
                ));
            }
            Rule::ConstraintPolicy(cp) => {
                app.push_message(format!(
                    "  {}.{} constraint {:?} -> {:?}",
                    cp.schema, cp.table, cp.constraint, cp.mode
                ));
            }
            Rule::ForeignKeyStrategy(fk) => {
                app.push_message(format!(
                    "  {}.{} fk_strategy -> {:?}",
                    fk.schema, fk.table, fk.mode
                ));
            }
        }
    }
    if !plan.rules_unsupported.is_empty() {
        app.push_raw(format!(
            "UNSUPPORTED: {} rules",
            plan.rules_unsupported.len()
        ));
    }
    app.push_raw("");
    Ok(())
}

fn cmd_plan_edit(app: &mut App, bypass_approval: bool, raw: &str) -> Result<(), CliError> {
    let plan_id = match &app.settings.active_plan_id {
        Some(id) => id.clone(),
        None => {
            app.push_message("missing active plan. use /plan new.");
            return Ok(());
        }
    };
    if matches!(app.settings.mode, WorkspaceMode::Insert) {
        app.push_message("insert mode not implemented (planned).");
        return Ok(());
    }

    if !bypass_approval && app.requires_approval() {
        let intent = WriteIntent::new(
            "edit plan.json",
            vec![app.paths.plans_dir.join(&plan_id).join("plan.json")],
        );
        return app.request_approval(intent, raw);
    }

    let plan_path = app.paths.plans_dir.join(&plan_id).join("plan.json");
    if !plan_path.exists() {
        app.push_message("plan.json not found.");
        return Ok(());
    }

    match open_in_editor(&plan_path) {
        Ok(()) => app.push_message("plan edited. run /plan validate."),
        Err(_) => {
            app.push_message("could not open editor. set $EDITOR or $VISUAL env var.");
            app.push_message(format!("file: {}", plan_path.display()));
        }
    }
    Ok(())
}

fn cmd_plan_validate(app: &mut App) -> Result<(), CliError> {
    let plan_id = match &app.settings.active_plan_id {
        Some(id) => id.clone(),
        None => {
            app.push_message("missing active plan.");
            return Ok(());
        }
    };
    let run_id = match &app.settings.active_run_id {
        Some(id) => id.clone(),
        None => {
            app.push_message("missing active run.");
            return Ok(());
        }
    };

    let plan_path = app.paths.plans_dir.join(&plan_id).join("plan.json");
    let schema_path = app.paths.runs_dir.join(&run_id).join("schema.json");
    if !plan_path.exists() {
        app.push_message("plan.json not found.");
        return Ok(());
    }
    if !schema_path.exists() {
        app.push_message("schema.json not found.");
        return Ok(());
    }

    let plan_json: Value = serde_json::from_str(&std::fs::read_to_string(&plan_path)?)?;
    let schema = read_schema(&schema_path)?;

    let plan_schema = serde_json::to_value(datalchemy_plan::plan_json_schema())?;
    let mut report = validate_plan_json(&plan_json, &plan_schema)
        .map_err(|err| CliError::Plan(err.to_string()))?;
    let schema_report = validate_plan_against_schema(&parse_plan(&plan_json)?, &schema);
    report.merge(schema_report);

    if report.is_ok() {
        app.push_message("plan validation ok.");
    } else {
        for issue in report.errors {
            app.push_message(format!(
                "error: {} {} ({})",
                issue.code, issue.path, issue.message
            ));
        }
    }
    for warning in report.warnings {
        app.push_message(format!(
            "warning: {} {} ({})",
            warning.code, warning.path, warning.message
        ));
    }
    Ok(())
}

fn cmd_generate(
    app: &mut App,
    args: Vec<&str>,
    bypass_approval: bool,
    raw: &str,
) -> Result<(), CliError> {
    let run_id = match &app.settings.active_run_id {
        Some(id) => id.clone(),
        None => {
            app.push_message("missing active run. use /introspect.");
            return Ok(());
        }
    };
    let plan_id = match &app.settings.active_plan_id {
        Some(id) => id.clone(),
        None => {
            app.push_message("missing active plan. use /plan new.");
            return Ok(());
        }
    };

    let out_id = extract_flag_value(&args, "--out-id").unwrap_or_else(|| new_artifact_id("out"));
    if !bypass_approval && app.requires_approval() {
        let intent = WriteIntent::new("generate dataset", vec![app.paths.out_dir.join(&out_id)]);
        return app.request_approval(intent, &command_with_id(raw, "--out-id", &out_id));
    }

    let schema_path = app.paths.runs_dir.join(&run_id).join("schema.json");
    let plan_path = app.paths.plans_dir.join(&plan_id).join("plan.json");
    if !schema_path.exists() || !plan_path.exists() {
        app.push_message("schema or plan not found");
        return Ok(());
    }

    let schema = read_schema(&schema_path)?;
    let plan_json: Value = serde_json::from_str(&std::fs::read_to_string(&plan_path)?)?;
    let plan_schema = serde_json::to_value(datalchemy_plan::plan_json_schema())?;
    let validated = validate_plan(&plan_json, &plan_schema, &schema)
        .map_err(|_| CliError::Plan("plan validation failed".to_string()))?;
    let plan = validated.plan;

    let final_dir = app.paths.out_dir.join(&out_id);
    if final_dir.exists() {
        return Err(CliError::InvalidConfig(format!(
            "output directory already exists: {}",
            final_dir.display()
        )));
    }
    std::fs::create_dir_all(&final_dir)?;

    let mut manifest = OutManifest {
        out_id: out_id.clone(),
        status: ArtifactStatus::Running,
        schema_run_id: run_id,
        plan_id,
        mode: "csv".to_string(),
        seed: plan.seed,
        scale: plan.targets.iter().map(|t| t.rows).sum(),
        artifact_version: crate::workspace::ARTIFACT_VERSION.to_string(),
        cli_version: crate::workspace::CLI_VERSION.to_string(),
        created_at: Utc::now().to_rfc3339(),
        finished_at: None,
    };
    let manifest_path = final_dir.join("out_manifest.json");
    write_json_atomic(&manifest_path, &manifest)?;

    let options = GenerateOptions {
        out_dir: app.paths.out_dir.clone(),
        ..GenerateOptions::default()
    };
    let engine = GenerationEngine::new(options);

    app.start_task("Generating CSV data...");
    let gen_result = engine.run(&schema, &plan);
    app.finish_task();

    match gen_result {
        Ok(result) => {
            move_dir_contents(&result.run_dir, &final_dir)?;
            write_json_atomic(&final_dir.join("generation_report.json"), &result.report)?;
            app.write_profile_config(&final_dir)?;
            manifest.status = ArtifactStatus::Ok;
            manifest.finished_at = Some(Utc::now().to_rfc3339());
            write_json_atomic(&manifest_path, &manifest)?;
            app.last_out_id = Some(out_id);
            app.push_message("generation completed.");
        }
        Err(err) => {
            manifest.status = ArtifactStatus::Error;
            manifest.finished_at = Some(Utc::now().to_rfc3339());
            write_json_atomic(&manifest_path, &manifest)?;
            app.push_message(format!("generation failed: {err}"));
        }
    }
    Ok(())
}

fn cmd_out(app: &mut App, args: Vec<&str>) -> Result<(), CliError> {
    if args.is_empty() {
        app.input_set("/out ".to_string());
        return Ok(());
    }

    if args[0] == "list" {
        let outs = list_dirs(&app.paths.out_dir)?;
        if outs.is_empty() {
            app.push_message("no outputs found.");
        }
        for out in outs {
            let active = app.last_out_id.as_deref() == Some(out.as_str());
            app.push_message(format!("{}{}", if active { "* " } else { "  " }, out));
        }
        return Ok(());
    }

    if args[0] == "preview" {
        let out_id = if args.len() > 1 {
            args[1].to_string()
        } else if let Some(last) = &app.last_out_id {
            last.clone()
        } else {
            app.push_message("no outputs found. run /generate first.");
            return Ok(());
        };

        let path = app.paths.out_dir.join(&out_id);
        if !path.exists() {
            app.push_message("output not found.");
            return Ok(());
        }

        let entries = list_preview_files(&path)?;
        let csv_files: Vec<&String> = entries.iter().filter(|e| e.ends_with(".csv")).collect();

        if csv_files.is_empty() {
            app.push_message("no CSV files found in output.");
            for entry in &entries {
                app.push_message(format!("  {entry}"));
            }
            return Ok(());
        }

        app.push_raw(format!("OUTPUT PREVIEW: {out_id}"));
        app.push_raw("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        for csv_file in &csv_files {
            let csv_path = path.join(csv_file);
            app.push_raw(format!("── {} ──", csv_file));
            match csv_preview(&csv_path, 5) {
                Ok(lines) => {
                    for line in lines {
                        app.push_raw(line);
                    }
                }
                Err(e) => {
                    app.push_message(format!("  error reading: {e}"));
                }
            }
            app.push_raw("");
        }

        // Also list non-csv files
        let other: Vec<&String> = entries.iter().filter(|e| !e.ends_with(".csv")).collect();
        if !other.is_empty() {
            app.push_raw("other files:");
            for entry in other {
                app.push_message(format!("  {entry}"));
            }
        }
        return Ok(());
    }

    app.input_set("/out ".to_string());
    Ok(())
}

fn cmd_eval(
    app: &mut App,
    args: Vec<&str>,
    bypass_approval: bool,
    raw: &str,
) -> Result<(), CliError> {
    let out_id = if let Some(id) = extract_flag_value(&args, "--out-id") {
        id
    } else if !args.is_empty() {
        args[0].to_string()
    } else if let Some(last) = &app.last_out_id {
        last.clone()
    } else {
        app.push_message("missing out_id. use /out to list.");
        return Ok(());
    };

    let eval_id = extract_flag_value(&args, "--eval-id").unwrap_or_else(|| new_artifact_id("eval"));
    if !bypass_approval && app.requires_approval() {
        let intent = WriteIntent::new("evaluate dataset", vec![app.paths.eval_dir.join(&eval_id)]);
        return app.request_approval(intent, &command_with_id(raw, "--eval-id", &eval_id));
    }

    let run_id = match &app.settings.active_run_id {
        Some(id) => id.clone(),
        None => {
            app.push_message("missing active run.");
            return Ok(());
        }
    };
    let plan_id = match &app.settings.active_plan_id {
        Some(id) => id.clone(),
        None => {
            app.push_message("missing active plan.");
            return Ok(());
        }
    };

    let schema_path = app.paths.runs_dir.join(&run_id).join("schema.json");
    let plan_path = app.paths.plans_dir.join(&plan_id).join("plan.json");
    if !schema_path.exists() || !plan_path.exists() {
        app.push_message("schema or plan not found");
        return Ok(());
    }

    let schema = read_schema(&schema_path)?;
    let plan_json: Value = serde_json::from_str(&std::fs::read_to_string(&plan_path)?)?;
    let plan = parse_plan(&plan_json)?;

    let eval_dir = app.paths.eval_dir.join(&eval_id);
    std::fs::create_dir_all(&eval_dir)?;

    let mut options = EvaluateOptions::default();
    options.out_dir = Some(eval_dir.clone());
    let engine = EvaluationEngine::new(options);
    let dataset_dir = app.paths.out_dir.join(&out_id);
    if !dataset_dir.exists() {
        app.push_message("dataset not found for eval.");
        return Ok(());
    }
    let mut manifest = crate::workspace::EvalManifest {
        eval_id: eval_id.clone(),
        status: ArtifactStatus::Running,
        out_id: out_id.clone(),
        checks_enabled: vec![
            "fk_consistency".to_string(),
            "nullability".to_string(),
            "uniqueness".to_string(),
        ],
        artifact_version: crate::workspace::ARTIFACT_VERSION.to_string(),
        cli_version: crate::workspace::CLI_VERSION.to_string(),
        created_at: Utc::now().to_rfc3339(),
        finished_at: None,
    };
    let manifest_path = eval_dir.join("eval_manifest.json");
    write_json_atomic(&manifest_path, &manifest)?;

    app.start_task("Evaluating dataset...");
    let eval_result = engine.run(&schema, &plan, &dataset_dir);
    app.finish_task();

    match eval_result {
        Ok(result) => {
            write_json_atomic(&eval_dir.join("evaluation_report.json"), &result.metrics)?;
            app.write_profile_config(&eval_dir)?;
            manifest.status = ArtifactStatus::Ok;
            manifest.finished_at = Some(Utc::now().to_rfc3339());
            write_json_atomic(&manifest_path, &manifest)?;
            app.push_message("evaluation completed.");
        }
        Err(err) => {
            manifest.status = ArtifactStatus::Error;
            manifest.finished_at = Some(Utc::now().to_rfc3339());
            write_json_atomic(&manifest_path, &manifest)?;
            app.push_message(format!("evaluation failed: {err}"));
        }
    }
    Ok(())
}

fn cmd_doctor(app: &mut App) -> Result<(), CliError> {
    let report = run_doctor(&app.paths, &app.settings, &app.profiles)?;
    if report.issues.is_empty() {
        app.push_message("doctor: no issues found.");
        return Ok(());
    }

    for issue in report.issues {
        let level = match issue.level {
            DoctorLevel::Warning => "warn",
            DoctorLevel::Error => "error",
        };
        if let Some(hint) = issue.hint {
            app.push_message(format!("{level}: {} ({hint})", issue.message));
        } else {
            app.push_message(format!("{level}: {}", issue.message));
        }
    }
    Ok(())
}

fn cmd_logs(app: &mut App, args: Vec<&str>) -> Result<(), CliError> {
    let path = if args.is_empty() {
        app.paths.cli_log_path()
    } else {
        app.paths.runs_dir.join(args[0]).join("logs.ndjson")
    };
    if !path.exists() {
        app.push_message("log not found.");
        return Ok(());
    }
    let lines = read_tail_lines(&path, 50)?;
    for line in lines {
        app.push_raw(line);
    }
    Ok(())
}

fn cmd_open(app: &mut App, args: Vec<&str>) -> Result<(), CliError> {
    if args.is_empty() {
        start_prompt(app, PromptContext::new("/open", vec!["File path:"]));
        return Ok(());
    }
    let path = PathBuf::from(args[0]);
    if !path.exists() {
        app.push_message("file not found.");
        return Ok(());
    }
    let lines = read_head_lines(&path, 80)?;
    for line in lines {
        app.push_raw(line);
    }
    Ok(())
}

fn cmd_secrets(
    app: &mut App,
    args: Vec<&str>,
    bypass_approval: bool,
    raw: &str,
) -> Result<(), CliError> {
    if args.is_empty() {
        app.input_set("/secrets ".to_string());
        return Ok(());
    }

    match args[0] {
        "status" => {
            let meta_path = app.paths.vault_meta_path();
            if !meta_path.exists() {
                app.push_message("vault: absent");
                return Ok(());
            }
            let meta: VaultMeta = serde_json::from_str(&std::fs::read_to_string(meta_path)?)?;
            app.push_message(format!("vault: {}", meta.status));
        }
        "import-env" => {
            let env_path = PathBuf::from(".env");
            if !env_path.exists() {
                app.push_message(".env not found.");
                return Ok(());
            }
            let loaded = load_env_file(&env_path)?;
            if let Some(value) = loaded.get("DATABASE_URL") {
                app.session_conn = Some(value.clone());
            }
            app.push_message("env loaded into session.");
        }
        "store-session" => {
            if args.len() < 2 {
                start_prompt(
                    app,
                    PromptContext::new("/secrets store-session", vec!["Passphrase:"]),
                );
                return Ok(());
            }
            if !bypass_approval && app.requires_approval() {
                let intent = WriteIntent::new(
                    "store session secrets",
                    vec![app.paths.vault_db_path(), app.paths.vault_meta_path()],
                );
                return app.request_approval(intent, raw);
            }
            let Some(conn) = &app.session_conn else {
                app.push_message("no session connection to store.");
                return Ok(());
            };
            let passphrase = args[1];
            encrypt_to_file(&app.paths.vault_db_path(), passphrase, conn)?;
            let meta = VaultMeta {
                status: "locked".to_string(),
                created_at: Some(Utc::now().to_rfc3339()),
            };
            write_json_atomic(&app.paths.vault_meta_path(), &meta)?;
            set_private_permissions(&app.paths.vault_meta_path())?;
            app.push_message("vault stored (locked).");
        }
        "unlock" => {
            if args.len() < 2 {
                start_prompt(
                    app,
                    PromptContext::new("/secrets unlock", vec!["Passphrase:"]),
                );
                return Ok(());
            }
            let passphrase = args[1];
            let conn = decrypt_from_file(&app.paths.vault_db_path(), passphrase)?;
            app.session_conn = Some(conn);
            let meta = VaultMeta {
                status: "unlocked".to_string(),
                created_at: Some(Utc::now().to_rfc3339()),
            };
            write_json_atomic(&app.paths.vault_meta_path(), &meta)?;
            set_private_permissions(&app.paths.vault_meta_path())?;
            app.push_message("vault unlocked for this session.");
        }
        "delete" => {
            if !bypass_approval && app.requires_approval() {
                let intent = WriteIntent::new(
                    "delete vault secrets",
                    vec![
                        app.paths.vault_db_path(),
                        app.paths.vault_llm_path(),
                        app.paths.vault_meta_path(),
                    ],
                );
                return app.request_approval(intent, raw);
            }
            if app.paths.vault_db_path().exists() {
                std::fs::remove_file(app.paths.vault_db_path())?;
            }
            if app.paths.vault_llm_path().exists() {
                std::fs::remove_file(app.paths.vault_llm_path())?;
            }
            let meta = VaultMeta {
                status: "absent".to_string(),
                created_at: Some(Utc::now().to_rfc3339()),
            };
            write_json_atomic(&app.paths.vault_meta_path(), &meta)?;
            set_private_permissions(&app.paths.vault_meta_path())?;
            app.push_message("vault deleted.");
        }
        _ => {
            app.input_set("/secrets ".to_string());
        }
    }
    Ok(())
}

fn cmd_llm(
    app: &mut App,
    args: Vec<&str>,
    bypass_approval: bool,
    raw: &str,
) -> Result<(), CliError> {
    if args.is_empty() {
        app.input_set("/llm ".to_string());
        return Ok(());
    }

    match args[0] {
        "status" => {
            app.push_message(format!(
                "llm: enabled={} provider={:?} model={}",
                app.settings.llm_enabled,
                app.settings.llm_provider,
                app.settings.llm_model.as_deref().unwrap_or("none")
            ));
        }
        "models" => {
            let models = app.llm_models.models.clone();
            for model in models {
                app.push_message(model);
            }
        }
        "off" => {
            if !bypass_approval && app.requires_approval() {
                let intent = WriteIntent::new("disable llm", vec![app.paths.settings_path()]);
                return app.request_approval(intent, raw);
            }
            app.settings.llm_enabled = false;
            app.settings.llm_provider = LlmProvider::Off;
            app.settings.llm_model = None;
            save_settings(&app.paths, &app.settings)?;
            app.push_message("llm disabled.");
        }
        "set" => {
            if args.len() < 3 {
                start_prompt(
                    app,
                    PromptContext::new("/llm set", vec!["Provider (gemini):", "Model name:"]),
                );
                return Ok(());
            }
            if !bypass_approval && app.requires_approval() {
                let intent =
                    WriteIntent::new("update llm settings", vec![app.paths.settings_path()]);
                return app.request_approval(intent, raw);
            }
            app.settings.llm_enabled = true;
            app.settings.llm_provider = parse_llm_provider(args[1])?;
            app.settings.llm_model = Some(args[2].to_string());
            save_settings(&app.paths, &app.settings)?;
            app.push_message("llm settings updated.");
        }
        _ => {
            app.input_set("/llm ".to_string());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_approval_policy(value: &str) -> Result<ApprovalPolicy, CliError> {
    match value {
        "always_allow" => Ok(ApprovalPolicy::AlwaysAllow),
        "ask_each_time" => Ok(ApprovalPolicy::AskEachTime),
        _ => Err(CliError::InvalidConfig(format!(
            "invalid approval_policy: {value}"
        ))),
    }
}

fn parse_workspace_mode(value: &str) -> Result<WorkspaceMode, CliError> {
    match value {
        "readonly_csv" => Ok(WorkspaceMode::ReadonlyCsv),
        "insert" => Ok(WorkspaceMode::Insert),
        "explore" => Ok(WorkspaceMode::Explore),
        _ => Err(CliError::InvalidConfig(format!("invalid mode: {value}"))),
    }
}

fn parse_privacy_mode(value: &str) -> Result<PrivacyMode, CliError> {
    match value {
        "normal" => Ok(PrivacyMode::Normal),
        "paranoid" => Ok(PrivacyMode::Paranoid),
        _ => Err(CliError::InvalidConfig(format!("invalid privacy: {value}"))),
    }
}

fn parse_llm_provider(value: &str) -> Result<LlmProvider, CliError> {
    match value {
        "gemini" => Ok(LlmProvider::Gemini),
        "off" => Ok(LlmProvider::Off),
        _ => Err(CliError::InvalidConfig(format!(
            "invalid llm_provider: {value}"
        ))),
    }
}

pub fn parse_introspect_options(args: &[&str]) -> IntrospectOptions {
    let mut options = IntrospectOptions {
        include_system_schemas: false,
        include_views: false,
        include_materialized_views: false,
        include_foreign_tables: false,
        include_indexes: true,
        include_comments: false,
        schemas: None,
    };

    let mut schemas = Vec::new();
    let mut iter = args.iter().copied();
    while let Some(arg) = iter.next() {
        match arg {
            "--include-system-schemas" => options.include_system_schemas = true,
            "--include-views" => options.include_views = true,
            "--include-materialized_views" => options.include_materialized_views = true,
            "--include-foreign-tables" => options.include_foreign_tables = true,
            "--include-indexes" => options.include_indexes = true,
            "--include-comments" => options.include_comments = true,
            "--schema" => {
                if let Some(schema) = iter.next() {
                    schemas.push(schema.to_string());
                }
            }
            _ => {}
        }
    }

    if !schemas.is_empty() {
        options.schemas = Some(schemas);
    }
    options
}

fn read_schema(path: &Path) -> Result<DatabaseSchema, CliError> {
    let content = std::fs::read_to_string(path)?;
    let schema: DatabaseSchema = serde_json::from_str(&content)?;
    Ok(schema)
}

fn parse_plan(plan_json: &Value) -> Result<Plan, CliError> {
    serde_json::from_value(plan_json.clone()).map_err(|err| CliError::Plan(err.to_string()))
}

fn provider_label(settings: &WorkspaceSettings) -> String {
    match settings.llm_provider {
        LlmProvider::Gemini => "gemini".to_string(),
        LlmProvider::Off => "off".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Smart plan generation (heuristic-based, no LLM)
// ---------------------------------------------------------------------------

/// Generate a smart plan by analyzing column names and types from the schema.
/// Uses heuristic matching to assign appropriate faker-rs generators.
fn smart_plan(schema: &DatabaseSchema) -> Plan {
    let mut targets = Vec::new();
    let mut rules = Vec::new();

    for db_schema in &schema.schemas {
        for table in &db_schema.tables {
            targets.push(Target {
                schema: db_schema.name.clone(),
                table: table.name.clone(),
                rows: 50,
                strategy: None,
            });

            for column in &table.columns {
                // Skip identity/generated columns
                if column.identity.is_some() || column.generated.is_some() {
                    continue;
                }
                // Skip columns with serial-like defaults (sequences)
                if let Some(ref def) = column.default {
                    let lower = def.to_lowercase();
                    if lower.contains("nextval(") || lower.contains("gen_random_uuid") {
                        continue;
                    }
                }

                if let Some(gen_id) = guess_generator(&column.name, &column.column_type) {
                    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
                        schema: db_schema.name.clone(),
                        table: table.name.clone(),
                        column: column.name.clone(),
                        generator: GeneratorRef::Id(gen_id),
                        params: None,
                        transforms: Vec::new(),
                    }));
                }
            }
        }
    }

    Plan {
        plan_version: PLAN_VERSION.to_string(),
        seed: 42,
        schema_ref: SchemaRef {
            schema_version: schema.schema_version.clone(),
            schema_fingerprint: schema.schema_fingerprint.clone(),
            engine: schema.engine.clone(),
        },
        global: Some(PlanGlobal {
            locale: Some("pt_BR".to_string()),
        }),
        targets,
        rules,
        rules_unsupported: Vec::new(),
        options: None,
    }
}

/// Heuristic generator mapping based on column name patterns and SQL types.
fn guess_generator(col_name: &str, col_type: &datalchemy_core::ColumnType) -> Option<String> {
    let name = col_name.to_lowercase();
    let udt = col_type.udt_name.to_lowercase();
    let dtype = col_type.data_type.to_lowercase();

    // --- UUID type ---
    if udt == "uuid" {
        return Some("primitive.uuid".to_string());
    }

    // --- Boolean type ---
    if udt == "bool" || dtype.contains("boolean") {
        return Some("primitive.bool".to_string());
    }

    // --- Name-based heuristics (check before generic type fallback) ---

    // Email
    if name.contains("email") || name.contains("e_mail") {
        return Some("semantic.person.email".to_string());
    }

    // Phone / telefone
    if name.contains("phone")
        || name.contains("telefone")
        || name.contains("celular")
        || name.contains("fone")
    {
        return Some("faker.phone_number.raw.PhoneNumber".to_string());
    }

    // Person name patterns
    if name == "nome"
        || name == "name"
        || name == "nome_completo"
        || name == "full_name"
        || name == "fullname"
    {
        return Some("faker.name.raw.Name".to_string());
    }
    if name == "primeiro_nome" || name == "first_name" || name == "firstname" {
        return Some("faker.name.raw.FirstName".to_string());
    }
    if name == "sobrenome" || name == "last_name" || name == "lastname" || name == "ultimo_nome" {
        return Some("faker.name.raw.LastName".to_string());
    }

    // Company
    if name.contains("empresa")
        || name.contains("company")
        || name == "razao_social"
        || name == "nome_fantasia"
    {
        return Some("faker.company.raw.CompanyName".to_string());
    }

    // Address
    if name.contains("endereco") || name.contains("address") || name == "logradouro" {
        return Some("faker.address.raw.StreetName".to_string());
    }
    if name == "cidade" || name == "city" {
        return Some("faker.address.raw.CityName".to_string());
    }
    if name == "estado" || name == "state" || name == "uf" {
        return Some("faker.address.raw.StateName".to_string());
    }
    if name == "cep"
        || name == "zip"
        || name == "zipcode"
        || name == "zip_code"
        || name == "codigo_postal"
    {
        return Some("faker.address.raw.ZipCode".to_string());
    }
    if name == "pais" || name == "country" {
        return Some("faker.address.raw.CountryName".to_string());
    }

    // URL / website
    if name.contains("url") || name.contains("website") || name.contains("site") {
        return Some("faker.internet.raw.DomainSuffix".to_string());
    }

    // Description / text
    if name.contains("descricao")
        || name.contains("description")
        || name.contains("observacao")
        || name.contains("obs")
        || name.contains("notas")
        || name.contains("notes")
        || name.contains("comentario")
    {
        return Some("faker.lorem.raw.Sentence".to_string());
    }

    // Title / titulo
    if name == "titulo" || name == "title" || name == "assunto" || name == "subject" {
        return Some("faker.lorem.raw.Words".to_string());
    }

    // Monetary / valor
    if name.contains("valor")
        || name.contains("preco")
        || name.contains("price")
        || name.contains("amount")
        || name.contains("custo")
        || name.contains("cost")
        || name.contains("salario")
        || name.contains("salary")
    {
        return Some("primitive.float".to_string());
    }

    // Quantity / count
    if name.contains("quantidade")
        || name.contains("qty")
        || name.contains("quantity")
        || name.contains("count")
        || name.contains("total")
    {
        return Some("primitive.int".to_string());
    }

    // Percentage
    if name.contains("percentual") || name.contains("percent") || name.contains("taxa") {
        return Some("primitive.float".to_string());
    }

    // Status / tipo / category (enum-like text)
    if name == "status"
        || name == "tipo"
        || name == "type"
        || name == "categoria"
        || name == "category"
    {
        // For enum-like, let the generator engine use fallback
        return None;
    }

    // --- Type-based fallback ---

    // Timestamps
    if udt.contains("timestamp") || dtype.contains("timestamp") {
        return Some("primitive.timestamp".to_string());
    }
    if udt == "date" || dtype == "date" {
        return Some("primitive.date".to_string());
    }
    if udt == "time" || dtype.contains("time without") {
        return Some("primitive.time".to_string());
    }

    // Numeric types
    if udt == "int2" || udt == "int4" || udt == "int8" || udt == "serial" || udt == "bigserial" {
        return Some("primitive.int".to_string());
    }
    if udt == "float4" || udt == "float8" || udt == "numeric" || udt == "decimal" || udt == "money"
    {
        return Some("primitive.float".to_string());
    }

    // JSON
    if udt == "json" || udt == "jsonb" {
        return Some("primitive.json".to_string());
    }

    // Text: only if it's a non-trivial text column
    if udt == "text" || udt == "varchar" || udt.starts_with("varchar") || udt == "bpchar" {
        // Generic text columns get a lorem generator
        return Some("faker.lorem.raw.Word".to_string());
    }

    None
}

// ---------------------------------------------------------------------------
// Command log sanitization
// ---------------------------------------------------------------------------

pub fn sanitize_command_for_log(input: &str) -> String {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return String::new();
    }

    if parts[0] == "/secrets" {
        if let Some(sub) = parts.get(1) {
            if *sub == "store-session" || *sub == "unlock" {
                return format!("/secrets {} <redacted>", sub);
            }
        }
    }

    if parts[0] == "/profiles" {
        if let Some(sub) = parts.get(1) {
            if *sub == "new" {
                if let Some(name) = parts.get(2) {
                    if parts.len() > 3 {
                        return format!("/profiles new {} <redacted>", name);
                    }
                    return format!("/profiles new {}", name);
                }
            }
        }
    }

    let redacted: Vec<String> = parts
        .into_iter()
        .map(|part| {
            if part.starts_with("postgres://")
                || part.starts_with("postgresql://")
                || part.starts_with("sqlite://")
            {
                "<redacted>".to_string()
            } else {
                part.to_string()
            }
        })
        .collect();
    redacted.join(" ")
}

// ---------------------------------------------------------------------------
// Command palette
// ---------------------------------------------------------------------------

pub fn command_palette_matches(app: &App, input: &str) -> Vec<PaletteEntry> {
    if !input.starts_with('/') {
        return Vec::new();
    }

    let query = input.trim();

    // --- Dynamic subcommand palettes with real data ---

    // /profiles set <name> — show existing profile names
    if input.starts_with("/profiles set ") {
        let mut entries = Vec::new();
        let mut names: Vec<String> = app.profiles.profiles.keys().cloned().collect();
        names.sort();
        for name in names {
            entries.push(pe(&format!("/profiles set {}", name), "set active profile"));
        }
        return filter_entries(entries, query);
    }
    // /profiles delete <name> — show existing profile names
    if input.starts_with("/profiles delete ") {
        let mut entries = Vec::new();
        let mut names: Vec<String> = app.profiles.profiles.keys().cloned().collect();
        names.sort();
        for name in names {
            entries.push(pe(&format!("/profiles delete {}", name), "remove profile"));
        }
        return filter_entries(entries, query);
    }
    // /runs set|inspect|delete <run_id> — show existing run IDs
    if input.starts_with("/runs set ") {
        let runs: Vec<String> = app.iter_runs().collect();
        let entries: Vec<PaletteEntry> = runs
            .iter()
            .map(|r| pe(&format!("/runs set {}", r), "set active run"))
            .collect();
        return filter_entries(entries, query);
    }
    if input.starts_with("/runs inspect ") {
        let runs: Vec<String> = app.iter_runs().collect();
        let entries: Vec<PaletteEntry> = runs
            .iter()
            .map(|r| pe(&format!("/runs inspect {}", r), "show run details"))
            .collect();
        return filter_entries(entries, query);
    }
    if input.starts_with("/runs delete ") {
        let runs: Vec<String> = app.iter_runs().collect();
        let entries: Vec<PaletteEntry> = runs
            .iter()
            .map(|r| pe(&format!("/runs delete {}", r), "delete run"))
            .collect();
        return filter_entries(entries, query);
    }
    // /plans set <plan_id> — show existing plan IDs
    if input.starts_with("/plans set ") {
        let plans: Vec<String> = app.iter_plans().collect();
        let entries: Vec<PaletteEntry> = plans
            .iter()
            .map(|p| pe(&format!("/plans set {}", p), "set active plan"))
            .collect();
        return filter_entries(entries, query);
    }
    // /out preview <out_id> — show existing output IDs
    if input.starts_with("/out preview ") {
        let outs = list_dirs(&app.paths.out_dir).unwrap_or_default();
        let entries: Vec<PaletteEntry> = outs
            .iter()
            .map(|o| pe(&format!("/out preview {}", o), "preview output"))
            .collect();
        return filter_entries(entries, query);
    }
    // /settings set <key> <value> — show valid values for a key
    if input.starts_with("/settings set ") {
        let after = input.trim_start_matches("/settings set ").trim();
        let parts: Vec<&str> = after.split_whitespace().collect();
        if parts.len() >= 1 && !after.is_empty() && !after.ends_with(' ') {
            // User is typing a key name, show valid keys
            return filter_entries(settings_key_palette(), query);
        }
        if parts.len() == 1 && after.ends_with(' ') {
            // Key selected, show valid values
            let key = parts[0];
            return filter_entries(settings_value_palette(key), query);
        }
        if parts.len() >= 2 {
            // Key + partial value, show valid values filtered
            let key = parts[0];
            return filter_entries(settings_value_palette(key), query);
        }
        // No key typed yet, show all keys
        return filter_entries(settings_key_palette(), query);
    }

    // --- Static subcommand palettes ---

    if input.starts_with("/db ") {
        return filter_entries(
            vec![
                pe("/db session", "set ephemeral connection string"),
                pe("/db change", "interactive connection setup"),
                pe("/db show-current", "show active connection details"),
                pe("/db test", "test connectivity"),
                pe("/db privileges", "inspect user privileges"),
            ],
            query,
        );
    }
    if input.starts_with("/profiles ") {
        return filter_entries(
            vec![
                pe("/profiles list", "list profiles"),
                pe("/profiles new", "create profile"),
                pe("/profiles set", "set active profile"),
                pe("/profiles delete", "remove profile"),
            ],
            query,
        );
    }
    if input.starts_with("/plan ") {
        return filter_entries(
            vec![
                pe("/plan new", "generate smart plan from schema"),
                pe("/plan edit", "edit plan.json in editor"),
                pe("/plan show", "show current plan summary"),
                pe("/plan validate", "validate plan against schema"),
            ],
            query,
        );
    }
    if input.starts_with("/runs ") {
        return filter_entries(
            vec![
                pe("/runs list", "list runs"),
                pe("/runs set", "set active run"),
                pe("/runs inspect", "show run details"),
                pe("/runs delete", "delete run"),
            ],
            query,
        );
    }
    if input.starts_with("/plans ") {
        return filter_entries(
            vec![
                pe("/plans list", "list plans"),
                pe("/plans set", "set active plan"),
            ],
            query,
        );
    }
    if input.starts_with("/out ") {
        return filter_entries(
            vec![
                pe("/out list", "list outputs"),
                pe("/out preview", "preview CSV content"),
            ],
            query,
        );
    }
    if input.starts_with("/settings ") {
        return filter_entries(
            vec![
                pe("/settings show", "show settings"),
                pe("/settings set", "update settings"),
            ],
            query,
        );
    }
    if input.starts_with("/secrets ") {
        return filter_entries(
            vec![
                pe("/secrets status", "vault status"),
                pe("/secrets import-env", "load .env into session"),
                pe("/secrets store-session", "store session secrets"),
                pe("/secrets unlock", "unlock vault"),
                pe("/secrets delete", "delete vault"),
            ],
            query,
        );
    }
    if input.starts_with("/llm ") {
        return filter_entries(
            vec![
                pe("/llm status", "show llm configuration"),
                pe("/llm models", "list models"),
                pe("/llm set", "set provider/model"),
                pe("/llm off", "disable llm"),
            ],
            query,
        );
    }

    // Main palette
    let entries = command_palette_entries(app);
    if query == "/" {
        return entries;
    }
    entries
        .into_iter()
        .filter(|entry| entry.command.starts_with(query))
        .collect()
}

fn settings_key_palette() -> Vec<PaletteEntry> {
    vec![
        pe(
            "/settings set approval_policy",
            "always_allow | ask_each_time",
        ),
        pe("/settings set mode", "readonly_csv | insert | explore"),
        pe("/settings set privacy", "normal | paranoid"),
        pe("/settings set llm_enabled", "true | false"),
        pe("/settings set llm_provider", "gemini | off"),
        pe("/settings set llm_model", "model name"),
    ]
}

fn settings_value_palette(key: &str) -> Vec<PaletteEntry> {
    match key {
        "approval_policy" => vec![
            pe(
                "/settings set approval_policy always_allow",
                "auto-approve writes",
            ),
            pe(
                "/settings set approval_policy ask_each_time",
                "prompt before writes",
            ),
        ],
        "mode" => vec![
            pe("/settings set mode readonly_csv", "read-only CSV output"),
            pe("/settings set mode insert", "insert directly into DB"),
            pe("/settings set mode explore", "explore schema only"),
        ],
        "privacy" => vec![
            pe("/settings set privacy normal", "show connection details"),
            pe("/settings set privacy paranoid", "hide all connection info"),
        ],
        "llm_enabled" => vec![
            pe("/settings set llm_enabled true", "enable LLM"),
            pe("/settings set llm_enabled false", "disable LLM"),
        ],
        "llm_provider" => vec![
            pe("/settings set llm_provider gemini", "Google Gemini"),
            pe("/settings set llm_provider off", "disable provider"),
        ],
        _ => Vec::new(),
    }
}

fn pe(command: &str, description: &str) -> PaletteEntry {
    PaletteEntry {
        command: command.to_string(),
        description: description.to_string(),
    }
}

fn filter_entries(entries: Vec<PaletteEntry>, query: &str) -> Vec<PaletteEntry> {
    entries
        .into_iter()
        .filter(|e| e.command.starts_with(query))
        .collect()
}

pub fn command_palette_entries(app: &App) -> Vec<PaletteEntry> {
    let mut entries = vec![
        pe("/profiles", "manage db profiles"),
        pe("/db", "database connection"),
        pe("/introspect", "capture schema.json"),
        pe("/runs", "manage runs"),
        pe("/plans", "manage plans"),
        pe("/plan new", "generate smart plan from schema"),
        pe("/plan edit", "edit plan.json in editor"),
        pe("/plan show", "show plan summary"),
        pe("/plan validate", "validate plan against schema"),
        pe("/generate", "generate CSV output"),
        pe("/out", "list / preview outputs"),
        pe("/eval", "evaluate last output"),
        pe("/doctor", "diagnose workspace"),
        pe("/logs", "show logs tail"),
        pe("/open", "preview a file"),
        pe("/secrets", "vault + env helpers"),
        pe("/llm", "configure LLM"),
        pe("/status", "workspace configuration"),
        pe("/settings", "configure workspace"),
    ];

    if app.paths.root.exists() {
        entries.push(pe("/reset", "delete workspace and restart"));
    } else {
        entries.push(pe("/init", "create workspace"));
    }
    entries.push(pe("/help", "show command list"));
    entries.push(pe("/exit", "quit"));

    entries
}
