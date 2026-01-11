use thiserror::Error;

/// Core error type shared across Datalchemy crates.
#[derive(Debug, Error)]
pub enum Error {
    /// Database error or adapter failure.
    #[error("database error: {0}")]
    Db(String),
    /// The schema violates internal invariants.
    #[error("invalid schema: {0}")]
    InvalidSchema(String),
    /// A requested feature is not yet supported.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// Catch-all error for unexpected failures.
    #[error("other error: {0}")]
    Other(String),
}

/// Convenience alias for results returned by Datalchemy crates.
pub type Result<T> = std::result::Result<T, Error>;
