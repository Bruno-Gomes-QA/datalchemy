use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::metrics::MetricsReport;

/// Options for dataset evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateOptions {
    /// Fail on constraint violations.
    pub strict: bool,
    /// Limit the number of examples emitted in the report.
    pub max_examples: usize,
    /// Emit violations.json with the full list of violations.
    pub write_violations: bool,
    /// Optional output directory override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_dir: Option<PathBuf>,
}

impl Default for EvaluateOptions {
    fn default() -> Self {
        Self {
            strict: true,
            max_examples: 20,
            write_violations: false,
            out_dir: None,
        }
    }
}

/// Structured violation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

/// Result of a dataset evaluation.
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub run_dir: PathBuf,
    pub metrics_path: PathBuf,
    pub report_path: PathBuf,
    pub violations_path: Option<PathBuf>,
    pub metrics: MetricsReport,
    pub report: String,
    pub violations: Vec<Violation>,
}
