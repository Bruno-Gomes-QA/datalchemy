use std::path::{Path, PathBuf};

use super::{WorkspaceError, WorkspaceResult};

#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    pub root: PathBuf,
    pub config_dir: PathBuf,
    pub secrets_dir: PathBuf,
    pub runs_dir: PathBuf,
    pub plans_dir: PathBuf,
    pub out_dir: PathBuf,
    pub eval_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl WorkspacePaths {
    pub fn new(root: PathBuf) -> Self {
        let config_dir = root.join("config");
        let secrets_dir = root.join("secrets");
        let runs_dir = root.join("runs");
        let plans_dir = root.join("plans");
        let out_dir = root.join("out");
        let eval_dir = root.join("eval");
        let logs_dir = root.join("logs");
        Self {
            root,
            config_dir,
            secrets_dir,
            runs_dir,
            plans_dir,
            out_dir,
            eval_dir,
            logs_dir,
        }
    }

    pub fn settings_path(&self) -> PathBuf {
        self.config_dir.join("settings.toml")
    }

    pub fn profiles_path(&self) -> PathBuf {
        self.config_dir.join("profiles.toml")
    }

    pub fn llm_models_path(&self) -> PathBuf {
        self.config_dir.join("llm_models.toml")
    }

    pub fn cli_log_path(&self) -> PathBuf {
        self.logs_dir.join("cli.log")
    }

    pub fn vault_meta_path(&self) -> PathBuf {
        self.secrets_dir.join("vault.meta.json")
    }

    pub fn vault_db_path(&self) -> PathBuf {
        self.secrets_dir.join("db.enc")
    }

    pub fn vault_llm_path(&self) -> PathBuf {
        self.secrets_dir.join("llm_gemini.enc")
    }

    pub fn ensure_dirs(&self) -> WorkspaceResult<()> {
        create_if_missing(&self.root)?;
        create_if_missing(&self.config_dir)?;
        create_if_missing(&self.secrets_dir)?;
        create_if_missing(&self.runs_dir)?;
        create_if_missing(&self.plans_dir)?;
        create_if_missing(&self.out_dir)?;
        create_if_missing(&self.eval_dir)?;
        create_if_missing(&self.logs_dir)?;
        Ok(())
    }
}

fn create_if_missing(path: &Path) -> WorkspaceResult<()> {
    if path.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(path).map_err(WorkspaceError::from)
}
