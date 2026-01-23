# Evidencia — pr_task_plan-schema-aware_4

## O que mudou
- Implementada a engine de geracao deterministica em `crates/datalchemy-generate` (CSV por tabela).
- Adicionados validadores de CHECK subset A, FK/UNIQUE/PK/NOT NULL e defaults basicos.
- Implementados generators basicos (uuid, email, name, ranges, enums) e heuristicas de unicidade.
- Criados exemplos e testes:
  - `crates/datalchemy-generate/examples/generate_csv.rs`
  - `crates/datalchemy-generate/tests/generate_csv.rs`
- Atualizado `datalchemy_structure.md` para refletir o Plan 4.

## Por que mudou
- Cumprir o Plan 4: gerar dataset reproduzivel a partir de `schema.json` + `plan.json` com constraints.

## Como validar
```bash
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema crates/datalchemy-introspect/tests/golden/postgres_minimal.schema.json \
  --out out/

cargo test -p datalchemy-generate
```

## Testes executados
- `cargo test -p datalchemy-generate`
