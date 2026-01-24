# Evidencia — pr_task_M1_registry_refactor

## O que mudou
- Registry de generators/transforms por ID string em `datalchemy-generate`.
- Refactor de `generators/` em `primitives/`, `transforms/`, `semantic/`.
- `plan.json` atualizado para aceitar `generator` como string e `transforms` por coluna.
- `schemas/plan.schema.json` regenerado com o novo contrato.
- `plans/examples/minimal.plan.json` atualizado para novos IDs.

## Por que mudou
- Preparar extensibilidade por IDs (breaking change do Plan) e padronizar registry deterministico.

## Como validar
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

cargo run -p datalchemy-plan --example emit_plan_json_schema > schemas/plan.schema.json
cargo run -p datalchemy-plan --example validate_plan -- \
  plans/examples/minimal.plan.json \
  --schema crates/datalchemy-introspect/tests/golden/postgres_minimal.schema.json
```

## Testes executados
- `cargo test` (falhou no teste de integracao do introspect por falta de `DATABASE_URL`/`TEST_DATABASE_URL`).
