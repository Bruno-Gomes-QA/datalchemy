use serde::{Deserialize, Serialize};

/// Primary key definition preserving column order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryKey {
    pub name: Option<String>,
    pub columns: Vec<String>,
}

/// Unique constraint definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueConstraint {
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub is_deferrable: bool,
    pub initially_deferred: bool,
}

/// Check constraint definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConstraint {
    pub name: Option<String>,
    pub expression: String,
}

/// Foreign key action semantics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FkAction {
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
    Unknown,
}

/// Foreign key match semantics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FkMatchType {
    Full,
    Partial,
    Simple,
    Unknown,
}

/// Foreign key definition preserving column ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKey {
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub referenced_schema: String,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_update: FkAction,
    pub on_delete: FkAction,
    pub match_type: FkMatchType,
    pub is_deferrable: bool,
    pub initially_deferred: bool,
}

/// Index definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub name: String,
    pub is_unique: bool,
    pub is_primary: bool,
    pub is_valid: bool,
    pub method: String,
    pub definition: String,
}

/// Table-level constraint definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Constraint {
    PrimaryKey(PrimaryKey),
    ForeignKey(ForeignKey),
    Unique(UniqueConstraint),
    Check(CheckConstraint),
}
