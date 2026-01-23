use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use datalchemy_core::{CheckConstraint, Constraint, DatabaseSchema, ForeignKey, Table};
use datalchemy_plan::{
    ColumnGeneratorRule, ConstraintKind, ConstraintMode, ForeignKeyMode, Plan, Rule,
};

use crate::checks::{CheckContext, CheckOutcome, evaluate_check};
use crate::errors::GenerationError;
use crate::generators::{GeneratedValue, GeneratorRegistry};
use crate::model::{GenerateOptions, GenerationIssue, GenerationReport, TableReport};
use crate::output::csv::write_table_csv;
use crate::planner::plan_tables;

/// Result of a generation run.
#[derive(Debug, Clone)]
pub struct GenerationResult {
    pub run_dir: PathBuf,
    pub report: GenerationReport,
}

/// Entry point for generating datasets from schema + plan.
#[derive(Debug, Clone)]
pub struct GenerationEngine {
    options: GenerateOptions,
}

impl GenerationEngine {
    pub fn new(options: GenerateOptions) -> Self {
        Self { options }
    }

    pub fn run(
        &self,
        schema: &DatabaseSchema,
        plan: &Plan,
    ) -> Result<GenerationResult, GenerationError> {
        let run_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%SZ").to_string();
        let run_dir = self
            .options
            .out_dir
            .join(format!("{timestamp}__run_{run_id}"));
        std::fs::create_dir_all(&run_dir)?;

        let plan_index = PlanIndex::new(plan);
        let tasks = plan_tables(schema, plan, self.options.auto_generate_parents)?;
        let schema_index = SchemaIndex::new(schema);
        let generator = GeneratorRegistry::new(plan_index.column_generators.clone(), schema);
        let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let mut report = GenerationReport::new(run_id.clone());

        let mut table_data: HashMap<String, TableData> = HashMap::new();

        for task in tasks {
            let table = schema_index
                .table(&task.schema, &task.table)
                .ok_or_else(|| {
                    GenerationError::InvalidPlan(format!(
                        "table '{}.{}' not found in schema",
                        task.schema, task.table
                    ))
                })?;
            let table_key = table_key(&task.schema, &task.table);

            let table_ctx = TableContext::new(&task.schema, table, schema, &plan_index, base_date);

            let table_seed = hash_seed(plan.seed, &table_key);
            let result = generate_table(
                &table_ctx,
                &generator,
                &plan_index,
                table_seed,
                task.rows,
                &self.options,
                &mut table_data,
                &mut report,
            )?;

            let csv_path = run_dir.join(format!("{}.{}.csv", task.schema, task.table));
            write_table_csv(&csv_path, table, &result.rows)?;

            report.tables.push(TableReport {
                schema: task.schema,
                table: task.table,
                rows_requested: task.rows,
                rows_generated: result.rows.len() as u64,
                retries: result.retries,
            });
            report.retries_total += result.retries;

            table_data.insert(table_key, result);
        }

        let plan_path = run_dir.join("resolved_plan.json");
        std::fs::write(&plan_path, serde_json::to_vec_pretty(plan)?)?;

        let report_path = run_dir.join("generation_report.json");
        std::fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

        Ok(GenerationResult { run_dir, report })
    }
}

struct TableData {
    rows: Vec<HashMap<String, GeneratedValue>>,
    retries: u64,
}

struct TableContext<'a> {
    schema: &'a str,
    table: &'a Table,
    primary_keys: Vec<Vec<String>>,
    unique_constraints: Vec<Vec<String>>,
    unique_columns: HashSet<String>,
    fk_columns: HashSet<String>,
    check_constraints: Vec<&'a CheckConstraint>,
    foreign_keys: Vec<&'a ForeignKey>,
    numeric_bounds: HashMap<String, NumericBounds>,
    base_date: NaiveDate,
}

