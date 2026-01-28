use std::collections::HashMap;

use chrono::NaiveDate;
use regex::Regex;

use crate::generators::GeneratedValue;

/// Result of evaluating a CHECK constraint expression.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckOutcome {
    Passed,
    Failed,
    Unsupported,
}

/// Context for evaluating CHECK constraints.
#[derive(Debug)]
pub struct CheckContext<'a> {
    pub values: &'a HashMap<String, GeneratedValue>,
    pub base_date: NaiveDate,
}

/// Evaluate a subset of CHECK expressions.
pub fn evaluate_check(expression: &str, ctx: &CheckContext<'_>) -> CheckOutcome {
    let expr = normalize_expression(expression);

    if let Some(parts) = split_and(&expr) {
        for part in parts {
            match evaluate_check(&part, ctx) {
                CheckOutcome::Passed => continue,
                CheckOutcome::Failed => return CheckOutcome::Failed,
                CheckOutcome::Unsupported => return CheckOutcome::Unsupported,
            }
        }
        return CheckOutcome::Passed;
    }

    if let Some((column, rest)) = parse_is_null_or(&expr) {
        if is_null(&column, ctx) {
            return CheckOutcome::Passed;
        }
        return evaluate_check(&rest, ctx);
    }

    if let Some((column, value)) = parse_is_not_null(&expr) {
        if column.is_empty() {
            return CheckOutcome::Unsupported;
        }
        return if is_null(&value, ctx) {
            CheckOutcome::Failed
        } else {
            CheckOutcome::Passed
        };
    }

    if let Some((column, values)) = parse_any_array(&expr) {
        return evaluate_in(&column, &values, ctx);
    }

    if let Some((column, values)) = parse_in_list(&expr) {
        return evaluate_in(&column, &values, ctx);
    }

    if let Some((column, min, max)) = parse_between(&expr) {
        return evaluate_between(&column, &min, &max, ctx);
    }

    if let Some((column, op, rhs)) = parse_comparison(&expr) {
        return evaluate_comparison(&column, &op, &rhs, ctx);
    }

    if let Some((needle, column, op, rhs)) = parse_position(&expr) {
        return evaluate_position(&needle, &column, &op, &rhs, ctx);
    }

    CheckOutcome::Unsupported
}

fn normalize_expression(expression: &str) -> String {
    let mut expr = expression.trim().to_string();
    if expr.to_uppercase().starts_with("CHECK") {
        expr = expr[5..].trim().to_string();
    }
    while expr.starts_with('(') && expr.ends_with(')') {
        expr = expr[1..expr.len() - 1].trim().to_string();
    }
    expr
}

fn split_and(expr: &str) -> Option<Vec<String>> {
    let lower = expr.to_lowercase();
    if !lower.contains(" and ") {
        return None;
    }
    if lower.contains(" between ") {
        return None;
    }
    let parts = lower
        .split(" and ")
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() > 1 { Some(parts) } else { None }
}

fn parse_is_null_or(expr: &str) -> Option<(String, String)> {
    let re = Regex::new(r"(?i)^\s*(\w+)\s+is\s+null\s+or\s+(.+)$").ok()?;
    let caps = re.captures(expr)?;
    Some((caps[1].to_lowercase(), caps[2].trim().to_string()))
}

fn parse_is_not_null(expr: &str) -> Option<(String, String)> {
    let re = Regex::new(r"(?i)^\s*(\w+)\s+is\s+not\s+null\s*$").ok()?;
    let caps = re.captures(expr)?;
    Some((caps[1].to_lowercase(), caps[1].to_lowercase()))
}

fn parse_in_list(expr: &str) -> Option<(String, Vec<String>)> {
    let re = Regex::new(r"(?i)^\s*(\w+)\s+in\s*\(([^\)]+)\)\s*$").ok()?;
    let caps = re.captures(expr)?;
    let values = caps[2].split(',').map(normalize_literal).collect();
    Some((caps[1].to_lowercase(), values))
}

