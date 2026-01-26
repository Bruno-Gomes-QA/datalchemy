use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use datalchemy_core::redact_connection_string;

use super::atomic::write_bytes_atomic;
use super::{WorkspaceError, WorkspacePaths, WorkspaceResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbProfile {
    pub engine: String,
    pub redacted: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub user: Option<String>,
}

impl DbProfile {
    pub fn from_connection(conn: &str) -> Self {
        let redacted = redact_connection_string(conn);
        Self {
            engine: redacted.engine.unwrap_or_else(|| "postgres".to_string()),
            redacted: redacted.redacted,
            host: redacted.host,
            port: redacted.port,
            database: redacted.database,
            user: redacted.user,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfilesConfig {
    pub profiles: BTreeMap<String, DbProfile>,
}

pub fn load_or_create_profiles(paths: &WorkspacePaths) -> WorkspaceResult<ProfilesConfig> {
    let path = paths.profiles_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        let profiles: ProfilesConfig = toml::from_str(&content)?;
        return Ok(profiles);
    }

    let profiles = ProfilesConfig::default();
    save_profiles(paths, &profiles)?;
    Ok(profiles)
}

pub fn save_profiles(paths: &WorkspacePaths, profiles: &ProfilesConfig) -> WorkspaceResult<()> {
    let path = paths.profiles_path();
    let encoded = toml::to_string_pretty(profiles)?;
    write_bytes_atomic(&path, encoded.as_bytes()).map_err(WorkspaceError::from)
}
