use std::collections::{BTreeSet, HashMap, VecDeque};

use datalchemy_core::{Constraint, DatabaseSchema};
use datalchemy_plan::Plan;

use crate::errors::GenerationError;

/// Planned generation task for a table.
#[derive(Debug, Clone)]
pub struct GenerationTask {
    pub schema: String,
    pub table: String,
    pub rows: u64,
}

/// Build a deterministic generation plan for tables.
pub fn plan_tables(
    schema: &DatabaseSchema,
    plan: &Plan,
    auto_generate_parents: bool,
) -> Result<Vec<GenerationTask>, GenerationError> {
    let mut rows_by_table: HashMap<String, u64> = HashMap::new();

    for target in &plan.targets {
        let key = table_key(&target.schema, &target.table);
        rows_by_table
            .entry(key)
            .and_modify(|rows| *rows = (*rows).max(target.rows))
            .or_insert(target.rows);
    }

    let parents = build_parent_map(schema);

    if auto_generate_parents {
        let mut queue: VecDeque<(String, u64)> = rows_by_table
            .iter()
            .map(|(key, rows)| (key.clone(), *rows))
            .collect();
        let mut visited: BTreeSet<String> = rows_by_table.keys().cloned().collect();

        while let Some((child, child_rows)) = queue.pop_front() {
            if let Some(parent_keys) = parents.get(&child) {
                for parent in parent_keys {
                    rows_by_table.entry(parent.clone()).or_insert(child_rows);
                    if visited.insert(parent.clone()) {
                        let rows = *rows_by_table.get(parent).unwrap_or(&child_rows);
                        queue.push_back((parent.clone(), rows));
                    }
                }
            }
        }
    }

    let order = datalchemy_core::build_fk_graph_report(schema)
        .topo_order
        .ok_or_else(|| GenerationError::Unsupported("cyclic FK graph".to_string()))?;

    let mut tasks = Vec::new();
    for key in order {
        if let Some(rows) = rows_by_table.get(&key) {
            let (schema_name, table_name) = split_key(&key)?;
            tasks.push(GenerationTask {
                schema: schema_name.to_string(),
                table: table_name.to_string(),
                rows: *rows,
            });
        }
    }

    if tasks.is_empty() {
        return Err(GenerationError::InvalidPlan(
            "no generation targets resolved".to_string(),
        ));
    }

    Ok(tasks)
}

fn build_parent_map(schema: &DatabaseSchema) -> HashMap<String, BTreeSet<String>> {
    let mut parents: HashMap<String, BTreeSet<String>> = HashMap::new();

    for db_schema in &schema.schemas {
        for table in &db_schema.tables {
            let child_key = table_key(&db_schema.name, &table.name);
            parents.entry(child_key.clone()).or_default();

            for constraint in &table.constraints {
                if let Constraint::ForeignKey(fk) = constraint {
                    let parent_key = table_key(&fk.referenced_schema, &fk.referenced_table);
                    parents
                        .entry(child_key.clone())
                        .or_default()
                        .insert(parent_key);
                }
            }
        }
    }

    parents
}

fn table_key(schema: &str, table: &str) -> String {
    format!("{schema}.{table}")
}

fn split_key(key: &str) -> Result<(&str, &str), GenerationError> {
    let mut parts = key.split('.');
    let schema = parts
        .next()
        .ok_or_else(|| GenerationError::InvalidPlan(format!("invalid table key '{key}'")))?;
    let table = parts
        .next()
        .ok_or_else(|| GenerationError::InvalidPlan(format!("invalid table key '{key}'")))?;
    Ok((schema, table))
}
