use std::any::Any;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rand::SeedableRng;
use rand::{Rng, RngCore};
use rand_chacha::ChaCha8Rng;
use serde_json::Value;
use tracing::{info, warn};

use datalchemy_core::{
    CheckConstraint, ColumnType, Constraint, DatabaseSchema, EnumType, ForeignKey, Table,
};
use datalchemy_plan::{
    ConstraintKind, ConstraintMode, ForeignKeyMode, GeneratorRef, Plan, Rule, TransformRule,
};

use crate::checks::{CheckContext, CheckOutcome, evaluate_check};
use crate::errors::GenerationError;
use crate::foreign::InMemoryForeignContext;
use crate::generators::{
    GeneratedValue, GeneratorContext, GeneratorRegistry, RowContext, TransformContext,
};
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
        let start = Instant::now();
        let run_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%SZ").to_string();
        let run_dir = self
            .options
            .out_dir
            .join(format!("{timestamp}__run_{run_id}"));
        std::fs::create_dir_all(&run_dir)?;

        let strict = plan
            .options
            .as_ref()
            .and_then(|opts| opts.strict)
            .unwrap_or(self.options.strict);
        let plan = normalize_plan(plan);
        let plan_index = PlanIndex::new(&plan, strict)?;
        let tasks = plan_tables(schema, &plan, self.options.auto_generate_parents)?;
        let tasks_count = tasks.len();
        let schema_index = SchemaIndex::new(schema);
        let enum_index = EnumIndex::new(schema);
        let registry = GeneratorRegistry::new();
        let mut foreign_context = InMemoryForeignContext::new();
        let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap_or_default();

        let plan_path = run_dir.join("resolved_plan.json");
        std::fs::write(&plan_path, serde_json::to_vec_pretty(&plan)?)?;

        let mut report = GenerationReport::new(run_id.clone());
        let mut bytes_written = 0_u64;
        let mut table_data: HashMap<String, TableData> = HashMap::new();

        info!(
            run_id = %run_id,
            tables = tasks_count,
            strict,
            seed = plan.seed,
            "generation started"
        );

        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || -> Result<(), GenerationError> {
                for task in tasks {
                    let schema_name = task.schema.clone();
                    let table_name = task.table.clone();
                    let table_start = Instant::now();
                    let table = schema_index
                        .table(&schema_name, &table_name)
                        .ok_or_else(|| {
                            GenerationError::InvalidPlan(format!(
                                "table '{}.{}' not found in schema",
                                schema_name, table_name
                            ))
                        })?;
                    let table_key = table_key(&schema_name, &table_name);

                    let table_ctx =
                        TableContext::new(&schema_name, table, schema, &plan_index, base_date);

                    let table_seed = hash_seed(plan.seed, &table_key);
                    info!(
                        schema = %schema_name,
                        table = %table_name,
                        rows = task.rows,
                        "generating table"
                    );

                    let result = generate_table(
                        &table_ctx,
                        &registry,
                        &enum_index,
                        &plan_index,
                        &mut foreign_context,
                        table_seed,
                        task.rows,
                        &self.options,
                        &mut table_data,
                        &mut report,
                    )?;

                    let csv_path = run_dir.join(format!("{}.{}.csv", schema_name, table_name));
                    bytes_written += write_table_csv(&csv_path, table, &result.rows)?;

                    report.tables.push(TableReport {
                        schema: schema_name.clone(),
                        table: table_name.clone(),
                        rows_requested: task.rows,
                        rows_generated: result.rows.len() as u64,
                        retries: result.retries,
                    });
                    report.retries_total += result.retries;

                    foreign_context.ingest_table(table_ctx.schema, table, &result.rows)?;
                    table_data.insert(table_key, result);

                    info!(
                        schema = %schema_name,
                        table = %table_name,
                        rows_generated = report.tables.last().map(|t| t.rows_generated).unwrap_or(0),
                        retries = report.tables.last().map(|t| t.retries).unwrap_or(0),
                        duration_ms = table_start.elapsed().as_millis() as u64,
                        "table generated"
                    );
                }

                Ok(())
            },
        ));

        let elapsed = start.elapsed();
        report.bytes_written = bytes_written;
        report.duration_ms = elapsed.as_millis() as u64;
        report.throughput_bytes_per_sec = if elapsed.as_secs_f64() > 0.0 {
            bytes_written as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let report_path = run_dir.join("generation_report.json");
        let write_report = |report: &GenerationReport| -> Result<(), GenerationError> {
            std::fs::write(&report_path, serde_json::to_vec_pretty(report)?)?;
            Ok(())
        };

        match outcome {
            Ok(Ok(())) => {
                write_report(&report)?;
                info!(
                    run_id = %run_id,
                    tables = report.tables.len(),
                    duration_ms = report.duration_ms,
                    bytes_written = report.bytes_written,
                    "generation completed"
                );
                Ok(GenerationResult { run_dir, report })
            }
            Ok(Err(err)) => {
                record_generation_failure(&mut report, err.to_string());
                write_report(&report)?;
                warn!(run_id = %run_id, error = %err, "generation failed");
                Err(err)
            }
            Err(panic) => {
                record_generation_failure(&mut report, panic_message(panic));
                write_report(&report)?;
                warn!(run_id = %run_id, "generation panicked");
                Err(GenerationError::Failed(report))
            }
        }
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
    check_constraints: Vec<&'a CheckConstraint>,
    foreign_keys: Vec<ForeignKey>,
    numeric_bounds: HashMap<String, NumericBounds>,
    current_date_columns: HashSet<String>,
    email_columns: HashSet<String>,
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
                    foreign_keys.push(fk.clone());
                }
            }
        }

        let numeric_bounds = extract_numeric_bounds(schema_name, table, plan_index);
        let current_date_columns = extract_current_date_columns(table);
        let email_columns = extract_email_columns(table);

        let _ = schema; // reserved for future schema-aware extensions

        Self {
            schema: schema_name,
            table,
            primary_keys,
            unique_constraints,
            unique_columns,
            check_constraints,
            foreign_keys,
            numeric_bounds,
            current_date_columns,
            email_columns,
            base_date,
        }
    }
}

