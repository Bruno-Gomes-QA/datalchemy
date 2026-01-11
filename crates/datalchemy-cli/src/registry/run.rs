use std::fs::{create_dir_all, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Utc};
use serde::Serialize;

use datalchemy_core::{DatabaseSchema, RedactedConnection};

use datalchemy_eval::SchemaMetrics;

use super::{RegistryError, RegistryResult};

/// Serializable options for runs.
#[derive(Debug, Clone, Serialize)]
pub struct RunOptions {
    pub include_system_schemas: bool,
    pub include_views: bool,
    pub include_materialized_views: bool,
    pub include_foreign_tables: bool,
    pub include_indexes: bool,
    pub include_comments: bool,
    pub schemas: Option<Vec<String>>,
}

/// Metadata captured at run start.
#[derive(Debug, Clone)]
pub struct RunContext {
    pub run_id: String,
    pub started_at: DateTime<Utc>,
    pub engine: String,
    pub schema_version: String,
    pub strict: bool,
    pub run_dir: PathBuf,
    pub out: Option<PathBuf>,
    pub options: RunOptions,
    pub connection: RedactedConnection,
}

/// JSON config written to each run directory.
#[derive(Debug, Serialize)]
pub struct RunConfig {
    pub run_id: String,
    pub started_at: String,
    pub engine: String,
    pub schema_version: String,
    pub strict: bool,
    pub options: RunOptions,
    pub connection: RedactedConnection,
    pub git: GitInfo,
}

/// Git metadata for reproducibility.
#[derive(Debug, Serialize)]
pub struct GitInfo {
    pub commit: Option<String>,
    pub dirty: Option<bool>,
}

/// Paths for run artifacts.
#[derive(Debug, Clone)]
pub struct RunPaths {
    pub schema_path: PathBuf,
    pub logs_path: PathBuf,
    pub metrics_path: PathBuf,
}

pub fn start_run(ctx: &RunContext) -> RegistryResult<RunPaths> {
    let timestamp = ctx.started_at.format("%Y-%m-%dT%H-%M-%SZ").to_string();
    let run_root = ctx
        .run_dir
        .join(format!("{timestamp}__run_{}", ctx.run_id));

    create_dir_all(&run_root)?;

    let schema_path = run_root.join("schema.json");
    let config_path = run_root.join("config.json");
    let logs_path = run_root.join("logs.ndjson");
    let metrics_path = run_root.join("metrics.json");

    let config = RunConfig {
        run_id: ctx.run_id.clone(),
        started_at: ctx.started_at.to_rfc3339(),
        engine: ctx.engine.clone(),
        schema_version: ctx.schema_version.clone(),
        strict: ctx.strict,
        options: ctx.options.clone(),
        connection: ctx.connection.clone(),
        git: collect_git_info(),
    };

    write_json(&config_path, &config)?;

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&logs_path)?;

    Ok(RunPaths {
        schema_path,
        logs_path,
        metrics_path,
    })
}

pub fn write_schema(
    paths: &RunPaths,
    schema: &DatabaseSchema,
    out_path: Option<&Path>,
) -> RegistryResult<()> {
    write_json(&paths.schema_path, schema)?;

    if let Some(out_path) = out_path {
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                create_dir_all(parent)?;
            }
        }
        write_json(out_path, schema)?;
    }

    Ok(())
}

pub fn write_metrics(paths: &RunPaths, metrics: &SchemaMetrics) -> RegistryResult<()> {
    write_json(&paths.metrics_path, metrics)
}

pub fn collect_git_info() -> GitInfo {
    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|value| !value.is_empty());

    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|output| !output.stdout.is_empty());

    GitInfo { commit, dirty }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> RegistryResult<()> {
    let file = OpenOptions::new().create(true).truncate(true).write(true).open(path)?;
    serde_json::to_writer_pretty(file, value).map_err(RegistryError::from)
}
