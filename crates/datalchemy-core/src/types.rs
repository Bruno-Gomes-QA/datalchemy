use serde::{Deserialize, Serialize};

/// Formatted and raw Postgres type metadata for a column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnType {
    /// User-friendly formatted type (e.g. `character varying(255)`).
    pub data_type: String,
    /// Namespace of the underlying type.
    pub udt_schema: String,
    /// Name of the underlying type.
    pub udt_name: String,
    pub character_max_length: Option<i32>,
    pub numeric_precision: Option<i32>,
    pub numeric_scale: Option<i32>,
    pub collation: Option<String>,
}

/// Identity generation strategy for columns using `GENERATED ... AS IDENTITY`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityGeneration {
    Always,
    ByDefault,
}

/// Kind of generated column supported by Postgres.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedKind {
    Stored,
}

/// Information about generated column expressions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedExpression {
    pub kind: GeneratedKind,
    pub expression: Option<String>,
}

/// Representation of Postgres enum types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumType {
    pub schema: String,
    pub name: String,
    pub labels: Vec<String>,
}
