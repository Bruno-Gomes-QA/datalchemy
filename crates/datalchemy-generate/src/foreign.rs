use std::collections::{BTreeMap, HashMap};

use datalchemy_core::{Constraint, Table};

use crate::errors::GenerationError;
use crate::generators::GeneratedValue;

pub trait ForeignContext {
    fn pick_fk(
        &mut self,
        schema: &str,
        table: &str,
        fk_column: &str,
    ) -> Result<GeneratedValue, GenerationError>;
    fn lookup_parent(
        &self,
        schema: &str,
        table: &str,
        pk: &GeneratedValue,
        col: &str,
    ) -> Option<GeneratedValue>;
}

#[derive(Debug, Default)]
pub struct InMemoryForeignContext {
    column_values: BTreeMap<String, BTreeMap<String, Vec<GeneratedValue>>>,
    rows_by_pk: BTreeMap<String, BTreeMap<String, HashMap<String, GeneratedValue>>>,
    cursor: BTreeMap<String, usize>,
}

impl InMemoryForeignContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ingest_table(
        &mut self,
        schema: &str,
        table: &Table,
        rows: &[HashMap<String, GeneratedValue>],
    ) -> Result<(), GenerationError> {
        let table_key = table_key(schema, &table.name);
        let pk_column = primary_key_column(table);

        let mut column_values: BTreeMap<String, Vec<GeneratedValue>> = BTreeMap::new();
        let mut row_map = BTreeMap::new();

        for row in rows {
            for column in &table.columns {
                let key = column.name.to_lowercase();
                if let Some(value) = row.get(&key) {
                    column_values.entry(key).or_default().push(value.clone());
                }
            }

            if let Some(pk_column) = pk_column.as_ref() {
                let pk_key = pk_column.to_lowercase();
                if let Some(value) = row.get(&pk_key) {
                    row_map.insert(value_key(value), row.clone());
                }
            }
        }

        self.column_values.insert(table_key.clone(), column_values);
        if pk_column.is_some() {
            self.rows_by_pk.insert(table_key, row_map);
        }

        Ok(())
    }
}

impl ForeignContext for InMemoryForeignContext {
    fn pick_fk(
        &mut self,
        schema: &str,
        table: &str,
        fk_column: &str,
    ) -> Result<GeneratedValue, GenerationError> {
        let table_key = table_key(schema, table);
        let column_key = fk_column.to_lowercase();
        let values = self
            .column_values
            .get(&table_key)
            .and_then(|columns| columns.get(&column_key))
            .ok_or_else(|| {
                GenerationError::Unsupported(format!(
                    "no parent rows for fk {}.{}.{}",
                    schema, table, fk_column
                ))
            })?;
        if values.is_empty() {
            return Err(GenerationError::Unsupported(format!(
                "no parent rows for fk {}.{}.{}",
                schema, table, fk_column
            )));
        }
        let cursor_key = format!("{table_key}.{column_key}");
        let idx = self.cursor.entry(cursor_key).or_insert(0);
        let value = values[*idx % values.len()].clone();
        *idx = (*idx + 1) % values.len();
        Ok(value)
    }

    fn lookup_parent(
        &self,
        schema: &str,
        table: &str,
        pk: &GeneratedValue,
        col: &str,
    ) -> Option<GeneratedValue> {
        let key = table_key(schema, table);
        let row_map = self.rows_by_pk.get(&key)?;
        let row = row_map.get(&value_key(pk))?;
        row.get(&col.to_lowercase()).cloned()
    }
}

fn primary_key_column(table: &Table) -> Option<String> {
    for constraint in &table.constraints {
        if let Constraint::PrimaryKey(pk) = constraint {
            if pk.columns.len() == 1 {
                return pk.columns.first().cloned();
            }
        }
    }
    None
}

fn table_key(schema: &str, table: &str) -> String {
    format!("{schema}.{table}")
}

fn value_key(value: &GeneratedValue) -> String {
    match value {
        GeneratedValue::Null => "<null>".to_string(),
        GeneratedValue::Bool(value) => value.to_string(),
        GeneratedValue::Int(value) => value.to_string(),
        GeneratedValue::Float(value) => value.to_string(),
        GeneratedValue::Text(value) | GeneratedValue::Uuid(value) => value.clone(),
        GeneratedValue::Date(value) => value.format("%Y-%m-%d").to_string(),
        GeneratedValue::Time(value) => value.format("%H:%M:%S").to_string(),
        GeneratedValue::Timestamp(value) => value.format("%Y-%m-%dT%H:%M:%S").to_string(),
    }
}
