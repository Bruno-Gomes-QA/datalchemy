use std::fs::OpenOptions;
use std::io::{BufRead, Write};
use std::path::Path;

use crate::CliError;

pub fn list_dirs(path: &Path) -> Result<Vec<String>, CliError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                entries.push(name.to_string());
            }
        }
    }
    entries.sort();
    Ok(entries)
}

pub fn list_preview_files(path: &Path) -> Result<Vec<String>, CliError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            if let Some(name) = entry.file_name().to_str() {
                entries.push(name.to_string());
            }
        }
    }
    entries.sort();
    Ok(entries)
}

pub fn move_dir_contents(src: &Path, dest: &Path) -> Result<(), CliError> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let target = dest.join(file_name);
        std::fs::rename(entry.path(), target)?;
    }
    std::fs::remove_dir(src)?;
    Ok(())
}

pub fn read_head_lines(path: &Path, max_lines: usize) -> Result<Vec<String>, CliError> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut lines = Vec::new();
    for line in reader.lines().take(max_lines) {
        lines.push(line?);
    }
    Ok(lines)
}

pub fn read_tail_lines(path: &Path, max_lines: usize) -> Result<Vec<String>, CliError> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut buffer: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    for line in reader.lines() {
        let line = line?;
        if buffer.len() == max_lines {
            buffer.pop_front();
        }
        buffer.push_back(line);
    }
    Ok(buffer.into_iter().collect())
}

pub fn append_line(path: &Path, line: &str) -> Result<(), CliError> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

pub fn open_in_editor(path: &Path) -> Result<(), CliError> {
    let editor = std::env::var("EDITOR").ok();
    let mut candidates = Vec::new();
    if let Some(value) = editor {
        candidates.push(value);
    }
    candidates.push("nano".to_string());
    candidates.push("vi".to_string());

    for candidate in candidates {
        let status = std::process::Command::new(&candidate).arg(path).status();
        match status {
            Ok(status) if status.success() => return Ok(()),
            Ok(_) => continue,
            Err(_) => continue,
        }
    }

    Err(CliError::InvalidConfig(
        "no suitable editor found (set $EDITOR)".to_string(),
    ))
}

pub fn set_private_permissions(path: &Path) -> Result<(), CliError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}

pub fn extract_flag_value(args: &[&str], flag: &str) -> Option<String> {
    let mut iter = args.iter().copied();
    while let Some(arg) = iter.next() {
        if arg == flag {
            return iter.next().map(|value| value.to_string());
        }
    }
    None
}

pub fn command_with_id(raw: &str, flag: &str, value: &str) -> String {
    if raw.contains(flag) {
        raw.to_string()
    } else {
        format!("{raw} {flag} {value}")
    }
}

/// Return visible portion of input and the cursor x offset, given a cursor byte position.
pub fn clipped_input(
    input: &str,
    total_width: usize,
    prefix_len: usize,
    cursor_pos: usize,
) -> (String, u16) {
    let max_len = total_width.saturating_sub(prefix_len + 1);
    if max_len == 0 {
        return (String::new(), 0);
    }

    // Compute char-level cursor offset
    let cursor_chars = input[..cursor_pos].chars().count();
    let total_chars = input.chars().count();

    if total_chars <= max_len {
        // Everything fits
        return (input.to_string(), cursor_chars as u16);
    }

    // Need to window: keep cursor visible
    let start_char = if cursor_chars < max_len {
        0
    } else {
        cursor_chars - max_len + 1
    };

    let visible: String = input.chars().skip(start_char).take(max_len).collect();
    let cursor_x = cursor_chars.saturating_sub(start_char) as u16;
    (visible, cursor_x)
}

/// Read a CSV file and return a formatted preview (first N rows with headers).
pub fn csv_preview(path: &Path, max_rows: usize) -> Result<Vec<String>, CliError> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut lines_iter = reader.lines();
    let mut result = Vec::new();

    // Read header
    let header = match lines_iter.next() {
        Some(Ok(h)) => h,
        _ => return Ok(vec!["(empty file)".to_string()]),
    };

    let columns: Vec<&str> = header.split(',').collect();
    let col_count = columns.len();
    result.push(format!("  {}", columns.join(" | ")));
    result.push(format!("  {}", "-".repeat(header.len().min(120))));

    let mut row_count = 0;
    for line in lines_iter.take(max_rows) {
        if let Ok(l) = line {
            let fields: Vec<&str> = l.splitn(col_count, ',').collect();
            result.push(format!("  {}", fields.join(" | ")));
            row_count += 1;
        }
    }

    // Count remaining
    let total_file = std::fs::File::open(path)?;
    let total_reader = std::io::BufReader::new(total_file);
    let total_lines = total_reader.lines().count().saturating_sub(1); // minus header
    if total_lines > row_count {
        result.push(format!("  ... ({} more rows)", total_lines - row_count));
    }

    Ok(result)
}
