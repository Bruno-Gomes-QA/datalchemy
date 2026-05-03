# F4 — Tipos completos + parâmetros avançados (sem sanitizers)

> Gerar Bool/Int/Float/Text/Uuid/Date/Time/Timestamp com params robustos.



## Objetivo
Suportar **todos os tipos** do `GeneratedValue` e parâmetros avançados desde já,
sem depender de sanitizers e com **erro direto** para qualquer param inválido.

## Estratégia
1) **primitive.***: implementações tipadas com params (min/max/scale/len).
2) **faker.***: majoritariamente Text (default), com overrides tipados quando fizer sentido.
3) **semantic.***: aliases para fakers específicos, com params opcionais.

## ParamSpec (validação forte)
Criar `ParamSpec` por generator_id (ou por kind) e validar:
- tipos (int/float/string/bool)
- ranges (min<=max)
- chaves desconhecidas → erro
- required keys → erro

### Params comuns (por kind)
**Text**: `min_len`, `max_len`, `pattern` (opcional), `charset` (opcional), `allow_empty`  
**Int/Float**: `min`, `max`  
**Decimal/Numeric**: `min`, `max`, `scale`  
**Date/Time/Timestamp**: `min`, `max` (ISO)

## Respeito ao schema
Como você pediu “erro direto”, recomendamos:
- se schema tem `character_max_length` e o faker gerar maior → erro
- se param `max_len` excede schema → erro

## Critérios de aceite
- [ ] primitive.* tipados: bool, int, float, uuid, date, time, timestamp, text.
- [ ] params avançados funcionam e são validados.
- [ ] erro direto em:
  - param desconhecido
  - tipo inválido
  - min > max
  - violação de max_len
- [ ] E2E Postgres roda com um plan de exemplo usando params.

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

## Exemplo de plan (novo)
Criar: `plans/examples/faker_baseline.plan.json` com:
- locale global `pt_BR`
- regras usando:
  - `semantic.person.name`
  - `semantic.person.email`
  - `semantic.address.city`
  - `primitive.int` com min/max
  - `primitive.timestamp` com range
