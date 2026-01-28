# F1 — Plan/Schema: migrar para generator.id string (sem dependência fake)

> Pré-requisito para usar catálogo grande com IDs estáveis.



**Objetivo**
Permitir que o `plan.json` referencie **IDs string** (semantic.* e faker.*) com `params` em JSON,
sem que o crate `datalchemy-plan` dependa de `fake-rs`.

> Você pediu “não misturar com o plan”: aqui interpretamos como **não adicionar fake-rs no plan**.
> O plan continua genérico: valida strings/params, e o generate resolve.

## O que mudar (alto nível)

1) **Contrato de plan**
- Hoje existe `ColumnGenerator` como enum (`Uuid`, `Email`, `Name`, `IntRange`, `DateRange`, `Regex`).
- Precisamos evoluir para:

```jsonc
{
  "schema": "public",
  "table": "usuarios",
  "column": "email",
  "generator": {
    "id": "semantic.person.email",
    "locale": "pt_BR", // opcional; default global
    "params": {
      "domain": "example.com",
      "max_len": 120
    }
  }
}
```

2) **Compat layer**
- Ainda aceitar a forma antiga (enum) por um tempo:
  - `Uuid` -> `primitive.uuid`
  - `Email` -> `semantic.person.email`
  - `Name` -> `semantic.person.name`
  - `IntRange` -> `primitive.int` com params `min/max`
  - etc.
- Normalizar para `resolved_plan.json` sempre com `generator.id`.

3) **Validação (sem fake)**
- `datalchemy-plan` valida:
  - `generator.id` não vazio
  - `params` é objeto (quando presente)
- A validação “ID existe no generate” fica no generate por enquanto:
  - unknown id -> erro (F0)

## Arquivos prováveis
- `crates/datalchemy-plan/src/*` (parser + structs)
- `schemas/plan.schema.json` (se existir; atualizar contrato)
- `crates/datalchemy-generate/...` (consumir `generator.id`)

## Critérios de aceite
- [ ] Plan antigo continua funcionando (compat layer).
- [ ] `resolved_plan.json` sempre contém `generator.id` string.
- [ ] `cargo fmt/clippy/test` passam.

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
