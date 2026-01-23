# Evidencia — pr_task_plan-schema-aware_3

## O que mudou
- Implementado o contrato do `plan.json` no crate `datalchemy-plan` com modelos, enums e JSON Schema via `schemars`.
- Criado `schemas/plan.schema.json` gerado pelo exemplo `emit_plan_json_schema`.
- Adicionado validador estrutural (JSON Schema) e schema-aware (referencias e compatibilidade de tipos).
- Criados exemplos e fixtures:
  - `plans/examples/minimal.plan.json`
  - `crates/datalchemy-plan/examples/emit_plan_json_schema.rs`
  - `crates/datalchemy-plan/examples/validate_plan.rs`
- Criados testes:
  - `crates/datalchemy-plan/tests/plan_json_schema.rs`
  - `crates/datalchemy-plan/tests/plan_validation.rs`

## Por que mudou
- Cumprir o Plan 3: estabelecer contrato versionado do plano e validar contra `schema.json` real.

## Como validar
```bash
cargo run -p datalchemy-plan --example emit_plan_json_schema > schemas/plan.schema.json
cargo run -p datalchemy-plan --example validate_plan -- plans/examples/minimal.plan.json --schema crates/datalchemy-introspect/tests/golden/postgres_minimal.schema.json
cargo test -p datalchemy-plan
```

## Testes executados
- `cargo test -p datalchemy-plan`
