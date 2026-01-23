use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use datalchemy_core::{CheckConstraint, ColumnType, Constraint, DatabaseSchema, ForeignKey};
use datalchemy_generate::checks::{CheckContext, CheckOutcome, evaluate_check};
use datalchemy_generate::generators::GeneratedValue;
use datalchemy_generate::model::GenerationReport;
use datalchemy_plan::{ConstraintKind, ConstraintMode, Plan, Rule};
use uuid::Uuid;

use crate::errors::EvalError;
use crate::metrics::{
    CheckConstraintStats, ColumnStats, ConstraintStats, ConstraintSummary, METRICS_VERSION,
    MetricsPlanRef, MetricsReport, MetricsSchemaRef, PerformanceMetrics, TableMetrics, WarningItem,
};
use crate::model::{EvaluateOptions, EvaluationResult, Violation};
use crate::report::render_report;

/// Evaluate datasets against schema + plan constraints.
#[derive(Debug, Clone)]
pub struct EvaluationEngine {
    options: EvaluateOptions,
}

impl EvaluationEngine {
    pub fn new(options: EvaluateOptions) -> Self {
        Self { options }
    }

    pub fn run(
        &self,
        schema: &DatabaseSchema,
        plan: &Plan,
        dataset_dir: &Path,
    ) -> Result<EvaluationResult, EvalError> {
        let total_start = Instant::now();
        let load_start = Instant::now();

        let run_id = detect_run_id(dataset_dir).unwrap_or_else(|| "unknown".to_string());
        let plan_index = PlanIndex::new(plan);
        let schema_index = SchemaIndex::new(schema);
        let target_tables = collect_target_tables(schema, plan, &schema_index)?;

        let mut warnings = Vec::new();
        let tables = load_tables(
            &schema_index,
            &target_tables,
            dataset_dir,
            &self.options,
            &mut warnings,
        )?;

        let load_ms = load_start.elapsed().as_millis();
        let validate_start = Instant::now();

        let mut violations = Vec::new();
        let mut column_stats = Vec::new();
        let mut constraint_summary = ConstraintSummary {
            not_null: ConstraintStats {
                checked: 0,
                violations: 0,
            },
            pk: ConstraintStats {
                checked: 0,
                violations: 0,
            },
            unique: ConstraintStats {
                checked: 0,
                violations: 0,
            },
            fk: ConstraintStats {
                checked: 0,
                violations: 0,
            },
            check: CheckConstraintStats {
                checked: 0,
                violations: 0,
                not_evaluated: 0,
            },
        };

        let mut table_metrics = build_table_metrics(plan, &target_tables, &tables);
        table_metrics.sort_by(|a, b| {
            (a.schema.clone(), a.table.clone()).cmp(&(b.schema.clone(), b.table.clone()))
        });

        let mut table_keys: Vec<String> = tables.keys().cloned().collect();
        table_keys.sort();

        for table_key in &table_keys {
            let data = match tables.get(table_key) {
                Some(data) => data,
                None => continue,
            };

            let table = schema_index
                .table(&data.schema, &data.table)
                .ok_or_else(|| {
                    EvalError::InvalidDataset(format!(
                        "table '{}.{}' not found in schema",
                        data.schema, data.table
                    ))
                })?;

            collect_column_stats(data, &mut column_stats);
            evaluate_table_constraints(
                table,
                data,
                &tables,
                &plan_index,
                &mut warnings,
                &mut violations,
                &mut constraint_summary,
            );
        }

        sort_warnings(&mut warnings);
        sort_violations(&mut violations);
        column_stats.sort_by(|a, b| {
            (a.schema.clone(), a.table.clone(), a.column.clone()).cmp(&(
                b.schema.clone(),
                b.table.clone(),
                b.column.clone(),
            ))
        });

        let validate_ms = validate_start.elapsed().as_millis();
        let total_ms = total_start.elapsed().as_millis();

        let metrics = MetricsReport {
            metrics_version: METRICS_VERSION.to_string(),
            run_id: run_id.clone(),
            schema_ref: MetricsSchemaRef {
                schema_version: schema.schema_version.clone(),
                schema_fingerprint: schema.schema_fingerprint.clone(),
            },
            plan_ref: MetricsPlanRef {
                plan_version: plan.plan_version.clone(),
                seed: plan.seed,
                plan_hash: None,
            },
            tables: table_metrics,
            column_stats,
            constraints: constraint_summary,
            warnings: warnings.clone(),
            performance: PerformanceMetrics {
                load_ms,
                validate_ms,
                total_ms,
            },
        };

        let report = render_report(&metrics, &violations, self.options.max_examples);
        let out_dir = self
            .options
            .out_dir
            .clone()
            .unwrap_or_else(|| dataset_dir.to_path_buf());
        std::fs::create_dir_all(&out_dir)?;

        let metrics_path = out_dir.join("metrics.json");
        std::fs::write(&metrics_path, serde_json::to_vec_pretty(&metrics)?)?;

        let report_path = out_dir.join("report.md");
        std::fs::write(&report_path, report.as_bytes())?;

        let violations_path = if self.options.write_violations {
            let path = out_dir.join("violations.json");
            std::fs::write(&path, serde_json::to_vec_pretty(&violations)?)?;
            Some(path)
        } else {
            None
        };

        if self.options.strict && !violations.is_empty() {
            return Err(EvalError::Violations(violations.len() as u64));
        }

        Ok(EvaluationResult {
            run_dir: out_dir,
            metrics_path,
            report_path,
            violations_path,
            metrics,
            report,
            violations,
        })
    }
}

