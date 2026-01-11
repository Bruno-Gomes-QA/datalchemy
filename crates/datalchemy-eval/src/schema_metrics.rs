use serde::{Deserialize, Serialize};

use datalchemy_core::{build_fk_graph_report, Constraint, DatabaseSchema};

/// Top-level metrics report for a schema snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetrics {
    pub schema_version: String,
    pub engine: String,
    pub counts: SchemaCounts,
    pub coverage: CoverageMetrics,
    pub fk_graph: FkGraphMetrics,
    pub warnings: Vec<String>,
}

/// Count summary for schema objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaCounts {
    pub schemas: usize,
    pub tables: usize,
    pub columns: usize,
    pub constraints: ConstraintCounts,
}

/// Count summary for constraint types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintCounts {
    pub primary_keys: usize,
    pub foreign_keys: usize,
    pub unique: usize,
    pub checks: usize,
}

/// Coverage metrics for the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageMetrics {
    pub tables_with_pk_pct: f64,
    pub tables_with_fk_pct: f64,
    pub columns_not_null_pct: f64,
}

/// FK graph metrics for the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FkGraphMetrics {
    pub edges: usize,
    pub has_cycle: bool,
    pub cycle: Option<Vec<String>>,
    pub topo_order: Option<Vec<String>>,
}

/// Collect metrics for a given schema snapshot.
pub fn collect_schema_metrics(schema: &DatabaseSchema) -> SchemaMetrics {
    let mut counts = SchemaCounts {
        schemas: 0,
        tables: 0,
        columns: 0,
        constraints: ConstraintCounts {
            primary_keys: 0,
            foreign_keys: 0,
            unique: 0,
            checks: 0,
        },
    };

    let mut tables_with_pk = 0usize;
    let mut tables_with_fk = 0usize;
    let mut not_null_columns = 0usize;

    for db_schema in &schema.schemas {
        counts.schemas += 1;
        for table in &db_schema.tables {
            counts.tables += 1;
            counts.columns += table.columns.len();
            not_null_columns += table.columns.iter().filter(|col| !col.is_nullable).count();

            let mut has_pk = false;
            let mut has_fk = false;

            for constraint in &table.constraints {
                match constraint {
                    Constraint::PrimaryKey(_) => {
                        counts.constraints.primary_keys += 1;
                        has_pk = true;
                    }
                    Constraint::ForeignKey(_) => {
                        counts.constraints.foreign_keys += 1;
                        has_fk = true;
                    }
                    Constraint::Unique(_) => {
                        counts.constraints.unique += 1;
                    }
                    Constraint::Check(_) => {
                        counts.constraints.checks += 1;
                    }
                }
            }

            if has_pk {
                tables_with_pk += 1;
            }
            if has_fk {
                tables_with_fk += 1;
            }
        }
    }

    let total_tables = counts.tables as f64;
    let total_columns = counts.columns as f64;

    let coverage = CoverageMetrics {
        tables_with_pk_pct: if total_tables > 0.0 {
            tables_with_pk as f64 / total_tables
        } else {
            0.0
        },
        tables_with_fk_pct: if total_tables > 0.0 {
            tables_with_fk as f64 / total_tables
        } else {
            0.0
        },
        columns_not_null_pct: if total_columns > 0.0 {
            not_null_columns as f64 / total_columns
        } else {
            0.0
        },
    };

    let graph_report = build_fk_graph_report(schema);
    let fk_graph = FkGraphMetrics {
        edges: graph_report.summary.edges,
        has_cycle: graph_report.cycle.is_some(),
        cycle: graph_report.cycle,
        topo_order: graph_report.topo_order,
    };

    SchemaMetrics {
        schema_version: schema.schema_version.clone(),
        engine: schema.engine.clone(),
        counts,
        coverage,
        fk_graph,
        warnings: Vec::new(),
    }
}
