use serde::{Deserialize, Serialize};

use super::atomic::write_bytes_atomic;
use super::{WorkspaceError, WorkspacePaths, WorkspaceResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModels {
    pub models: Vec<String>,
}

impl Default for LlmModels {
    fn default() -> Self {
        Self {
            models: vec!["gemini-1.5-flash".to_string(), "gemini-1.5-pro".to_string()],
        }
    }
}

pub fn load_or_create_llm_models(paths: &WorkspacePaths) -> WorkspaceResult<LlmModels> {
    let path = paths.llm_models_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        let models: LlmModels = toml::from_str(&content)?;
        return Ok(models);
    }

    let models = LlmModels::default();
    save_llm_models(paths, &models)?;
    Ok(models)
}

fn save_llm_models(paths: &WorkspacePaths, models: &LlmModels) -> WorkspaceResult<()> {
    let path = paths.llm_models_path();
    let encoded = toml::to_string_pretty(models)?;
    write_bytes_atomic(&path, encoded.as_bytes()).map_err(WorkspaceError::from)
}
