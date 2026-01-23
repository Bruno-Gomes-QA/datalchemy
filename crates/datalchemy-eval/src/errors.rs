use thiserror::Error;

/// Errors emitted by the evaluation engine.
#[derive(Debug, Error)]
pub enum EvalError {
    #[error("invalid dataset: {0}")]
    InvalidDataset(String),
    #[error("validation failed with {0} violation(s)")]
    Violations(u64),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
