//! Database introspection adapters.

pub mod adapter;
pub mod options;
pub mod postgres;

pub use adapter::Adapter;
pub use options::IntrospectOptions;
pub use postgres::{PostgresAdapter, introspect_postgres, introspect_postgres_with_options};

pub use datalchemy_core::DatabaseSchema;