#[derive(Debug, Clone)]
struct ColumnInfo {
    name: String,
    is_nullable: bool,
    column_type: ColumnType,
}

#[derive(Debug, Clone)]
struct TableData {
    schema: String,
    table: String,
    columns: Vec<ColumnInfo>,
    column_lookup: HashMap<String, usize>,
    rows: Vec<Vec<GeneratedValue>>,
    rows_found: u64,
    null_counts: Vec<u64>,
    missing_columns: Vec<String>,
}

impl TableData {
    fn column_index(&self, column: &str) -> Option<usize> {
        self.column_lookup.get(&column.to_lowercase()).copied()
    }

    fn has_missing_column(&self, column: &str) -> bool {
        let column = column.to_lowercase();
        self.missing_columns
            .iter()
            .any(|name| name.to_lowercase() == column)
    }
}

struct SchemaIndex<'a> {
    tables: HashMap<String, &'a datalchemy_core::Table>,
}

impl<'a> SchemaIndex<'a> {
    fn new(schema: &'a DatabaseSchema) -> Self {
        let mut tables = HashMap::new();
        for db_schema in &schema.schemas {
            for table in &db_schema.tables {
                tables.insert(table_key(&db_schema.name, &table.name), table);
            }
        }
        Self { tables }
    }

    fn table(&self, schema: &str, table: &str) -> Option<&'a datalchemy_core::Table> {
        self.tables.get(&table_key(schema, table)).copied()
    }
}

struct PlanIndex {
    constraint_policies: HashMap<String, ConstraintMode>,
}

impl PlanIndex {
    fn new(plan: &Plan) -> Self {
        let mut constraint_policies = HashMap::new();

        for rule in &plan.rules {
            if let Rule::ConstraintPolicy(rule) = rule {
                let key = constraint_key(&rule.schema, &rule.table, rule.constraint.clone());
                constraint_policies.insert(key, rule.mode.clone());
            }
        }

        Self {
            constraint_policies,
        }
    }

    fn constraint_mode(&self, schema: &str, table: &str, kind: ConstraintKind) -> ConstraintMode {
        self.constraint_policies
            .get(&constraint_key(schema, table, kind))
            .cloned()
            .unwrap_or(ConstraintMode::Enforce)
    }
}