impl<'a> TableContext<'a> {
    fn new(
        schema_name: &'a str,
        table: &'a Table,
        schema: &'a DatabaseSchema,
        plan_index: &PlanIndex,
        base_date: NaiveDate,
    ) -> Self {
        let mut primary_keys = Vec::new();
        let mut unique_constraints = Vec::new();
        let mut check_constraints = Vec::new();
        let mut foreign_keys = Vec::new();
        let mut unique_columns = HashSet::new();
        let mut fk_columns = HashSet::new();

        for constraint in &table.constraints {
            match constraint {
                Constraint::PrimaryKey(pk) => {
                    primary_keys.push(pk.columns.clone());
                    for column in &pk.columns {
                        unique_columns.insert(column.to_lowercase());
                    }
                }
                Constraint::Unique(unique) => {
                    unique_constraints.push(unique.columns.clone());
                    for column in &unique.columns {
                        unique_columns.insert(column.to_lowercase());
                    }
                }
                Constraint::Check(check) => check_constraints.push(check),
                Constraint::ForeignKey(fk) => {
                    for column in &fk.columns {
                        fk_columns.insert(column.to_lowercase());
                    }
                    foreign_keys.push(fk);
                }
            }
        }

        let numeric_bounds = extract_numeric_bounds(schema_name, table, plan_index);

        let _ = schema; // reserved for future schema-aware extensions

        Self {
            schema: schema_name,
            table,
            primary_keys,
            unique_constraints,
            unique_columns,
            fk_columns,
            check_constraints,
            foreign_keys,
            numeric_bounds,
            base_date,
        }
    }
}

struct PlanIndex {
    column_generators: Vec<ColumnGeneratorRule>,
    constraint_policies: HashMap<String, ConstraintMode>,
    fk_strategies: HashMap<String, ForeignKeyMode>,
    allow_fk_disable: bool,
}

impl PlanIndex {
    fn new(plan: &Plan) -> Self {
        let mut column_generators = Vec::new();
        let mut constraint_policies = HashMap::new();
        let mut fk_strategies = HashMap::new();

        for rule in &plan.rules {
            match rule {
                Rule::ColumnGenerator(rule) => column_generators.push(rule.clone()),
                Rule::ConstraintPolicy(rule) => {
                    let key = constraint_key(&rule.schema, &rule.table, rule.constraint.clone());
                    constraint_policies.insert(key, rule.mode.clone());
                }
                Rule::ForeignKeyStrategy(rule) => {
                    let key = table_key(&rule.schema, &rule.table);
                    fk_strategies.insert(key, rule.mode.clone());
                }
            }
        }

        let allow_fk_disable = plan
            .options
            .as_ref()
            .and_then(|opts| opts.allow_fk_disable)
            .unwrap_or(false);

        Self {
            column_generators,
            constraint_policies,
            fk_strategies,
            allow_fk_disable,
        }
    }

    fn constraint_mode(&self, schema: &str, table: &str, kind: ConstraintKind) -> ConstraintMode {
        self.constraint_policies
            .get(&constraint_key(schema, table, kind))
            .cloned()
            .unwrap_or(ConstraintMode::Enforce)
    }

    fn fk_mode(&self, schema: &str, table: &str) -> ForeignKeyMode {
        self.fk_strategies
            .get(&table_key(schema, table))
            .cloned()
            .unwrap_or(ForeignKeyMode::Respect)
    }
}

struct SchemaIndex<'a> {
    tables: HashMap<String, &'a Table>,
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

    fn table(&self, schema: &str, table: &str) -> Option<&'a Table> {
        self.tables.get(&table_key(schema, table)).copied()
    }
}

