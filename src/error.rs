use anyhow::Error as AnyhowError;
use sqlx::Error as SqlxError;

/// Library-wide error type.
///
/// Keeps database and schema issues distinct so callers can react accordingly.
#[derive(Debug)]
pub enum Error {
    /// Errors returned by `sqlx` when executing queries.
    Db(SqlxError),
    /// The database schema violates expected invariants or cannot be represented.
    InvalidSchema(String),
    /// A requested capability is not implemented or cannot be handled.
    Unsupported(String),
    /// Catch-all for unexpected failures.
    Other(AnyhowError),
}

/// Convenience alias for library results.
pub type Result<T> = std::result::Result<T, Error>;

impl From<SqlxError> for Error {
    fn from(value: SqlxError) -> Self {
        Error::Db(value)
    }
}

impl From<AnyhowError> for Error {
    fn from(value: AnyhowError) -> Self {
        Error::Other(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Db(err) => write!(f, "database error: {err}"),
            Error::InvalidSchema(msg) => write!(f, "invalid schema: {msg}"),
            Error::Unsupported(msg) => write!(f, "unsupported: {msg}"),
            Error::Other(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for Error {}
