# F6 — Substituir geradores atuais pelo backend fake-rs

> Descartar o pouco que existe hoje e ganhar cobertura imediata.



## Objetivo
Trocar o comportamento atual do `GeneratorRegistry` (o mod.rs antigo com random_email/random_name + heurísticas)
por:

- resolução por `generator.id` (F1)
- execução via adapter fake-rs (F2)
- catálogo grande (F3)
- tipos+params (F4)
- locales (F5)

## Passos (sem quebrar o repo)

1) **Congelar compat**
- antes de remover, rodar E2E e guardar outputs em evidence.

2) **Remover heurísticas do mod.rs**
- apagar:
  - `if name contains email -> random_email`
  - `if contains nome -> random_name`
  - `if tipo -> values[...]`
- substituir por:
  - se não há regra: usar `primitive.<type>` baseado no tipo do schema.

3) **Roteamento default por tipo**
- uuid -> `primitive.uuid`
- integer -> `primitive.int`
- numeric -> `primitive.decimal` (scale)
- date/time/timestamp -> primitives correspondentes
- text -> `primitive.text`

4) **Manter compat layer**
- plan antigo (enum) ainda deve funcionar.

5) **Adicionar plans de exemplo**
- `plans/examples/faker_baseline.plan.json`
- `plans/examples/faker_ptbr.plan.json`
- `plans/examples/faker_enus.plan.json`

## Critérios de aceite
- [ ] Geração funciona sem os geradores antigos.
- [ ] E2E Postgres passa.
- [ ] Unknown generator_id -> erro.
- [ ] params inválidos -> erro.

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
