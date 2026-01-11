# Evidencia - pr_task_instropect-database-schema-2

## O que mudou
- Contrato `schema.json` atualizado para `schema_version = 0.2` e campo `schema_fingerprint`.
- JSON Schema oficial gerado em `schemas/schema.schema.json` via `schemars`.
- Fixtures reestruturadas para CRM em portugues, com SQL por tabela e dados separados.
- Script `scripts/postgres_docker.sh` para subir Postgres local e aplicar fixtures.
- Golden file criado em `crates/datalchemy-introspect/tests/golden/postgres_minimal.schema.json`.
- Testes de contrato: validacao JSON Schema + comparacao com golden file.
- Documentacao adicionada em `docs/schema_json.md` e README atualizado.

## Por que mudou
Estabilizar o contrato do `schema.json` e padronizar um ambiente local de testes independente, com fixtures consistentes e scripts repetiveis.

## Como validar
```bash
export DATABASE_URL="postgres://datalchemy:datalchemy@localhost:5432/datalchemy_crm"
export TEST_DATABASE_URL="postgres://datalchemy:datalchemy@localhost:5432/datalchemy_crm"

# subir Postgres local e aplicar fixtures
./scripts/postgres_docker.sh

# regenerar JSON Schema
cargo run -p datalchemy-core --example emit_schema_json_schema > schemas/schema.schema.json

# gerar golden file
cargo run -p datalchemy-introspect --example dump_json > crates/datalchemy-introspect/tests/golden/postgres_minimal.schema.json

# rodar testes
cargo test
```

## Evidencia
- `cargo run -p datalchemy-core --example emit_schema_json_schema` gerou `schemas/schema.schema.json`.
- `cargo run -p datalchemy-introspect --example dump_json` gerou golden file.
- `cargo test` passou localmente com `DATABASE_URL`/`TEST_DATABASE_URL` apontando para `datalchemy_crm`.
