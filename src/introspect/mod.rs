//! Introspection entrypoints and shared options.

use sqlx::PgPool;

use crate::error::Result;
use crate::model::DatabaseSchema;

pub mod postgres;

/// Options that control how introspection behaves.
#[derive(Debug, Clone)]
pub struct IntrospectOptions {
    pub include_system_schemas: bool,
    pub include_views: bool,
    pub include_materialized_views: bool,
    pub include_foreign_tables: bool,
    pub include_indexes: bool,
    pub include_comments: bool,
    pub schemas: Option<Vec<String>>,
}

impl Default for IntrospectOptions {
    fn default() -> Self {
        Self {
            include_system_schemas: false,
            include_views: true,
            include_materialized_views: true,
            include_foreign_tables: true,
            include_indexes: true,
            include_comments: true,
            schemas: None,
        }
    }
}

/// High-level trait for anything capable of introspecting a database.
#[async_trait::async_trait]
pub trait Introspector {
    async fn introspect(&self, opts: &IntrospectOptions) -> Result<DatabaseSchema>;
}

/// Introspect Postgres with default options.
pub async fn introspect_postgres(pool: &PgPool) -> Result<DatabaseSchema> {
    introspect_postgres_with_options(pool, IntrospectOptions::default()).await
}

/// Introspect Postgres with caller-provided options.
pub async fn introspect_postgres_with_options(
    pool: &PgPool,
    opts: IntrospectOptions,
) -> Result<DatabaseSchema> {
    postgres::introspect(pool, &opts).await
}
