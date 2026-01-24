# Evidencia — pr_task_M0_baseline

## O que mudou
- Expandido `GenerationReport` com contadores deterministas (usage, fallback, heuristics, PII, warnings).
- Warnings padronizados via `tracing` + agregacao em `generation_report.json`.
- Strict mode passou a ser configuravel via `plan.options.strict` (default false).
- Regras de fallback/heuristica/unknown generator ajustadas para strict vs non-strict.

## Por que mudou
- Implementar guard rails do M0 (strict, warnings, cobertura) e garantir observabilidade sem vazar PII.

## Como validar
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

./scripts/postgres_docker.sh
cargo run -p datalchemy-cli -- introspect --conn "$DATABASE_URL" --run-dir runs/
RUN_DIR=$(ls -1d runs/* | sort | tail -n 1)

cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/

OUT_DIR=$(ls -1d out/* | sort | tail -n 1)
cat "$OUT_DIR/generation_report.json"
```

## Testes executados
- `cargo test` (falhou no teste de integracao do introspect por falta de `DATABASE_URL`/`TEST_DATABASE_URL`).
