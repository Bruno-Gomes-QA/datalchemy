mod logging;
mod run;

pub use logging::init_run_logging;
pub use run::{RunContext, RunOptions, start_run, write_metrics, write_schema};

use thiserror::Error;

/// Registry-level errors for run artifacts.
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("logging error: {0}")]
    Logging(String),
}

/// Result type for registry operations.
pub type RegistryResult<T> = std::result::Result<T, RegistryError>;
