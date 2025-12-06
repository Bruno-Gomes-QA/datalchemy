use std::collections::BTreeMap;

use sqlx::PgPool;

use crate::error::Result;
use crate::introspect::IntrospectOptions;
use crate::model::{DatabaseSchema, Schema};

mod mapper;
mod queries;

/// Introspect a Postgres database according to the provided options.
pub async fn introspect(pool: &PgPool, opts: &IntrospectOptions) -> Result<DatabaseSchema> {
    let database = queries::fetch_database_name(pool).await?;
    let schemas = mapper::filter_schemas(queries::list_schemas(pool).await?, opts);
    let enums = mapper::map_enums(queries::list_enums(pool).await?, opts);

    let mut schema_map: BTreeMap<String, Schema> = BTreeMap::new();

    for schema in schemas {
        let raw_tables = queries::list_tables_in_schema(pool, &schema).await?;
        let mapped_tables = mapper::map_tables(raw_tables, opts);

        let mut tables = BTreeMap::new();
        for (table_name, mut table) in mapped_tables {
            let raw_columns = queries::list_columns(pool, &schema, &table_name).await?;
            table.columns = mapper::map_columns(raw_columns, opts);

            let raw_pk = queries::get_primary_key(pool, &schema, &table_name).await?;
            table.primary_key = mapper::map_primary_key(raw_pk);

            let raw_uniques = queries::list_unique_constraints(pool, &schema, &table_name).await?;
            table.uniques = mapper::map_unique_constraints(raw_uniques);

            let raw_checks = queries::list_check_constraints(pool, &schema, &table_name).await?;
            table.checks = mapper::map_check_constraints(raw_checks);

            let raw_fks = queries::list_foreign_keys(pool, &schema, &table_name).await?;
            table.foreign_keys = mapper::map_foreign_keys(raw_fks);

            if opts.include_indexes {
                let raw_indexes = queries::list_indexes(pool, &schema, &table_name).await?;
                table.indexes = mapper::map_indexes(raw_indexes);
            }

            tables.insert(table_name, table);
        }

        schema_map.insert(schema, Schema { tables });
    }

    Ok(DatabaseSchema {
        database,
        schemas: schema_map,
        enums,
    })
}
