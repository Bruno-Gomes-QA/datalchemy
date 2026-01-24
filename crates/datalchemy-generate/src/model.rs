use std::collections::BTreeMap;
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
            strict: false,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator_id: Option<String>,
}

/// Report for a generation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationReport {
    pub run_id: String,
    pub tables: Vec<TableReport>,
    pub retries_total: u64,
    pub generator_usage: BTreeMap<String, u64>,
    pub transform_usage: BTreeMap<String, u64>,
    pub fallback_count: u64,
    pub heuristic_count: u64,
    pub unknown_generator_id_count: u64,
    pub pii_columns_touched: BTreeMap<String, u64>,
    pub warnings_by_code: BTreeMap<String, u64>,
    pub warnings: Vec<GenerationIssue>,
    pub unsupported: Vec<GenerationIssue>,
}

impl GenerationReport {
    pub fn new(run_id: String) -> Self {
        Self {
            run_id,
            tables: Vec::new(),
            retries_total: 0,
            generator_usage: BTreeMap::new(),
            transform_usage: BTreeMap::new(),
            fallback_count: 0,
            heuristic_count: 0,
            unknown_generator_id_count: 0,
            pii_columns_touched: BTreeMap::new(),
            warnings_by_code: BTreeMap::new(),
            warnings: Vec::new(),
            unsupported: Vec::new(),
        }
    }

    pub fn record_generator_usage(&mut self, id: &str) {
        *self.generator_usage.entry(id.to_string()).or_insert(0) += 1;
    }

    pub fn record_transform_usage(&mut self, id: &str) {
        *self.transform_usage.entry(id.to_string()).or_insert(0) += 1;
    }

    pub fn record_fallback(&mut self) {
        self.fallback_count += 1;
    }

    pub fn record_heuristic(&mut self) {
        self.heuristic_count += 1;
    }

    pub fn record_unknown_generator(&mut self) {
        self.unknown_generator_id_count += 1;
    }

    pub fn record_pii(&mut self, tag: &str) {
        *self.pii_columns_touched.entry(tag.to_string()).or_insert(0) += 1;
    }

    pub fn record_warning(&mut self, issue: GenerationIssue) {
        *self.warnings_by_code.entry(issue.code.clone()).or_insert(0) += 1;
        self.warnings.push(issue);
    }

    pub fn record_unsupported(&mut self, issue: GenerationIssue) {
        *self.warnings_by_code.entry(issue.code.clone()).or_insert(0) += 1;
        self.unsupported.push(issue);
    }
}
