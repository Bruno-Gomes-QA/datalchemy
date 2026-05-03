# F3 — Catálogo grande auto-gerado (faker.*) + aliases estáveis (semantic.*)

> Cobrir fake-rs sem escrever wrapper para tudo.



## Objetivo
Gerar automaticamente um catálogo grande de IDs `faker.*` (espelho do fake-rs) e manter um conjunto
de IDs estáveis `semantic.*`/`primitive.*` como “interface recomendada” do Datalchemy.

## Arquitetura do catálogo

### 1) Arquivo de overrides (humano edita)
Criar:
- `crates/datalchemy-generate/faker_catalog/overrides.toml`

Estrutura sugerida:
```toml
[[alias]]
id = "semantic.person.name"
target = "faker.name.raw.Name"
kind = "text"
locales = ["pt_BR", "en_US"]

[[alias]]
id = "primitive.int"
target = "primitive.int" # tratado no adapter (Faker::<i64>)
kind = "int"
params = ["min", "max"]
```

### 2) Ferramenta de geração (máquina gera)
Criar um tool no repo:
- opção A: `tools/gen_faker_catalog.rs`
- opção B: `crates/datalchemy-xtask/` (xtask pattern)

Ela deve:
1) localizar o source do crate `fake` via `cargo metadata`;
2) varrer `fake/src/faker/**` e coletar:
   - módulo (path)
   - nome do struct
3) gerar um arquivo Rust em:
   - `crates/datalchemy-generate/src/faker_rs/catalog_gen.rs`

Conteúdo gerado:
- lista ordenada de IDs `faker.<mod>.<Struct>`
- `match` (id, locale) -> chamada concreta

> Para começar rápido: marcar tudo como `Text` e ir adicionando overrides para tipos não-texto (F4).

### 3) Merge final (gerado + overrides)
O adapter une:
- entries geradas (`faker.*`)
- aliases estáveis (`semantic.*` e `primitive.*`)

**Erro direto**:
- unknown id -> erro
- id duplicado -> erro em build/test

## Critérios de aceite
- [ ] `catalog_gen.rs` gerado e compilando.
- [ ] `list_ids()` retorna centenas+ de IDs.
- [ ] `semantic.*` funciona via alias.
- [ ] IDs ausentes → erro direto.

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