fn parse_between(expr: &str) -> Option<(String, String, String)> {
    let re = Regex::new(r"(?i)^\s*(\w+)\s+between\s+([^\s]+)\s+and\s+([^\s]+)\s*$").ok()?;
    let caps = re.captures(expr)?;
    Some((
        caps[1].to_lowercase(),
        normalize_literal(&caps[2]),
        normalize_literal(&caps[3]),
    ))
}

fn parse_comparison(expr: &str) -> Option<(String, String, String)> {
    let re = Regex::new(r"(?i)^\s*(\w+)\s*(=|>=|<=|>|<)\s*([^\s]+)\s*$").ok()?;
    let caps = re.captures(expr)?;
    Some((
        caps[1].to_lowercase(),
        caps[2].to_string(),
        normalize_literal(&caps[3]),
    ))
}

fn parse_position(expr: &str) -> Option<(String, String, String, String)> {
    let re = Regex::new(
        r"(?i)^\s*position\(\(?\s*'\s*([^']*)\s*'(?:::text)?\s*\)?\s+in\s+\(?\s*(\w+)\s*\)?\s*\)\s*(=|>=|<=|>|<)\s*(\d+)\s*$",
    )
    .ok()?;
    let caps = re.captures(expr)?;
    Some((
        caps[1].to_string(),
        caps[2].to_lowercase(),
        caps[3].to_string(),
        caps[4].to_string(),
    ))
}

fn parse_any_array(expr: &str) -> Option<(String, Vec<String>)> {
    let re = Regex::new(r"(?i)^\s*(\w+)\s*=\s*any\s*\(array\[([^\]]+)\]\)\s*$").ok()?;
    let caps = re.captures(expr)?;
    let values = caps[2].split(',').map(normalize_literal).collect();
    Some((caps[1].to_lowercase(), values))
}

fn evaluate_in(column: &str, values: &[String], ctx: &CheckContext<'_>) -> CheckOutcome {
    let value = match get_value(column, ctx) {
        Some(value) => value,
        None => return CheckOutcome::Unsupported,
    };

    let candidate = match value.as_str() {
        Some(value) => value,
        None => return CheckOutcome::Unsupported,
    };

    if values.iter().any(|v| v == candidate) {
        CheckOutcome::Passed
    } else {
        CheckOutcome::Failed
    }
}

fn evaluate_between(column: &str, min: &str, max: &str, ctx: &CheckContext<'_>) -> CheckOutcome {
    let value = match get_value(column, ctx) {
        Some(value) => value,
        None => return CheckOutcome::Unsupported,
    };

    if let Some(num) = value.as_f64() {
        let min_val = min.parse::<f64>().ok();
        let max_val = max.parse::<f64>().ok();
        if let (Some(min_val), Some(max_val)) = (min_val, max_val) {
            return if num >= min_val && num <= max_val {
                CheckOutcome::Passed
            } else {
                CheckOutcome::Failed
            };
        }
    }

    if let Some(date) = value.as_date() {
        let min_date = parse_date_literal(min, ctx.base_date);
        let max_date = parse_date_literal(max, ctx.base_date);
        if let (Some(min_date), Some(max_date)) = (min_date, max_date) {
            return if date >= min_date && date <= max_date {
                CheckOutcome::Passed
            } else {
                CheckOutcome::Failed
            };
        }
    }

    CheckOutcome::Unsupported
}

fn evaluate_comparison(column: &str, op: &str, rhs: &str, ctx: &CheckContext<'_>) -> CheckOutcome {
    let left = match get_value(column, ctx) {
        Some(value) => value,
        None => return CheckOutcome::Unsupported,
    };

    if let Some(num) = left.as_f64()
        && let Some(rhs_val) = parse_numeric_or_column(rhs, ctx).and_then(|v| v.as_f64())
    {
        return compare_f64(num, rhs_val, op);
    }

    if let Some(date) = left.as_date()
        && let Some(rhs_date) =
            parse_date_literal(rhs, ctx.base_date).or_else(|| parse_column_date(rhs, ctx))
    {
        return compare_date(date, rhs_date, op);
    }

    if let Some(text) = left.as_str()
        && let Some(rhs_text) = parse_text_literal(rhs)
    {
        return compare_text(text, &rhs_text, op);
    }

    CheckOutcome::Unsupported
}