fn collect_target_tables(
    schema: &DatabaseSchema,
    plan: &Plan,
    schema_index: &SchemaIndex<'_>,
) -> Result<BTreeSet<String>, EvalError> {
    let mut targets = BTreeSet::new();

    if plan.targets.is_empty() {
        for db_schema in &schema.schemas {
            for table in &db_schema.tables {
                targets.insert(table_key(&db_schema.name, &table.name));
            }
        }
        return Ok(targets);
    }

    for target in &plan.targets {
        let target_key = table_key(&target.schema, &target.table);
        targets.insert(target_key.clone());
        if let Some(table) = schema_index.table(&target.schema, &target.table) {
            for constraint in &table.constraints {
                if let Constraint::ForeignKey(fk) = constraint {
                    targets.insert(table_key(&fk.referenced_schema, &fk.referenced_table));
                }
            }
        }
    }

    Ok(targets)
}

fn load_tables(
    schema_index: &SchemaIndex<'_>,
    target_tables: &BTreeSet<String>,
    dataset_dir: &Path,
    options: &EvaluateOptions,
    warnings: &mut Vec<WarningItem>,
) -> Result<BTreeMap<String, TableData>, EvalError> {
    let mut tables = BTreeMap::new();

    for table_key in target_tables {
        let (schema_name, table_name) = split_table_key(table_key)?;
        let table = match schema_index.table(schema_name, table_name) {
            Some(table) => table,
            None => {
                warnings.push(WarningItem {
                    code: "missing_schema_table".to_string(),
                    path: table_key.clone(),
                    message: format!("table '{table_key}' not found in schema"),
                    hint: Some("check plan targets against schema.json".to_string()),
                });
                continue;
            }
        };

        let csv_path = dataset_dir.join(format!("{table_key}.csv"));
        if !csv_path.exists() {
            warnings.push(WarningItem {
                code: "missing_table".to_string(),
                path: table_key.clone(),
                message: format!("dataset file not found: {}", csv_path.display()),
                hint: Some("ensure generation produced the CSV file".to_string()),
            });
            continue;
        }

        let data = load_table_csv(schema_name, table_name, table, &csv_path, options, warnings)?;
        tables.insert(table_key.clone(), data);
    }

    Ok(tables)
}

fn load_table_csv(
    schema: &str,
    table: &str,
    table_def: &datalchemy_core::Table,
    path: &Path,
    options: &EvaluateOptions,
    warnings: &mut Vec<WarningItem>,
) -> Result<TableData, EvalError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)?;

    let headers = reader
        .headers()
        .map_err(EvalError::Csv)?
        .iter()
        .map(|h| h.to_string())
        .collect::<Vec<_>>();
    let header_map = headers
        .iter()
        .enumerate()
        .map(|(idx, name)| (name.to_lowercase(), idx))
        .collect::<HashMap<_, _>>();

    let mut columns = table_def.columns.clone();
    columns.sort_by_key(|col| col.ordinal_position);

    let column_infos = columns
        .iter()
        .map(|col| ColumnInfo {
            name: col.name.clone(),
            is_nullable: col.is_nullable,
            column_type: col.column_type.clone(),
        })
        .collect::<Vec<_>>();

    let mut column_positions = Vec::with_capacity(column_infos.len());
    let mut column_lookup = HashMap::new();
    let mut missing_columns = Vec::new();

    for (idx, col) in column_infos.iter().enumerate() {
        column_lookup.insert(col.name.to_lowercase(), idx);
        match header_map.get(&col.name.to_lowercase()) {
            Some(position) => column_positions.push(Some(*position)),
            None => {
                column_positions.push(None);
                missing_columns.push(col.name.clone());
            }
        }
    }

    let mut extra_columns = Vec::new();
    for header in &headers {
        if !column_lookup.contains_key(&header.to_lowercase()) {
            extra_columns.push(header.clone());
        }
    }

    if !missing_columns.is_empty() {
        warnings.push(WarningItem {
            code: "missing_columns".to_string(),
            path: format!("{}.{}", schema, table),
            message: format!("missing columns: {}", missing_columns.join(", ")),
            hint: Some("regenerate dataset to include all columns".to_string()),
        });
    }

    if !extra_columns.is_empty() {
        warnings.push(WarningItem {
            code: "extra_columns".to_string(),
            path: format!("{}.{}", schema, table),
            message: format!("unexpected columns: {}", extra_columns.join(", ")),
            hint: Some("remove extra columns or update schema".to_string()),
        });
    }

    let mut rows = Vec::new();
    let mut null_counts = vec![0u64; column_infos.len()];
    for (row_idx, result) in reader.records().enumerate() {
        let record = result?;
        let mut row = Vec::with_capacity(column_infos.len());
        for (col_idx, col) in column_infos.iter().enumerate() {
            let value = match column_positions[col_idx] {
                Some(pos) => record.get(pos).unwrap_or_default(),
                None => "",
            };

            match parse_value(col, value) {
                Ok(parsed) => {
                    if parsed.is_null() {
                        null_counts[col_idx] += 1;
                    }
                    row.push(parsed);
                }
                Err(message) => {
                    warnings.push(WarningItem {
                        code: "invalid_value".to_string(),
                        path: format!("{}.{}.{}:{}", schema, table, col.name, row_idx + 1),
                        message,
                        hint: Some("check CSV serialization for this column".to_string()),
                    });
                    if options.strict {
                        return Err(EvalError::InvalidDataset(format!(
                            "invalid value at {}.{}.{} row {}",
                            schema,
                            table,
                            col.name,
                            row_idx + 1
                        )));
                    }
                    null_counts[col_idx] += 1;
                    row.push(GeneratedValue::Null);
                }
            }
        }
        rows.push(row);
    }

    Ok(TableData {
        schema: schema.to_string(),
        table: table.to_string(),
        columns: column_infos,
        column_lookup,
        rows_found: rows.len() as u64,
        rows,
        null_counts,
        missing_columns,
    })
}

