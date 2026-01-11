use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::constraints::{Constraint, Index};
use crate::types::{ColumnType, EnumType, GeneratedExpression, IdentityGeneration};

/// Top-level schema snapshot for a database.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DatabaseSchema {
    /// Contract version for this schema format.
    pub schema_version: String,
    /// Database engine identifier (e.g. `postgres`).
    pub engine: String,
    /// Database name when available.
    pub database: Option<String>,
    /// Schemas captured from the database.
    pub schemas: Vec<Schema>,
    /// Enum types captured across schemas.
    pub enums: Vec<EnumType>,
    /// Optional fingerprint of the schema for cache/validation purposes.
    pub schema_fingerprint: Option<String>,
}

/// A Postgres namespace containing tables and related objects.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Schema {
    pub name: String,
    pub tables: Vec<Table>,
}

/// A table-like object (table, view, materialized view, foreign table, partitioned table).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Table {
    pub name: String,
    pub kind: TableKind,
    pub comment: Option<String>,
    pub columns: Vec<Column>,
    pub constraints: Vec<Constraint>,
    pub indexes: Vec<Index>,
}

/// Kind of table represented in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TableKind {
    Table,
    PartitionedTable,
    View,
    MaterializedView,
    ForeignTable,
    Other(String),
}

/// Column metadata for a table-like object.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Column {
    pub ordinal_position: i16,
    pub name: String,
    pub column_type: ColumnType,
    pub is_nullable: bool,
    pub default: Option<String>,
    pub identity: Option<IdentityGeneration>,
    pub generated: Option<GeneratedExpression>,
    pub comment: Option<String>,
}