fn evaluate_position(
    needle: &str,
    column: &str,
    op: &str,
    rhs: &str,
    ctx: &CheckContext<'_>,
) -> CheckOutcome {
    let value = match get_value(column, ctx).and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return CheckOutcome::Unsupported,
    };

    let pos = value.find(needle).map(|idx| idx as i64 + 1).unwrap_or(0);
    let rhs_val = rhs.parse::<i64>().ok();
    if let Some(rhs_val) = rhs_val {
        return compare_i64(pos, rhs_val, op);
    }

    CheckOutcome::Unsupported
}

fn parse_numeric_or_column(rhs: &str, ctx: &CheckContext<'_>) -> Option<GeneratedValue> {
    if let Ok(value) = rhs.parse::<f64>() {
        return Some(GeneratedValue::Float(value));
    }
    get_value(rhs, ctx).cloned()
}

fn parse_text_literal(rhs: &str) -> Option<String> {
    if rhs.starts_with('\'') && rhs.ends_with('\'') && rhs.len() >= 2 {
        return Some(rhs[1..rhs.len() - 1].to_string());
    }
    None
}

fn parse_date_literal(rhs: &str, base_date: NaiveDate) -> Option<NaiveDate> {
    if rhs.eq_ignore_ascii_case("current_date") {
        return Some(base_date);
    }
    if rhs.starts_with('\'') && rhs.ends_with('\'') {
        let trimmed = &rhs[1..rhs.len() - 1];
        return NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").ok();
    }
    None
}

fn parse_column_date(column: &str, ctx: &CheckContext<'_>) -> Option<NaiveDate> {
    get_value(column, ctx).and_then(|value| value.as_date())
}

fn compare_f64(left: f64, right: f64, op: &str) -> CheckOutcome {
    let pass = match op {
        ">" => left > right,
        ">=" => left >= right,
        "<" => left < right,
        "<=" => left <= right,
        "=" => (left - right).abs() < f64::EPSILON,
        _ => false,
    };
    if pass {
        CheckOutcome::Passed
    } else {
        CheckOutcome::Failed
    }
}

fn compare_i64(left: i64, right: i64, op: &str) -> CheckOutcome {
    let pass = match op {
        ">" => left > right,
        ">=" => left >= right,
        "<" => left < right,
        "<=" => left <= right,
        "=" => left == right,
        _ => false,
    };
    if pass {
        CheckOutcome::Passed
    } else {
        CheckOutcome::Failed
    }
}

fn compare_date(left: NaiveDate, right: NaiveDate, op: &str) -> CheckOutcome {
    let pass = match op {
        ">" => left > right,
        ">=" => left >= right,
        "<" => left < right,
        "<=" => left <= right,
        "=" => left == right,
        _ => false,
    };
    if pass {
        CheckOutcome::Passed
    } else {
        CheckOutcome::Failed
    }
}

fn compare_text(left: &str, right: &str, op: &str) -> CheckOutcome {
    let pass = match op {
        "=" => left == right,
        _ => false,
    };
    if pass {
        CheckOutcome::Passed
    } else {
        CheckOutcome::Failed
    }
}

fn is_null(column: &str, ctx: &CheckContext<'_>) -> bool {
    get_value(column, ctx)
        .map(|value| value.is_null())
        .unwrap_or(false)
}

fn get_value<'a>(column: &str, ctx: &'a CheckContext<'_>) -> Option<&'a GeneratedValue> {
    let key = column.to_lowercase();
    ctx.values.get(&key)
}

fn normalize_literal(value: &str) -> String {
    let trimmed = value.trim().trim_matches('(').trim_matches(')');
    let without_cast = match trimmed.split_once("::") {
        Some((left, _)) => left.trim(),
        None => trimmed,
    };
    let stripped = without_cast.trim();
    if stripped.starts_with('\'') && stripped.ends_with('\'') && stripped.len() >= 2 {
        stripped[1..stripped.len() - 1].to_string()
    } else {
        stripped.to_string()
    }
}
