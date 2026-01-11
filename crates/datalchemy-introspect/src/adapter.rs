use async_trait::async_trait;

use datalchemy_core::{DatabaseSchema, Result};

use crate::options::IntrospectOptions;

/// Trait implemented by database adapters that can introspect schemas.
#[async_trait]
pub trait Adapter {
    /// Returns the engine identifier (e.g. `postgres`).
    fn engine(&self) -> &'static str;

    /// Introspect the database and return a schema snapshot.
    async fn introspect(&self, opts: &IntrospectOptions) -> Result<DatabaseSchema>;
}
