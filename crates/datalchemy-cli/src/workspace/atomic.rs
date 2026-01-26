use std::fs::{OpenOptions, create_dir_all};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{WorkspaceError, WorkspaceResult};

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> WorkspaceResult<()> {
    let data = serde_json::to_vec_pretty(value)?;
    write_bytes_atomic(path, &data)
}

pub fn write_bytes_atomic(path: &Path, data: &[u8]) -> WorkspaceResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            create_dir_all(parent)?;
        }
    }

    let tmp_path = temp_path(path)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&tmp_path)?;
    file.write_all(data)?;
    file.sync_all()?;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            sync_dir(parent)?;
        }
    }

    std::fs::rename(&tmp_path, path)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            sync_dir(parent)?;
        }
    }

    Ok(())
}

fn temp_path(path: &Path) -> WorkspaceResult<PathBuf> {
    let file_name = path
        .file_name()
        .ok_or_else(|| WorkspaceError::Invalid("invalid path for atomic write".to_string()))?;
    let tmp_name = format!("{}.tmp", file_name.to_string_lossy());
    Ok(path.with_file_name(tmp_name))
}

fn sync_dir(path: &Path) -> io::Result<()> {
    let dir = OpenOptions::new().read(true).open(path)?;
    dir.sync_all()
}