fn collect_column_stats(table: &TableData, stats: &mut Vec<ColumnStats>) {
    for (idx, col) in table.columns.iter().enumerate() {
        stats.push(ColumnStats {
            schema: table.schema.clone(),
            table: table.table.clone(),
            column: col.name.clone(),
            null_count: table.null_counts[idx],
        });
    }
}

fn evaluate_table_constraints(
    table: &datalchemy_core::Table,
    data: &TableData,
    tables: &BTreeMap<String, TableData>,
    plan_index: &PlanIndex,
    warnings: &mut Vec<WarningItem>,
    violations: &mut Vec<Violation>,
    summary: &mut ConstraintSummary,
) {
    evaluate_not_null(data, violations, summary);
    evaluate_unique(table, data, warnings, violations, summary);
    evaluate_foreign_keys(table, data, tables, warnings, violations, summary);
    evaluate_checks(table, data, plan_index, warnings, violations, summary);
}

fn evaluate_not_null(
    data: &TableData,
    violations: &mut Vec<Violation>,
    summary: &mut ConstraintSummary,
) {
    for (idx, col) in data.columns.iter().enumerate() {
        if col.is_nullable || data.has_missing_column(&col.name) {
            continue;
        }
        summary.not_null.checked += 1;
        let nulls = data.null_counts[idx];
        if nulls > 0 {
            summary.not_null.violations += nulls;
            violations.push(Violation {
                code: "not_null".to_string(),
                path: format!("{}.{}.{}", data.schema, data.table, col.name),
                message: format!("{} null value(s) found", nulls),
                row_index: None,
                example: None,
            });
        }
    }
}

fn evaluate_unique(
    table: &datalchemy_core::Table,
    data: &TableData,
    warnings: &mut Vec<WarningItem>,
    violations: &mut Vec<Violation>,
    summary: &mut ConstraintSummary,
) {
    for constraint in &table.constraints {
        match constraint {
            Constraint::PrimaryKey(pk) => {
                summary.pk.checked += 1;
                let count = check_unique_constraint(
                    "primary_key",
                    &data.schema,
                    &data.table,
                    &pk.columns,
                    data,
                    warnings,
                    violations,
                );
                summary.pk.violations += count;
            }
            Constraint::Unique(unique) => {
                summary.unique.checked += 1;
                let count = check_unique_constraint(
                    "unique",
                    &data.schema,
                    &data.table,
                    &unique.columns,
                    data,
                    warnings,
                    violations,
                );
                summary.unique.violations += count;
            }
            _ => {}
        }
    }
}

