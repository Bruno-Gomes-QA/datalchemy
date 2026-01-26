use serde::{Deserialize, Serialize};

use super::atomic::write_bytes_atomic;
use super::{WorkspaceError, WorkspacePaths, WorkspaceResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    AlwaysAllow,
    AskEachTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMode {
    ReadonlyCsv,
    Insert,
    Explore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyMode {
    Normal,
    Paranoid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    Gemini,
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    pub approval_policy: ApprovalPolicy,
    pub active_profile: Option<String>,
    pub mode: WorkspaceMode,
    pub active_run_id: Option<String>,
    pub active_plan_id: Option<String>,
    pub privacy: PrivacyMode,
    pub llm_enabled: bool,
    pub llm_provider: LlmProvider,
    pub llm_model: Option<String>,
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            approval_policy: ApprovalPolicy::AskEachTime,
            active_profile: None,
            mode: WorkspaceMode::ReadonlyCsv,
            active_run_id: None,
            active_plan_id: None,
            privacy: PrivacyMode::Normal,
            llm_enabled: false,
            llm_provider: LlmProvider::Off,
            llm_model: None,
        }
    }
}

pub fn load_or_create_settings(paths: &WorkspacePaths) -> WorkspaceResult<WorkspaceSettings> {
    let path = paths.settings_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        let settings: WorkspaceSettings = toml::from_str(&content)?;
        return Ok(settings);
    }

    let settings = WorkspaceSettings::default();
    save_settings(paths, &settings)?;
    Ok(settings)
}

pub fn save_settings(paths: &WorkspacePaths, settings: &WorkspaceSettings) -> WorkspaceResult<()> {
    let path = paths.settings_path();
    let encoded = toml::to_string_pretty(settings)?;
    write_bytes_atomic(&path, encoded.as_bytes()).map_err(WorkspaceError::from)
}
