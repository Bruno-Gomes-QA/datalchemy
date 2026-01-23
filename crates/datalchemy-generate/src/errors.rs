use thiserror::Error;

use crate::model::GenerationReport;

/// Errors emitted by the generation engine.
#[derive(Debug, Error)]
pub enum GenerationError {
    #[error("invalid plan: {0}")]
    InvalidPlan(String),
    #[error("unsupported feature: {0}")]
    Unsupported(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),
    #[error("generation failed")]
    Failed(GenerationReport),
}
