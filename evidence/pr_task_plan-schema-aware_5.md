# Evidencia — pr_task_plan-schema-aware_5

## O que mudou
- Implementado avaliador de datasets no `crates/datalchemy-eval` (loader CSV, validadores NOT NULL/PK/UNIQUE/FK/CHECK subset, `metrics.json` e `report.md`).
- Adicionado exemplo `crates/datalchemy-eval/examples/evaluate_run.rs` para rodar a avaliacao em um run.
- Atualizado `datalchemy_structure.md` com a nova responsabilidade do crate de avaliacao.

## Por que mudou
- Cumprir o Plan 5: validar dados gerados com metricas e relatorio deterministico, comparavel entre runs.

## Como validar
```bash
cargo check -p datalchemy-eval

cargo run -p datalchemy-eval --example evaluate_run -- \
  --plan plans/examples/minimal.plan.json \
  --schema crates/datalchemy-introspect/tests/golden/postgres_minimal.schema.json \
  --run out/2026-01-11T18-17-35Z__run_6197121f-bae1-4321-92f9-54add577bc8e
```

## Testes executados
- `cargo check -p datalchemy-eval`
