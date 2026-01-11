use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::constraints::Constraint;
use crate::schema::DatabaseSchema;

/// Summary of FK graph structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FkGraphSummary {
    pub nodes: usize,
    pub edges: usize,
}

/// Report for FK dependency ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FkGraphReport {
    pub summary: FkGraphSummary,
    pub topo_order: Option<Vec<String>>,
    pub cycle: Option<Vec<String>>,
}

/// Build a deterministic FK dependency report for a database schema.
pub fn build_fk_graph_report(schema: &DatabaseSchema) -> FkGraphReport {
    let graph = build_adjacency(schema);
    let nodes = graph.len();
    let edges = graph.values().map(|targets| targets.len()).sum();
    let summary = FkGraphSummary { nodes, edges };

    match toposort(&graph) {
        Ok(order) => FkGraphReport {
            summary,
            topo_order: Some(order),
            cycle: None,
        },
        Err(cycle) => FkGraphReport {
            summary,
            topo_order: None,
            cycle: Some(cycle),
        },
    }
}

fn build_adjacency(schema: &DatabaseSchema) -> BTreeMap<String, BTreeSet<String>> {
    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for db_schema in &schema.schemas {
        for table in &db_schema.tables {
            let table_key = format!("{}.{}", db_schema.name, table.name);
            graph.entry(table_key.clone()).or_default();

            for constraint in &table.constraints {
                if let Constraint::ForeignKey(fk) = constraint {
                    let referenced = format!("{}.{}", fk.referenced_schema, fk.referenced_table);
                    graph.entry(referenced.clone()).or_default();
                    graph
                        .entry(referenced)
                        .or_default()
                        .insert(table_key.clone());
                }
            }
        }
    }

    graph
}

fn toposort(graph: &BTreeMap<String, BTreeSet<String>>) -> Result<Vec<String>, Vec<String>> {
    let mut indegree: BTreeMap<String, usize> = BTreeMap::new();

    for node in graph.keys() {
        indegree.entry(node.clone()).or_insert(0);
    }

    for (node, targets) in graph {
        for target in targets {
            let entry = indegree.entry(target.clone()).or_insert(0);
            *entry += 1;
        }
        indegree.entry(node.clone()).or_insert(0);
    }

    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter_map(|(node, count)| {
            if *count == 0 {
                Some(node.clone())
            } else {
                None
            }
        })
        .collect();

    let mut order = Vec::with_capacity(graph.len());
    let mut indegree = indegree;

    while let Some(node) = ready.iter().next().cloned() {
        ready.remove(&node);
        order.push(node.clone());

        if let Some(targets) = graph.get(&node) {
            for target in targets {
                if let Some(count) = indegree.get_mut(target) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.insert(target.clone());
                    }
                }
            }
        }
    }

    if order.len() == graph.len() {
        Ok(order)
    } else {
        let cycle_nodes: Vec<String> = indegree
            .into_iter()
            .filter_map(|(node, count)| if count > 0 { Some(node) } else { None })
            .collect();
        Err(cycle_nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::{Constraint, ForeignKey};
    use crate::schema::{Column, DatabaseSchema, Schema, Table, TableKind};
    use crate::types::ColumnType;

    fn column(name: &str) -> Column {
        Column {
            ordinal_position: 1,
            name: name.to_string(),
            column_type: ColumnType {
                data_type: "int".to_string(),
                udt_schema: "pg_catalog".to_string(),
                udt_name: "int4".to_string(),
                character_max_length: None,
                numeric_precision: None,
                numeric_scale: None,
                collation: None,
            },
            is_nullable: false,
            default: None,
            identity: None,
            generated: None,
            comment: None,
        }
    }

    #[test]
    fn toposort_reports_cycle() {
        let fk = ForeignKey {
            name: Some("fk_self".to_string()),
            columns: vec!["id".to_string()],
            referenced_schema: "public".to_string(),
            referenced_table: "users".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_update: crate::constraints::FkAction::NoAction,
            on_delete: crate::constraints::FkAction::NoAction,
            match_type: crate::constraints::FkMatchType::Simple,
            is_deferrable: false,
            initially_deferred: false,
        };

        let schema = DatabaseSchema {
            schema_version: "0.2".to_string(),
            engine: "postgres".to_string(),
            database: Some("db".to_string()),
            schemas: vec![Schema {
                name: "public".to_string(),
                tables: vec![Table {
                    name: "users".to_string(),
                    kind: TableKind::Table,
                    comment: None,
                    columns: vec![column("id")],
                    constraints: vec![Constraint::ForeignKey(fk)],
                    indexes: Vec::new(),
                }],
            }],
            enums: Vec::new(),
            schema_fingerprint: None,
        };

        let report = build_fk_graph_report(&schema);
        assert!(report.topo_order.is_none());
        assert!(
            report
                .cycle
                .as_ref()
                .unwrap()
                .contains(&"public.users".to_string())
        );
    }

    #[test]
    fn toposort_orders_dependencies() {
        let fk = ForeignKey {
            name: Some("fk_orders_user".to_string()),
            columns: vec!["user_id".to_string()],
            referenced_schema: "public".to_string(),
            referenced_table: "users".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_update: crate::constraints::FkAction::NoAction,
            on_delete: crate::constraints::FkAction::NoAction,
            match_type: crate::constraints::FkMatchType::Simple,
            is_deferrable: false,
            initially_deferred: false,
        };

        let schema = DatabaseSchema {
            schema_version: "0.2".to_string(),
            engine: "postgres".to_string(),
            database: Some("db".to_string()),
            schemas: vec![Schema {
                name: "public".to_string(),
                tables: vec![
                    Table {
                        name: "orders".to_string(),
                        kind: TableKind::Table,
                        comment: None,
                        columns: vec![column("id"), column("user_id")],
                        constraints: vec![Constraint::ForeignKey(fk)],
                        indexes: Vec::new(),
                    },
                    Table {
                        name: "users".to_string(),
                        kind: TableKind::Table,
                        comment: None,
                        columns: vec![column("id")],
                        constraints: Vec::new(),
                        indexes: Vec::new(),
                    },
                ],
            }],
            enums: Vec::new(),
            schema_fingerprint: None,
        };

        let report = build_fk_graph_report(&schema);
        let order = report.topo_order.expect("expected toposort");
        let users_idx = order
            .iter()
            .position(|item| item == "public.users")
            .unwrap();
        let orders_idx = order
            .iter()
            .position(|item| item == "public.orders")
            .unwrap();
        assert!(users_idx < orders_idx);
    }
}