fn generate_table(
    ctx: &TableContext<'_>,
    generator: &GeneratorRegistry,
    plan_index: &PlanIndex,
    table_seed: u64,
    rows: u64,
    options: &GenerateOptions,
    table_data: &mut HashMap<String, TableData>,
    report: &mut GenerationReport,
) -> Result<TableData, GenerationError> {
    let mut retries_total = 0;

    for _ in 0..options.max_attempts_table {
        let mut rows_out = Vec::new();
        let mut unique_sets = build_unique_sets(ctx);
        let mut failed = false;

        for row_index in 0..rows {
            let mut row_attempts = 0;
            loop {
                row_attempts += 1;
                let mut rng =
                    ChaCha8Rng::seed_from_u64(hash_row_seed(table_seed, row_index, row_attempts));
                let mut row = HashMap::new();

                if plan_index.fk_mode(ctx.schema, &ctx.table.name) == ForeignKeyMode::Respect {
                    apply_foreign_keys(ctx, &mut row, &mut rng, table_data)?;
                } else if !plan_index.allow_fk_disable {
                    report.warnings.push(GenerationIssue {
                        level: "warning".to_string(),
                        code: "fk_disable_without_flag".to_string(),
                        message: format!(
                            "foreign keys disabled for '{}.{}' without allow_fk_disable",
                            ctx.schema, ctx.table.name
                        ),
                        path: None,
                    });
                }

                let mut columns = ctx.table.columns.clone();
                columns.sort_by_key(|col| col.ordinal_position);

                for column in columns {
                    let key = column.name.to_lowercase();
                    if row.contains_key(&key) {
                        continue;
                    }

                    let mut value = if let Some(rule) =
                        generator.rule_for(ctx.schema, &ctx.table.name, &column.name)
                    {
                        if ctx.unique_columns.contains(&key) && !ctx.fk_columns.contains(&key) {
                            generate_unique_from_rule(rule, &column, row_index, ctx.base_date)
                        } else {
                            generator.generate(
                                ctx.schema,
                                &ctx.table.name,
                                &column,
                                ctx.base_date,
                                &mut rng,
                            )?
                        }
                    } else if ctx.unique_columns.contains(&key) && !ctx.fk_columns.contains(&key) {
                        generate_unique_value(&column, row_index, ctx.base_date)
                    } else if let Some(default_value) =
                        generate_default(&column, ctx.base_date, &mut rng)
                    {
                        default_value
                    } else {
                        if column.is_nullable && rng.gen_bool(0.1) {
                            row.insert(key.clone(), GeneratedValue::Null);
                            continue;
                        }
                        generator.generate(
                            ctx.schema,
                            &ctx.table.name,
                            &column,
                            ctx.base_date,
                            &mut rng,
                        )?
                    };

                    if let Some(bounds) = ctx.numeric_bounds.get(&key) {
                        value = apply_numeric_bounds(value, bounds);
                    }

                    row.insert(key.clone(), value);
                }

                if let Some(error) = enforce_not_null(ctx, &row) {
                    if row_attempts >= options.max_attempts_row {
                        if options.strict {
                            return Err(error);
                        }
                        failed = true;
                        break;
                    }
                    retries_total += 1;
                    continue;
                }

                if let Some(outcome) = evaluate_checks(ctx, plan_index, &row, report) {
                    match outcome {
                        CheckOutcome::Passed => {}
                        CheckOutcome::Failed => {
                            if row_attempts >= options.max_attempts_row {
                                if options.strict {
                                    return Err(GenerationError::Unsupported(
                                        "check constraint failed too many times".to_string(),
                                    ));
                                }
                                failed = true;
                                break;
                            }
                            retries_total += 1;
                            continue;
                        }
                        CheckOutcome::Unsupported => {
                            if options.strict {
                                return Err(GenerationError::Unsupported(
                                    "unsupported CHECK constraint".to_string(),
                                ));
                            }
                        }
                    }
                }

                if !check_uniques(&mut unique_sets, &row) {
                    if row_attempts >= options.max_attempts_row {
                        if options.strict {
                            return Err(GenerationError::Unsupported(
                                "unique constraint failed too many times".to_string(),
                            ));
                        }
                        failed = true;
                        break;
                    }
                    retries_total += 1;
                    continue;
                }

                rows_out.push(row);
                break;
            }

            if failed {
                break;
            }
        }

        if !failed {
            return Ok(TableData {
                rows: rows_out,
                retries: retries_total,
            });
        }
    }

    Err(GenerationError::Unsupported(
        "failed to generate table within attempt limit".to_string(),
    ))
}

fn apply_foreign_keys(
    ctx: &TableContext<'_>,
    row: &mut HashMap<String, GeneratedValue>,
    rng: &mut ChaCha8Rng,
    table_data: &mut HashMap<String, TableData>,
) -> Result<(), GenerationError> {
    for fk in &ctx.foreign_keys {
        let parent_key = table_key(&fk.referenced_schema, &fk.referenced_table);
        let parent = table_data.get(&parent_key).ok_or_else(|| {
            GenerationError::Unsupported(format!(
                "missing parent table '{}' for foreign key",
                parent_key
            ))
        })?;

        if parent.rows.is_empty() {
            return Err(GenerationError::Unsupported(format!(
                "parent table '{}' has no rows",
                parent_key
            )));
        }

        let index = rng.gen_range(0..parent.rows.len());
        let parent_row = &parent.rows[index];

        for (child_col, parent_col) in fk.columns.iter().zip(&fk.referenced_columns) {
            let parent_value = parent_row
                .get(&parent_col.to_lowercase())
                .ok_or_else(|| {
                    GenerationError::Unsupported(format!(
                        "missing referenced column '{}' in parent row",
                        parent_col
                    ))
                })?
                .clone();
            row.insert(child_col.to_lowercase(), parent_value);
        }
    }

    Ok(())
}

