//! Evaluation helpers for schema metrics.

pub mod schema_metrics;

pub use schema_metrics::{
    ConstraintCounts, CoverageMetrics, FkGraphMetrics, SchemaCounts, SchemaMetrics,
    collect_schema_metrics,
};
