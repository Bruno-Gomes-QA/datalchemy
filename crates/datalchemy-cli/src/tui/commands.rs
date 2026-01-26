use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use serde_json::Value;

use datalchemy_core::{DatabaseSchema, redact_connection_string, validate_schema};
use datalchemy_eval::{EvaluateOptions, EvaluationEngine, collect_schema_metrics};
use datalchemy_generate::{GenerateOptions, GenerationEngine};
use datalchemy_introspect::{IntrospectOptions, introspect_postgres_with_options};
use datalchemy_plan::{
    PLAN_VERSION, Plan, SchemaRef, Target, validate_plan, validate_plan_against_schema,
    validate_plan_json,
};

use crate::CliError;
use crate::tui::secrets::{VaultMeta, decrypt_from_file, encrypt_to_file, load_env_file};
use crate::tui::state::{App, AppEvent, PaletteEntry, SetupStep, UiState};
use crate::tui::utils::{
    append_line, command_with_id, extract_flag_value, list_dirs, list_preview_files,
    move_dir_contents, open_in_editor, read_head_lines, read_tail_lines, set_private_permissions,
};
use crate::workspace::{
    ApprovalPolicy, ArtifactStatus, DbProfile, DoctorLevel, LlmProvider, OutManifest, PlanMeta,
    PrivacyMode, RunManifest, RunOptions, WorkspaceMode, WorkspaceSettings, WriteIntent,
    load_or_create_llm_models, load_or_create_profiles, load_or_create_settings, new_artifact_id,
    run_doctor, save_profiles, save_settings, write_bytes_atomic, write_json_atomic,
};
use sqlx::{Row, postgres::PgPoolOptions};

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
            app.push_message(format!("unknown command: {command}"));
            Ok(())
        }
    }
}

pub fn cmd_help(app: &mut App) -> Result<(), CliError> {
    app.push_message("COMMANDS");
    app.push_message("workspace:");
    if app.paths.root.exists() {
        app.push_message("  /reset");
    } else {
        app.push_message("  /init");
    }
    app.push_message("  /status");
    app.push_message("  /doctor");
    app.push_message("  /logs [<run_id>]");
    app.push_message("  /open <path>");
    app.push_message("");
    app.push_message("db + profiles:");
    app.push_message("  /profiles list");
    app.push_message("  /profiles new <name> <conn_string>");
    app.push_message("  /profiles set <name>");
    app.push_message("  /profiles delete <name>");
    app.push_message("  /db session");
    app.push_message("  /db change");
    app.push_message("  /db show-current");
    app.push_message("  /db test");
    app.push_message("  /db privileges");
    app.push_message("");
    app.push_message("pipeline:");
    app.push_message("  /introspect [--schema <name> ...]");
    app.push_message("  /runs list");
    app.push_message("  /runs set <run_id>");
    app.push_message("  /runs inspect <run_id>");
    app.push_message("  /runs delete <run_id>");
    app.push_message("  /plans list");
    app.push_message("  /plans set <plan_id>");
    app.push_message("  /plan new");
    app.push_message("  /plan edit");
    app.push_message("  /plan validate");
    app.push_message("  /generate");
    app.push_message("  /out list");
    app.push_message("  /out preview <out_id>");
    app.push_message("  /eval [<out_id>]");
    app.push_message("");
    app.push_message("settings:");
    app.push_message("  /settings show");
    app.push_message("  /settings set <key> <value>");
    app.push_message("");
    app.push_message("llm + secrets:");
    app.push_message("  /llm models");
    app.push_message("  /llm set <provider> <model>");
    app.push_message("  /llm off");
    app.push_message("  /secrets status");
    app.push_message("  /secrets import-env");
    app.push_message("  /secrets store-session <passphrase>");
    app.push_message("  /secrets unlock <passphrase>");
    app.push_message("  /secrets delete");
    app.push_message("");
    app.push_message("/help");
    app.push_message("/exit");
    app.push_message("note: avoid passing secrets on the command line.");
    Ok(())
}

