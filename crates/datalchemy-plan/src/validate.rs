use std::collections::{HashMap, HashSet};

use datalchemy_core::{ColumnType, Constraint, DatabaseSchema};
use jsonschema::JSONSchema;
use serde_json::Value;

use crate::errors::{IssueSeverity, PlanError, ValidationIssue, ValidationReport};
use crate::model::{
    ConstraintKind, ConstraintMode, ConstraintPolicyRule, ForeignKeyMode, ForeignKeyStrategyRule,
    Plan, Rule, Target, UnsupportedRule,
};

/// Validated plan with accumulated warnings.
#[derive(Debug, Clone)]
pub struct ValidatedPlan {
    pub plan: Plan,
    pub warnings: Vec<ValidationIssue>,
}

/// Validate a plan JSON document against the plan JSON Schema.
pub fn validate_plan_json(
    plan_json: &Value,
    plan_schema: &Value,
) -> Result<ValidationReport, PlanError> {
    let compiled =
        JSONSchema::compile(plan_schema).map_err(|err| PlanError::Schema(err.to_string()))?;

    let mut report = ValidationReport::default();

    if let Err(errors) = compiled.validate(plan_json) {
        for error in errors {
            let path = normalized_json_pointer(&error.instance_path.to_string());
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "schema_violation",
                path,
                error.to_string(),
                None,
            ));
        }
    }

    Ok(report)
}

/// Validate a parsed plan against a database schema snapshot.
pub fn validate_plan_against_schema(plan: &Plan, schema: &DatabaseSchema) -> ValidationReport {
    let mut report = ValidationReport::default();

    validate_schema_ref(plan, schema, &mut report);

    let schema_index = build_schema_index(schema);
    validate_targets(&plan.targets, &schema_index, &mut report);
    validate_rules(plan, &schema_index, &mut report);
    validate_unsupported(&plan.rules_unsupported, &schema_index, &mut report);

    report
}

/// Validate the plan end-to-end, returning structured issues on failure.
pub fn validate_plan(
    plan_json: &Value,
    plan_schema: &Value,
    schema: &DatabaseSchema,
) -> Result<ValidatedPlan, ValidationReport> {
    let structural = match validate_plan_json(plan_json, plan_schema) {
        Ok(report) => report,
        Err(err) => {
            let mut report = ValidationReport::default();
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "schema_validation_error",
                "/",
                err.to_string(),
                None,
            ));
            return Err(report);
        }
    };

    if !structural.is_ok() {
        return Err(structural);
    }

    let plan: Plan = match serde_json::from_value(plan_json.clone()) {
        Ok(plan) => plan,
        Err(err) => {
            let mut report = ValidationReport::default();
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "invalid_plan_json",
                "/",
                err.to_string(),
                None,
            ));
            return Err(report);
        }
    };

    let schema_report = validate_plan_against_schema(&plan, schema);
    if !schema_report.is_ok() {
        return Err(schema_report);
    }

    Ok(ValidatedPlan {
        plan,
        warnings: schema_report.warnings,
    })
}

