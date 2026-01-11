# pr_task_instrospect-database-schema

## Visao geral
Esta entrega reorganiza o repositorio em workspace e entrega o pipeline de introspeccao com registry de runs.

- `datalchemy-core`: contratos do schema, validacao e redaction.
- `datalchemy-introspect`: adapter Postgres e queries.
- `datalchemy-cli`: comando `datalchemy introspect` + registry de runs.
- `datalchemy-eval`: metricas minimas do schema.

## Como funciona o fluxo de introspeccao
1) O CLI valida argumentos e detecta a engine pelo scheme da URL.
2) `registry::start_run` cria a pasta `runs/<timestamp>__run_<id>/` e grava `config.json`.
3) O adapter Postgres introspecta o schema e gera `schema.json` deterministico.
4) O schema e validado internamente (FK/PK/UNIQUE referenciando colunas existentes).
5) O coletor de metricas gera `metrics.json` e registra o grafo de FK.
6) Logs estruturados sao gravados em `logs.ndjson`.

## Arquivos gerados por run
- `schema.json`: snapshot do schema com enums, tabelas, colunas e constraints.
- `config.json`: engine, opcoes, versao do schema e connection redigida.
- `logs.ndjson`: eventos `run_started`, `introspection_started`, `schema_written`, `metrics_written`, `run_finished`.
- `metrics.json`: contagens, cobertura e status do grafo de FK.

## Como executar o CLI
```bash
cargo run -p datalchemy-cli -- introspect --conn "postgres://user:pass@localhost:5432/db" --run-dir runs/
```

## Como rodar o exemplo dump_json
```bash
export DATABASE_URL="postgres://user:pass@localhost:5432/db"
cargo run -p datalchemy-introspect --example dump_json > schema.json
```

## Como rodar testes
### Subir Postgres via Docker
```bash
docker compose -f docker/compose.postgres.yml up -d
export TEST_DATABASE_URL="postgres://datalchemy:datalchemy@localhost:5432/datalchemy"
```

### Rodar testes do workspace
```bash
cargo test
```

### Encerrar o Postgres
```bash
docker compose -f docker/compose.postgres.yml down
```
