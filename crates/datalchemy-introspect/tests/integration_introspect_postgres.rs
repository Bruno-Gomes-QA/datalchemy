use anyhow::{Context, Result, anyhow};
use datalchemy_core::{Constraint, FkAction, FkMatchType, TableKind};
use datalchemy_introspect::{IntrospectOptions, introspect_postgres_with_options};
use jsonschema::{Draft, JSONSchema};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::path::{Path, PathBuf};
use std::{env, fs};

fn database_url() -> Result<String> {
    env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .context("set TEST_DATABASE_URL or DATABASE_URL for integration tests")
}

async fn run_fixture(pool: &PgPool, path: &Path) -> Result<()> {
    let script =
        fs::read_to_string(path).with_context(|| format!("reading fixture {}", path.display()))?;

    for statement in script.split(';') {
        let sql = statement.trim();
        if sql.is_empty() {
            continue;
        }

        sqlx::query(sql)
            .execute(pool)
            .await
            .with_context(|| format!("executing fixture {}", path.display()))?;
    }

    Ok(())
}

async fn reset_fixtures(pool: &PgPool) -> Result<()> {
    for path in fixture_paths()? {
        run_fixture(pool, &path).await?;
    }
    Ok(())
}

fn fixture_paths() -> Result<Vec<PathBuf>> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sql/postgres");
    let mut paths = Vec::new();

    collect_sql_files(&base.join("tables"), &mut paths)?;
    collect_sql_files(&base.join("data"), &mut paths)?;

    Ok(paths)
}

fn collect_sql_files(dir: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading fixtures dir {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("sql"))
        .collect();

    entries.sort();
    output.extend(entries);
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

    let crm_schema = snapshot
        .schemas
        .iter()
        .find(|schema| schema.name == "crm")
        .ok_or_else(|| anyhow!("expected 'crm' schema"))?;

    let usuarios = crm_schema
        .tables
        .iter()
        .find(|table| table.name == "usuarios")
        .ok_or_else(|| anyhow!("expected usuarios table"))?;
    assert_eq!(usuarios.kind, TableKind::Table);

    let user_columns: Vec<&str> = usuarios
        .columns
        .iter()
        .map(|col| col.name.as_str())
        .collect();
    assert_eq!(
        user_columns,
        vec![
            "id",
            "nome",
            "email",
            "telefone",
            "ativo",
            "data_criacao",
            "data_atualizacao"
        ]
    );

    let user_unique = usuarios
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            Constraint::Unique(unique) => unique.name.as_deref(),
            _ => None,
        })
        .ok_or_else(|| anyhow!("usuarios unique constraint missing"))?;
    assert_eq!(user_unique, "usuarios_email_unique");

    let contatos = crm_schema
        .tables
        .iter()
        .find(|table| table.name == "contatos")
        .ok_or_else(|| anyhow!("expected contatos table"))?;

    let contato_fk = contatos
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            Constraint::ForeignKey(fk) if fk.columns == ["empresa_id"] => Some(fk),
            _ => None,
        })
        .ok_or_else(|| anyhow!("contatos empresa fk missing"))?;
    assert_eq!(contato_fk.referenced_table, "empresas");

    let oportunidades = crm_schema
        .tables
        .iter()
        .find(|table| table.name == "oportunidades")
        .ok_or_else(|| anyhow!("expected oportunidades table"))?;

    let oportunidade_fk = oportunidades
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            Constraint::ForeignKey(fk) if fk.columns == ["etapa_id"] => Some(fk),
            _ => None,
        })
        .ok_or_else(|| anyhow!("oportunidades etapa fk missing"))?;
    assert_eq!(oportunidade_fk.referenced_table, "etapas_funil");
    assert_eq!(oportunidade_fk.match_type, FkMatchType::Simple);
    assert_eq!(oportunidade_fk.on_delete, FkAction::NoAction);

    let checks: Vec<&str> = oportunidades
        .constraints
        .iter()
        .filter_map(|constraint| match constraint {
            Constraint::Check(check) => Some(check.expression.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        checks.iter().any(|expr| expr.contains("valor_estimado")),
        "expected valor_estimado check"
    );

    let produtos = crm_schema
        .tables
        .iter()
        .find(|table| table.name == "produtos")
        .ok_or_else(|| anyhow!("expected produtos table"))?;

    let produto_indexes: Vec<&str> = produtos
        .indexes
        .iter()
        .map(|idx| idx.name.as_str())
        .collect();
    assert!(produto_indexes.contains(&"produtos_sku_unique"));

    assert!(snapshot.enums.iter().any(|en| {
        en.schema == "crm"
            && en.name == "status_lead"
            && en.labels == vec!["novo", "qualificado", "perdido"]
    }));

    let schema_schema_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/schema.schema.json");
    let schema_schema_text = fs::read_to_string(&schema_schema_path)
        .with_context(|| format!("reading {}", schema_schema_path.display()))?;
    let schema_schema_json: serde_json::Value = serde_json::from_str(&schema_schema_text)?;
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema_schema_json)
        .map_err(|err| anyhow!("invalid schema schema: {err}"))?;

    let instance = serde_json::to_value(&snapshot)?;
    if let Err(errors) = compiled.validate(&instance) {
        let details = errors
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(anyhow!("schema.json failed validation: {details}"));
    }

    let golden_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/postgres_minimal.schema.json");
    let golden_text = fs::read_to_string(&golden_path)
        .with_context(|| format!("reading {}", golden_path.display()))?;
    let rendered = serde_json::to_string_pretty(&snapshot)?;
    assert_eq!(rendered, golden_text.trim_end());

    Ok(())
}
