use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rand::SeedableRng;
use rand::{Rng, RngCore};
use rand_chacha::ChaCha8Rng;
use serde_json::Value;
use tracing::warn;

use datalchemy_core::{
    CheckConstraint, ColumnType, Constraint, DatabaseSchema, EnumType, ForeignKey, Table,
};
use datalchemy_plan::{ConstraintKind, ConstraintMode, ForeignKeyMode, Plan, Rule, TransformRule};

use crate::checks::{CheckContext, CheckOutcome, evaluate_check};
use crate::errors::GenerationError;
use crate::generators::{GeneratedValue, GeneratorContext, GeneratorRegistry, TransformContext};
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

        let strict = plan
            .options
            .as_ref()
            .and_then(|opts| opts.strict)
            .unwrap_or(self.options.strict);
        let plan_index = PlanIndex::new(plan, strict);
        let tasks = plan_tables(schema, plan, self.options.auto_generate_parents)?;
        let schema_index = SchemaIndex::new(schema);
        let enum_index = EnumIndex::new(schema);
        let registry = GeneratorRegistry::new();
        let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap_or_else(NaiveDate::default);

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
                &registry,
                &enum_index,
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

struct ColumnRule {
    generator_id: String,
    params: Option<Value>,
    transforms: Vec<TransformRule>,
}

struct PlanIndex {
    column_rules: HashMap<String, ColumnRule>,
    constraint_policies: HashMap<String, ConstraintMode>,
    fk_strategies: HashMap<String, ForeignKeyMode>,
    allow_fk_disable: bool,
    strict: bool,
}

