//! Public API surface for the datalchemy library.
//!
//! Exposes stable types and entrypoints while keeping Postgres-specific
//! implementation details internal.

pub mod error;
pub mod introspect;
pub mod model;
mod utils;

pub use error::{Error, Result};
pub use introspect::{IntrospectOptions, introspect_postgres, introspect_postgres_with_options};
pub use model::{
    CheckConstraint, Column, ColumnType, DatabaseSchema, EnumType, FkAction, FkMatchType,
    ForeignKey, GeneratedExpression, GeneratedKind, IdentityGeneration, Index, PrimaryKey, Schema,
    Table, TableKind, UniqueConstraint,
};
