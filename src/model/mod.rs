//! Data structures that represent database schemas in memory.

mod constraints;
mod schema;
mod types;

pub use constraints::{
    CheckConstraint, FkAction, FkMatchType, ForeignKey, Index, PrimaryKey, UniqueConstraint,
};
pub use schema::{Column, DatabaseSchema, Schema, Table, TableKind};
pub use types::{ColumnType, EnumType, GeneratedExpression, GeneratedKind, IdentityGeneration};