struct ColumnRule {
    generator_id: String,
    generator_locale: Option<String>,
    params: Option<Value>,
    transforms: Vec<TransformRule>,
    input_columns: Vec<String>,
}

struct PlanIndex {
    column_rules: HashMap<String, ColumnRule>,
    constraint_policies: HashMap<String, ConstraintMode>,
    fk_strategies: HashMap<String, ForeignKeyMode>,
    allow_fk_disable: bool,
    global_locale: Option<String>,
    strict: bool,
}

fn normalize_plan(plan: &Plan) -> Plan {
    let mut plan = plan.clone();
    let mut rules = Vec::with_capacity(plan.rules.len());
    for rule in plan.rules {
        let normalized = match rule {
            Rule::ColumnGenerator(mut rule) => {
                let spec = rule.normalized_generator();
                rule.generator = GeneratorRef::Spec(spec);
                rule.params = None;
                Rule::ColumnGenerator(rule)
            }
            other => other,
        };
        rules.push(normalized);
    }
    plan.rules = rules;
    plan
}

impl PlanIndex {
    fn new(plan: &Plan, strict: bool) -> Result<Self, GenerationError> {
        let mut column_rules = HashMap::new();
        let mut constraint_policies = HashMap::new();
        let mut fk_strategies = HashMap::new();
        let global_locale = plan
            .global
            .as_ref()
            .and_then(|global| global.locale.as_ref())
            .map(|value| value.to_string());

        for rule in &plan.rules {
            match rule {
                Rule::ColumnGenerator(rule) => {
                    let key = column_key(&rule.schema, &rule.table, &rule.column);
                    let params = rule.generator_params().cloned();
                    column_rules.insert(
                        key,
                        ColumnRule {
                            generator_id: rule.generator_id().to_string(),
                            generator_locale: rule
                                .generator_locale()
                                .map(|value| value.to_string())
                                .or_else(|| global_locale.clone()),
                            params: params.clone(),
                            transforms: rule.transforms.clone(),
                            input_columns: parse_input_columns_strict(&params)?,
                        },
                    );
                }
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

        Ok(Self {
            column_rules,
            constraint_policies,
            fk_strategies,
            allow_fk_disable,
            global_locale,
            strict,
        })
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

    fn column_rule(&self, schema: &str, table: &str, column: &str) -> Option<&ColumnRule> {
        self.column_rules.get(&column_key(schema, table, column))
    }
}

fn parse_input_columns_strict(params: &Option<Value>) -> Result<Vec<String>, GenerationError> {
    let Some(params) = params else {
        return Ok(Vec::new());
    };
    let Some(value) = params.get("input_columns") else {
        return Ok(Vec::new());
    };
    let Some(array) = value.as_array() else {
        return Err(GenerationError::InvalidPlan(
            "input_columns must be an array of strings".to_string(),
        ));
    };

    let mut columns = Vec::new();
    for entry in array {
        let column = entry.as_str().ok_or_else(|| {
            GenerationError::InvalidPlan("input_columns must be strings".to_string())
        })?;
        columns.push(column.to_string());
    }

    Ok(columns)
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

struct EnumIndex {
    enums: HashMap<String, EnumType>,
}

impl EnumIndex {
    fn new(schema: &DatabaseSchema) -> Self {
        let mut enums = HashMap::new();
        for enum_type in &schema.enums {
            enums.insert(
                enum_key(&enum_type.schema, &enum_type.name),
                enum_type.clone(),
            );
        }
        Self { enums }
    }

    fn values_for(&self, column: &datalchemy_core::Column) -> Option<&[String]> {
        let key = enum_key(&column.column_type.udt_schema, &column.column_type.udt_name);
        self.enums
            .get(&key)
            .map(|enum_type| enum_type.labels.as_slice())
    }
}

fn generate_table(
    ctx: &TableContext<'_>,
    registry: &GeneratorRegistry,
    enum_index: &EnumIndex,
    plan_index: &PlanIndex,
    foreign_context: &mut InMemoryForeignContext,
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
                    apply_foreign_keys(ctx, plan_index, &mut row, &mut rng, table_data)?;
                } else if !plan_index.allow_fk_disable {
                    record_warning(
                        report,
                        GenerationIssue {
                            level: "warning".to_string(),
                            code: "fk_disable_without_flag".to_string(),
                            message: format!(
                                "foreign keys disabled for '{}.{}' without allow_fk_disable",
                                ctx.schema, ctx.table.name
                            ),
                            path: None,
                            schema: Some(ctx.schema.to_string()),
                            table: Some(ctx.table.name.clone()),
                            column: None,
                            generator_id: None,
                        },
                    );
                }

                let mut columns = ctx.table.columns.clone();
                columns.sort_by_key(|col| col.ordinal_position);

                let mut base_columns = Vec::new();
                let mut derive_columns = Vec::new();
                for column in columns {
                    let rule = plan_index.column_rule(ctx.schema, &ctx.table.name, &column.name);
                    if rule
                        .map(|rule| is_derive_generator(&rule.generator_id))
                        .unwrap_or(false)
                    {
                        derive_columns.push(column);
                    } else {
                        base_columns.push(column);
                    }
                }

                let derive_order = resolve_derive_order(ctx, plan_index, &derive_columns)?;

                for column in base_columns.iter().chain(derive_order.iter()) {
                    let key = column.name.to_lowercase();
                    if row.contains_key(&key) {
                        continue;
                    }

                    let value = generate_column_value(
                        ctx,
                        column,
                        row_index,
                        &row,
                        registry,
                        enum_index,
                        plan_index,
                        foreign_context,
                        &mut rng,
                        report,
                    )?;

                    row.insert(key.clone(), value);
                }

                apply_row_transforms(
                    ctx, &mut row, row_index, registry, plan_index, &mut rng, report,
                )?;

                if let Some(error) = enforce_not_null(ctx, &row) {
                    if row_attempts >= options.max_attempts_row {
                        if plan_index.strict {
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
                                if plan_index.strict {
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
                            if plan_index.strict {
                                return Err(GenerationError::Unsupported(
                                    "unsupported CHECK constraint".to_string(),
                                ));
                            }
                        }
                    }
                }

                if !check_uniques(&mut unique_sets, &row) {
                    if row_attempts >= options.max_attempts_row {
                        if plan_index.strict {
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
    plan_index: &PlanIndex,
    row: &mut HashMap<String, GeneratedValue>,
    rng: &mut ChaCha8Rng,
    table_data: &mut HashMap<String, TableData>,
) -> Result<(), GenerationError> {
    for fk in &ctx.foreign_keys {
        let mut skip_fk = false;
        for child_col in &fk.columns {
            let child_key = child_col.to_lowercase();
            if row.contains_key(&child_key)
                || plan_index
                    .column_rule(ctx.schema, &ctx.table.name, child_col)
                    .is_some()
            {
                skip_fk = true;
                break;
            }
        }
        if skip_fk {
            continue;
        }

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

        let index = rng.random_range(0..parent.rows.len());
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

fn is_derive_generator(generator_id: &str) -> bool {
    generator_id.starts_with("derive.")
}

fn resolve_derive_order(
    ctx: &TableContext<'_>,
    plan_index: &PlanIndex,
    derive_columns: &[datalchemy_core::Column],
) -> Result<Vec<datalchemy_core::Column>, GenerationError> {
    if derive_columns.is_empty() {
        return Ok(Vec::new());
    }

    let table_columns: HashSet<String> = ctx
        .table
        .columns
        .iter()
        .map(|col| col.name.to_lowercase())
        .collect();
    let mut derive_map = HashMap::new();
    let mut order_keys = HashMap::new();
    let mut indegree = HashMap::new();
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

    for column in derive_columns {
        let name = column.name.to_lowercase();
        derive_map.insert(name.clone(), column.clone());
        order_keys.insert(name.clone(), (column.ordinal_position, name.clone()));
        indegree.insert(name, 0_usize);
    }

    for column in derive_columns {
        let name = column.name.to_lowercase();
        let inputs = plan_index
            .column_rule(ctx.schema, &ctx.table.name, &column.name)
            .map(|rule| rule.input_columns.as_slice())
            .unwrap_or(&[]);

        for input in inputs {
            let input_lower = input.to_lowercase();
            if !table_columns.contains(&input_lower) {
                return Err(GenerationError::InvalidPlan(format!(
                    "input column '{}' not found for '{}.{}.{}'",
                    input, ctx.schema, ctx.table.name, column.name
                )));
            }
            if indegree.contains_key(&input_lower) {
                if let Some(entry) = indegree.get_mut(&name) {
                    *entry += 1;
                }
                dependents
                    .entry(input_lower)
                    .or_default()
                    .push(name.clone());
            }
        }
    }

    let mut ready = BTreeSet::new();
    for (name, degree) in &indegree {
        if *degree == 0 && let Some(key) = order_keys.get(name) {
            ready.insert(key.clone());
        }
    }

    let mut ordered = Vec::new();
    while let Some(key) = ready.iter().next().cloned() {
        ready.remove(&key);
        let name = key.1.clone();
        ordered.push(name.clone());

        if let Some(children) = dependents.get(&name) {
            for child in children {
                if let Some(entry) = indegree.get_mut(child) {
                    *entry = entry.saturating_sub(1);
                    if *entry == 0 && let Some(key) = order_keys.get(child) {
                        ready.insert(key.clone());
                    }
                }
            }
        }
    }

    if ordered.len() != derive_columns.len() {
        return Err(GenerationError::InvalidPlan(
            "cyclic derive dependencies detected".to_string(),
        ));
    }

    Ok(ordered
        .into_iter()
        .filter_map(|name| derive_map.get(&name).cloned())
        .collect())
}

fn generate_column_value(
    ctx: &TableContext<'_>,
    column: &datalchemy_core::Column,
    row_index: u64,
    row: &RowContext,
    registry: &GeneratorRegistry,
    enum_index: &EnumIndex,
    plan_index: &PlanIndex,
    foreign_context: &mut InMemoryForeignContext,
    rng: &mut ChaCha8Rng,
    report: &mut GenerationReport,
) -> Result<GeneratedValue, GenerationError> {
    let key = column.name.to_lowercase();
    let unique_hint = ctx.unique_columns.contains(&key);

    let rule = plan_index.column_rule(ctx.schema, &ctx.table.name, &column.name);
    let mut value = if let Some(rule) = rule {
        if unique_hint && !is_derive_generator(&rule.generator_id) {
            generate_unique_from_rule(rule, column, row_index, ctx.base_date)
        } else {
            generate_from_rule(
                rule,
                ctx,
                column,
                row_index,
                row,
                registry,
                enum_index,
                foreign_context,
                rng,
                report,
                plan_index,
            )?
        }
    } else if let Some(default) = generate_default(column, ctx.base_date, rng) {
        default
    } else if let Some((generator_id, value, tags)) = generate_from_default_generator(
        ctx,
        column,
        row_index,
        row,
        registry,
        enum_index,
        foreign_context,
        rng,
        plan_index.global_locale.as_deref(),
    )? {
        report.record_generator_usage(generator_id);
        record_pii_tags(report, column, tags);
        value
    } else if unique_hint {
        generate_unique_value(column, row_index, ctx.base_date)
    } else {
        generate_from_fallback(
            ctx,
            column,
            row_index,
            row,
            registry,
            enum_index,
            foreign_context,
            rng,
            report,
            plan_index,
        )?
    };

    if rule.is_none() && ctx.current_date_columns.contains(&key) {
        value = clamp_to_base_date(value, ctx.base_date);
    }

    if let Some(bounds) = ctx.numeric_bounds.get(&key) {
        value = apply_numeric_bounds(value, bounds);
    }

    Ok(value)
}

fn apply_row_transforms(
    ctx: &TableContext<'_>,
    row: &mut RowContext,
    row_index: u64,
    registry: &GeneratorRegistry,
    plan_index: &PlanIndex,
    rng: &mut ChaCha8Rng,
    report: &mut GenerationReport,
) -> Result<(), GenerationError> {
    let mut columns = ctx.table.columns.clone();
    columns.sort_by_key(|col| col.ordinal_position);

    for column in &columns {
        let Some(rule) = plan_index.column_rule(ctx.schema, &ctx.table.name, &column.name) else {
            continue;
        };
        if rule.transforms.is_empty() {
            continue;
        }

        let key = column.name.to_lowercase();
        let value = match row.get(&key).cloned() {
            Some(value) => value,
            None => continue,
        };
        let next = apply_transforms(
            rule, value, ctx, column, row_index, registry, rng, report, plan_index,
        )?;
        row.insert(key, next);
    }

    Ok(())
}

fn generate_from_rule(
    rule: &ColumnRule,
    ctx: &TableContext<'_>,
    column: &datalchemy_core::Column,
    row_index: u64,
    row: &RowContext,
    registry: &GeneratorRegistry,
    enum_index: &EnumIndex,
    foreign_context: &mut InMemoryForeignContext,
    rng: &mut ChaCha8Rng,
    report: &mut GenerationReport,
    _plan_index: &PlanIndex,
) -> Result<GeneratedValue, GenerationError> {
    let generator_id = rule.generator_id.as_str();
    let generator = match registry.generator(generator_id) {
        Some(generator) => generator,
        None => {
            report.record_unknown_generator();
            let issue = issue_for_column(
                "unknown_generator_id",
                format!(
                    "unknown generator id '{}' for '{}.{}.{}'",
                    generator_id, ctx.schema, ctx.table.name, column.name
                ),
                ctx,
                column,
                Some(generator_id),
            );
            record_warning(report, issue);
            return Err(GenerationError::InvalidPlan(format!(
                "unknown generator id '{}'",
                generator_id
            )));
        }
    };

    let mut generator_ctx = GeneratorContext {
        schema: ctx.schema,
        table: &ctx.table.name,
        column,
        foreign_keys: &ctx.foreign_keys,
        base_date: ctx.base_date,
        row_index,
        enum_values: enum_index.values_for(column),
        row,
        foreign: Some(foreign_context),
        generator_locale: rule.generator_locale.as_deref(),
    };

    let value = match generator.generate(&mut generator_ctx, rule.params.as_ref(), rng) {
        Ok(value) => value,
        Err(err) => {
            let issue = issue_for_column(
                "invalid_generator_params",
                format!("invalid generator params for '{}': {}", generator_id, err),
                ctx,
                column,
                Some(generator_id),
            );
            record_warning(report, issue);
            return Err(err);
        }
    };

    report.record_generator_usage(generator_id);
    record_pii_tags(report, column, generator.pii_tags());

    Ok(value)
}

fn generate_from_fallback(
    ctx: &TableContext<'_>,
    column: &datalchemy_core::Column,
    row_index: u64,
    row: &RowContext,
    registry: &GeneratorRegistry,
    enum_index: &EnumIndex,
    foreign_context: &mut InMemoryForeignContext,
    rng: &mut ChaCha8Rng,
    report: &mut GenerationReport,
    plan_index: &PlanIndex,
) -> Result<GeneratedValue, GenerationError> {
    if plan_index.strict {
        return Err(GenerationError::Unsupported(format!(
            "fallback generation forbidden in strict mode for '{}.{}.{}'",
            ctx.schema, ctx.table.name, column.name
        )));
    }

    if let Some((generator_id, value, tags)) = generate_from_default_generator(
        ctx,
        column,
        row_index,
        row,
        registry,
        enum_index,
        foreign_context,
        rng,
        plan_index.global_locale.as_deref(),
    )? {
        record_fallback_warning(report, ctx, column, Some(generator_id));
        report.record_generator_usage(generator_id);
        record_pii_tags(report, column, tags);
        return Ok(value);
    }

    record_fallback_warning(report, ctx, column, None);
    let value = fallback_for_type(column, ctx.base_date, rng);
    record_pii_tags(report, column, &[]);
    Ok(value)
}

fn apply_transforms(
    rule: &ColumnRule,
    mut value: GeneratedValue,
    ctx: &TableContext<'_>,
    column: &datalchemy_core::Column,
    row_index: u64,
    registry: &GeneratorRegistry,
    rng: &mut ChaCha8Rng,
    report: &mut GenerationReport,
    plan_index: &PlanIndex,
) -> Result<GeneratedValue, GenerationError> {
    for transform_rule in &rule.transforms {
        let transform_id = transform_rule.transform.as_str();
        let transform = match registry.transform(transform_id) {
            Some(transform) => transform,
            None => {
                let issue = issue_for_column(
                    "unknown_transform_id",
                    format!(
                        "unknown transform id '{}' for '{}.{}.{}'",
                        transform_id, ctx.schema, ctx.table.name, column.name
                    ),
                    ctx,
                    column,
                    Some(&rule.generator_id),
                );
                record_warning(report, issue);

                if plan_index.strict {
                    return Err(GenerationError::InvalidPlan(format!(
                        "unknown transform id '{}'",
                        transform_id
                    )));
                }
                continue;
            }
        };

        let transform_ctx = TransformContext {
            schema: ctx.schema,
            table: &ctx.table.name,
            column,
            base_date: ctx.base_date,
            row_index,
            strict: plan_index.strict,
        };

        match transform.apply(
            value.clone(),
            &transform_ctx,
            transform_rule.params.as_ref(),
            rng,
        ) {
            Ok(next) => {
                value = next;
                report.record_transform_usage(transform_id);
            }
            Err(err) => {
                if plan_index.strict {
                    return Err(err);
                }
                let issue = issue_for_column(
                    "invalid_transform_params",
                    format!("invalid transform params for '{}': {}", transform_id, err),
                    ctx,
                    column,
                    Some(&rule.generator_id),
                );
                record_warning(report, issue);
            }
        }
    }

    Ok(value)
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
            let bytes: [u8; 16] = rng.random();
            Some(GeneratedValue::Uuid(
                uuid::Uuid::from_bytes(bytes).to_string(),
            ))
        }
        "now()" | "current_timestamp" => {
            let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_default();
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

fn generate_from_default_generator(
    ctx: &TableContext<'_>,
    column: &datalchemy_core::Column,
    row_index: u64,
    row: &RowContext,
    registry: &GeneratorRegistry,
    enum_index: &EnumIndex,
    foreign_context: &mut InMemoryForeignContext,
    rng: &mut ChaCha8Rng,
    locale: Option<&str>,
) -> Result<Option<(&'static str, GeneratedValue, &'static [&'static str])>, GenerationError> {
    let generator_id = default_generator_id_for_column(ctx, column, enum_index);
    let Some(generator) = registry.generator(generator_id) else {
        return Ok(None);
    };
    let mut generator_ctx = GeneratorContext {
        schema: ctx.schema,
        table: &ctx.table.name,
        column,
        foreign_keys: &ctx.foreign_keys,
        base_date: ctx.base_date,
        row_index,
        enum_values: enum_index.values_for(column),
        row,
        foreign: Some(foreign_context),
        generator_locale: locale,
    };
    let value = generator.generate(&mut generator_ctx, None, rng)?;
    Ok(Some((generator_id, value, generator.pii_tags())))
}

fn default_generator_id_for_column(
    ctx: &TableContext<'_>,
    column: &datalchemy_core::Column,
    enum_index: &EnumIndex,
) -> &'static str {
    if enum_index.values_for(column).is_some() {
        return "primitive.enum";
    }
    if ctx.email_columns.contains(&column.name.to_lowercase()) {
        return "semantic.person.email";
    }
    let data_type = normalize_type(&column.column_type).to_lowercase();
    match data_type.as_str() {
        "uuid" => "primitive.uuid",
        "smallint" | "integer" | "bigint" => "primitive.int",
        "numeric" | "decimal" => "primitive.decimal.numeric",
        "real" | "double precision" => "primitive.float",
        "boolean" => "primitive.bool",
        "date" => "primitive.date",
        "time with time zone" | "time without time zone" => "primitive.time",
        "timestamp with time zone" | "timestamp without time zone" => "primitive.timestamp",
        "character varying" | "character" | "varchar" | "bpchar" | "text" => "primitive.text",
        _ => "primitive.text",
    }
}

fn fallback_for_type(
    column: &datalchemy_core::Column,
    base_date: NaiveDate,
    rng: &mut ChaCha8Rng,
) -> GeneratedValue {
    let data_type = normalize_type(&column.column_type);
    match data_type.as_str() {
        "uuid" => GeneratedValue::Uuid(random_uuid(rng)),
        "smallint" | "integer" | "bigint" => {
            let value = rng.random_range(1..=100000);
            GeneratedValue::Int(value)
        }
        "numeric" => {
            if column.column_type.numeric_scale.unwrap_or(0) > 0 {
                let value = rng.random_range(0.0..=100000.0);
                GeneratedValue::Float(value)
            } else {
                let value = rng.random_range(1..=100000);
                GeneratedValue::Int(value)
            }
        }
        "boolean" => GeneratedValue::Bool(rng.random_bool(0.5)),
        "date" => {
            let offset = rng.random_range(0..=365) as i64;
            GeneratedValue::Date(base_date + chrono::Duration::days(offset))
        }
        "timestamp with time zone" | "timestamp without time zone" => {
            let offset = rng.random_range(0..=365) as i64;
            let date = base_date + chrono::Duration::days(offset);
            let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_default();
            GeneratedValue::Timestamp(NaiveDateTime::new(date, time))
        }
        "time with time zone" | "time without time zone" => {
            let seconds = rng.random_range(0..=86399);
            let time = safe_time_from_seconds(seconds);
            GeneratedValue::Time(time)
        }
        _ => {
            let mut value = format!("{}_{}", column.name, rng.random::<u32>());
            if let Some(max_len) = column.column_type.character_max_length {
                value.truncate(max_len as usize);
            }
            GeneratedValue::Text(value)
        }
    }
}

fn normalize_type(column_type: &ColumnType) -> String {
    column_type
        .data_type
        .split('(')
        .next()
        .unwrap_or(&column_type.data_type)
        .trim()
        .to_string()
}

fn random_uuid(rng: &mut ChaCha8Rng) -> String {
    let mut bytes = [0_u8; 16];
    rng.fill_bytes(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes).to_string()
}

fn safe_time_from_seconds(seconds: u32) -> NaiveTime {
    NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0).unwrap_or_default()
}

fn record_pii_tags(
    report: &mut GenerationReport,
    column: &datalchemy_core::Column,
    generator_tags: &'static [&'static str],
) {
    let mut tags = BTreeSet::new();
    for tag in generator_tags {
        tags.insert(*tag);
    }
    for tag in column_pii_tags(&column.name) {
        tags.insert(tag);
    }
    for tag in tags {
        report.record_pii(tag);
    }
}

fn column_pii_tags(column_name: &str) -> Vec<&'static str> {
    let name = column_name.to_lowercase();
    let mut tags = Vec::new();
    if name.contains("email") {
        tags.push("pii.email");
    }
    if name.contains("cpf") {
        tags.push("pii.cpf");
    }
    if name.contains("cnpj") {
        tags.push("pii.cnpj");
    }
    if name.contains("rg") {
        tags.push("pii.rg");
    }
    if name.contains("telefone") || name.contains("phone") {
        tags.push("pii.phone");
    }
    if name.contains("nome") || name.contains("name") {
        tags.push("pii.name");
    }
    if name.contains("endereco") || name.contains("logradouro") {
        tags.push("pii.address");
    }
    if name.contains("cep") || name.contains("cidade") || name == "uf" {
        tags.push("pii.location");
    }
    if name.contains("ip") || name.contains("url") {
        tags.push("pii.network");
    }
    tags
}

fn record_fallback_warning(
    report: &mut GenerationReport,
    ctx: &TableContext<'_>,
    column: &datalchemy_core::Column,
    generator_id: Option<&str>,
) {
    report.record_fallback();
    let issue = issue_for_column(
        "fallback_used",
        format!(
            "fallback used for '{}.{}.{}'",
            ctx.schema, ctx.table.name, column.name
        ),
        ctx,
        column,
        generator_id,
    );
    record_warning(report, issue);
}

fn issue_for_column(
    code: &str,
    message: String,
    ctx: &TableContext<'_>,
    column: &datalchemy_core::Column,
    generator_id: Option<&str>,
) -> GenerationIssue {
    GenerationIssue {
        level: "warning".to_string(),
        code: code.to_string(),
        message,
        path: Some(format!("{}.{}.{}", ctx.schema, ctx.table.name, column.name)),
        schema: Some(ctx.schema.to_string()),
        table: Some(ctx.table.name.clone()),
        column: Some(column.name.clone()),
        generator_id: generator_id.map(|value| value.to_string()),
    }
}

fn record_warning(report: &mut GenerationReport, issue: GenerationIssue) {
    log_issue(&issue);
    report.record_warning(issue);
}

fn record_unsupported(report: &mut GenerationReport, issue: GenerationIssue) {
    log_issue(&issue);
    report.record_unsupported(issue);
}

fn record_generation_failure(report: &mut GenerationReport, message: String) {
    let issue = GenerationIssue {
        level: "error".to_string(),
        code: "generation_failed".to_string(),
        message,
        path: None,
        schema: None,
        table: None,
        column: None,
        generator_id: None,
    };
    record_unsupported(report, issue);
}

fn panic_message(panic: Box<dyn Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        "panic during generation".to_string()
    }
}

fn log_issue(issue: &GenerationIssue) {
    warn!(
        code = %issue.code,
        schema = issue.schema.as_deref().unwrap_or(""),
        table = issue.table.as_deref().unwrap_or(""),
        column = issue.column.as_deref().unwrap_or(""),
        generator_id = issue.generator_id.as_deref().unwrap_or(""),
        message = %issue.message
    );
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
                    record_warning(
                        report,
                        GenerationIssue {
                            level: "warning".to_string(),
                            code: "check_failed".to_string(),
                            message: format!(
                                "check constraint failed on '{}.{}'",
                                ctx.schema, ctx.table.name
                            ),
                            path: None,
                            schema: Some(ctx.schema.to_string()),
                            table: Some(ctx.table.name.clone()),
                            column: None,
                            generator_id: None,
                        },
                    );
                    continue;
                }
                outcome = CheckOutcome::Failed;
            }
            CheckOutcome::Unsupported => {
                record_unsupported(
                    report,
                    GenerationIssue {
                        level: "warning".to_string(),
                        code: "check_unsupported".to_string(),
                        message: format!(
                            "unsupported CHECK constraint on '{}.{}'",
                            ctx.schema, ctx.table.name
                        ),
                        path: None,
                        schema: Some(ctx.schema.to_string()),
                        table: Some(ctx.table.name.clone()),
                        column: None,
                        generator_id: None,
                    },
                );
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

fn extract_current_date_columns(table: &Table) -> HashSet<String> {
    let mut columns = HashSet::new();
    let re_column = regex::Regex::new(r"(?i)(\w+)\s*(<=|>=|<|>)\s*current_date").ok();
    let re_reverse = regex::Regex::new(r"(?i)current_date\s*(<=|>=|<|>)\s*(\w+)").ok();

    for constraint in &table.constraints {
        let Constraint::Check(check) = constraint else {
            continue;
        };
        if let Some(re_column) = &re_column {
            for caps in re_column.captures_iter(&check.expression) {
                columns.insert(caps[1].to_lowercase());
            }
        }
        if let Some(re_reverse) = &re_reverse {
            for caps in re_reverse.captures_iter(&check.expression) {
                columns.insert(caps[2].to_lowercase());
            }
        }
    }

    columns
}

fn extract_email_columns(table: &Table) -> HashSet<String> {
    let mut columns = HashSet::new();
    let re_position = regex::Regex::new(
        r"(?i)position\(\(?\s*'\s*[^']*\s*'(?:::text)?\s*\)?\s+in\s+\(?\s*(\w+)\s*\)?\s*\)",
    )
    .ok();

    for constraint in &table.constraints {
        let Constraint::Check(check) = constraint else {
            continue;
        };
        if let Some(re_position) = &re_position {
            for caps in re_position.captures_iter(&check.expression) {
                columns.insert(caps[1].to_lowercase());
            }
        }
    }

    columns
}

fn clamp_to_base_date(value: GeneratedValue, base_date: NaiveDate) -> GeneratedValue {
    match value {
        GeneratedValue::Date(_) => GeneratedValue::Date(base_date),
        GeneratedValue::Timestamp(_) => {
            let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_default();
            GeneratedValue::Timestamp(NaiveDateTime::new(base_date, time))
        }
        other => other,
    }
}

fn generate_unique_value(
    column: &datalchemy_core::Column,
    row_index: u64,
    base_date: NaiveDate,
) -> GeneratedValue {
    let data_type = normalize_type(&column.column_type);

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
            let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_default();
            GeneratedValue::Timestamp(NaiveDateTime::new(date, time))
        }
        "time with time zone" | "time without time zone" => {
            let seconds = (row_index % 86400) as u32;
            let time = safe_time_from_seconds(seconds);
            GeneratedValue::Time(time)
        }
        _ => {
            let mut value = format!("value_{:05}", row_index + 1);
            if let Some(max_len) = column.column_type.character_max_length {
                value.truncate(max_len as usize);
            }
            GeneratedValue::Text(value)
        }
    }
}

fn generate_unique_from_rule(
    rule: &ColumnRule,
    column: &datalchemy_core::Column,
    row_index: u64,
    base_date: NaiveDate,
) -> GeneratedValue {
    match rule.generator_id.as_str() {
        "semantic.br.email.safe"
        | "semantic.person.email"
        | "faker.internet.raw.SafeEmail"
        | "faker.internet.raw.FreeEmail" => {
            GeneratedValue::Text(format!("user{:05}@example.com", row_index + 1))
        }
        "primitive.uuid" | "primitive.uuid.v4" => {
            let value = uuid::Uuid::from_u128(row_index as u128 + 1).to_string();
            GeneratedValue::Uuid(value)
        }
        "semantic.br.name" => GeneratedValue::Text(format!("Pessoa {}", row_index + 1)),
        "primitive.int" | "primitive.int.range" | "primitive.int.sequence_hint" => {
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
        "primitive.float" | "primitive.float.range" | "primitive.decimal.numeric" => {
            let min = rule
                .params
                .as_ref()
                .and_then(|params| params.get("min"))
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0);
            let max = rule
                .params
                .as_ref()
                .and_then(|params| params.get("max"))
                .and_then(|value| value.as_f64())
                .unwrap_or(f64::MAX);
            let mut value = min + row_index as f64 + 1.0;
            if value > max {
                value = max;
            }
            if rule.generator_id.as_str() == "primitive.decimal.numeric" {
                let scale = rule
                    .params
                    .as_ref()
                    .and_then(|params| params.get("scale"))
                    .and_then(|value| value.as_i64())
                    .unwrap_or(2)
                    .max(0) as i32;
                let factor = 10_f64.powi(scale);
                value = (value * factor).round() / factor;
            }
            GeneratedValue::Float(value)
        }
        "primitive.date"
        | "primitive.date.range"
        | "primitive.timestamp"
        | "primitive.timestamp.range" => {
            let date = base_date + chrono::Duration::days(row_index as i64);
            if rule.generator_id.as_str() == "primitive.timestamp"
                || rule.generator_id.as_str() == "primitive.timestamp.range"
            {
                let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_default();
                GeneratedValue::Timestamp(NaiveDateTime::new(date, time))
            } else {
                GeneratedValue::Date(date)
            }
        }
        "primitive.time" | "primitive.time.range" => {
            let seconds = (row_index % 86400) as u32;
            let time = safe_time_from_seconds(seconds);
            GeneratedValue::Time(time)
        }
        "semantic.br.cpf" => GeneratedValue::Text(format!("{:011}", row_index + 1)),
        "semantic.br.cnpj" => GeneratedValue::Text(format!("{:014}", row_index + 1)),
        "primitive.text" | "primitive.text.pattern" | "primitive.text.lorem" => {
            GeneratedValue::Text(format!("{}_{}", column.name, row_index + 1))
        }
        _ => generate_unique_value(column, row_index, base_date),
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

fn column_key(schema: &str, table: &str, column: &str) -> String {
    format!("{schema}.{table}.{column}")
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

fn enum_key(schema: &str, name: &str) -> String {
    format!("{schema}.{name}")
}
