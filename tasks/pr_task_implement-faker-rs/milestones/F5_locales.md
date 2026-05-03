# F5 — Locales: pt_BR + en_US desde o começo

> Locale como parte do GenerationContext; override por coluna.



## Objetivo
Suportar `pt_BR` e `en_US` em todos os generators fake-based, com regras claras:

- default global (`plan.global.locale`)
- override por coluna (`generator.locale`)
- erro direto se locale não suportado pelo generator

## Design

### 1) Enum interna de locale (string -> tipo fake)
Criar:
```rust
pub enum LocaleKey { PtBr, EnUs }
```

Mapear:
- `"pt_BR"` -> `fake::locales::PT_BR`
- `"en_US"` -> locale equivalente no fake-rs (confirmar se é `EN`, `EN_US` etc.)

### 2) Catálogo declara suporte de locale
Cada entry do catálogo:
- `locales: &[LocaleKey]`

### 3) Sem fallback
Locale não suportado → erro.

## Critérios de aceite
- [ ] trocar locale global e ver a mudança de outputs (pelo menos em name/address).
- [ ] override por coluna funciona.
- [ ] locale inválido → erro.

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
