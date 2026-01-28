use std::collections::{BTreeMap, HashMap};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rand::RngCore;
use serde_json::Value;

use datalchemy_core::{Column, ForeignKey};

use crate::errors::GenerationError;
use crate::foreign::ForeignContext;

pub mod derive;
pub mod domain;
pub mod faker_rs;
pub mod primitives;
pub mod semantic;
pub mod transforms;

pub type RowContext = HashMap<String, GeneratedValue>;

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

/// Context for generators with schema-aware hints.
pub struct GeneratorContext<'a> {
    pub schema: &'a str,
    pub table: &'a str,
    pub column: &'a Column,
    pub foreign_keys: &'a [ForeignKey],
    pub base_date: NaiveDate,
    pub row_index: u64,
    pub enum_values: Option<&'a [String]>,
    pub row: &'a RowContext,
    pub foreign: Option<&'a mut dyn ForeignContext>,
    pub generator_locale: Option<&'a str>,
}

/// Context for transforms.
pub struct TransformContext<'a> {
    pub schema: &'a str,
    pub table: &'a str,
    pub column: &'a Column,
    pub base_date: NaiveDate,
    pub row_index: u64,
    pub strict: bool,
}

/// Generator trait resolved by string identifiers.
pub trait Generator: Send + Sync {
    fn id(&self) -> &'static str;
    fn generate(
        &self,
        ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn RngCore,
    ) -> Result<GeneratedValue, GenerationError>;
    fn pii_tags(&self) -> &'static [&'static str] {
        &[]
    }
}

/// Transform trait resolved by string identifiers.
pub trait Transform: Send + Sync {
    fn id(&self) -> &'static str;
    fn apply(
        &self,
        input: GeneratedValue,
        ctx: &TransformContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn RngCore,
    ) -> Result<GeneratedValue, GenerationError>;
}

/// Registry for generators and transforms.
#[derive(Default)]
pub struct GeneratorRegistry {
    generators: BTreeMap<&'static str, Box<dyn Generator>>,
    transforms: BTreeMap<&'static str, Box<dyn Transform>>,
}

impl GeneratorRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        primitives::register(&mut registry);
        transforms::register(&mut registry);
        semantic::register(&mut registry);
        derive::register(&mut registry);
        domain::register(&mut registry);
        faker_rs::register(&mut registry);
        registry
    }

    pub fn register_generator(&mut self, generator: Box<dyn Generator>) {
        self.generators.insert(generator.id(), generator);
    }

    pub fn register_transform(&mut self, transform: Box<dyn Transform>) {
        self.transforms.insert(transform.id(), transform);
    }

    pub fn generator(&self, id: &str) -> Option<&dyn Generator> {
        self.generators.get(id).map(|generator| generator.as_ref())
    }

    pub fn generator_ids(&self) -> Vec<&'static str> {
        self.generators.keys().copied().collect()
    }

    pub fn transform(&self, id: &str) -> Option<&dyn Transform> {
        self.transforms.get(id).map(|transform| transform.as_ref())
    }
}
