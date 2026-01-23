use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Options for the generation engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateOptions {
    /// Directory where run artifacts are written.
    pub out_dir: PathBuf,
    /// Fail on unsupported behavior or constraint violations.
    pub strict: bool,
    /// Maximum attempts to build a single row.
    pub max_attempts_row: u32,
    /// Maximum attempts to generate a table.
    pub max_attempts_table: u32,
    /// Automatically generate missing parent tables for FKs.
    pub auto_generate_parents: bool,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            out_dir: PathBuf::from("out"),
            strict: true,
            max_attempts_row: 50,
            max_attempts_table: 5,
            auto_generate_parents: true,
        }
    }
}

/// Summary of a generated table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableReport {
    pub schema: String,
    pub table: String,
    pub rows_requested: u64,
    pub rows_generated: u64,
    pub retries: u64,
}

/// Structured generation issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationIssue {
    pub level: String,
    pub code: String,
    pub message: String,
    pub path: Option<String>,
}

/// Report for a generation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationReport {
    pub run_id: String,
    pub tables: Vec<TableReport>,
    pub retries_total: u64,
    pub warnings: Vec<GenerationIssue>,
    pub unsupported: Vec<GenerationIssue>,
}

impl GenerationReport {
    pub fn new(run_id: String) -> Self {
        Self {
            run_id,
            tables: Vec::new(),
            retries_total: 0,
            warnings: Vec::new(),
            unsupported: Vec::new(),
        }
    }
}
