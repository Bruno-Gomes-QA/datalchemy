use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;
use serde_json::{Map, Value};

use crate::errors::GenerationError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamKind {
    Bool,
    Int,
    Float,
    String,
    Date,
    Time,
    Timestamp,
}

#[derive(Clone, Copy, Debug)]
pub struct ParamSpec {
    pub key: &'static str,
    pub kind: ParamKind,
    pub required: bool,
}

impl ParamSpec {
    pub const fn new(key: &'static str, kind: ParamKind, required: bool) -> Self {
        Self {
            key,
            kind,
            required,
        }
    }
}

pub struct ParamMap<'a> {
    map: Option<&'a Map<String, Value>>,
}

pub struct TextLimits {
    pub min_len: Option<usize>,
    pub max_len: Option<usize>,
    pub allow_empty: bool,
    pub schema_max: Option<usize>,
}

pub fn validate_params<'a>(
    params: Option<&'a Value>,
    specs: &[ParamSpec],
    ctx: &'static str,
) -> Result<ParamMap<'a>, GenerationError> {
    let map = match params {
        None => None,
        Some(Value::Object(map)) => Some(map),
        Some(_) => {
            return Err(GenerationError::InvalidPlan(format!(
                "{ctx}: params must be a JSON object"
            )));
        }
    };

    if let Some(map) = map {
        for (key, value) in map {
            let Some(spec) = specs.iter().find(|spec| spec.key == key.as_str()) else {
                return Err(GenerationError::InvalidPlan(format!(
                    "{ctx}: unknown param '{key}'"
                )));
            };
            validate_kind(ctx, key, spec.kind, value)?;
        }
    }

    for spec in specs {
        if spec.required && !map.is_some_and(|map| map.contains_key(spec.key)) {
            return Err(GenerationError::InvalidPlan(format!(
                "{ctx}: missing required param '{}'",
                spec.key
            )));
        }
    }

    Ok(ParamMap { map })
}

impl<'a> ParamMap<'a> {
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.map
            .and_then(|map| map.get(key))
            .and_then(|value| value.as_i64())
    }

    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.map
            .and_then(|map| map.get(key))
            .and_then(|value| value.as_u64())
            .and_then(|value| u32::try_from(value).ok())
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.map
            .and_then(|map| map.get(key))
            .and_then(|value| value.as_f64())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.map
            .and_then(|map| map.get(key))
            .and_then(|value| value.as_bool())
    }

    pub fn get_str(&self, key: &str) -> Option<&'a str> {
        self.map
            .and_then(|map| map.get(key))
            .and_then(|value| value.as_str())
    }
}

fn validate_kind(
    ctx: &'static str,
    key: &str,
    kind: ParamKind,
    value: &Value,
) -> Result<(), GenerationError> {
    let valid = match kind {
        ParamKind::Bool => value.is_boolean(),
        ParamKind::Int => value.as_i64().is_some(),
        ParamKind::Float => value.as_f64().is_some(),
        ParamKind::String => value.is_string(),
        ParamKind::Date => value.as_str().and_then(parse_date_value).is_some(),
        ParamKind::Time => value.as_str().and_then(parse_time_value).is_some(),
        ParamKind::Timestamp => value.as_str().and_then(parse_timestamp_value).is_some(),
    };

    if valid {
        Ok(())
    } else {
        Err(GenerationError::InvalidPlan(format!(
            "{ctx}: invalid value for param '{key}'"
        )))
    }
}

pub fn parse_date_value(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

pub fn parse_time_value(value: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(value, "%H:%M:%S")
        .ok()
        .or_else(|| NaiveTime::parse_from_str(value, "%H:%M:%S%.f").ok())
}

pub fn parse_timestamp_value(value: &str) -> Option<NaiveDateTime> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.naive_utc())
        .or_else(|| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").ok())
        .or_else(|| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").ok())
}

pub fn text_limits(
    params: &ParamMap<'_>,
    ctx: &'static str,
    schema_max: Option<i32>,
) -> Result<TextLimits, GenerationError> {
    let min_len_value = params.get_i64("min_len");
    let max_len_value = params.get_i64("max_len");
    if let Some(value) = min_len_value
        && value < 0
    {
        return Err(GenerationError::InvalidPlan(format!(
            "{ctx}: min_len must be >= 0"
        )));
    }
    if let Some(value) = max_len_value
        && value < 0
    {
        return Err(GenerationError::InvalidPlan(format!(
            "{ctx}: max_len must be >= 0"
        )));
    }
    let min_len = min_len_value.map(|value| value as usize);
    let max_len = max_len_value.map(|value| value as usize);
    let allow_empty = params.get_bool("allow_empty").unwrap_or(false);
    let schema_max = schema_max.map(|value| value as usize);

    if let (Some(min_len), Some(max_len)) = (min_len, max_len)
        && min_len > max_len
    {
        return Err(GenerationError::InvalidPlan(format!(
            "{ctx}: min_len must be <= max_len"
        )));
    }

    if let (Some(max_len), Some(schema_max)) = (max_len, schema_max)
        && max_len > schema_max
    {
        return Err(GenerationError::InvalidPlan(format!(
            "{ctx}: max_len exceeds schema limit"
        )));
    }

    Ok(TextLimits {
        min_len,
        max_len,
        allow_empty,
        schema_max,
    })
}

pub fn validate_text_value(
    ctx: &'static str,
    value: &str,
    limits: &TextLimits,
) -> Result<(), GenerationError> {
    let len = text_length(value);
    if !limits.allow_empty && value.is_empty() {
        return Err(GenerationError::InvalidPlan(format!(
            "{ctx}: empty text not allowed"
        )));
    }
    if let Some(min_len) = limits.min_len
        && len < min_len
    {
        return Err(GenerationError::InvalidPlan(format!(
            "{ctx}: value shorter than min_len"
        )));
    }
    if let Some(max_len) = limits.max_len
        && len > max_len
    {
        return Err(GenerationError::InvalidPlan(format!(
            "{ctx}: value exceeds max_len"
        )));
    }
    if let Some(schema_max) = limits.schema_max
        && len > schema_max
    {
        return Err(GenerationError::InvalidPlan(format!(
            "{ctx}: value exceeds schema limit"
        )));
    }
    Ok(())
}

pub fn validate_text_constraints(
    ctx: &'static str,
    value: &str,
    limits: &TextLimits,
    pattern: Option<&str>,
    charset: Option<&str>,
) -> Result<(), GenerationError> {
    validate_text_value(ctx, value, limits)?;

    if let Some(pattern) = pattern {
        let regex = Regex::new(pattern).map_err(|err| {
            GenerationError::InvalidPlan(format!("{ctx}: invalid pattern: {err}"))
        })?;
        if !regex.is_match(value) {
            return Err(GenerationError::InvalidPlan(format!(
                "{ctx}: value does not match pattern"
            )));
        }
    }

    if let Some(charset) = charset
        && !value.chars().all(|ch| charset.contains(ch))
    {
        return Err(GenerationError::InvalidPlan(format!(
            "{ctx}: value contains characters outside charset"
        )));
    }

    Ok(())
}

fn text_length(value: &str) -> usize {
    value.chars().count()
}