fn validate_schema_ref(plan: &Plan, schema: &DatabaseSchema, report: &mut ValidationReport) {
    if plan.schema_ref.engine != schema.engine {
        report.push_error(ValidationIssue::new(
            IssueSeverity::Error,
            "engine_mismatch",
            "/schema_ref/engine",
            format!(
                "plan engine '{}' does not match schema engine '{}'",
                plan.schema_ref.engine, schema.engine
            ),
            Some("update schema_ref.engine to match the schema.json".to_string()),
        ));
    }

    if plan.schema_ref.schema_version != schema.schema_version {
        report.push_error(ValidationIssue::new(
            IssueSeverity::Error,
            "schema_version_mismatch",
            "/schema_ref/schema_version",
            format!(
                "plan schema_version '{}' does not match schema_version '{}'",
                plan.schema_ref.schema_version, schema.schema_version
            ),
            Some("update schema_ref.schema_version to match schema.json".to_string()),
        ));
    }

    match (
        &plan.schema_ref.schema_fingerprint,
        &schema.schema_fingerprint,
    ) {
        (Some(plan_fp), Some(schema_fp)) => {
            if plan_fp != schema_fp {
                report.push_error(ValidationIssue::new(
                    IssueSeverity::Error,
                    "schema_fingerprint_mismatch",
                    "/schema_ref/schema_fingerprint",
                    "schema_fingerprint does not match schema.json".to_string(),
                    Some("regenerate the plan using the current schema.json".to_string()),
                ));
            }
        }
        (Some(_), None) => {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "schema_fingerprint_missing",
                "/schema_ref/schema_fingerprint",
                "plan expects a schema_fingerprint but schema.json has none".to_string(),
                Some(
                    "remove schema_fingerprint from the plan or add it to schema.json".to_string(),
                ),
            ));
        }
        (None, Some(_)) => {
            report.push_warning(ValidationIssue::new(
                IssueSeverity::Warning,
                "schema_fingerprint_not_set",
                "/schema_ref",
                "schema.json includes a fingerprint but the plan does not".to_string(),
                Some("add schema_fingerprint to the plan for stricter validation".to_string()),
            ));
        }
        (None, None) => {}
    }
}

fn validate_targets(targets: &[Target], schema_index: &SchemaIndex, report: &mut ValidationReport) {
    if targets.is_empty() {
        report.push_error(ValidationIssue::new(
            IssueSeverity::Error,
            "targets_empty",
            "/targets",
            "plan requires at least one target".to_string(),
            Some("add at least one target table".to_string()),
        ));
        return;
    }

    let mut seen = HashSet::new();

    for (idx, target) in targets.iter().enumerate() {
        let base_path = format!("/targets/{idx}");
        if target.rows == 0 {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "rows_zero",
                format!("{base_path}/rows"),
                "rows must be greater than zero".to_string(),
                Some("set rows to a positive integer".to_string()),
            ));
        }

        let schema_name = target.schema.as_str();
        let table_name = target.table.as_str();

        match schema_index.schemas.get(schema_name) {
            None => {
                report.push_error(ValidationIssue::new(
                    IssueSeverity::Error,
                    "unknown_schema",
                    format!("{base_path}/schema"),
                    format!("schema '{}' not found in schema.json", schema_name),
                    None,
                ));
                continue;
            }
            Some(schema_tables) => {
                if !schema_tables.tables.contains_key(table_name) {
                    report.push_error(ValidationIssue::new(
                        IssueSeverity::Error,
                        "unknown_table",
                        format!("{base_path}/table"),
                        format!(
                            "table '{}.{}' not found in schema.json",
                            schema_name, table_name
                        ),
                        None,
                    ));
                }
            }
        }

        let target_key = format!("{schema_name}.{table_name}");
        if !seen.insert(target_key) {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "duplicate_target",
                base_path,
                "duplicate target for the same table".to_string(),
                Some("merge duplicate targets into a single entry".to_string()),
            ));
        }
    }
}

fn validate_rules(plan: &Plan, schema_index: &SchemaIndex, report: &mut ValidationReport) {
    let mut column_generators: HashMap<String, String> = HashMap::new();
    let mut constraint_policies: HashMap<String, ConstraintMode> = HashMap::new();
    let mut fk_policies: HashMap<String, ForeignKeyMode> = HashMap::new();

    for (idx, rule) in plan.rules.iter().enumerate() {
        let base_path = format!("/rules/{idx}");
        match rule {
            Rule::ColumnGenerator(rule) => {
                validate_column_generator_rule(
                    rule,
                    &base_path,
                    schema_index,
                    &mut column_generators,
                    report,
                );
            }
            Rule::ConstraintPolicy(rule) => {
                validate_constraint_policy_rule(
                    rule,
                    &base_path,
                    schema_index,
                    &mut constraint_policies,
                    report,
                );
            }
            Rule::ForeignKeyStrategy(rule) => {
                validate_foreign_key_strategy_rule(
                    rule,
                    &base_path,
                    schema_index,
                    &mut fk_policies,
                    report,
                    plan.options.as_ref(),
                );
            }
        }
    }
}