impl PlanIndex {
    fn new(plan: &Plan, strict: bool) -> Self {
        let mut column_rules = HashMap::new();
        let mut constraint_policies = HashMap::new();
        let mut fk_strategies = HashMap::new();

        for rule in &plan.rules {
            match rule {
                Rule::ColumnGenerator(rule) => {
                    let key = column_key(&rule.schema, &rule.table, &rule.column);
                    column_rules.insert(
                        key,
                        ColumnRule {
                            generator_id: rule.generator.clone(),
                            params: rule.params.clone(),
                            transforms: rule.transforms.clone(),
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

        Self {
            column_rules,
            constraint_policies,
            fk_strategies,
            allow_fk_disable,
            strict,
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

    fn column_rule(&self, schema: &str, table: &str, column: &str) -> Option<&ColumnRule> {
        self.column_rules.get(&column_key(schema, table, column))
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

                for column in columns {
                    let key = column.name.to_lowercase();
                    if row.contains_key(&key) {
                        continue;
                    }

                    let rule = plan_index.column_rule(ctx.schema, &ctx.table.name, &column.name);
                    let is_unique =
                        ctx.unique_columns.contains(&key) && !ctx.fk_columns.contains(&key);

                    let mut value = if let Some(rule) = rule {
                        if is_unique {
                            if registry.generator(&rule.generator_id).is_none() {
                                report.record_unknown_generator();
                                let issue = issue_for_column(
                                    "unknown_generator_id",
                                    format!(
                                        "unknown generator id '{}' for '{}.{}.{}'",
                                        rule.generator_id, ctx.schema, ctx.table.name, column.name
                                    ),
                                    ctx,
                                    &column,
                                    Some(&rule.generator_id),
                                );
                                record_warning(report, issue);
                                if plan_index.strict {
                                    return Err(GenerationError::InvalidPlan(format!(
                                        "unknown generator id '{}'",
                                        rule.generator_id
                                    )));
                                }
                                let value =
                                    generate_unique_value(&column, row_index, ctx.base_date);
                                record_pii_tags(report, &column, &[]);
                                value
                            } else {
                                let value = generate_unique_from_rule(
                                    rule,
                                    &column,
                                    row_index,
                                    ctx.base_date,
                                );
                                report.record_generator_usage(&rule.generator_id);
                                record_pii_tags(
                                    report,
                                    &column,
                                    pii_tags_for_generator_id(&rule.generator_id),
                                );
                                value
                            }
                        } else {
                            generate_from_rule(
                                rule, ctx, &column, row_index, registry, enum_index, &mut rng,
                                report, plan_index,
                            )?
                        }
                    } else if is_unique {
                        let value = generate_unique_value(&column, row_index, ctx.base_date);
                        record_pii_tags(report, &column, &[]);
                        value
                    } else if let Some(default_value) =
                        generate_default(&column, ctx.base_date, &mut rng)
                    {
                        record_pii_tags(report, &column, &[]);
                        default_value
                    } else {
                        if column.is_nullable && rng.gen_bool(0.1) {
                            row.insert(key.clone(), GeneratedValue::Null);
                            continue;
                        }
                        generate_from_fallback(
                            ctx, &column, row_index, registry, enum_index, &mut rng, report,
                            plan_index,
                        )?
                    };

                    if let Some(rule) = rule {
                        value = apply_transforms(
                            rule, value, ctx, &column, row_index, registry, &mut rng, report,
                            plan_index,
                        )?;
                    }

                    if let Some(bounds) = ctx.numeric_bounds.get(&key) {
                        value = apply_numeric_bounds(value, bounds);
                    }

                    row.insert(key.clone(), value);
                }

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

fn generate_from_rule(
    rule: &ColumnRule,
    ctx: &TableContext<'_>,
    column: &datalchemy_core::Column,
    row_index: u64,
    registry: &GeneratorRegistry,
    enum_index: &EnumIndex,
    rng: &mut ChaCha8Rng,
    report: &mut GenerationReport,
    plan_index: &PlanIndex,
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

            if plan_index.strict {
                return Err(GenerationError::InvalidPlan(format!(
                    "unknown generator id '{}'",
                    generator_id
                )));
            }

            return generate_from_fallback(
                ctx, column, row_index, registry, enum_index, rng, report, plan_index,
            );
        }
    };

    let generator_ctx = GeneratorContext {
        schema: ctx.schema,
        table: &ctx.table.name,
        column,
        base_date: ctx.base_date,
        row_index,
        enum_values: enum_index.values_for(column),
    };

    let value = match generator.generate(&generator_ctx, rule.params.as_ref(), rng) {
        Ok(value) => value,
        Err(err) => {
            if plan_index.strict {
                return Err(err);
            }
            let issue = issue_for_column(
                "invalid_generator_params",
                format!("invalid generator params for '{}': {}", generator_id, err),
                ctx,
                column,
                Some(generator_id),
            );
            record_warning(report, issue);
            match generator.generate(&generator_ctx, None, rng) {
                Ok(value) => value,
                Err(_) => {
                    record_fallback_warning(report, ctx, column, Some(generator_id));
                    return generate_from_fallback(
                        ctx, column, row_index, registry, enum_index, rng, report, plan_index,
                    );
                }
            }
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
    registry: &GeneratorRegistry,
    enum_index: &EnumIndex,
    rng: &mut ChaCha8Rng,
    report: &mut GenerationReport,
    plan_index: &PlanIndex,
) -> Result<GeneratedValue, GenerationError> {
    if let Some(generator) = registry.generator("primitive.enum") {
        if enum_index.values_for(column).is_some() {
            let generator_ctx = GeneratorContext {
                schema: ctx.schema,
                table: &ctx.table.name,
                column,
                base_date: ctx.base_date,
                row_index,
                enum_values: enum_index.values_for(column),
            };
            let value = generator.generate(&generator_ctx, None, rng)?;
            report.record_generator_usage("primitive.enum");
            record_pii_tags(report, column, generator.pii_tags());
            return Ok(value);
        }
    }

    if let Some((generator_id, value, tags)) =
        generate_with_heuristic(ctx, column, row_index, registry, enum_index, rng)?
    {
        if plan_index.strict {
            return Err(GenerationError::Unsupported(format!(
                "heuristic generation forbidden in strict mode for '{}.{}.{}'",
                ctx.schema, ctx.table.name, column.name
            )));
        }
        report.record_heuristic();
        if let Some(generator_id) = generator_id {
            report.record_generator_usage(generator_id);
        }
        record_pii_tags(report, column, tags);
        let issue = issue_for_column(
            "heuristic_used",
            format!(
                "heuristic generator used for '{}.{}.{}'",
                ctx.schema, ctx.table.name, column.name
            ),
            ctx,
            column,
            generator_id,
        );
        record_warning(report, issue);
        return Ok(value);
    }

    if plan_index.strict {
        return Err(GenerationError::Unsupported(format!(
            "fallback generation forbidden in strict mode for '{}.{}.{}'",
            ctx.schema, ctx.table.name, column.name
        )));
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
            let bytes: [u8; 16] = rng.r#gen();
            Some(GeneratedValue::Uuid(
                uuid::Uuid::from_bytes(bytes).to_string(),
            ))
        }
        "now()" | "current_timestamp" => {
            let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_else(NaiveTime::default);
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

fn generate_with_heuristic(
    ctx: &TableContext<'_>,
    column: &datalchemy_core::Column,
    row_index: u64,
    registry: &GeneratorRegistry,
    enum_index: &EnumIndex,
    rng: &mut ChaCha8Rng,
) -> Result<
    Option<(
        Option<&'static str>,
        GeneratedValue,
        &'static [&'static str],
    )>,
    GenerationError,
> {
    if let Some(value) = heuristic_inline_value(column, rng) {
        return Ok(Some((Some("heuristic.tipo"), value, EMPTY_TAGS)));
    }

    if let Some(generator_id) = heuristic_generator_id(column) {
        if let Some(generator) = registry.generator(generator_id) {
            let generator_ctx = GeneratorContext {
                schema: ctx.schema,
                table: &ctx.table.name,
                column,
                base_date: ctx.base_date,
                row_index,
                enum_values: enum_index.values_for(column),
            };
            let value = generator.generate(&generator_ctx, None, rng)?;
            return Ok(Some((Some(generator_id), value, generator.pii_tags())));
        }
    }

    Ok(None)
}

fn heuristic_generator_id(column: &datalchemy_core::Column) -> Option<&'static str> {
    let name = column.name.to_lowercase();
    if name == "id" && normalize_type(&column.column_type) == "uuid" {
        return Some("primitive.uuid.v4");
    }
    if name.contains("email") {
        return Some("semantic.br.email.safe");
    }
    if name.contains("nome") || name.contains("name") {
        return Some("semantic.br.name");
    }
    if name.contains("cpf") {
        return Some("semantic.br.cpf");
    }
    if name.contains("cnpj") {
        return Some("semantic.br.cnpj");
    }
    if name.contains("telefone") || name.contains("phone") {
        return Some("semantic.br.phone");
    }
    if name.contains("cep") {
        return Some("semantic.br.cep");
    }
    if name.contains("cidade") || name.contains("city") {
        return Some("semantic.br.city");
    }
    if name == "uf" || name.contains("estado") {
        return Some("semantic.br.uf");
    }
    if name.contains("endereco") || name.contains("logradouro") {
        return Some("semantic.br.address");
    }
    if name.contains("ip") {
        return Some("semantic.br.ip");
    }
    if name.contains("url") {
        return Some("semantic.br.url");
    }
    None
}

fn heuristic_inline_value(
    column: &datalchemy_core::Column,
    rng: &mut ChaCha8Rng,
) -> Option<GeneratedValue> {
    let name = column.name.to_lowercase();
    if name == "tipo" {
        let values = ["tarefa", "reuniao", "anotacao"];
        let value = values[rng.gen_range(0..values.len())];
        return Some(GeneratedValue::Text(value.to_string()));
    }
    None
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
            let value = rng.gen_range(1..=100000);
            GeneratedValue::Int(value)
        }
        "numeric" => {
            if column.column_type.numeric_scale.unwrap_or(0) > 0 {
                let value = rng.gen_range(0.0..=100000.0);
                GeneratedValue::Float(value)
            } else {
                let value = rng.gen_range(1..=100000);
                GeneratedValue::Int(value)
            }
        }
        "boolean" => GeneratedValue::Bool(rng.gen_bool(0.5)),
        "date" => {
            let offset = rng.gen_range(0..=365) as i64;
            GeneratedValue::Date(base_date + chrono::Duration::days(offset))
        }
        "timestamp with time zone" | "timestamp without time zone" => {
            let offset = rng.gen_range(0..=365) as i64;
            let date = base_date + chrono::Duration::days(offset);
            let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_else(NaiveTime::default);
            GeneratedValue::Timestamp(NaiveDateTime::new(date, time))
        }
        "time with time zone" | "time without time zone" => {
            let seconds = rng.gen_range(0..=86399);
            let time = safe_time_from_seconds(seconds);
            GeneratedValue::Time(time)
        }
        _ => {
            let mut value = format!("{}_{}", column.name, rng.r#gen::<u32>());
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
    NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0).unwrap_or_else(NaiveTime::default)
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

fn pii_tags_for_generator_id(generator_id: &str) -> &'static [&'static str] {
    match generator_id {
        "semantic.br.name" => &["pii.name"],
        "semantic.br.email.safe" => &["pii.email"],
        "semantic.br.phone" => &["pii.phone"],
        "semantic.br.cpf" => &["pii.cpf"],
        "semantic.br.cnpj" => &["pii.cnpj"],
        "semantic.br.rg" => &["pii.rg"],
        "semantic.br.cep" | "semantic.br.uf" | "semantic.br.city" => &["pii.location"],
        "semantic.br.address" => &["pii.address"],
        "semantic.br.ip" | "semantic.br.url" => &["pii.network"],
        _ => EMPTY_TAGS,
    }
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

const EMPTY_TAGS: &[&str] = &[];

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
            let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_else(NaiveTime::default);
            GeneratedValue::Timestamp(NaiveDateTime::new(date, time))
        }
        "time with time zone" | "time without time zone" => {
            let seconds = (row_index % 86400) as u32;
            let time = safe_time_from_seconds(seconds);
            GeneratedValue::Time(time)
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
    rule: &ColumnRule,
    column: &datalchemy_core::Column,
    row_index: u64,
    base_date: NaiveDate,
) -> GeneratedValue {
    match rule.generator_id.as_str() {
        "semantic.br.email.safe" => {
            GeneratedValue::Text(format!("user{:05}@example.com", row_index + 1))
        }
        "primitive.uuid.v4" => {
            let value = uuid::Uuid::from_u128(row_index as u128 + 1).to_string();
            GeneratedValue::Uuid(value)
        }
        "semantic.br.name" => GeneratedValue::Text(format!("Pessoa {}", row_index + 1)),
        "primitive.int.range" | "primitive.int.sequence_hint" => {
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
        "primitive.date.range" | "primitive.timestamp.range" => {
            let date = base_date + chrono::Duration::days(row_index as i64);
            if rule.generator_id.as_str() == "primitive.timestamp.range" {
                let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_else(NaiveTime::default);
                GeneratedValue::Timestamp(NaiveDateTime::new(date, time))
            } else {
                GeneratedValue::Date(date)
            }
        }
        "semantic.br.cpf" => GeneratedValue::Text(format!("{:011}", row_index + 1)),
        "semantic.br.cnpj" => GeneratedValue::Text(format!("{:014}", row_index + 1)),
        "primitive.text.pattern" | "primitive.text.lorem" => {
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
