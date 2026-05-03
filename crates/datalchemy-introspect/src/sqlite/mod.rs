//! SQLite database introspection adapter.

mod mapper;
mod queries;

use sqlx::SqlitePool;

use datalchemy_core::{DatabaseSchema, Result, SCHEMA_VERSION, Schema};

use crate::adapter::Adapter;
use crate::options::IntrospectOptions;

/// Adapter for SQLite databases.
#[derive(Debug, Clone)]
pub struct SqliteAdapter {
    pool: SqlitePool,
}

impl SqliteAdapter {
    /// Create a new adapter using a pre-configured pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl Adapter for SqliteAdapter {
    fn engine(&self) -> &'static str {
        "sqlite"
    }

    async fn introspect(&self, opts: &IntrospectOptions) -> Result<DatabaseSchema> {
        introspect(&self.pool, opts).await
    }
}

/// Introspect SQLite with default options.
pub async fn introspect_sqlite(pool: &SqlitePool) -> Result<DatabaseSchema> {
    introspect_sqlite_with_options(pool, IntrospectOptions::default()).await
}

/// Introspect SQLite with caller-provided options.
pub async fn introspect_sqlite_with_options(
    pool: &SqlitePool,
    opts: IntrospectOptions,
) -> Result<DatabaseSchema> {
    introspect(pool, &opts).await
}

/// Introspect a SQLite database according to the provided options.
async fn introspect(pool: &SqlitePool, opts: &IntrospectOptions) -> Result<DatabaseSchema> {
    let table_names = queries::list_tables(pool).await?;

    let mut tables = Vec::new();

    for table_name in table_names {
        let raw_columns = queries::list_columns(pool, &table_name).await?;
        let raw_fks = queries::list_foreign_keys(pool, &table_name).await?;
        let raw_indexes = queries::list_indexes(pool, &table_name).await?;

        let mut table = mapper::map_table(&table_name, raw_columns);
        table.constraints.extend(
            mapper::map_foreign_keys(raw_fks)
                .into_iter()
                .map(datalchemy_core::Constraint::ForeignKey),
        );

        if opts.include_indexes {
            table.indexes = mapper::map_indexes(raw_indexes, pool, &table_name).await;
        }

        tables.push(table);
    }

    tables.sort_by(|a, b| a.name.cmp(&b.name));

    let schema = Schema {
        name: "main".to_string(),
        tables,
    };

    Ok(DatabaseSchema {
        schema_version: SCHEMA_VERSION.to_string(),
        engine: "sqlite".to_string(),
        database: None,
        schemas: vec![schema],
        enums: Vec::new(),
        schema_fingerprint: None,
    })
}
