//! Connection-string helpers shared across TUI modules.

/// Returns `true` when the connection string uses a supported database engine.
pub fn is_supported_connection(conn: &str) -> bool {
    conn.starts_with("postgres://")
        || conn.starts_with("postgresql://")
        || conn.starts_with("sqlite://")
}

/// Returns `true` when the connection string points to a SQLite database.
pub fn is_sqlite(conn: &str) -> bool {
    conn.starts_with("sqlite://")
}
