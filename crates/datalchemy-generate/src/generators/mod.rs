use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rand::Rng;
use rand::seq::SliceRandom;
use serde_json::Value;

use datalchemy_core::{Column, ColumnType, DatabaseSchema, EnumType};
use datalchemy_plan::{ColumnGenerator, ColumnGeneratorRule};

use crate::errors::GenerationError;

/// Generated value for a column.
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratedValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Uuid(String),
    Date(NaiveDate),
    Time(NaiveTime),
    Timestamp(NaiveDateTime),
}

impl GeneratedValue {
    pub fn is_null(&self) -> bool {
        matches!(self, GeneratedValue::Null)
    }

    pub fn to_csv(&self, column: &Column) -> String {
        match self {
            GeneratedValue::Null => String::new(),
            GeneratedValue::Bool(value) => value.to_string(),
            GeneratedValue::Int(value) => value.to_string(),
            GeneratedValue::Float(value) => {
                if let Some(scale) = column.column_type.numeric_scale {
                    let scale = scale as usize;
                    format!("{value:.scale$}")
                } else {
                    value.to_string()
                }
            }
            GeneratedValue::Text(value) | GeneratedValue::Uuid(value) => value.clone(),
            GeneratedValue::Date(value) => value.format("%Y-%m-%d").to_string(),
            GeneratedValue::Time(value) => value.format("%H:%M:%S").to_string(),
            GeneratedValue::Timestamp(value) => value.format("%Y-%m-%dT%H:%M:%S").to_string(),
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            GeneratedValue::Int(value) => Some(*value as f64),
            GeneratedValue::Float(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            GeneratedValue::Int(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            GeneratedValue::Text(value) | GeneratedValue::Uuid(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn as_date(&self) -> Option<NaiveDate> {
        match self {
            GeneratedValue::Date(value) => Some(*value),
            GeneratedValue::Timestamp(value) => Some(value.date()),
            _ => None,
        }
    }
}

/// Generator registry with plan rules and schema enums.
#[derive(Debug, Clone)]
pub struct GeneratorRegistry {
    column_rules: std::collections::HashMap<String, ColumnGeneratorRule>,
    enums: std::collections::HashMap<String, EnumType>,
}

impl GeneratorRegistry {
    pub fn new(rules: Vec<ColumnGeneratorRule>, schema: &DatabaseSchema) -> Self {
        let mut column_rules = std::collections::HashMap::new();
        for rule in rules {
            let key = key(&rule.schema, &rule.table, &rule.column);
            column_rules.insert(key, rule);
        }

        let mut enums = std::collections::HashMap::new();
        for enum_type in &schema.enums {
            enums.insert(
                enum_key(&enum_type.schema, &enum_type.name),
                enum_type.clone(),
            );
        }

        Self {
            column_rules,
            enums,
        }
    }

    pub fn generate(
        &self,
        schema: &str,
        table: &str,
        column: &Column,
        base_date: NaiveDate,
        rng: &mut impl Rng,
    ) -> Result<GeneratedValue, GenerationError> {
        if let Some(rule) = self.column_rules.get(&key(schema, table, &column.name)) {
            return generate_from_rule(rule, column, base_date, rng);
        }

        if let Some(enum_type) = self.enums.get(&enum_key(
            &column.column_type.udt_schema,
            &column.column_type.udt_name,
        )) {
            return pick_enum(enum_type, rng);
        }

        if column.name == "id" && normalize_type(&column.column_type) == "uuid" {
            return Ok(GeneratedValue::Uuid(random_uuid(rng)));
        }

        let name_lower = column.name.to_lowercase();
        if name_lower.contains("email") {
            return Ok(GeneratedValue::Text(random_email(rng)));
        }

        if name_lower.contains("nome") {
            return Ok(GeneratedValue::Text(random_name(rng)));
        }

        if name_lower == "tipo" {
            let values = ["tarefa", "reuniao", "anotacao"];
            let value = values.choose(rng).unwrap_or(&"tarefa");
            return Ok(GeneratedValue::Text(value.to_string()));
        }

        fallback_for_type(column, base_date, rng)
    }

    pub fn has_rule(&self, schema: &str, table: &str, column: &str) -> bool {
        self.column_rules.contains_key(&key(schema, table, column))
    }

    pub fn rule_for(
        &self,
        schema: &str,
        table: &str,
        column: &str,
    ) -> Option<&ColumnGeneratorRule> {
        self.column_rules.get(&key(schema, table, column))
    }
}

fn generate_from_rule(
    rule: &ColumnGeneratorRule,
    column: &Column,
    base_date: NaiveDate,
    rng: &mut impl Rng,
) -> Result<GeneratedValue, GenerationError> {
    match rule.generator {
        ColumnGenerator::Uuid => Ok(GeneratedValue::Uuid(random_uuid(rng))),
        ColumnGenerator::Email => Ok(GeneratedValue::Text(random_email(rng))),
        ColumnGenerator::Name => Ok(GeneratedValue::Text(random_name(rng))),
        ColumnGenerator::IntRange => {
            let (min, max) = parse_range_i64(rule.params.as_ref(), 0, 10000)?;
            let value = rng.gen_range(min..=max);
            Ok(GeneratedValue::Int(value))
        }
        ColumnGenerator::DateRange => {
            let (min, max) = parse_range_date(
                rule.params.as_ref(),
                base_date,
                base_date + chrono::Duration::days(365),
            )?;
            let span = (max - min).num_days().max(1);
            let offset = rng.gen_range(0..=span) as i64;
            Ok(GeneratedValue::Date(min + chrono::Duration::days(offset)))
        }
        ColumnGenerator::Regex => {
            let value = format!("{}_{:x}", column.name, rng.r#gen::<u32>());
            Ok(GeneratedValue::Text(value))
        }
    }
}

fn fallback_for_type(
    column: &Column,
    base_date: NaiveDate,
    rng: &mut impl Rng,
) -> Result<GeneratedValue, GenerationError> {
    let data_type = normalize_type(&column.column_type);
    match data_type.as_str() {
        "uuid" => Ok(GeneratedValue::Uuid(random_uuid(rng))),
        "smallint" | "integer" | "bigint" => {
            let value = rng.gen_range(1..=100000);
            Ok(GeneratedValue::Int(value))
        }
        "numeric" => {
            if column.column_type.numeric_scale.unwrap_or(0) > 0 {
                let value = rng.gen_range(0.0..=100000.0);
                Ok(GeneratedValue::Float(value))
            } else {
                let value = rng.gen_range(1..=100000);
                Ok(GeneratedValue::Int(value))
            }
        }
        "boolean" => Ok(GeneratedValue::Bool(rng.gen_bool(0.5))),
        "date" => {
            let offset = rng.gen_range(0..=365) as i64;
            Ok(GeneratedValue::Date(
                base_date + chrono::Duration::days(offset),
            ))
        }
        "timestamp with time zone" | "timestamp without time zone" => {
            let offset = rng.gen_range(0..=365) as i64;
            let date = base_date + chrono::Duration::days(offset);
            let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
            Ok(GeneratedValue::Timestamp(NaiveDateTime::new(date, time)))
        }
        "time with time zone" | "time without time zone" => {
            let seconds = rng.gen_range(0..=86399);
            let time = NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0)
                .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
            Ok(GeneratedValue::Time(time))
        }
        _ => {
            let mut value = format!("{}_{}", column.name, rng.r#gen::<u32>());
            if let Some(max_len) = column.column_type.character_max_length {
                value.truncate(max_len as usize);
            }
            Ok(GeneratedValue::Text(value))
        }
    }
}

fn random_uuid(rng: &mut impl Rng) -> String {
    let bytes: [u8; 16] = rng.r#gen();
    uuid::Uuid::from_bytes(bytes).to_string()
}

fn random_email(rng: &mut impl Rng) -> String {
    let user = format!("user{:04}", rng.gen_range(1..=9999));
    format!("{user}@example.com")
}

fn random_name(rng: &mut impl Rng) -> String {
    let first = [
        "Ana", "Bruno", "Carlos", "Daniela", "Eduardo", "Fernanda", "Gustavo", "Helena",
    ];
    let last = [
        "Silva", "Santos", "Oliveira", "Souza", "Lima", "Costa", "Ribeiro", "Almeida",
    ];
    let first = first.choose(rng).unwrap_or(&"Pessoa");
    let last = last.choose(rng).unwrap_or(&"Teste");
    format!("{first} {last}")
}

fn pick_enum(enum_type: &EnumType, rng: &mut impl Rng) -> Result<GeneratedValue, GenerationError> {
    let value = enum_type
        .labels
        .choose(rng)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    Ok(GeneratedValue::Text(value))
}

fn parse_range_i64(
    params: Option<&Value>,
    default_min: i64,
    default_max: i64,
) -> Result<(i64, i64), GenerationError> {
    let min = params
        .and_then(|p| p.get("min"))
        .and_then(|v| v.as_i64())
        .unwrap_or(default_min);
    let max = params
        .and_then(|p| p.get("max"))
        .and_then(|v| v.as_i64())
        .unwrap_or(default_max);
    if min > max {
        return Err(GenerationError::InvalidPlan(
            "int_range min must be <= max".to_string(),
        ));
    }
    Ok((min, max))
}

fn parse_range_date(
    params: Option<&Value>,
    default_min: NaiveDate,
    default_max: NaiveDate,
) -> Result<(NaiveDate, NaiveDate), GenerationError> {
    let min = params
        .and_then(|p| p.get("min"))
        .and_then(|v| v.as_str())
        .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok())
        .unwrap_or(default_min);
    let max = params
        .and_then(|p| p.get("max"))
        .and_then(|v| v.as_str())
        .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok())
        .unwrap_or(default_max);
    if min > max {
        return Err(GenerationError::InvalidPlan(
            "date_range min must be <= max".to_string(),
        ));
    }
    Ok((min, max))
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

fn key(schema: &str, table: &str, column: &str) -> String {
    format!("{schema}.{table}.{column}")
}

fn enum_key(schema: &str, name: &str) -> String {
    format!("{schema}.{name}")
}
