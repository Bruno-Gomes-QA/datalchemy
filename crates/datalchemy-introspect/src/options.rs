/// Options that control how introspection behaves.
#[derive(Debug, Clone)]
pub struct IntrospectOptions {
    pub include_system_schemas: bool,
    pub include_views: bool,
    pub include_materialized_views: bool,
    pub include_foreign_tables: bool,
    pub include_indexes: bool,
    pub include_comments: bool,
    pub schemas: Option<Vec<String>>,
}

impl Default for IntrospectOptions {
    fn default() -> Self {
        Self {
            include_system_schemas: false,
            include_views: true,
            include_materialized_views: true,
            include_foreign_tables: true,
            include_indexes: true,
            include_comments: true,
            schemas: None,
        }
    }
}
