use std::path::Path;

use serde::Deserialize;

use super::manifests::ARTIFACT_VERSION;
use super::profiles::ProfilesConfig;
use super::settings::WorkspaceSettings;
use super::{WorkspacePaths, WorkspaceResult};

#[derive(Debug, Clone)]
pub enum DoctorLevel {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct DoctorIssue {
    pub level: DoctorLevel,
    pub message: String,
    pub hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub issues: Vec<DoctorIssue>,
}

impl DoctorReport {
    fn push(&mut self, level: DoctorLevel, message: impl Into<String>, hint: Option<String>) {
        self.issues.push(DoctorIssue {
            level,
            message: message.into(),
            hint,
        });
    }
}

pub fn run_doctor(
    paths: &WorkspacePaths,
    settings: &WorkspaceSettings,
    profiles: &ProfilesConfig,
) -> WorkspaceResult<DoctorReport> {
    let mut report = DoctorReport { issues: Vec::new() };

    check_dir(&mut report, &paths.root, "workspace root");
    check_dir(&mut report, &paths.config_dir, "config");
    check_dir(&mut report, &paths.secrets_dir, "secrets");
    check_dir(&mut report, &paths.runs_dir, "runs");
    check_dir(&mut report, &paths.plans_dir, "plans");
    check_dir(&mut report, &paths.out_dir, "out");
    check_dir(&mut report, &paths.eval_dir, "eval");
    check_dir(&mut report, &paths.logs_dir, "logs");

    if settings.active_profile.is_none() {
        report.push(
            DoctorLevel::Warning,
            "active profile not set",
            Some("use /profiles set <name> or /db to configure one".to_string()),
        );
    } else if let Some(active) = &settings.active_profile {
        if !profiles.profiles.contains_key(active) {
            report.push(
                DoctorLevel::Error,
                format!("active profile '{active}' not found in profiles.toml"),
                Some("use /profiles to list or /db to create one".to_string()),
            );
        }
    }

    check_manifest_versions(&paths.runs_dir, "run_manifest.json", &mut report)?;
    check_manifest_versions(&paths.plans_dir, "plan.meta.json", &mut report)?;
    check_manifest_versions(&paths.out_dir, "out_manifest.json", &mut report)?;
    check_manifest_versions(&paths.eval_dir, "eval_manifest.json", &mut report)?;
    check_secret_permissions(&paths.vault_meta_path(), &mut report)?;
    check_secret_permissions(&paths.vault_db_path(), &mut report)?;
    check_secret_permissions(&paths.vault_llm_path(), &mut report)?;

    Ok(report)
}

fn check_dir(report: &mut DoctorReport, path: &Path, label: &str) {
    if !path.exists() {
        report.push(
            DoctorLevel::Error,
            format!("{label} directory missing"),
            Some(format!("create {label} directory")),
        );
    }
}

fn check_manifest_versions(
    root: &Path,
    filename: &str,
    report: &mut DoctorReport,
) -> WorkspaceResult<()> {
    if !root.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join(filename);
        if !manifest_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&manifest_path)?;
        let parsed: ManifestVersion = serde_json::from_str(&content)?;
        if parsed.artifact_version != ARTIFACT_VERSION {
            report.push(
                DoctorLevel::Warning,
                format!("artifact version mismatch in {}", manifest_path.display()),
                Some("regenerate the artifact or run a migration when available".to_string()),
            );
        }
    }

    Ok(())
}

fn check_secret_permissions(path: &Path, report: &mut DoctorReport) -> WorkspaceResult<()> {
    if !path.exists() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::metadata(path)?.permissions();
        let mode = perms.mode() & 0o777;
        if mode != 0o600 {
            report.push(
                DoctorLevel::Warning,
                format!("secret file permissions not 0600: {}", path.display()),
                Some("chmod 600 <file>".to_string()),
            );
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ManifestVersion {
    artifact_version: String,
}
