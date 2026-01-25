# Evidencia — pr_task_M2_primitives_transforms

## O que mudou
- Implementado catalogo de primitives (bool, int/float ranges, decimal, text pattern/lorem, uuid, date/time/timestamp, enum).
- Implementado catalogo de transforms (null_rate, truncate, format, prefix_suffix, casing, weighted_choice).
- Atualizado plan para aceitar `transforms` e adicionado `plans/examples/m2_primitives.plan.json`.
- Criados testes unitarios de primitives/transforms.

## Por que mudou
- Cobrir o baseline de geracao simples (80%) e introduzir pipeline de transforms por coluna.

## Como validar
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

./scripts/postgres_docker.sh
cargo run -p datalchemy-cli -- introspect --conn "$DATABASE_URL" --run-dir runs/
RUN_DIR=$(ls -1d runs/* | sort | tail -n 1)

cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/m2_primitives.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/
```

## Testes executados
- `cargo test` (falhou no teste de integracao do introspect por falta de `DATABASE_URL`/`TEST_DATABASE_URL`).
