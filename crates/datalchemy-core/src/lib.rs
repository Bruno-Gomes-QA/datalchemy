//! Core contracts and helpers for Datalchemy.
//!
//! This crate defines the canonical schema types, validation helpers, and
//! utilities shared across adapters and the CLI.

pub mod constraints;
pub mod error;
pub mod graph;
pub mod redaction;
pub mod schema;
pub mod types;
pub mod validation;

pub use constraints::{
    CheckConstraint, Constraint, FkAction, FkMatchType, ForeignKey, Index, PrimaryKey,
    UniqueConstraint,
};
pub use error::{Error, Result};
pub use graph::{build_fk_graph_report, FkGraphReport, FkGraphSummary};
pub use redaction::{redact_connection_string, RedactedConnection};
pub use schema::{Column, DatabaseSchema, Schema, Table, TableKind};
pub use types::{ColumnType, EnumType, GeneratedExpression, GeneratedKind, IdentityGeneration};
pub use validation::validate_schema;

/// Current schema contract version for `schema.json` artifacts.
pub const SCHEMA_VERSION: &str = "0.1";
