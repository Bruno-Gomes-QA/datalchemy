use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;

use datalchemy::introspect_postgres;

#[tokio::main]
async fn main() -> Result<()> {
    let db_url = std::env::var("DATABASE_URL").context("set DATABASE_URL")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&db_url)
        .await
        .context("failed to connect to Postgres")?;

    let snapshot = introspect_postgres(&pool).await?;
    println!("{}", serde_json::to_string_pretty(&snapshot)?);

    Ok(())
}