fn check_unique_constraint(
    kind: &str,
    schema: &str,
    table: &str,
    columns: &[String],
    data: &TableData,
    warnings: &mut Vec<WarningItem>,
    violations: &mut Vec<Violation>,
) -> u64 {
    let mut indices = Vec::new();
    for column in columns {
        if data.has_missing_column(column) {
            warnings.push(WarningItem {
                code: "missing_column".to_string(),
                path: format!("{}.{}.{}", schema, table, column),
                message: "column missing in dataset".to_string(),
                hint: Some("regenerate dataset with full headers".to_string()),
            });
            return 0;
        }
        if let Some(idx) = data.column_index(column) {
            indices.push(idx);
        } else {
            return 0;
        }
    }

    let mut seen = HashSet::new();
    let mut violations_count = 0u64;

    for (row_idx, row) in data.rows.iter().enumerate() {
        let values = indices
            .iter()
            .map(|idx| row.get(*idx).cloned().unwrap_or(GeneratedValue::Null))
            .collect::<Vec<_>>();
        let has_null = values.iter().any(|value| value.is_null());
        if has_null {
            if kind == "primary_key" {
                violations_count += 1;
                violations.push(Violation {
                    code: "primary_key".to_string(),
                    path: format!("{}.{}.{}", schema, table, columns.join(",")),
                    message: "null value in primary key".to_string(),
                    row_index: Some(row_idx as u64 + 1),
                    example: None,
                });
            }
            continue;
        }

        let key = tuple_key(&values);
        if !seen.insert(key.clone()) {
            violations_count += 1;
            violations.push(Violation {
                code: kind.to_string(),
                path: format!("{}.{}.{}", schema, table, columns.join(",")),
                message: "duplicate key detected".to_string(),
                row_index: Some(row_idx as u64 + 1),
                example: Some(key),
            });
        }
    }

    violations_count
}

fn evaluate_foreign_keys(
    table: &datalchemy_core::Table,
    data: &TableData,
    tables: &BTreeMap<String, TableData>,
    warnings: &mut Vec<WarningItem>,
    violations: &mut Vec<Violation>,
    summary: &mut ConstraintSummary,
) {
    for constraint in &table.constraints {
        if let Constraint::ForeignKey(fk) = constraint {
            summary.fk.checked += 1;
            let count = check_foreign_key(data, fk, tables, warnings, violations);
            summary.fk.violations += count;
        }
    }
}

