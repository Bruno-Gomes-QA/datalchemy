use chrono::{NaiveTime, Timelike};
use rand::Rng;
use serde_json::Value;

use crate::errors::GenerationError;
use crate::generators::{GeneratedValue, Generator, GeneratorContext, GeneratorRegistry};

pub fn register(registry: &mut GeneratorRegistry) {
    registry.register_generator(Box::new(EmailFromNameGenerator));
    registry.register_generator(Box::new(UpdatedAfterCreatedGenerator));
    registry.register_generator(Box::new(EndAfterStartGenerator));
    registry.register_generator(Box::new(MoneyTotalGenerator));
    registry.register_generator(Box::new(FkGenerator));
    registry.register_generator(Box::new(ParentValueGenerator));
}

struct EmailFromNameGenerator;

impl Generator for EmailFromNameGenerator {
    fn id(&self) -> &'static str {
        "derive.email_from_name"
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        _rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let input_columns = input_columns(params)?;
        if input_columns.is_empty() {
            return Err(GenerationError::InvalidPlan(
                "derive.email_from_name requires input_columns".to_string(),
            ));
        }

        let mut parts = Vec::new();
        for column in input_columns {
            let key = column.to_lowercase();
            let value = ctx.row.get(&key).ok_or_else(|| {
                GenerationError::InvalidPlan(format!(
                    "derive.email_from_name missing column '{}'",
                    column
                ))
            })?;
            let value = value_to_string(value);
            let value = sanitize_identifier(&value);
            if !value.is_empty() {
                parts.push(value);
            }
        }

        let local = if parts.is_empty() {
            format!("user{}", ctx.row_index + 1)
        } else {
            parts.join(".")
        };
        let domain = params
            .and_then(|params| params.get("domain"))
            .and_then(|value| value.as_str())
            .unwrap_or("example.com");
        Ok(GeneratedValue::Text(format!("{local}@{domain}")))
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.email"]
    }
}

struct UpdatedAfterCreatedGenerator;

impl Generator for UpdatedAfterCreatedGenerator {
    fn id(&self) -> &'static str {
        "derive.updated_after_created"
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let input_columns = input_columns(params)?;
        let source = input_columns
            .first()
            .ok_or_else(|| {
                GenerationError::InvalidPlan(
                    "derive.updated_after_created requires input_columns".to_string(),
                )
            })?
            .to_lowercase();
        let value = ctx.row.get(&source).ok_or_else(|| {
            GenerationError::InvalidPlan(format!(
                "derive.updated_after_created missing column '{}'",
                source
            ))
        })?;
        derive_after(value, params, rng)
    }
}

struct EndAfterStartGenerator;

impl Generator for EndAfterStartGenerator {
    fn id(&self) -> &'static str {
        "derive.end_after_start"
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let input_columns = input_columns(params)?;
        let source = input_columns
            .first()
            .ok_or_else(|| {
                GenerationError::InvalidPlan(
                    "derive.end_after_start requires input_columns".to_string(),
                )
            })?
            .to_lowercase();
        let value = ctx.row.get(&source).ok_or_else(|| {
            GenerationError::InvalidPlan(format!(
                "derive.end_after_start missing column '{}'",
                source
            ))
        })?;
        derive_after(value, params, rng)
    }
}

struct MoneyTotalGenerator;

impl Generator for MoneyTotalGenerator {
    fn id(&self) -> &'static str {
        "derive.money_total"
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        _rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let input_columns = input_columns(params)?;
        if input_columns.len() < 2 {
            return Err(GenerationError::InvalidPlan(
                "derive.money_total requires input_columns (price, qty, [discount])".to_string(),
            ));
        }

        let price = column_numeric(ctx, &input_columns[0])?;
        let qty = column_numeric(ctx, &input_columns[1])?;
        let discount = if input_columns.len() > 2 {
            column_numeric(ctx, &input_columns[2])?
        } else {
            0.0
        };

        let total = price * qty - discount;
        Ok(GeneratedValue::Float(total))
    }
}

struct FkGenerator;

impl Generator for FkGenerator {
    fn id(&self) -> &'static str {
        "derive.fk"
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        _rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let foreign = ctx.foreign.as_deref_mut().ok_or_else(|| {
            GenerationError::Unsupported("foreign context not available".to_string())
        })?;
        let fk = ctx
            .foreign_keys
            .iter()
            .find(|fk| {
                fk.columns
                    .iter()
                    .any(|col| col.eq_ignore_ascii_case(&ctx.column.name))
            })
            .ok_or_else(|| {
                GenerationError::InvalidPlan(format!(
                    "derive.fk requires foreign key constraint for '{}.{}.{}'",
                    ctx.schema, ctx.table, ctx.column.name
                ))
            })?;

        let index = fk
            .columns
            .iter()
            .position(|col| col.eq_ignore_ascii_case(&ctx.column.name))
            .unwrap_or(0);
        let parent_col = fk.referenced_columns.get(index).ok_or_else(|| {
            GenerationError::InvalidPlan("derive.fk referenced column not found".to_string())
        })?;

        foreign.pick_fk(&fk.referenced_schema, &fk.referenced_table, parent_col)
    }
}

struct ParentValueGenerator;

