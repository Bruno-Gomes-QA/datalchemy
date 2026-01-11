//! Database introspection adapters.

pub mod adapter;
pub mod options;
pub mod postgres;

pub use adapter::Adapter;
pub use options::IntrospectOptions;
pub use postgres::{introspect_postgres, introspect_postgres_with_options, PostgresAdapter};

pub use datalchemy_core::DatabaseSchema;
