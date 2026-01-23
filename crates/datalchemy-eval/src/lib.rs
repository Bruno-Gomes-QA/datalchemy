//! Evaluation helpers for schema and dataset metrics.

pub mod engine;
pub mod errors;
pub mod metrics;
pub mod model;
pub mod report;
pub mod schema_metrics;

pub use engine::EvaluationEngine;
pub use errors::EvalError;
pub use metrics::{
    CheckConstraintStats, ColumnStats, ConstraintStats, ConstraintSummary, METRICS_VERSION,
    MetricsPlanRef, MetricsReport, MetricsSchemaRef, PerformanceMetrics, TableMetrics, WarningItem,
};
pub use model::{EvaluateOptions, EvaluationResult, Violation};
pub use schema_metrics::{
    ConstraintCounts, CoverageMetrics, FkGraphMetrics, SchemaCounts, SchemaMetrics,
    collect_schema_metrics,
};