impl Generator for ParentValueGenerator {
    fn id(&self) -> &'static str {
        "derive.parent_value"
    }

    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        _rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let input_columns = input_columns(params)?;
        let fk_column = input_columns.first().ok_or_else(|| {
            GenerationError::InvalidPlan(
                "derive.parent_value requires input_columns with fk column".to_string(),
            )
        })?;
        let fk_value = ctx.row.get(&fk_column.to_lowercase()).ok_or_else(|| {
            GenerationError::InvalidPlan(format!(
                "derive.parent_value missing fk column '{}'",
                fk_column
            ))
        })?;

        let params = params.ok_or_else(|| {
            GenerationError::InvalidPlan(
                "derive.parent_value requires parent_schema/table/column".to_string(),
            )
        })?;
        let parent_schema = params
            .get("parent_schema")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                GenerationError::InvalidPlan(
                    "derive.parent_value requires parent_schema".to_string(),
                )
            })?;
        let parent_table = params
            .get("parent_table")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                GenerationError::InvalidPlan(
                    "derive.parent_value requires parent_table".to_string(),
                )
            })?;
        let parent_column = params
            .get("parent_column")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                GenerationError::InvalidPlan(
                    "derive.parent_value requires parent_column".to_string(),
                )
            })?;

        let foreign = ctx.foreign.as_deref().ok_or_else(|| {
            GenerationError::Unsupported("foreign context not available".to_string())
        })?;

        let value = foreign
            .lookup_parent(parent_schema, parent_table, fk_value, parent_column)
            .ok_or_else(|| {
                GenerationError::Unsupported(format!(
                    "derive.parent_value parent not found for '{}.{}.{}'",
                    parent_schema, parent_table, parent_column
                ))
            })?;

        Ok(value)
    }
}

fn input_columns(params: Option<&Value>) -> Result<Vec<String>, GenerationError> {
    let Some(params) = params else {
        return Ok(Vec::new());
    };
    let Some(array) = params.get("input_columns") else {
        return Ok(Vec::new());
    };
    let Some(values) = array.as_array() else {
        return Err(GenerationError::InvalidPlan(
            "input_columns must be an array of strings".to_string(),
        ));
    };

    let mut columns = Vec::new();
    for value in values {
        let column = value.as_str().ok_or_else(|| {
            GenerationError::InvalidPlan("input_columns must be strings".to_string())
        })?;
        columns.push(column.to_string());
    }
    Ok(columns)
}

fn column_numeric(ctx: &GeneratorContext<'_>, column: &str) -> Result<f64, GenerationError> {
    let value = ctx.row.get(&column.to_lowercase()).ok_or_else(|| {
        GenerationError::InvalidPlan(format!("derive.money_total missing column '{}'", column))
    })?;
    match value {
        GeneratedValue::Int(value) => Ok(*value as f64),
        GeneratedValue::Float(value) => Ok(*value),
        _ => Err(GenerationError::InvalidPlan(format!(
            "derive.money_total column '{}' is not numeric",
            column
        ))),
    }
}

fn value_to_string(value: &GeneratedValue) -> String {
    match value {
        GeneratedValue::Null => String::new(),
        GeneratedValue::Bool(value) => value.to_string(),
        GeneratedValue::Int(value) => value.to_string(),
        GeneratedValue::Float(value) => value.to_string(),
        GeneratedValue::Text(value) | GeneratedValue::Uuid(value) => value.clone(),
        GeneratedValue::Date(value) => value.format("%Y-%m-%d").to_string(),
        GeneratedValue::Time(value) => value.format("%H:%M:%S").to_string(),
        GeneratedValue::Timestamp(value) => value.format("%Y-%m-%dT%H:%M:%S").to_string(),
    }
}

fn sanitize_identifier(value: &str) -> String {
    let mut out = String::new();
    let mut last_dot = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dot = false;
        } else if ch.is_whitespace() || ch == '-' || ch == '_' || ch == '.' {
            if !last_dot {
                out.push('.');
                last_dot = true;
            }
        }
    }
    out.trim_matches('.').to_string()
}

fn derive_after(
    value: &GeneratedValue,
    params: Option<&Value>,
    rng: &mut dyn rand::RngCore,
) -> Result<GeneratedValue, GenerationError> {
    match value {
        GeneratedValue::Timestamp(value) => {
            let max_seconds = params
                .and_then(|params| params.get("max_seconds"))
                .and_then(|value| value.as_i64())
                .unwrap_or(86_400)
                .max(0);
            let delta = rng.gen_range(0..=max_seconds) as i64;
            Ok(GeneratedValue::Timestamp(
                *value + chrono::Duration::seconds(delta),
            ))
        }
        GeneratedValue::Date(value) => {
            let max_days = params
                .and_then(|params| params.get("max_days"))
                .and_then(|value| value.as_i64())
                .unwrap_or(30)
                .max(0);
            let delta = rng.gen_range(0..=max_days) as i64;
            Ok(GeneratedValue::Date(*value + chrono::Duration::days(delta)))
        }
        GeneratedValue::Time(value) => {
            let max_seconds = params
                .and_then(|params| params.get("max_seconds"))
                .and_then(|value| value.as_i64())
                .unwrap_or(3_600)
                .max(0);
            let start = value.num_seconds_from_midnight() as i64;
            let delta = rng.gen_range(0..=max_seconds) as i64;
            let end = (start + delta).min(86_399);
            let time = NaiveTime::from_num_seconds_from_midnight_opt(end as u32, 0)
                .unwrap_or_else(NaiveTime::default);
            Ok(GeneratedValue::Time(time))
        }
        GeneratedValue::Null => Ok(GeneratedValue::Null),
        _ => Err(GenerationError::InvalidPlan(
            "derive generator expects date/time/timestamp".to_string(),
        )),
    }
}
