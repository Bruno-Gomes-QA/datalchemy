# F2 — Adicionar fake-rs e criar Adapter único no generate

> fake-rs vira o backend baseline para geração.



## Objetivo
Integrar o crate `fake` (cksac/fake-rs) como dependência **direta** de `datalchemy-generate`,
com um **Adapter único** que centraliza todas as chamadas ao faker, para:

- não espalhar `fake::faker::*` pelo repo,
- reduzir acoplamento,
- habilitar catálogo grande (F3),
- suportar todos os tipos base (F4),
- suportar locales (F5).

## Justificativa da dependência “pesada”
AGENTS.md diz “sem dependências pesadas sem justificativa”.
Justificativas aqui (documentar na task/evidence):

- `fake-rs` entrega cobertura enorme de “primitivos realistas” de imediato;
- acelera a etapa de enriquecimento de generators sem criar tudo do zero;
- permite foco do time em correlação e domínio (futuro), não em “nome/email/cidade”.

Fonte: docs do crate + repo.  
- Feature flags (referência): https://docs.rs/crate/fake/latest/features  (ajuda a controlar o que compila)
- Crate docs: https://docs.rs/fake/latest/fake/

## Dependência (Cargo.toml)
No `crates/datalchemy-generate/Cargo.toml`:

- Fixar versão (AGENTS: sem ^ ou ~). Recomendado:
  - `fake = "=4.4.0"`

- Ativar features para cobrir o que vamos usar (evitar `cli`).

Exemplo (ajuste conforme build):
```toml
fake = {
  version = "=4.4.0",
  default-features = true,
  features = [
    "derive",
    "chrono",
    "chrono-tz",
    "time",
    "uuid",
    "ulid",
    "serde_json",
    "random_color",
    "email_address",
    "geo",
    "http",
    "rust_decimal"
  ]
}
```

## Adapter: `FakeRsAdapter`
Criar um módulo novo:
- `crates/datalchemy-generate/src/faker_rs/adapter.rs`

Interface sugerida:
- `generate_value(id, locale, params, rng) -> GeneratedValue`
- `list_ids() -> &[&'static str]`
- `validate(id, locale, params)` (erro direto)

**Regra**: o resto do engine nunca importa `fake::faker::*`. Só o adapter.

## Critérios de aceite
- [ ] fake-rs compila no workspace.
- [ ] adapter existe e é o único ponto com imports `fake::*`.
- [ ] engine chama adapter para pelo menos 3 IDs (ex.: name/email/city) e 1 tipo numérico.

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