fn check_foreign_key(
    data: &TableData,
    fk: &ForeignKey,
    tables: &BTreeMap<String, TableData>,
    warnings: &mut Vec<WarningItem>,
    violations: &mut Vec<Violation>,
) -> u64 {
    if fk.columns.len() != fk.referenced_columns.len() {
        warnings.push(WarningItem {
            code: "fk_mismatch".to_string(),
            path: format!("{}.{}", data.schema, data.table),
            message: "foreign key column count mismatch".to_string(),
            hint: Some("check schema.json for FK definition".to_string()),
        });
        return 0;
    }

    for column in &fk.columns {
        if data.has_missing_column(column) {
            warnings.push(WarningItem {
                code: "missing_column".to_string(),
                path: format!("{}.{}.{}", data.schema, data.table, column),
                message: "column missing in dataset".to_string(),
                hint: Some("regenerate dataset with full headers".to_string()),
            });
            return 0;
        }
    }

    let parent_key = table_key(&fk.referenced_schema, &fk.referenced_table);
    let parent = match tables.get(&parent_key) {
        Some(table) => table,
        None => {
            warnings.push(WarningItem {
                code: "missing_parent_table".to_string(),
                path: format!("{}.{}", data.schema, data.table),
                message: format!("parent table '{}' not found in dataset", parent_key),
                hint: Some("include parent tables in generation targets".to_string()),
            });
            return 0;
        }
    };

    let child_indices = fk
        .columns
        .iter()
        .filter_map(|col| data.column_index(col))
        .collect::<Vec<_>>();
    let parent_indices = fk
        .referenced_columns
        .iter()
        .filter_map(|col| parent.column_index(col))
        .collect::<Vec<_>>();

    if child_indices.len() != fk.columns.len()
        || parent_indices.len() != fk.referenced_columns.len()
    {
        warnings.push(WarningItem {
            code: "missing_parent_column".to_string(),
            path: format!("{}.{}", data.schema, data.table),
            message: "foreign key columns missing in dataset".to_string(),
            hint: Some("check CSV headers for FK columns".to_string()),
        });
        return 0;
    }

    let mut parent_keys = HashSet::new();
    for row in &parent.rows {
        let values = parent_indices
            .iter()
            .map(|idx| row.get(*idx).cloned().unwrap_or(GeneratedValue::Null))
            .collect::<Vec<_>>();
        if values.iter().any(|value| value.is_null()) {
            continue;
        }
        parent_keys.insert(tuple_key(&values));
    }

    let mut violations_count = 0u64;
    for (row_idx, row) in data.rows.iter().enumerate() {
        let values = child_indices
            .iter()
            .map(|idx| row.get(*idx).cloned().unwrap_or(GeneratedValue::Null))
            .collect::<Vec<_>>();
        if values.iter().any(|value| value.is_null()) {
            continue;
        }
        let key = tuple_key(&values);
        if !parent_keys.contains(&key) {
            violations_count += 1;
            violations.push(Violation {
                code: "foreign_key".to_string(),
                path: format!(
                    "{}.{} -> {}.{}",
                    data.schema, data.table, fk.referenced_schema, fk.referenced_table
                ),
                message: "broken foreign key reference".to_string(),
                row_index: Some(row_idx as u64 + 1),
                example: Some(key),
            });
        }
    }

    violations_count
}