fn generate_default(
    column: &datalchemy_core::Column,
    base_date: NaiveDate,
    rng: &mut ChaCha8Rng,
) -> Option<GeneratedValue> {
    let default = column.default.as_ref()?.trim();
    let normalized = normalize_default(default);

    match normalized.as_str() {
        "gen_random_uuid()" | "uuid_generate_v4()" => {
            let bytes: [u8; 16] = rng.r#gen();
            Some(GeneratedValue::Uuid(
                uuid::Uuid::from_bytes(bytes).to_string(),
            ))
        }
        "now()" | "current_timestamp" => {
            let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
            Some(GeneratedValue::Timestamp(NaiveDateTime::new(
                base_date, time,
            )))
        }
        "current_date" => Some(GeneratedValue::Date(base_date)),
        "true" => Some(GeneratedValue::Bool(true)),
        "false" => Some(GeneratedValue::Bool(false)),
        _ => {
            if let Ok(value) = normalized.parse::<i64>() {
                return Some(GeneratedValue::Int(value));
            }
            if let Ok(value) = normalized.parse::<f64>() {
                return Some(GeneratedValue::Float(value));
            }
            if normalized.starts_with('\'') && normalized.ends_with('\'') {
                return Some(GeneratedValue::Text(
                    normalized[1..normalized.len() - 1].to_string(),
                ));
            }
            None
        }
    }
}

fn normalize_default(default: &str) -> String {
    let mut value = default
        .trim()
        .trim_matches('(')
        .trim_matches(')')
        .to_string();
    if let Some((left, _)) = value.split_once("::") {
        value = left.trim().to_string();
    }
    value
}

fn enforce_not_null(
    ctx: &TableContext<'_>,
    row: &HashMap<String, GeneratedValue>,
) -> Option<GenerationError> {
    for column in &ctx.table.columns {
        if !column.is_nullable {
            let key = column.name.to_lowercase();
            if row.get(&key).map(|value| value.is_null()).unwrap_or(true) {
                return Some(GenerationError::Unsupported(format!(
                    "column '{}.{}.{}' is null",
                    ctx.schema, ctx.table.name, column.name
                )));
            }
        }
    }
    None
}

fn evaluate_checks(
    ctx: &TableContext<'_>,
    plan_index: &PlanIndex,
    row: &HashMap<String, GeneratedValue>,
    report: &mut GenerationReport,
) -> Option<CheckOutcome> {
    let mode = plan_index.constraint_mode(ctx.schema, &ctx.table.name, ConstraintKind::Check);
    if mode == ConstraintMode::Ignore {
        return None;
    }

    let mut outcome = CheckOutcome::Passed;
    for check in &ctx.check_constraints {
        let check_ctx = CheckContext {
            values: row,
            base_date: ctx.base_date,
        };
        match evaluate_check(&check.expression, &check_ctx) {
            CheckOutcome::Passed => {}
            CheckOutcome::Failed => {
                if mode == ConstraintMode::Warn {
                    report.warnings.push(GenerationIssue {
                        level: "warning".to_string(),
                        code: "check_failed".to_string(),
                        message: format!(
                            "check constraint failed on '{}.{}'",
                            ctx.schema, ctx.table.name
                        ),
                        path: None,
                    });
                    continue;
                }
                outcome = CheckOutcome::Failed;
            }
            CheckOutcome::Unsupported => {
                report.unsupported.push(GenerationIssue {
                    level: "warning".to_string(),
                    code: "check_unsupported".to_string(),
                    message: format!(
                        "unsupported CHECK constraint on '{}.{}'",
                        ctx.schema, ctx.table.name
                    ),
                    path: None,
                });
                if mode == ConstraintMode::Enforce {
                    outcome = CheckOutcome::Unsupported;
                }
            }
        }
    }

    Some(outcome)
}

fn build_unique_sets(ctx: &TableContext<'_>) -> Vec<UniqueSet> {
    let mut sets = Vec::new();
    for pk in &ctx.primary_keys {
        sets.push(UniqueSet::new(pk.clone()));
    }
    for unique in &ctx.unique_constraints {
        sets.push(UniqueSet::new(unique.clone()));
    }
    sets
}

