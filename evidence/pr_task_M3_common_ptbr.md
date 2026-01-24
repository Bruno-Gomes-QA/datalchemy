# Evidencia — pr_task_M3_common_ptbr

## O que mudou
- Assets loader com cache e assets pt-BR em `crates/datalchemy-generate/assets/pt_BR/`.
- Geradores semanticos pt-BR (nome, email, phone, cpf, cnpj, rg, cep, uf, city, address, money, ip, url).
- Transform `transform.mask` (hash/redact/format_preserving).
- Adicionado `plans/examples/m3_ptbr.plan.json`.

## Por que mudou
- Cobrir geracao pt-BR realista e reforcar LGPD via mascaramento deterministico.

## Como validar
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

./scripts/postgres_docker.sh
cargo run -p datalchemy-cli -- introspect --conn "$DATABASE_URL" --run-dir runs/
RUN_DIR=$(ls -1d runs/* | sort | tail -n 1)

cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/m3_ptbr.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/
```

## Testes executados
- `cargo test` (falhou no teste de integracao do introspect por falta de `DATABASE_URL`/`TEST_DATABASE_URL`).