fn validate_unsupported(
    rules: &[UnsupportedRule],
    schema_index: &SchemaIndex,
    report: &mut ValidationReport,
) {
    for (idx, rule) in rules.iter().enumerate() {
        let base_path = format!("/rules_unsupported/{idx}");
        if let Some(reference) = &rule.reference {
            match schema_index.schemas.get(reference.schema.as_str()) {
                None => {
                    report.push_warning(ValidationIssue::new(
                        IssueSeverity::Warning,
                        "unsupported_unknown_schema",
                        format!("{base_path}/reference/schema"),
                        format!(
                            "schema '{}' not found for unsupported rule",
                            reference.schema
                        ),
                        None,
                    ));
                    continue;
                }
                Some(schema_tables) => {
                    if !schema_tables.tables.contains_key(reference.table.as_str()) {
                        report.push_warning(ValidationIssue::new(
                            IssueSeverity::Warning,
                            "unsupported_unknown_table",
                            format!("{base_path}/reference/table"),
                            format!(
                                "table '{}.{}' not found for unsupported rule",
                                reference.schema, reference.table
                            ),
                            None,
                        ));
                        continue;
                    }
                }
            }

            if let Some(column) = &reference.column {
                if let Some(table) = schema_index
                    .schemas
                    .get(reference.schema.as_str())
                    .and_then(|schema_tables| schema_tables.tables.get(reference.table.as_str()))
                {
                    if !table.columns.contains_key(column.as_str()) {
                        report.push_warning(ValidationIssue::new(
                            IssueSeverity::Warning,
                            "unsupported_unknown_column",
                            format!("{base_path}/reference/column"),
                            format!(
                                "column '{}.{}.{}' not found for unsupported rule",
                                reference.schema, reference.table, column
                            ),
                            None,
                        ));
                    }
                }
            }
        }
    }
}

fn validate_column_generator_rule(
    rule: &crate::model::ColumnGeneratorRule,
    base_path: &str,
    schema_index: &SchemaIndex,
    column_generators: &mut HashMap<String, String>,
    report: &mut ValidationReport,
) {
    let schema_name = rule.schema.as_str();
    let table_name = rule.table.as_str();
    let column_name = rule.column.as_str();

    let table = match schema_index
        .schemas
        .get(schema_name)
        .and_then(|schema_tables| schema_tables.tables.get(table_name))
    {
        Some(table) => table,
        None => {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "unknown_column_target",
                format!("{base_path}/table"),
                format!(
                    "table '{}.{}' not found for column generator",
                    schema_name, table_name
                ),
                None,
            ));
            return;
        }
    };

    let column = match table.columns.get(column_name) {
        Some(column) => column,
        None => {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "unknown_column",
                format!("{base_path}/column"),
                format!(
                    "column '{}.{}.{}' not found for column generator",
                    schema_name, table_name, column_name
                ),
                None,
            ));
            return;
        }
    };

    let key = format!("{schema_name}.{table_name}.{column_name}");
    if let Some(existing) = column_generators.get(&key) {
        if existing != &rule.generator {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "duplicate_generator_rule",
                base_path.to_string(),
                "multiple generators defined for the same column".to_string(),
                Some("keep only one generator per column".to_string()),
            ));
            return;
        }
    } else {
        column_generators.insert(key, rule.generator.clone());
    }

    match generator_compatible(&rule.generator, &column.column_type) {
        Some(true) => {}
        Some(false) => {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "incompatible_generator",
                base_path.to_string(),
                format!(
                    "generator '{}' is not compatible with column type '{}'",
                    rule.generator, column.column_type.data_type
                ),
                None,
            ));
        }
        None => {
            report.push_warning(ValidationIssue::new(
                IssueSeverity::Warning,
                "unknown_generator_id",
                base_path.to_string(),
                format!(
                    "generator '{}' is not recognized by the validator",
                    rule.generator
                ),
                Some("ensure the generator exists in datalchemy-generate".to_string()),
            ));
        }
    }

    let mut seen_transforms = HashSet::new();
    for (idx, transform) in rule.transforms.iter().enumerate() {
        let transform_id = transform.transform.as_str();
        if transform_id.trim().is_empty() {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "transform_empty_id",
                format!("{base_path}/transforms/{idx}/transform"),
                "transform id must be a non-empty string".to_string(),
                None,
            ));
            continue;
        }
        if !seen_transforms.insert(transform_id) {
            report.push_warning(ValidationIssue::new(
                IssueSeverity::Warning,
                "duplicate_transform",
                format!("{base_path}/transforms/{idx}/transform"),
                format!("duplicate transform '{}' for the same column", transform_id),
                None,
            ));
        }
    }
}

