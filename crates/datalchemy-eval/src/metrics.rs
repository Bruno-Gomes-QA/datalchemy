use serde::{Deserialize, Serialize};

/// Metrics contract version for dataset evaluation.
pub const METRICS_VERSION: &str = "0.1";

/// Machine-readable metrics for a dataset evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsReport {
    pub metrics_version: String,
    pub run_id: String,
    pub schema_ref: MetricsSchemaRef,
    pub plan_ref: MetricsPlanRef,
    pub tables: Vec<TableMetrics>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub column_stats: Vec<ColumnStats>,
    pub constraints: ConstraintSummary,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<WarningItem>,
    pub performance: PerformanceMetrics,
}

/// Reference metadata for schema inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSchemaRef {
    pub schema_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_fingerprint: Option<String>,
}

/// Reference metadata for plan inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsPlanRef {
    pub plan_version: String,
    pub seed: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_hash: Option<String>,
}

/// Per-table row counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMetrics {
    pub schema: String,
    pub table: String,
    pub rows_found: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows_expected: Option<u64>,
}

/// Optional per-column statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStats {
    pub schema: String,
    pub table: String,
    pub column: String,
    pub null_count: u64,
}

/// Summary of constraint validation outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintSummary {
    pub not_null: ConstraintStats,
    pub pk: ConstraintStats,
    pub unique: ConstraintStats,
    pub fk: ConstraintStats,
    pub check: CheckConstraintStats,
}

/// Generic constraint counter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintStats {
    pub checked: u64,
    pub violations: u64,
}

/// Check constraint counters with not-evaluated tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConstraintStats {
    pub checked: u64,
    pub violations: u64,
    pub not_evaluated: u64,
}

/// Structured warning entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarningItem {
    pub code: String,
    pub path: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Performance timings for the evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub load_ms: u128,
    pub validate_ms: u128,
    pub total_ms: u128,
}