fn cmd_status(app: &mut App) -> Result<(), CliError> {
    app.push_message("");
    app.push_message("WORKSPACE STATUS");
    app.push_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    app.push_message(format!("Root:      {}", app.paths.root.display()));
    app.push_message(format!(
        "Profile:   {}",
        app.settings.active_profile.as_deref().unwrap_or("none")
    ));
    app.push_message(format!("Mode:      {}", app.mode_display()));
    app.push_message(format!("LLM:       {}", app.llm_display()));

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

    // Stats
    let run_count = app.iter_runs().count();
    let plan_count = app.iter_plans().count();
    let out_count = list_dirs(&app.paths.out_dir)?.len();
    app.push_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
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

    app.push_message("");
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
    app.input.clear();
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
        app.push_message("usage: /settings show | /settings set <key> <value>");
        return Ok(());
    }

    if args[0] == "show" {
        app.push_message("SETTINGS");
        app.push_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
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
        app.push_message("usage: /settings set <key> <value>");
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
        app.push_message("usage: /profiles list | new | set | delete");
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
                app.push_message("usage: /profiles new <name> <conn_string>");
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
                app.push_message("usage: /profiles set <name>");
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
                app.push_message("usage: /profiles delete <name>");
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
            app.push_message("usage: /profiles list | new | set | delete");
        }
    }
    Ok(())
}