fn validate_constraint_policy_rule(
    rule: &ConstraintPolicyRule,
    base_path: &str,
    schema_index: &SchemaIndex,
    policies: &mut HashMap<String, ConstraintMode>,
    report: &mut ValidationReport,
) {
    let schema_name = rule.schema.as_str();
    let table_name = rule.table.as_str();

    let table = match schema_index
        .schemas
        .get(schema_name)
        .and_then(|schema_tables| schema_tables.tables.get(table_name))
    {
        Some(table) => table,
        None => {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "unknown_constraint_target",
                format!("{base_path}/table"),
                format!(
                    "table '{}.{}' not found for constraint policy",
                    schema_name, table_name
                ),
                None,
            ));
            return;
        }
    };

    let key = format!("{schema_name}.{table_name}.{:?}", rule.constraint);
    if let Some(existing) = policies.get(&key) {
        if existing != &rule.mode {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "duplicate_constraint_policy",
                base_path.to_string(),
                "multiple constraint policies for the same table and constraint".to_string(),
                Some("keep only one policy per constraint kind".to_string()),
            ));
            return;
        }
    } else {
        policies.insert(key, rule.mode.clone());
    }

    if !table_has_constraint(table, rule.constraint.clone()) {
        report.push_warning(ValidationIssue::new(
            IssueSeverity::Warning,
            "constraint_not_found",
            base_path.to_string(),
            format!(
                "table '{}.{}' does not define constraint kind '{:?}'",
                schema_name, table_name, rule.constraint
            ),
            None,
        ));
    }
}

fn validate_foreign_key_strategy_rule(
    rule: &ForeignKeyStrategyRule,
    base_path: &str,
    schema_index: &SchemaIndex,
    policies: &mut HashMap<String, ForeignKeyMode>,
    report: &mut ValidationReport,
    options: Option<&crate::model::PlanOptions>,
) {
    let schema_name = rule.schema.as_str();
    let table_name = rule.table.as_str();

    let table = match schema_index
        .schemas
        .get(schema_name)
        .and_then(|schema_tables| schema_tables.tables.get(table_name))
    {
        Some(table) => table,
        None => {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "unknown_fk_target",
                format!("{base_path}/table"),
                format!(
                    "table '{}.{}' not found for foreign key strategy",
                    schema_name, table_name
                ),
                None,
            ));
            return;
        }
    };

    let key = format!("{schema_name}.{table_name}");
    if let Some(existing) = policies.get(&key) {
        if existing != &rule.mode {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "duplicate_fk_strategy",
                base_path.to_string(),
                "multiple foreign key strategies for the same table".to_string(),
                Some("keep only one foreign key strategy per table".to_string()),
            ));
            return;
        }
    } else {
        policies.insert(key, rule.mode.clone());
    }

    if !table_has_foreign_keys(table) {
        report.push_warning(ValidationIssue::new(
            IssueSeverity::Warning,
            "fk_strategy_without_fk",
            base_path.to_string(),
            format!(
                "table '{}.{}' has no foreign keys; strategy has no effect",
                schema_name, table_name
            ),
            None,
        ));
    }

    if rule.mode == ForeignKeyMode::Disable {
        let allow = options
            .and_then(|opts| opts.allow_fk_disable)
            .unwrap_or(false);
        if !allow {
            report.push_warning(ValidationIssue::new(
                IssueSeverity::Warning,
                "fk_disable_without_flag",
                base_path.to_string(),
                "foreign key disable requested without allow_fk_disable".to_string(),
                Some("set options.allow_fk_disable=true to acknowledge".to_string()),
            ));
        }
    }
}