fn check_uniques(sets: &mut [UniqueSet], row: &HashMap<String, GeneratedValue>) -> bool {
    for set in sets {
        let key = set.key_for(row);
        if !set.seen.insert(key) {
            return false;
        }
    }
    true
}

struct UniqueSet {
    columns: Vec<String>,
    seen: HashSet<String>,
}

impl UniqueSet {
    fn new(columns: Vec<String>) -> Self {
        Self {
            columns: columns.into_iter().map(|c| c.to_lowercase()).collect(),
            seen: HashSet::new(),
        }
    }

    fn key_for(&self, row: &HashMap<String, GeneratedValue>) -> String {
        let mut key_parts = Vec::new();
        for column in &self.columns {
            let value = row
                .get(column)
                .map(value_to_key)
                .unwrap_or_else(|| "<null>".to_string());
            key_parts.push(value);
        }
        key_parts.join("|")
    }
}

fn value_to_key(value: &GeneratedValue) -> String {
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

#[derive(Debug, Clone, Copy)]
struct NumericBounds {
    min: Option<f64>,
    max: Option<f64>,
}

fn extract_numeric_bounds(
    schema: &str,
    table: &Table,
    plan_index: &PlanIndex,
) -> HashMap<String, NumericBounds> {
    let mode = plan_index.constraint_mode(schema, &table.name, ConstraintKind::Check);
    if mode == ConstraintMode::Ignore {
        return HashMap::new();
    }

    let mut bounds = HashMap::new();
    for constraint in &table.constraints {
        if let Constraint::Check(check) = constraint {
            let expr = check.expression.to_lowercase();
            let expr = expr.replace("check", "");
            apply_numeric_constraints(&expr, &mut bounds);
        }
    }
    bounds
}

fn apply_numeric_constraints(expr: &str, bounds: &mut HashMap<String, NumericBounds>) {
    for part in expr.split(" and ") {
        if let Some((column, min, max)) = parse_between_bounds(part) {
            update_bounds(bounds, &column, Some(min), Some(max));
            continue;
        }
        if let Some((column, op, value)) = parse_numeric_comparison(part) {
            match op.as_str() {
                ">=" => update_bounds(bounds, &column, Some(value), None),
                ">" => update_bounds(bounds, &column, Some(value + 1.0), None),
                "<=" => update_bounds(bounds, &column, None, Some(value)),
                "<" => update_bounds(bounds, &column, None, Some(value - 1.0)),
                _ => {}
            }
        }
    }
}

fn parse_between_bounds(expr: &str) -> Option<(String, f64, f64)> {
    let expr = expr.trim();
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }
    if parts[1] != "between" || parts[3] != "and" {
        return None;
    }
    let column = parts[0].trim_matches('(').trim().to_lowercase();
    let min = normalize_number(parts[2])?;
    let max = normalize_number(parts[4])?;
    Some((column, min, max))
}

fn parse_numeric_comparison(expr: &str) -> Option<(String, String, f64)> {
    let re = regex::Regex::new(r"(?i)^\s*(\w+)\s*(>=|<=|>|<)\s*([^\s]+)\s*$").ok()?;
    let caps = re.captures(expr)?;
    let column = caps[1].to_lowercase();
    let op = caps[2].to_string();
    let value = normalize_number(&caps[3])?;
    Some((column, op, value))
}

fn normalize_number(raw: &str) -> Option<f64> {
    let value = raw
        .trim()
        .trim_matches('(')
        .trim_matches(')')
        .split_once("::")
        .map(|(left, _)| left.trim())
        .unwrap_or(raw.trim());
    value.parse::<f64>().ok()
}

fn update_bounds(
    bounds: &mut HashMap<String, NumericBounds>,
    column: &str,
    min: Option<f64>,
    max: Option<f64>,
) {
    let entry = bounds.entry(column.to_string()).or_insert(NumericBounds {
        min: None,
        max: None,
    });
    if let Some(min) = min {
        entry.min = Some(entry.min.map(|v| v.max(min)).unwrap_or(min));
    }
    if let Some(max) = max {
        entry.max = Some(entry.max.map(|v| v.min(max)).unwrap_or(max));
    }
}

