use std::collections::{HashMap, HashSet};

use datalchemy_core::{Constraint, DatabaseSchema};
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
                let missing = schema_index
                    .schemas
                    .get(reference.schema.as_str())
                    .and_then(|schema_tables| schema_tables.tables.get(reference.table.as_str()))
                    .map(|table| !table.columns.contains_key(column.as_str()))
                    .unwrap_or(false);
                if missing {
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

    let _column = match table.columns.get(column_name) {
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

    validate_input_columns(rule, base_path, table, report);
    validate_parent_reference(rule, base_path, schema_index, report);

    let generator_id = rule.generator_id().trim();
    if generator_id.is_empty() {
        report.push_error(ValidationIssue::new(
            IssueSeverity::Error,
            "empty_generator_id",
            format!("{base_path}/generator"),
            "generator id must be a non-empty string".to_string(),
            None,
        ));
        return;
    }
    if let Some(params) = rule.generator_params()
        && !params.is_object()
    {
        let params_path = if rule.generator.params().is_some() {
            format!("{base_path}/generator/params")
        } else {
            format!("{base_path}/params")
        };
        report.push_error(ValidationIssue::new(
            IssueSeverity::Error,
            "invalid_generator_params",
            params_path,
            "generator params must be a JSON object".to_string(),
            None,
        ));
        return;
    }

    let key = format!("{schema_name}.{table_name}.{column_name}");
    if let Some(existing) = column_generators.get(&key) {
        if existing != generator_id {
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
        column_generators.insert(key, generator_id.to_string());
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

fn validate_input_columns(
    rule: &crate::model::ColumnGeneratorRule,
    base_path: &str,
    table: &TableInfo,
    report: &mut ValidationReport,
) {
    let Some(params) = rule.generator_params() else {
        return;
    };
    let params_path = if rule.generator.params().is_some() {
        format!("{base_path}/generator/params")
    } else {
        format!("{base_path}/params")
    };
    let Some(value) = params.get("input_columns") else {
        return;
    };
    let Some(array) = value.as_array() else {
        report.push_error(ValidationIssue::new(
            IssueSeverity::Error,
            "invalid_input_columns",
            format!("{params_path}/input_columns"),
            "input_columns must be an array of strings".to_string(),
            None,
        ));
        return;
    };

    for (idx, entry) in array.iter().enumerate() {
        let column = match entry.as_str() {
            Some(column) => column,
            None => {
                report.push_error(ValidationIssue::new(
                    IssueSeverity::Error,
                    "invalid_input_columns",
                    format!("{params_path}/input_columns/{idx}"),
                    "input_columns must contain only strings".to_string(),
                    None,
                ));
                continue;
            }
        };

        if !table.columns.contains_key(column) {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "unknown_input_column",
                format!("{params_path}/input_columns/{idx}"),
                format!(
                    "input column '{}.{}.{}' not found",
                    rule.schema, rule.table, column
                ),
                None,
            ));
        }
    }
}

fn validate_parent_reference(
    rule: &crate::model::ColumnGeneratorRule,
    base_path: &str,
    schema_index: &SchemaIndex,
    report: &mut ValidationReport,
) {
    if rule.generator_id() != "derive.parent_value" {
        return;
    }

    let params = match rule.generator_params() {
        Some(params) => params,
        None => {
            let params_path = if rule.generator.params().is_some() {
                format!("{base_path}/generator/params")
            } else {
                format!("{base_path}/params")
            };
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "missing_parent_reference",
                params_path,
                "derive.parent_value requires parent_schema/parent_table/parent_column".to_string(),
                None,
            ));
            return;
        }
    };
    let params_path = if rule.generator.params().is_some() {
        format!("{base_path}/generator/params")
    } else {
        format!("{base_path}/params")
    };

    let parent_schema = match params.get("parent_schema").and_then(|value| value.as_str()) {
        Some(value) => value,
        None => {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "missing_parent_reference",
                format!("{params_path}/parent_schema"),
                "derive.parent_value requires parent_schema".to_string(),
                None,
            ));
            return;
        }
    };
    let parent_table = match params.get("parent_table").and_then(|value| value.as_str()) {
        Some(value) => value,
        None => {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "missing_parent_reference",
                format!("{params_path}/parent_table"),
                "derive.parent_value requires parent_table".to_string(),
                None,
            ));
            return;
        }
    };
    let parent_column = match params.get("parent_column").and_then(|value| value.as_str()) {
        Some(value) => value,
        None => {
            report.push_error(ValidationIssue::new(
                IssueSeverity::Error,
                "missing_parent_reference",
                format!("{params_path}/parent_column"),
                "derive.parent_value requires parent_column".to_string(),
                None,
            ));
            return;
        }
    };

    let Some(schema_tables) = schema_index.schemas.get(parent_schema) else {
        report.push_error(ValidationIssue::new(
            IssueSeverity::Error,
            "unknown_parent_reference",
            format!("{base_path}/params/parent_schema"),
            format!("parent schema '{}' not found", parent_schema),
            None,
        ));
        return;
    };
    let Some(table) = schema_tables.tables.get(parent_table) else {
        report.push_error(ValidationIssue::new(
            IssueSeverity::Error,
            "unknown_parent_reference",
            format!("{base_path}/params/parent_table"),
            format!(
                "parent table '{}.{}' not found",
                parent_schema, parent_table
            ),
            None,
        ));
        return;
    };
    if !table.columns.contains_key(parent_column) {
        report.push_error(ValidationIssue::new(
            IssueSeverity::Error,
            "unknown_parent_reference",
            format!("{base_path}/params/parent_column"),
            format!(
                "parent column '{}.{}.{}' not found",
                parent_schema, parent_table, parent_column
            ),
            None,
        ));
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
    #[allow(dead_code)]
    is_nullable: bool,
}