fn evaluate_checks(
    table: &datalchemy_core::Table,
    data: &TableData,
    plan_index: &PlanIndex,
    warnings: &mut Vec<WarningItem>,
    violations: &mut Vec<Violation>,
    summary: &mut ConstraintSummary,
) {
    let checks = table
        .constraints
        .iter()
        .filter_map(|constraint| {
            if let Constraint::Check(check) = constraint {
                Some(check)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    summary.check.checked += checks.len() as u64;
    if checks.is_empty() {
        return;
    }

    for check in checks {
        let mode = plan_index.constraint_mode(&data.schema, &data.table, ConstraintKind::Check);
        match evaluate_check_constraint(check, data, warnings, violations) {
            CheckEvaluation::Passed => {}
            CheckEvaluation::Failed(count) => {
                summary.check.violations += count;
            }
            CheckEvaluation::Unsupported => {
                summary.check.not_evaluated += 1;
                match mode {
                    ConstraintMode::Enforce => {
                        summary.check.violations += 1;
                        violations.push(Violation {
                            code: "check".to_string(),
                            path: format!("{}.{}", data.schema, data.table),
                            message: format!("unsupported check expression: {}", check.expression),
                            row_index: None,
                            example: None,
                        });
                    }
                    ConstraintMode::Warn => {
                        warnings.push(WarningItem {
                            code: "check_unsupported".to_string(),
                            path: format!("{}.{}", data.schema, data.table),
                            message: format!(
                                "check expression not evaluated: {}",
                                check.expression
                            ),
                            hint: Some("simplify the CHECK or switch policy".to_string()),
                        });
                    }
                    ConstraintMode::Ignore => {}
                }
            }
        }
    }
}

enum CheckEvaluation {
    Passed,
    Failed(u64),
    Unsupported,
}

fn evaluate_check_constraint(
    check: &CheckConstraint,
    data: &TableData,
    warnings: &mut Vec<WarningItem>,
    violations: &mut Vec<Violation>,
) -> CheckEvaluation {
    if data.rows.is_empty() {
        return CheckEvaluation::Passed;
    }

    let base_date = match NaiveDate::from_ymd_opt(2024, 1, 1) {
        Some(date) => date,
        None => {
            warnings.push(WarningItem {
                code: "base_date_error".to_string(),
                path: format!("{}.{}", data.schema, data.table),
                message: "invalid base date for CHECK evaluation".to_string(),
                hint: Some("verify evaluator base date".to_string()),
            });
            return CheckEvaluation::Unsupported;
        }
    };
    let mut failures = 0u64;

    for (row_idx, row) in data.rows.iter().enumerate() {
        let mut values = HashMap::with_capacity(data.columns.len());
        for (col_idx, col) in data.columns.iter().enumerate() {
            values.insert(
                col.name.to_lowercase(),
                row.get(col_idx).cloned().unwrap_or(GeneratedValue::Null),
            );
        }

        let ctx = CheckContext {
            values: &values,
            base_date,
        };
        match evaluate_check(&check.expression, &ctx) {
            CheckOutcome::Passed => {}
            CheckOutcome::Failed => {
                failures += 1;
                violations.push(Violation {
                    code: "check".to_string(),
                    path: format!("{}.{}", data.schema, data.table),
                    message: "check constraint failed".to_string(),
                    row_index: Some(row_idx as u64 + 1),
                    example: Some(check.expression.clone()),
                });
            }
            CheckOutcome::Unsupported => {
                return CheckEvaluation::Unsupported;
            }
        }
    }

    if failures == 0 {
        CheckEvaluation::Passed
    } else {
        CheckEvaluation::Failed(failures)
    }
}

fn build_table_metrics(
    plan: &Plan,
    target_tables: &BTreeSet<String>,
    tables: &BTreeMap<String, TableData>,
) -> Vec<TableMetrics> {
    let mut expected = HashMap::new();
    for target in &plan.targets {
        expected.insert(table_key(&target.schema, &target.table), target.rows);
    }

    let mut metrics = Vec::new();
    for table_key in target_tables {
        let (schema, table) = match split_table_key(table_key) {
            Ok((schema, table)) => (schema.to_string(), table.to_string()),
            Err(_) => continue,
        };
        let rows_found = tables
            .get(table_key)
            .map(|data| data.rows_found)
            .unwrap_or(0);
        let rows_expected = expected.get(table_key).copied();
        metrics.push(TableMetrics {
            schema,
            table,
            rows_found,
            rows_expected,
        });
    }

    metrics
}

fn parse_value(column: &ColumnInfo, value: &str) -> Result<GeneratedValue, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        return Ok(GeneratedValue::Null);
    }

    let normalized_type = normalize_type(&column.column_type);
    match normalized_type.as_str() {
        "uuid" => Uuid::parse_str(trimmed)
            .map(|value| GeneratedValue::Uuid(value.to_string()))
            .map_err(|_| format!("invalid uuid '{}'", trimmed)),
        "smallint" | "integer" | "bigint" => trimmed
            .parse::<i64>()
            .map(GeneratedValue::Int)
            .map_err(|_| format!("invalid integer '{}'", trimmed)),
        "numeric" | "decimal" => {
            let scale = column.column_type.numeric_scale.unwrap_or(0);
            if scale > 0 {
                trimmed
                    .parse::<f64>()
                    .map(GeneratedValue::Float)
                    .map_err(|_| format!("invalid numeric '{}'", trimmed))
            } else if let Ok(value) = trimmed.parse::<i64>() {
                Ok(GeneratedValue::Int(value))
            } else {
                trimmed
                    .parse::<f64>()
                    .map(GeneratedValue::Float)
                    .map_err(|_| format!("invalid numeric '{}'", trimmed))
            }
        }
        "real" | "double precision" => trimmed
            .parse::<f64>()
            .map(GeneratedValue::Float)
            .map_err(|_| format!("invalid float '{}'", trimmed)),
        "boolean" => parse_bool(trimmed)
            .map(GeneratedValue::Bool)
            .ok_or_else(|| format!("invalid boolean '{}'", trimmed)),
        "date" => NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
            .map(GeneratedValue::Date)
            .map_err(|_| format!("invalid date '{}'", trimmed)),
        "timestamp with time zone" | "timestamp without time zone" => {
            NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S")
                .map(GeneratedValue::Timestamp)
                .map_err(|_| format!("invalid timestamp '{}'", trimmed))
        }
        "time with time zone" | "time without time zone" => {
            NaiveTime::parse_from_str(trimmed, "%H:%M:%S")
                .map(GeneratedValue::Time)
                .map_err(|_| format!("invalid time '{}'", trimmed))
        }
        _ => Ok(GeneratedValue::Text(trimmed.to_string())),
    }
    .map_err(|err| err)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_lowercase().as_str() {
        "true" | "t" | "1" => Some(true),
        "false" | "f" | "0" => Some(false),
        _ => None,
    }
}

fn normalize_type(column_type: &ColumnType) -> String {
    column_type
        .data_type
        .split('(')
        .next()
        .unwrap_or(&column_type.data_type)
        .trim()
        .to_lowercase()
}

fn detect_run_id(dataset_dir: &Path) -> Option<String> {
    let report_path = dataset_dir.join("generation_report.json");
    if report_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(report_path) {
            if let Ok(report) = serde_json::from_str::<GenerationReport>(&contents) {
                return Some(report.run_id);
            }
        }
    }

    let name = dataset_dir.file_name()?.to_string_lossy();
    if let Some((_, run_part)) = name.split_once("__run_") {
        return Some(run_part.to_string());
    }

    None
}

