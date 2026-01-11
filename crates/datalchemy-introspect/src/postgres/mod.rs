use sqlx::PgPool;

use datalchemy_core::{DatabaseSchema, Result, Schema, SCHEMA_VERSION};

use crate::adapter::Adapter;
use crate::options::IntrospectOptions;

mod mapper;
mod queries;
mod utils;

/// Adapter for PostgreSQL databases.
#[derive(Debug, Clone)]
pub struct PostgresAdapter {
    pool: PgPool,
}

impl PostgresAdapter {
    /// Create a new adapter using a pre-configured pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl Adapter for PostgresAdapter {
    fn engine(&self) -> &'static str {
        "postgres"
    }

    async fn introspect(&self, opts: &IntrospectOptions) -> Result<DatabaseSchema> {
        introspect(&self.pool, opts).await
    }
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
    introspect(pool, &opts).await
}

/// Introspect a Postgres database according to the provided options.
pub async fn introspect(pool: &PgPool, opts: &IntrospectOptions) -> Result<DatabaseSchema> {
    let database = queries::fetch_database_name(pool).await?;
    let schemas = mapper::filter_schemas(queries::list_schemas(pool).await?, opts);
    let mut enums = mapper::map_enums(queries::list_enums(pool).await?, opts);

    let mut schema_items = Vec::new();

    for schema_name in schemas {
        let raw_tables = queries::list_tables_in_schema(pool, &schema_name).await?;
        let mut tables = mapper::map_tables(raw_tables, opts);

        for table in &mut tables {
            let raw_columns = queries::list_columns(pool, &schema_name, &table.name).await?;
            table.columns = mapper::map_columns(raw_columns, opts);

            let raw_pk = queries::get_primary_key(pool, &schema_name, &table.name).await?;
            let raw_uniques =
                queries::list_unique_constraints(pool, &schema_name, &table.name).await?;
            let raw_checks =
                queries::list_check_constraints(pool, &schema_name, &table.name).await?;
            let raw_fks = queries::list_foreign_keys(pool, &schema_name, &table.name).await?;

            let mut constraints = Vec::new();
            if let Some(pk) = mapper::map_primary_key(raw_pk) {
                constraints.push(datalchemy_core::Constraint::PrimaryKey(pk));
            }
            constraints.extend(
                mapper::map_unique_constraints(raw_uniques)
                    .into_iter()
                    .map(datalchemy_core::Constraint::Unique),
            );
            constraints.extend(
                mapper::map_check_constraints(raw_checks)
                    .into_iter()
                    .map(datalchemy_core::Constraint::Check),
            );
            constraints.extend(
                mapper::map_foreign_keys(raw_fks)
                    .into_iter()
                    .map(datalchemy_core::Constraint::ForeignKey),
            );
            mapper::sort_constraints(&mut constraints);
            table.constraints = constraints;

            if opts.include_indexes {
                let raw_indexes =
                    queries::list_indexes(pool, &schema_name, &table.name).await?;
                table.indexes = mapper::map_indexes(raw_indexes);
            }
        }

        tables.sort_by(|left, right| left.name.cmp(&right.name));
        schema_items.push(Schema {
            name: schema_name,
            tables,
        });
    }

    schema_items.sort_by(|left, right| left.name.cmp(&right.name));
    enums.sort_by(|left, right| {
        left.schema
            .cmp(&right.schema)
            .then_with(|| left.name.cmp(&right.name))
    });

    Ok(DatabaseSchema {
        schema_version: SCHEMA_VERSION.to_string(),
        engine: "postgres".to_string(),
        database: Some(database),
        schemas: schema_items,
        enums,
        fingerprint: None,
    })
}
