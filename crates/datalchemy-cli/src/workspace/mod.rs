mod approval;
mod atomic;
mod doctor;
mod ids;
mod llm_models;
mod manifests;
mod paths;
mod profiles;
mod settings;

pub use approval::WriteIntent;
pub use atomic::{write_bytes_atomic, write_json_atomic};
pub use doctor::{DoctorLevel, run_doctor};
pub use ids::new_artifact_id;
pub use llm_models::{LlmModels, load_or_create_llm_models};
pub use manifests::{
    ARTIFACT_VERSION, ArtifactStatus, CLI_VERSION, EvalManifest, OutManifest, PlanMeta,
    RunManifest, RunOptions,
};
pub use paths::WorkspacePaths;
pub use profiles::{DbProfile, ProfilesConfig, load_or_create_profiles, save_profiles};
pub use settings::{
    ApprovalPolicy, LlmProvider, PrivacyMode, WorkspaceMode, WorkspaceSettings,
    load_or_create_settings, save_settings,
};

use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml decode error: {0}")]
    TomlDecode(#[from] toml::de::Error),
    #[error("toml encode error: {0}")]
    TomlEncode(#[from] toml::ser::Error),
    #[error("invalid workspace state: {0}")]
    Invalid(String),
}

pub type WorkspaceResult<T> = Result<T, WorkspaceError>;