fn tuple_key(values: &[GeneratedValue]) -> String {
    values
        .iter()
        .map(|value| escape_key_component(&value_key(value)))
        .collect::<Vec<_>>()
        .join("|")
}

fn value_key(value: &GeneratedValue) -> String {
    match value {
        GeneratedValue::Null => "null".to_string(),
        GeneratedValue::Bool(value) => value.to_string(),
        GeneratedValue::Int(value) => value.to_string(),
        GeneratedValue::Float(value) => value.to_string(),
        GeneratedValue::Text(value) | GeneratedValue::Uuid(value) => value.clone(),
        GeneratedValue::Date(value) => value.format("%Y-%m-%d").to_string(),
        GeneratedValue::Time(value) => value.format("%H:%M:%S").to_string(),
        GeneratedValue::Timestamp(value) => value.format("%Y-%m-%dT%H:%M:%S").to_string(),
    }
}

fn escape_key_component(value: &str) -> String {
    value.replace('\\', "\\\\").replace('|', "\\|")
}

fn table_key(schema: &str, table: &str) -> String {
    format!("{schema}.{table}")
}

fn split_table_key(table_key: &str) -> Result<(&str, &str), EvalError> {
    table_key
        .split_once('.')
        .ok_or_else(|| EvalError::InvalidDataset(format!("invalid table key: {table_key}")))
}

fn constraint_key(schema: &str, table: &str, kind: ConstraintKind) -> String {
    format!("{}.{}.{}", schema, table, constraint_kind_key(kind))
}

fn constraint_kind_key(kind: ConstraintKind) -> &'static str {
    match kind {
        ConstraintKind::Check => "check",
        ConstraintKind::Unique => "unique",
        ConstraintKind::NotNull => "not_null",
        ConstraintKind::PrimaryKey => "primary_key",
        ConstraintKind::ForeignKey => "foreign_key",
    }
}

fn sort_warnings(warnings: &mut Vec<WarningItem>) {
    warnings
        .sort_by(|a, b| (a.path.clone(), a.code.clone()).cmp(&(b.path.clone(), b.code.clone())));
}

fn sort_violations(violations: &mut Vec<Violation>) {
    violations.sort_by(|a, b| {
        (
            a.path.clone(),
            a.code.clone(),
            a.row_index.unwrap_or_default(),
        )
            .cmp(&(
                b.path.clone(),
                b.code.clone(),
                b.row_index.unwrap_or_default(),
            ))
    });
}
