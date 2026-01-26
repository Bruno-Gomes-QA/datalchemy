use serde::{Deserialize, Serialize};

pub const ARTIFACT_VERSION: &str = "0.1";
pub const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ArtifactStatus {
    Running,
    Ok,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    pub run_id: String,
    pub status: ArtifactStatus,
    pub db_profile: String,
    pub introspect_options: RunOptions,
    pub schema_fingerprint: Option<String>,
    pub artifact_version: String,
    pub cli_version: String,
    pub created_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOptions {
    pub include_system_schemas: bool,
    pub include_views: bool,
    pub include_materialized_views: bool,
    pub include_foreign_tables: bool,
    pub include_indexes: bool,
    pub include_comments: bool,
    pub schemas: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMeta {
    pub plan_id: String,
    pub status: ArtifactStatus,
    pub schema_run_id: String,
    pub schema_fingerprint: Option<String>,
    pub provider: String,
    pub model: String,
    pub mock: bool,
    pub artifact_version: String,
    pub cli_version: String,
    pub created_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutManifest {
    pub out_id: String,
    pub status: ArtifactStatus,
    pub schema_run_id: String,
    pub plan_id: String,
    pub mode: String,
    pub seed: u64,
    pub scale: u64,
    pub artifact_version: String,
    pub cli_version: String,
    pub created_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalManifest {
    pub eval_id: String,
    pub status: ArtifactStatus,
    pub out_id: String,
    pub checks_enabled: Vec<String>,
    pub artifact_version: String,
    pub cli_version: String,
    pub created_at: String,
    pub finished_at: Option<String>,
}