fn apply_numeric_bounds(value: GeneratedValue, bounds: &NumericBounds) -> GeneratedValue {
    match value {
        GeneratedValue::Int(value) => {
            let mut value = value as f64;
            if let Some(min) = bounds.min {
                value = value.max(min);
            }
            if let Some(max) = bounds.max {
                value = value.min(max);
            }
            GeneratedValue::Int(value.round() as i64)
        }
        GeneratedValue::Float(value) => {
            let mut value = value;
            if let Some(min) = bounds.min {
                value = value.max(min);
            }
            if let Some(max) = bounds.max {
                value = value.min(max);
            }
            GeneratedValue::Float(value)
        }
        other => other,
    }
}

fn generate_unique_value(
    column: &datalchemy_core::Column,
    row_index: u64,
    base_date: NaiveDate,
) -> GeneratedValue {
    let data_type = column
        .column_type
        .data_type
        .split('(')
        .next()
        .unwrap_or(&column.column_type.data_type)
        .trim()
        .to_string();

    match data_type.as_str() {
        "uuid" => {
            let value = uuid::Uuid::from_u128(row_index as u128 + 1).to_string();
            GeneratedValue::Uuid(value)
        }
        "smallint" | "integer" | "bigint" | "numeric" => GeneratedValue::Int(row_index as i64 + 1),
        "date" => {
            let date = base_date + chrono::Duration::days(row_index as i64);
            GeneratedValue::Date(date)
        }
        "timestamp with time zone" | "timestamp without time zone" => {
            let date = base_date + chrono::Duration::days(row_index as i64);
            let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
            GeneratedValue::Timestamp(NaiveDateTime::new(date, time))
        }
        _ => {
            let name = column.name.to_lowercase();
            if name.contains("email") {
                GeneratedValue::Text(format!("user{:05}@example.com", row_index + 1))
            } else if name.contains("cnpj") {
                GeneratedValue::Text(format!("{:014}", row_index + 1))
            } else if name.contains("sku") {
                GeneratedValue::Text(format!("SKU-{:05}", row_index + 1))
            } else if name.contains("codigo") {
                GeneratedValue::Text(format!("COD-{:05}", row_index + 1))
            } else {
                GeneratedValue::Text(format!("{}_{}", column.name, row_index + 1))
            }
        }
    }
}

fn generate_unique_from_rule(
    rule: &ColumnGeneratorRule,
    column: &datalchemy_core::Column,
    row_index: u64,
    base_date: NaiveDate,
) -> GeneratedValue {
    match rule.generator {
        datalchemy_plan::ColumnGenerator::Email => {
            GeneratedValue::Text(format!("user{:05}@example.com", row_index + 1))
        }
        datalchemy_plan::ColumnGenerator::Uuid => {
            let value = uuid::Uuid::from_u128(row_index as u128 + 1).to_string();
            GeneratedValue::Uuid(value)
        }
        datalchemy_plan::ColumnGenerator::Name => {
            GeneratedValue::Text(format!("Pessoa {}", row_index + 1))
        }
        datalchemy_plan::ColumnGenerator::IntRange => {
            let min = rule
                .params
                .as_ref()
                .and_then(|params| params.get("min"))
                .and_then(|value| value.as_i64())
                .unwrap_or(0);
            let max = rule
                .params
                .as_ref()
                .and_then(|params| params.get("max"))
                .and_then(|value| value.as_i64())
                .unwrap_or(i64::MAX);
            let mut value = min.saturating_add(row_index as i64 + 1);
            if value > max {
                value = max;
            }
            GeneratedValue::Int(value)
        }
        datalchemy_plan::ColumnGenerator::DateRange => {
            let date = base_date + chrono::Duration::days(row_index as i64);
            GeneratedValue::Date(date)
        }
        datalchemy_plan::ColumnGenerator::Regex => {
            GeneratedValue::Text(format!("{}_{}", column.name, row_index + 1))
        }
    }
}

fn hash_seed(seed: u64, key: &str) -> u64 {
    let mut hash = seed ^ 0xcbf29ce484222325;
    for byte in key.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn hash_row_seed(table_seed: u64, row_index: u64, attempt: u32) -> u64 {
    let mut hash = table_seed ^ row_index.wrapping_mul(0x9e3779b97f4a7c15);
    hash ^= attempt as u64;
    hash = hash.wrapping_mul(0x100000001b3);
    hash
}

fn table_key(schema: &str, table: &str) -> String {
    format!("{schema}.{table}")
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
