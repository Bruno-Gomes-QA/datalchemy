# Evidencia - pr_task_instrospect-database-schema

## O que mudou
- Workspace criado com crates `datalchemy-core`, `datalchemy-introspect`, `datalchemy-cli`, `datalchemy-eval` e stubs `datalchemy-plan`/`datalchemy-generate`.
- Modelo de schema unificado em `datalchemy-core` com constraints e validacao interna.
- Adapter Postgres refatorado para gerar `DatabaseSchema` padronizado.
- CLI `datalchemy introspect` com registry de runs e logs NDJSON.
- Fixtures e docker compose movidos para `fixtures/sql/postgres` e `docker/compose.postgres.yml`.

## Por que mudou
A entrega segue o PIT: separar contratos, introspeccao e CLI para garantir evolucao incremental, artefatos reprodutiveis e uma base solida para multi-DB e planejamento futuro.

## Como validar
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings

docker compose -f docker/compose.postgres.yml up -d
export TEST_DATABASE_URL="postgres://datalchemy:datalchemy@localhost:5432/datalchemy"

cargo test

cargo run -p datalchemy-cli -- introspect --conn "postgres://datalchemy:datalchemy@localhost:5432/datalchemy" --run-dir runs/

cargo run -p datalchemy-introspect --example dump_json > schema.json
```

## Evidencia
- Nao executei testes neste ambiente (sandbox read-only). Os comandos acima reproduzem a validacao local.
