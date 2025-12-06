use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::types::{ColumnType, EnumType, GeneratedExpression, IdentityGeneration};
use super::{CheckConstraint, ForeignKey, Index, PrimaryKey, UniqueConstraint};

/// Top-level schema snapshot for a database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    /// Database name reported by `current_database()`.
    pub database: String,
    /// Schemas keyed by name for deterministic output.
    pub schemas: BTreeMap<String, Schema>,
    /// Enum types available in the database.
    pub enums: Vec<EnumType>,
}

/// A Postgres namespace containing tables and related objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Tables keyed by name for deterministic ordering.
    pub tables: BTreeMap<String, Table>,
}

/// A table-like object (table, view, materialized view, foreign table, partitioned table).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub kind: TableKind,
    pub comment: Option<String>,
    pub columns: Vec<Column>,
    pub primary_key: Option<PrimaryKey>,
    pub uniques: Vec<UniqueConstraint>,
    pub checks: Vec<CheckConstraint>,
    pub foreign_keys: Vec<ForeignKey>,
    pub indexes: Vec<Index>,
}

/// Kind of table represented in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TableKind {
    Table,
    PartitionedTable,
    View,
    MaterializedView,
    ForeignTable,
    Other(String),
}

/// Column metadata for a table-like object.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
