use crate::metrics::{ConstraintSummary, MetricsReport};
use crate::model::Violation;

/// Render a deterministic markdown report from metrics and violations.
pub fn render_report(
    metrics: &MetricsReport,
    violations: &[Violation],
    max_examples: usize,
) -> String {
    let mut lines = Vec::new();

    lines.push("# Datalchemy Evaluation Report".to_string());
    lines.push(String::new());
    lines.push("## Run summary".to_string());
    lines.push(format!("- run_id: {}", metrics.run_id));
    lines.push(format!(
        "- schema_version: {}",
        metrics.schema_ref.schema_version
    ));
    lines.push(format!("- plan_version: {}", metrics.plan_ref.plan_version));
    lines.push(format!("- seed: {}", metrics.plan_ref.seed));
    lines.push(String::new());

    lines.push("## Targets and row counts".to_string());
    lines.push("| table | rows_expected | rows_found |".to_string());
    lines.push("| --- | --- | --- |".to_string());
    for table in &metrics.tables {
        let expected = table
            .rows_expected
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        lines.push(format!(
            "| {}.{} | {} | {} |",
            table.schema, table.table, expected, table.rows_found
        ));
    }
    lines.push(String::new());

    lines.push("## Constraint summary".to_string());
    lines.push("| constraint | checked | violations | not_evaluated |".to_string());
    lines.push("| --- | --- | --- | --- |".to_string());
    push_constraint_row(&mut lines, "not_null", &metrics.constraints, None);
    push_constraint_row(&mut lines, "pk", &metrics.constraints, None);
    push_constraint_row(&mut lines, "unique", &metrics.constraints, None);
    push_constraint_row(&mut lines, "fk", &metrics.constraints, None);
    push_constraint_row(
        &mut lines,
        "check",
        &metrics.constraints,
        Some(metrics.constraints.check.not_evaluated),
    );
    lines.push(String::new());

    if !metrics.warnings.is_empty() {
        lines.push("## Warnings".to_string());
        for warning in &metrics.warnings {
            let hint = warning
                .hint
                .as_ref()
                .map(|hint| format!(" (hint: {hint})"))
                .unwrap_or_default();
            lines.push(format!("- {}: {}{}", warning.path, warning.message, hint));
        }
        lines.push(String::new());
    }

    if !violations.is_empty() {
        lines.push("## Top violations".to_string());
        for violation in violations.iter().take(max_examples) {
            let row = violation
                .row_index
                .map(|row| format!(" row {row}"))
                .unwrap_or_default();
            let example = violation
                .example
                .as_ref()
                .map(|value| format!(" example={value}"))
                .unwrap_or_default();
            lines.push(format!(
                "- {}{}: {}{}",
                violation.path, row, violation.message, example
            ));
        }
        lines.push(String::new());
    }

    lines.push("## Recommendations".to_string());
    lines.extend(recommendations(metrics, violations));
    lines.join("\n")
}

fn push_constraint_row(
    lines: &mut Vec<String>,
    name: &str,
    summary: &ConstraintSummary,
    not_evaluated: Option<u64>,
) {
    let (checked, violations, not_eval) = match name {
        "not_null" => (summary.not_null.checked, summary.not_null.violations, 0),
        "pk" => (summary.pk.checked, summary.pk.violations, 0),
        "unique" => (summary.unique.checked, summary.unique.violations, 0),
        "fk" => (summary.fk.checked, summary.fk.violations, 0),
        "check" => (
            summary.check.checked,
            summary.check.violations,
            not_evaluated.unwrap_or(0),
        ),
        _ => (0, 0, 0),
    };
    let not_eval = if name == "check" {
        not_eval.to_string()
    } else {
        "-".to_string()
    };
    lines.push(format!(
        "| {} | {} | {} | {} |",
        name, checked, violations, not_eval
    ));
}

fn recommendations(metrics: &MetricsReport, violations: &[Violation]) -> Vec<String> {
    let mut lines = Vec::new();
    if metrics.constraints.not_null.violations > 0 {
        lines.push("- revise generators for NOT NULL columns with nulls.".to_string());
    }
    if metrics.constraints.unique.violations > 0 || metrics.constraints.pk.violations > 0 {
        lines.push("- increase unique key space or add explicit generators.".to_string());
    }
    if metrics.constraints.fk.violations > 0 {
        lines.push("- ensure parent tables are generated before children.".to_string());
    }
    if metrics.constraints.check.not_evaluated > 0 {
        lines.push("- simplify CHECK expressions or switch policy to warn/ignore.".to_string());
    }
    if violations.is_empty() {
        lines.push("- no violations detected; compare metrics across runs for drift.".to_string());
    }
    lines
}