fn cmd_db(app: &mut App, args: Vec<&str>) -> Result<(), CliError> {
    if args.is_empty() {
        // Trigger subcommand palette by setting input with space
        app.input = "/db ".to_string();
        // Since we are inside the command handler, the input is already drained.
        // Creating a new input state will be picked up by the next render loop.
        // The command palette logic will see "/db " and show subcommands.
        return Ok(());
    }

    match args[0] {
        "session" => {
            app.push_message("");
            app.push_message("Session connection (not saved).");
            app.push_message("Paste Postgres connection string:");
            app.ui_state = UiState::Setup(SetupStep::DbSession);
            app.input.clear();
        }
        "show-current" => {
            app.push_message("");
            app.push_message("DATABASE CONNECTION");
            app.push_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

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
            app.push_message("");
        }
        "change" => {
            // Pipeline like welcome
            app.push_message("");
            if app.settings.active_profile.is_none() {
                app.push_message("missing active profile. use /profiles new or /profiles set.");
                return Ok(());
            }
            app.push_message("Update session connection for active profile.");
            app.push_message("Paste Postgres connection string:");
            app.ui_state = UiState::Setup(SetupStep::DbChange);
            // Clear any residual input
            app.input.clear();
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
            app.runtime.spawn(async move {
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
            app.runtime.spawn(async move {
                match PgPoolOptions::new()
                    .acquire_timeout(std::time::Duration::from_secs(5))
                    .connect(&conn)
                    .await
                {
                    Ok(pool) => {
                        let q = sqlx::query("SELECT current_user, current_database(), version()");
                        match q.fetch_one(&pool).await {
                            Ok(row) => {
                                let user: String = row.try_get("current_user").unwrap_or_default();
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
            });
        }
        _ => {
            app.push_message("usage: /db [session|change|show-current|test|privileges]");
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

    let result = app.runtime.block_on(async {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&conn)
            .await?;
        let schema = introspect_postgres_with_options(&pool, options).await?;
        Ok::<DatabaseSchema, CliError>(schema)
    });

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
        app.push_message("usage: /runs list | set <run_id> | inspect <run_id> | delete <run_id>");
        return Ok(());
    }

    match args[0] {
        "list" => {
            let runs = list_dirs(&app.paths.runs_dir)?;
            if runs.is_empty() {
                app.push_message("no runs found.");
                return Ok(());
            }
            app.push_message("RUNS");
            app.push_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
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
                app.push_message("usage: /runs set <run_id>");
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
                app.push_message("usage: /runs inspect <run_id>");
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
            app.push_message("RUN DETAILS");
            app.push_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
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
            app.push_message("options:");
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
                app.push_message("usage: /runs delete <run_id>");
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
            app.push_message(
                "usage: /runs list | set <run_id> | inspect <run_id> | delete <run_id>",
            );
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
        app.push_message("usage: /plans list | set <plan_id>");
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
            app.push_message("usage: /plans set <plan_id>");
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

    app.push_message("usage: /plans list | set <plan_id>");
    Ok(())
}

fn cmd_plan(
    app: &mut App,
    args: Vec<&str>,
    bypass_approval: bool,
    raw: &str,
) -> Result<(), CliError> {
    if args.is_empty() {
        app.push_message("usage: /plan new|edit|validate");
        return Ok(());
    }
    match args[0] {
        "new" => cmd_plan_new(app, args.clone(), bypass_approval, raw),
        "edit" => cmd_plan_edit(app, bypass_approval, raw),
        "validate" => cmd_plan_validate(app),
        _ => {
            app.push_message("usage: /plan new|edit|validate");
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

    let plan = mock_plan(&schema);
    let plan_json = serde_json::to_vec_pretty(&plan)?;
    write_bytes_atomic(&plan_dir.join("plan.json"), &plan_json)?;

    let meta = PlanMeta {
        plan_id: plan_id.clone(),
        status: ArtifactStatus::Ok,
        schema_run_id: run_id,
        schema_fingerprint: schema.schema_fingerprint.clone(),
        provider: provider_label(&app.settings),
        model: app
            .settings
            .llm_model
            .clone()
            .unwrap_or_else(|| "mock".to_string()),
        mock: true,
        artifact_version: crate::workspace::ARTIFACT_VERSION.to_string(),
        cli_version: crate::workspace::CLI_VERSION.to_string(),
        created_at: Utc::now().to_rfc3339(),
        finished_at: Some(Utc::now().to_rfc3339()),
    };
    write_json_atomic(&plan_dir.join("plan.meta.json"), &meta)?;

    write_bytes_atomic(&plan_dir.join("prompt.txt"), b"mock plan generated")?;
    write_bytes_atomic(
        &plan_dir.join("llm_transcript.jsonl"),
        b"{\"role\":\"system\",\"content\":\"mock\"}\n",
    )?;

    app.settings.active_plan_id = Some(plan_id);
    save_settings(&app.paths, &app.settings)?;
    app.push_message("plan created.");
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

    open_in_editor(&plan_path)?;
    app.push_message("plan edited. run /plan validate.");
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

    match engine.run(&schema, &plan) {
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
        app.push_message("usage: /out list | preview <out_id>");
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

    if args[0] == "preview" && args.len() > 1 {
        let path = app.paths.out_dir.join(args[1]);
        if !path.exists() {
            app.push_message("output not found.");
            return Ok(());
        }
        let entries = list_preview_files(&path)?;
        for entry in entries {
            app.push_message(entry);
        }
        return Ok(());
    }

    app.push_message("usage: /out list | preview <out_id>");
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

    match engine.run(&schema, &plan, &dataset_dir) {
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
        app.push_message(line);
    }
    Ok(())
}

fn cmd_open(app: &mut App, args: Vec<&str>) -> Result<(), CliError> {
    if args.is_empty() {
        app.push_message("usage: /open <path>");
        return Ok(());
    }
    let path = PathBuf::from(args[0]);
    if !path.exists() {
        app.push_message("file not found.");
        return Ok(());
    }
    let lines = read_head_lines(&path, 80)?;
    for line in lines {
        app.push_message(line);
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
        app.push_message("usage: /secrets status|import-env|store-session|unlock|delete");
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
                app.push_message("usage: /secrets store-session <passphrase>");
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
                app.push_message("usage: /secrets unlock <passphrase>");
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
            app.push_message("usage: /secrets status|import-env|store-session|unlock|delete");
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
        app.push_message(format!(
            "llm: enabled={} provider={:?} model={}",
            app.settings.llm_enabled,
            app.settings.llm_provider,
            app.settings.llm_model.as_deref().unwrap_or("none")
        ));
        return Ok(());
    }

    match args[0] {
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
                app.push_message("usage: /llm set <provider> <model>");
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
            app.push_message("usage: /llm models|set|off");
        }
    }
    Ok(())
}

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

fn mock_plan(schema: &DatabaseSchema) -> Plan {
    let mut targets = Vec::new();
    for db_schema in &schema.schemas {
        for table in &db_schema.tables {
            targets.push(Target {
                schema: db_schema.name.clone(),
                table: table.name.clone(),
                rows: 10,
                strategy: None,
            });
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
        targets,
        rules: Vec::new(),
        rules_unsupported: Vec::new(),
        options: None,
    }
}

fn provider_label(settings: &WorkspaceSettings) -> String {
    match settings.llm_provider {
        LlmProvider::Gemini => "gemini".to_string(),
        LlmProvider::Off => "off".to_string(),
    }
}

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
            if part.starts_with("postgres://") || part.starts_with("postgresql://") {
                "<redacted>".to_string()
            } else {
                part.to_string()
            }
        })
        .collect();
    redacted.join(" ")
}

pub fn command_palette_matches(app: &App, input: &str) -> Vec<PaletteEntry> {
    if !input.starts_with('/') {
        return Vec::new();
    }

    // Check if we are matching subcommands
    let parts: Vec<&str> = input.split_whitespace().collect();

    // If we have "/db " (prefix + space), show db subcommands
    // Note: input.starts_with("/db ") handles the case where user typed space
    // and if parts.len() >= 1.
    if input.starts_with("/db ") {
        let _sub_query = if parts.len() > 1 { parts[1] } else { "" };
        let entries = vec![
            PaletteEntry {
                command: "/db session",
                description: "set ephemeral connection string",
            },
            PaletteEntry {
                command: "/db change",
                description: "interactive connection setup wizard",
            },
            PaletteEntry {
                command: "/db show-current",
                description: "show active connection details",
            },
            PaletteEntry {
                command: "/db test",
                description: "test connectivity",
            },
            PaletteEntry {
                command: "/db privileges",
                description: "inspect user privileges",
            },
        ];

        return entries
            .into_iter()
            .filter(|e| e.command.starts_with(input.trim()))
            .collect();
    }

    if input.starts_with("/profiles ") {
        let entries = vec![
            PaletteEntry {
                command: "/profiles list",
                description: "list profiles",
            },
            PaletteEntry {
                command: "/profiles new",
                description: "create profile",
            },
            PaletteEntry {
                command: "/profiles set",
                description: "set active profile",
            },
            PaletteEntry {
                command: "/profiles delete",
                description: "remove profile",
            },
        ];
        return entries
            .into_iter()
            .filter(|e| e.command.starts_with(input.trim()))
            .collect();
    }

    if input.starts_with("/plan ") {
        let entries = vec![
            PaletteEntry {
                command: "/plan new",
                description: "create a mock plan",
            },
            PaletteEntry {
                command: "/plan edit",
                description: "edit plan.json",
            },
            PaletteEntry {
                command: "/plan validate",
                description: "validate plan.json",
            },
        ];
        return entries
            .into_iter()
            .filter(|e| e.command.starts_with(input.trim()))
            .collect();
    }

    if input.starts_with("/runs ") {
        let entries = vec![
            PaletteEntry {
                command: "/runs list",
                description: "list runs",
            },
            PaletteEntry {
                command: "/runs set",
                description: "set active run",
            },
            PaletteEntry {
                command: "/runs inspect",
                description: "show run details",
            },
            PaletteEntry {
                command: "/runs delete",
                description: "delete run",
            },
        ];
        return entries
            .into_iter()
            .filter(|e| e.command.starts_with(input.trim()))
            .collect();
    }

    if input.starts_with("/plans ") {
        let entries = vec![
            PaletteEntry {
                command: "/plans list",
                description: "list plans",
            },
            PaletteEntry {
                command: "/plans set",
                description: "set active plan",
            },
        ];
        return entries
            .into_iter()
            .filter(|e| e.command.starts_with(input.trim()))
            .collect();
    }

    if input.starts_with("/out ") {
        let entries = vec![
            PaletteEntry {
                command: "/out list",
                description: "list outputs",
            },
            PaletteEntry {
                command: "/out preview",
                description: "list files for out_id",
            },
        ];
        return entries
            .into_iter()
            .filter(|e| e.command.starts_with(input.trim()))
            .collect();
    }

    if input.starts_with("/settings ") {
        let entries = vec![
            PaletteEntry {
                command: "/settings show",
                description: "show settings",
            },
            PaletteEntry {
                command: "/settings set",
                description: "update settings",
            },
        ];
        return entries
            .into_iter()
            .filter(|e| e.command.starts_with(input.trim()))
            .collect();
    }

    if input.starts_with("/secrets ") {
        let entries = vec![
            PaletteEntry {
                command: "/secrets status",
                description: "vault status",
            },
            PaletteEntry {
                command: "/secrets import-env",
                description: "load .env into session",
            },
            PaletteEntry {
                command: "/secrets store-session",
                description: "store session secrets",
            },
            PaletteEntry {
                command: "/secrets unlock",
                description: "unlock vault",
            },
            PaletteEntry {
                command: "/secrets delete",
                description: "delete vault",
            },
        ];
        return entries
            .into_iter()
            .filter(|e| e.command.starts_with(input.trim()))
            .collect();
    }

    if input.starts_with("/llm ") {
        let entries = vec![
            PaletteEntry {
                command: "/llm models",
                description: "list models",
            },
            PaletteEntry {
                command: "/llm set",
                description: "set provider/model",
            },
            PaletteEntry {
                command: "/llm off",
                description: "disable llm",
            },
        ];
        return entries
            .into_iter()
            .filter(|e| e.command.starts_with(input.trim()))
            .collect();
    }

    // Default main command matching
    let query = input.trim();
    let entries = command_palette_entries(app);
    if query == "/" {
        return entries;
    }
    entries
        .into_iter()
        .filter(|entry| entry.command.starts_with(query))
        .collect()
}

pub fn command_palette_entries(app: &App) -> Vec<PaletteEntry> {
    let mut entries = vec![
        PaletteEntry {
            command: "/profiles",
            description: "manage db profiles",
        },
        PaletteEntry {
            command: "/db",
            description: "set session connection",
        },
        PaletteEntry {
            command: "/introspect",
            description: "capture schema.json",
        },
        PaletteEntry {
            command: "/runs",
            description: "manage runs",
        },
        PaletteEntry {
            command: "/plans",
            description: "manage plans",
        },
        PaletteEntry {
            command: "/plan new",
            description: "create a mock plan",
        },
        PaletteEntry {
            command: "/plan edit",
            description: "edit plan.json",
        },
        PaletteEntry {
            command: "/plan validate",
            description: "validate plan.json",
        },
        PaletteEntry {
            command: "/generate",
            description: "generate CSV output",
        },
        PaletteEntry {
            command: "/out",
            description: "list outputs",
        },
        PaletteEntry {
            command: "/eval",
            description: "evaluate last output",
        },
        PaletteEntry {
            command: "/doctor",
            description: "diagnose workspace",
        },
        PaletteEntry {
            command: "/logs",
            description: "show logs tail",
        },
        PaletteEntry {
            command: "/open",
            description: "preview a file",
        },
        PaletteEntry {
            command: "/secrets",
            description: "vault + env helpers",
        },
        PaletteEntry {
            command: "/llm",
            description: "configure LLM",
        },
    ];

    entries.push(PaletteEntry {
        command: "/status",
        description: "show current configuration",
    });
    entries.push(PaletteEntry {
        command: "/settings",
        description: "configure workspace",
    });
    if app.paths.root.exists() {
        entries.push(PaletteEntry {
            command: "/reset",
            description: "delete workspace and restart",
        });
    } else {
        entries.push(PaletteEntry {
            command: "/init",
            description: "create workspace",
        });
    }
    entries.push(PaletteEntry {
        command: "/help",
        description: "show command list",
    });
    entries.push(PaletteEntry {
        command: "/exit",
        description: "quit",
    });

    entries
}
