# F7 — Docs + lista de generators + testes de contrato

> Fechar a integração com documentação e ferramentas de inspeção.



## Objetivo
Deixar a integração fake-rs “usável” por qualquer dev/IA:

- comando para listar IDs disponíveis (`--list-generators`)
- docs de como escolher IDs e params
- testes de contrato mínimos (catalog compila, ids únicos, params validados)

## Entregas

1) **CLI/Example: listar generators**
- `cargo run -p datalchemy-generate --example list_generators`
- Saída ordenada.

2) **Docs**
- `docs/faker_integration.md`
  - `semantic.*` vs `faker.*`
  - locale
  - params avançados
  - exemplos

3) **Testes**
- ids únicos e ordenados
- invalid param errors
- unknown id errors

## Critérios de aceite
- [ ] docs claras e exemplos funcionando
- [ ] list_generators imprime ids
- [ ] tests passam

## Validação
```bash
# Qualidade
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

# E2E Postgres
./scripts/postgres_docker.sh

# Introspecção (schema.json) — usa DATABASE_URL
cargo run -p datalchemy-cli -- introspect \
  --conn "$DATABASE_URL" \
  --run-dir runs/

RUN_DIR=$(ls -1d runs/* | sort | tail -n 1)
echo "RUN_DIR=$RUN_DIR"

# Validar plan (se existir example validate)
cargo run -p datalchemy-plan --example validate_plan -- \
  plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" || true

# Gerar CSV
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/

OUT_DIR=$(ls -1d out/* | sort | tail -n 1)
echo "OUT_DIR=$OUT_DIR"

# Avaliar
cargo run -p datalchemy-eval --example evaluate_run -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --run "$OUT_DIR"
```
