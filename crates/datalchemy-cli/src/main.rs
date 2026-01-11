mod registry;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::{Args, Parser, Subcommand};
use datalchemy_core::{redact_connection_string, validate_schema, Error as CoreError, SCHEMA_VERSION};
use datalchemy_eval::collect_schema_metrics;
use datalchemy_introspect::{introspect_postgres_with_options, IntrospectOptions};
use registry::{init_run_logging, start_run, write_metrics, write_schema, RunContext, RunOptions};
use sqlx::postgres::PgPoolOptions;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
enum CliError {
    #[error("registry error: {0}")]
    Registry(#[from] registry::RegistryError),
    #[error("core error: {0}")]
    Core(#[from] CoreError),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("unsupported engine: {0}")]
    UnsupportedEngine(String),
}

#[derive(Parser, Debug)]
#[command(name = "datalchemy", version, about = "Datalchemy CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Introspect(IntrospectArgs),
}

#[derive(Args, Debug)]
struct IntrospectArgs {
    /// Database connection string (flag form).
    #[arg(long, value_name = "CONNECTION_STRING", conflicts_with = "conn_pos")]
    conn: Option<String>,
    /// Database connection string (positional form).
    #[arg(value_name = "CONNECTION_STRING", required_unless_present = "conn")]
    conn_pos: Option<String>,
    /// Output directory for runs.
    #[arg(long, default_value = "runs")]
    run_dir: PathBuf,
    /// Optional output path for schema.json.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Schema name(s) to include.
    #[arg(long, value_name = "SCHEMA")]
    schema: Vec<String>,
    /// Fail on cycles or unsupported features.
    #[arg(long, default_value_t = false)]
    strict: bool,
    /// Redact credentials in artifacts.
    #[arg(long, default_value_t = true)]
    redact: bool,
    /// Include system schemas such as pg_catalog.
    #[arg(long, default_value_t = false)]
    include_system_schemas: bool,
    /// Include views in introspection.
    #[arg(long, default_value_t = true)]
    include_views: bool,
    /// Include materialized views in introspection.
    #[arg(long, default_value_t = true)]
    include_materialized_views: bool,
    /// Include foreign tables in introspection.
    #[arg(long, default_value_t = true)]
    include_foreign_tables: bool,
    /// Include indexes in introspection.
    #[arg(long, default_value_t = true)]
    include_indexes: bool,
    /// Include comments in introspection.
    #[arg(long, default_value_t = true)]
    include_comments: bool,
}

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let cli = Cli::parse();

    match cli.command {
        Command::Introspect(args) => run_introspect(args).await,
    }
}

async fn run_introspect(args: IntrospectArgs) -> Result<(), CliError> {
    let IntrospectArgs {
        conn,
        conn_pos,
        run_dir,
        out,
        schema,
        strict,
        redact,
        include_system_schemas,
        include_views,
        include_materialized_views,
        include_foreign_tables,
        include_indexes,
        include_comments,
    } = args;

    if !redact {
        let message = if std::env::var("CI").is_ok() {
            "redaction cannot be disabled in CI"
        } else {
            "redaction cannot be disabled"
        };
        return Err(CliError::InvalidConfig(message.to_string()));
    }

    let conn = match (conn, conn_pos) {
        (Some(value), None) => value,
        (None, Some(value)) => value,
        (Some(_), Some(_)) => {
            return Err(CliError::InvalidConfig(
                "use either --conn or positional connection string".to_string(),
            ))
        }
        (None, None) => {
            return Err(CliError::InvalidConfig(
                "connection string is required".to_string(),
            ))
        }
    };

    let engine = detect_engine(&conn)?;

    let options = IntrospectOptions {
        include_system_schemas,
        include_views,
        include_materialized_views,
        include_foreign_tables,
        include_indexes,
        include_comments,
        schemas: if schema.is_empty() {
            None
        } else {
            Some(schema.clone())
        },
    };

    let run_options = RunOptions {
        include_system_schemas: options.include_system_schemas,
        include_views: options.include_views,
        include_materialized_views: options.include_materialized_views,
        include_foreign_tables: options.include_foreign_tables,
        include_indexes: options.include_indexes,
        include_comments: options.include_comments,
        schemas: options.schemas.clone(),
    };

    let run_id = Uuid::new_v4().to_string();
    let started_at = chrono::Utc::now();
    let connection = redact_connection_string(&conn);
    let run_ctx = RunContext {
        run_id: run_id.clone(),
        started_at,
        engine: engine.to_string(),
        schema_version: SCHEMA_VERSION.to_string(),
        strict,
        run_dir,
        out,
        options: run_options,
        connection,
    };

    let run_paths = start_run(&run_ctx)?;
    init_run_logging(&run_paths.logs_path)?;

    tracing::info!(event = "run_started", run_id = %run_id, engine = %engine);
    tracing::info!(event = "engine_detected", engine = %engine);

    let timer = Instant::now();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&conn)
        .await?;

    tracing::info!(event = "introspection_started");

    let schema = introspect_postgres_with_options(&pool, options).await?;
    validate_schema(&schema)?;

    tracing::info!(event = "introspection_finished");

    let metrics = collect_schema_metrics(&schema);

    write_schema(&run_paths, &schema, run_ctx.out.as_deref())?;
    tracing::info!(event = "schema_written", path = %run_paths.schema_path.display());

    write_metrics(&run_paths, &metrics)?;
    tracing::info!(event = "metrics_written", path = %run_paths.metrics_path.display());

    if run_ctx.strict && metrics.fk_graph.has_cycle {
        return Err(CliError::InvalidConfig(
            "foreign key graph contains cycles".to_string(),
        ));
    }

    let duration_ms = timer.elapsed().as_millis();
    tracing::info!(event = "run_finished", status = "success", duration_ms = duration_ms);

    Ok(())
}

fn detect_engine(conn: &str) -> Result<&'static str, CliError> {
    if conn.starts_with("postgres://") || conn.starts_with("postgresql://") {
        Ok("postgres")
    } else {
        Err(CliError::UnsupportedEngine(conn.to_string()))
    }
}
