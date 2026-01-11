use std::collections::{BTreeMap, BTreeSet};

use crate::constraints::Constraint;
use crate::error::{Error, Result};
use crate::schema::DatabaseSchema;

/// Validate internal consistency of a database schema.
///
/// This checks:
/// - duplicate schemas/tables/columns
/// - primary key columns exist
/// - foreign key columns and referenced targets exist
pub fn validate_schema(schema: &DatabaseSchema) -> Result<()> {
    let mut catalog: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();

    for db_schema in &schema.schemas {
        if catalog.contains_key(&db_schema.name) {
            return Err(Error::InvalidSchema(format!(
                "duplicate schema name: {}",
                db_schema.name
            )));
        }

        let mut tables = BTreeMap::new();
        for table in &db_schema.tables {
            if tables.contains_key(&table.name) {
                return Err(Error::InvalidSchema(format!(
                    "duplicate table name: {}.{}",
                    db_schema.name, table.name
                )));
            }

            let mut columns = BTreeSet::new();
            for column in &table.columns {
                if !columns.insert(column.name.clone()) {
                    return Err(Error::InvalidSchema(format!(
                        "duplicate column name: {}.{}.{}",
                        db_schema.name, table.name, column.name
                    )));
                }
            }

            tables.insert(table.name.clone(), columns);
        }

        catalog.insert(db_schema.name.clone(), tables);
    }

    for db_schema in &schema.schemas {
        for table in &db_schema.tables {
            let columns = catalog
                .get(&db_schema.name)
                .and_then(|tables| tables.get(&table.name))
                .ok_or_else(|| {
                    Error::InvalidSchema(format!(
                        "missing table in catalog: {}.{}",
                        db_schema.name, table.name
                    ))
                })?;

            for constraint in &table.constraints {
                match constraint {
                    Constraint::PrimaryKey(pk) => {
                        for column in &pk.columns {
                            if !columns.contains(column) {
                                return Err(Error::InvalidSchema(format!(
                                    "primary key column not found: {}.{}.{}",
                                    db_schema.name, table.name, column
                                )));
                            }
                        }
                    }
                    Constraint::ForeignKey(fk) => {
                        for column in &fk.columns {
                            if !columns.contains(column) {
                                return Err(Error::InvalidSchema(format!(
                                    "foreign key column not found: {}.{}.{}",
                                    db_schema.name, table.name, column
                                )));
                            }
                        }

                        let ref_columns = catalog
                            .get(&fk.referenced_schema)
                            .and_then(|tables| tables.get(&fk.referenced_table))
                            .ok_or_else(|| {
                                Error::InvalidSchema(format!(
                                    "referenced table not found: {}.{}",
                                    fk.referenced_schema, fk.referenced_table
                                ))
                            })?;

                        for column in &fk.referenced_columns {
                            if !ref_columns.contains(column) {
                                return Err(Error::InvalidSchema(format!(
                                    "referenced column not found: {}.{}.{}",
                                    fk.referenced_schema, fk.referenced_table, column
                                )));
                            }
                        }
                    }
                    Constraint::Unique(unique) => {
                        for column in &unique.columns {
                            if !columns.contains(column) {
                                return Err(Error::InvalidSchema(format!(
                                    "unique column not found: {}.{}.{}",
                                    db_schema.name, table.name, column
                                )));
                            }
                        }
                    }
                    Constraint::Check(_) => {}
                }
            }
        }
    }

    Ok(())
}
