//! SQLite catalog queries using PRAGMA statements.

use sqlx::{Row, SqlitePool};

use datalchemy_core::Result;

fn db_err(err: sqlx::Error) -> datalchemy_core::Error {
    datalchemy_core::Error::Db(err.to_string())
}

/// Raw column info from `PRAGMA table_info`.
pub struct RawColumn {
    pub cid: i64,
    pub name: String,
    pub col_type: String,
    pub notnull: bool,
    pub dflt_value: Option<String>,
    pub pk: bool,
}

/// Raw foreign key info from `PRAGMA foreign_key_list`.
pub struct RawForeignKey {
    pub id: i64,
    pub seq: i64,
    pub table: String,
    pub from: String,
    pub to: String,
    pub on_update: String,
    pub on_delete: String,
    pub r#match: String,
}

/// Raw index info from `PRAGMA index_list`.
pub struct RawIndex {
    pub name: String,
    pub unique: bool,
    pub origin: String,
}

/// List all user tables (excluding internal SQLite tables).
pub async fn list_tables(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT name FROM sqlite_master
         WHERE type = 'table'
         AND name NOT LIKE 'sqlite_%'
         ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let names = rows
        .into_iter()
        .map(|r| r.try_get::<String, _>("name").map_err(db_err))
        .collect::<Result<Vec<_>>>()?;

    Ok(names)
}

/// List columns for a table via `PRAGMA table_info`.
pub async fn list_columns(pool: &SqlitePool, table: &str) -> Result<Vec<RawColumn>> {
    let query = format!("PRAGMA table_info(\"{}\")", table);
    let rows = sqlx::query(&query).fetch_all(pool).await.map_err(db_err)?;

    let mut columns = Vec::new();
    for row in rows {
        columns.push(RawColumn {
            cid: row.try_get::<i64, _>("cid").map_err(db_err)?,
            name: row.try_get::<String, _>("name").map_err(db_err)?,
            col_type: row.try_get::<String, _>("type").map_err(db_err)?,
            notnull: row.try_get::<bool, _>("notnull").map_err(db_err)?,
            dflt_value: row
                .try_get::<Option<String>, _>("dflt_value")
                .map_err(db_err)?,
            pk: row.try_get::<bool, _>("pk").map_err(db_err)?,
        });
    }
    Ok(columns)
}

/// List foreign keys for a table via `PRAGMA foreign_key_list`.
pub async fn list_foreign_keys(pool: &SqlitePool, table: &str) -> Result<Vec<RawForeignKey>> {
    let query = format!("PRAGMA foreign_key_list(\"{}\")", table);
    let rows = sqlx::query(&query).fetch_all(pool).await.map_err(db_err)?;

    let mut fks = Vec::new();
    for row in rows {
        fks.push(RawForeignKey {
            id: row.try_get::<i64, _>("id").map_err(db_err)?,
            seq: row.try_get::<i64, _>("seq").map_err(db_err)?,
            table: row.try_get::<String, _>("table").map_err(db_err)?,
            from: row.try_get::<String, _>("from").map_err(db_err)?,
            to: row.try_get::<String, _>("to").map_err(db_err)?,
            on_update: row.try_get::<String, _>("on_update").map_err(db_err)?,
            on_delete: row.try_get::<String, _>("on_delete").map_err(db_err)?,
            r#match: row.try_get::<String, _>("match").map_err(db_err)?,
        });
    }
    Ok(fks)
}

/// List indexes for a table via `PRAGMA index_list`.
pub async fn list_indexes(pool: &SqlitePool, table: &str) -> Result<Vec<RawIndex>> {
    let query = format!("PRAGMA index_list(\"{}\")", table);
    let rows = sqlx::query(&query).fetch_all(pool).await.map_err(db_err)?;

    let mut indexes = Vec::new();
    for row in rows {
        indexes.push(RawIndex {
            name: row.try_get::<String, _>("name").map_err(db_err)?,
            unique: row.try_get::<bool, _>("unique").map_err(db_err)?,
            origin: row.try_get::<String, _>("origin").map_err(db_err)?,
        });
    }
    Ok(indexes)
}

/// List columns of an index via `PRAGMA index_info`.
pub async fn list_index_columns(pool: &SqlitePool, index_name: &str) -> Result<Vec<String>> {
    let query = format!("PRAGMA index_info(\"{}\")", index_name);
    let rows = sqlx::query(&query).fetch_all(pool).await.map_err(db_err)?;

    let mut cols = Vec::new();
    for row in rows {
        let name: String = row.try_get("name").map_err(db_err)?;
        cols.push(name);
    }
    Ok(cols)
}