fn generator_compatible(generator: &str, column_type: &ColumnType) -> Option<bool> {
    let normalized = normalize_type(&column_type.data_type);
    let data_type = normalized.as_str();
    let is_text = matches!(
        data_type,
        "text" | "character varying" | "character" | "bpchar"
    );
    let is_numeric = matches!(data_type, "smallint" | "integer" | "bigint" | "numeric");
    let is_float = matches!(data_type, "numeric" | "real" | "double precision");
    let is_date = data_type == "date";
    let is_time = matches!(data_type, "time without time zone" | "time with time zone");
    let is_timestamp = matches!(
        data_type,
        "timestamp without time zone" | "timestamp with time zone"
    );

    let compatibility = match generator {
        "primitive.uuid.v4" => data_type == "uuid",
        "primitive.bool" => data_type == "boolean",
        "primitive.int.range" | "primitive.int.sequence_hint" => is_numeric,
        "primitive.float.range" | "primitive.decimal.numeric" | "semantic.br.money.brl" => {
            is_numeric || is_float
        }
        "primitive.text.pattern" | "primitive.text.lorem" => is_text,
        "primitive.date.range" => is_date || is_timestamp,
        "primitive.time.range" => is_time,
        "primitive.timestamp.range" => is_timestamp || is_date,
        "semantic.br.name"
        | "semantic.br.email.safe"
        | "semantic.br.phone"
        | "semantic.br.cpf"
        | "semantic.br.cnpj"
        | "semantic.br.rg"
        | "semantic.br.cep"
        | "semantic.br.uf"
        | "semantic.br.city"
        | "semantic.br.address"
        | "semantic.br.ip"
        | "semantic.br.url" => is_text,
        _ => return None,
    };
    Some(compatibility)
}

fn normalize_type(data_type: &str) -> String {
    let base = data_type.split('(').next().unwrap_or(data_type).trim();
    base.to_string()
}

fn table_has_foreign_keys(table: &TableInfo) -> bool {
    table
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, Constraint::ForeignKey(_)))
}

fn table_has_constraint(table: &TableInfo, kind: ConstraintKind) -> bool {
    match kind {
        ConstraintKind::Check => table
            .constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::Check(_))),
        ConstraintKind::Unique => table
            .constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::Unique(_))),
        ConstraintKind::PrimaryKey => table
            .constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::PrimaryKey(_))),
        ConstraintKind::ForeignKey => table
            .constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::ForeignKey(_))),
        ConstraintKind::NotNull => table.columns.values().any(|column| !column.is_nullable),
    }
}

fn build_schema_index(schema: &DatabaseSchema) -> SchemaIndex {
    let mut schemas = HashMap::new();

    for schema_entry in &schema.schemas {
        let mut tables = HashMap::new();
        for table in &schema_entry.tables {
            let mut columns = HashMap::new();
            for column in &table.columns {
                columns.insert(
                    column.name.clone(),
                    ColumnInfo {
                        column_type: column.column_type.clone(),
                        is_nullable: column.is_nullable,
                    },
                );
            }
            tables.insert(
                table.name.clone(),
                TableInfo {
                    columns,
                    constraints: table.constraints.clone(),
                },
            );
        }
        schemas.insert(schema_entry.name.clone(), SchemaTables { tables });
    }

    SchemaIndex { schemas }
}

fn normalized_json_pointer(pointer: &str) -> String {
    if pointer.is_empty() {
        "/".to_string()
    } else {
        pointer.to_string()
    }
}

struct SchemaIndex {
    schemas: HashMap<String, SchemaTables>,
}

struct SchemaTables {
    tables: HashMap<String, TableInfo>,
}

struct TableInfo {
    columns: HashMap<String, ColumnInfo>,
    constraints: Vec<Constraint>,
}

struct ColumnInfo {
    column_type: ColumnType,
    #[allow(dead_code)]
    is_nullable: bool,
}
