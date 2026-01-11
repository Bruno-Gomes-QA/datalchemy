use anyhow::{anyhow, Context, Result};
use datalchemy_core::{Constraint, FkAction, FkMatchType, IdentityGeneration, TableKind};
use datalchemy_introspect::{introspect_postgres_with_options, IntrospectOptions};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, fs};

const FIXTURE_PATHS: &[&str] = &[
    "fixtures/sql/postgres/001_schema.sql",
    "fixtures/sql/postgres/002_data.sql",
];

fn database_url() -> Result<String> {
    env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .context("set TEST_DATABASE_URL or DATABASE_URL for integration tests")
}

async fn run_fixture(pool: &PgPool, path: &str) -> Result<()> {
    let script = fs::read_to_string(path).with_context(|| format!("reading fixture {path}"))?;

    for statement in script.split(';') {
        let sql = statement.trim();
        if sql.is_empty() {
            continue;
        }

        sqlx::query(sql)
            .execute(pool)
            .await
            .with_context(|| format!("executing fixture {path}"))?;
    }

    Ok(())
}

async fn reset_fixtures(pool: &PgPool) -> Result<()> {
    for path in FIXTURE_PATHS {
        run_fixture(pool, path).await?;
    }
    Ok(())
}

#[tokio::test]
async fn introspects_schema_with_constraints_and_indexes() -> Result<()> {
    let db_url = database_url()?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&db_url)
        .await
        .context("connecting to Postgres")?;

    reset_fixtures(&pool).await?;

    let snapshot = introspect_postgres_with_options(&pool, IntrospectOptions::default()).await?;

    assert!(
        !snapshot
            .schemas
            .iter()
            .any(|schema| schema.name == "pg_catalog"),
        "system schemas should be filtered by default"
    );

    let app_schema = snapshot
        .schemas
        .iter()
        .find(|schema| schema.name == "app")
        .ok_or_else(|| anyhow!("expected 'app' schema"))?;

    let users = app_schema
        .tables
        .iter()
        .find(|table| table.name == "users")
        .ok_or_else(|| anyhow!("expected users table"))?;
    assert_eq!(users.kind, TableKind::Table);

    let user_columns: Vec<&str> = users.columns.iter().map(|col| col.name.as_str()).collect();
    assert_eq!(
        user_columns,
        vec![
            "id",
            "email",
            "full_name",
            "age",
            "status",
            "created_at",
            "bio"
        ]
    );

    let id_col = users
        .columns
        .iter()
        .find(|col| col.name == "id")
        .ok_or_else(|| anyhow!("id column missing"))?;
    assert_eq!(id_col.identity, Some(IdentityGeneration::Always));

    let pk = users
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            Constraint::PrimaryKey(pk) => Some(pk),
            _ => None,
        })
        .ok_or_else(|| anyhow!("primary key missing"))?;
    assert_eq!(pk.columns, vec!["id"]);

    let unique_names: Vec<&str> = users
        .constraints
        .iter()
        .filter_map(|constraint| match constraint {
            Constraint::Unique(unique) => unique.name.as_deref(),
            _ => None,
        })
        .collect();
    assert!(unique_names.contains(&"users_email_key"));

    let check_exprs: Vec<&str> = users
        .constraints
        .iter()
        .filter_map(|constraint| match constraint {
            Constraint::Check(check) => Some(check.expression.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        check_exprs.iter().any(|expr| expr.contains("age >= 0")),
        "age constraint should be captured"
    );

    let orders = app_schema
        .tables
        .iter()
        .find(|table| table.name == "orders")
        .ok_or_else(|| anyhow!("expected orders table"))?;

    let fk = orders
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            Constraint::ForeignKey(fk) if fk.name.as_deref() == Some("orders_user_fk") => {
                Some(fk)
            }
            _ => None,
        })
        .ok_or_else(|| anyhow!("orders_user_fk missing"))?;
    assert_eq!(fk.columns, vec!["user_id"]);
    assert_eq!(fk.referenced_table, "users");
    assert_eq!(fk.referenced_columns, vec!["id"]);
    assert_eq!(fk.on_delete, FkAction::Cascade);
    assert_eq!(fk.on_update, FkAction::NoAction);
    assert_eq!(fk.match_type, FkMatchType::Simple);

    let def_unique = orders
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            Constraint::Unique(unique)
                if unique.name.as_deref() == Some("orders_user_status_unique") =>
            {
                Some(unique)
            }
            _ => None,
        })
        .ok_or_else(|| anyhow!("orders_user_status_unique missing"))?;
    assert!(def_unique.is_deferrable);
    assert!(def_unique.initially_deferred);

    let index_names: Vec<&str> = users.indexes.iter().map(|idx| idx.name.as_str()).collect();
    assert!(index_names.contains(&"users_pkey"));
    assert!(index_names.contains(&"users_email_key"));
    assert!(index_names.contains(&"idx_users_status"));

    let view = app_schema
        .tables
        .iter()
        .find(|table| table.name == "active_users")
        .ok_or_else(|| anyhow!("expected active_users view"))?;
    assert_eq!(view.kind, TableKind::View);

    assert!(snapshot.enums.iter().any(|en| {
        en.schema == "app" && en.name == "status" && en.labels == vec!["pending", "active", "disabled"]
    }));

    Ok(())
}
