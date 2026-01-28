# Plan — Suporte completo a fake-rs no Datalchemy (Adapter + Catálogo grande)

> Plano consolidado com milestones F0–F7.



**Repo:** https://github.com/Bruno-Gomes-QA/datalchemy  
**Branch:** master  
**Data:** 2026-01-25

Este plano implementa uma integração “fake-rs first”:
- fake-rs como backend baseline (não opcional)
- IDs estáveis do Datalchemy (semantic.* / primitive.*)
- catálogo enorme gerado automaticamente (faker.*)
- suporte pt_BR + en_US desde o começo
- parâmetros avançados agora
- erro direto em qualquer divergência (ids/params/locale)
- sem sanitizers por enquanto

## Trilhos obrigatórios (AGENTS.md)

- **Tasks + Evidence**: toda mudança precisa de `tasks/issue_task_*.md` e `evidence/<task_id>.md`.
- **Qualidade**: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- **Logs**: `tracing` (nunca `println!` em libs).
- **Erros**: `thiserror`.
- **Dependências pesadas**: só com justificativa clara (este plano inclui a justificativa).
- **Determinismo** (do AGENTS): output ordenado/estável (sem HashMap em output).  
  **Nota deste plano:** você pediu “determinismo fora do escopo”, então aqui tratamos como *não-objetivo de produto*, mas **mantemos ordenação estável** e **passagem de RNG** para não violar AGENTS e para manter a porta aberta.

---

## Arquitetura alvo

```
datalchemy-generate/
  src/
    engine.rs
    generators/
    faker_rs/
      adapter.rs              # único módulo que importa fake::faker::*
      catalog_gen.rs          # auto-gerado (faker.*)
      params.rs               # ParamSpec + validação
      locales.rs              # pt_BR / en_US
  faker_catalog/
    overrides.toml            # aliases semantic.* e overrides de tipos/params
tools/
  gen_faker_catalog.rs        # gera catalog_gen.rs
```

---

## Milestones
1) F0 — decisões e trilhos  
2) F1 — plan com generator.id string + compat layer  
3) F2 — dependência fake-rs + adapter único  
4) F3 — catálogo grande auto-gerado + aliases semantic.*  
5) F4 — tipos completos + params avançados  
6) F5 — locales pt_BR + en_US  
7) F6 — substituir geradores antigos  
8) F7 — docs + list_generators + testes de contrato

---

## Regras “hard” do plano
- Não criar wrapper manual por faker: tudo via catálogo/codegen.
- Plan não conhece fake-rs: só IDs do Datalchemy.
- Erro direto: unknown id/param/locale -> erro.
- Sem sanitizers por enquanto (mas logs continuam sem valores).
- Manter ordenação estável nos outputs/listas (AGENTS).

---

## Runbook de validação
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

---

## Templates
## Template de task (crie antes de codar)

Crie: `tasks/issue_task_<YYYYMMDD>_<slug>.md`

```md
# Task: <título curto>

- ID: issue_task_<YYYYMMDD>_<slug>
- Owner: <nome>
- Status: Draft | In Progress | Done
- Milestone: <F0 | F1 | ...>
- Crates: <lista>
- Risco: Baixo | Médio | Alto

## Contexto
<por que esta task existe>

## Objetivo
<resultado final observável>

## Não-objetivos
<o que explicitamente não será feito>

## Entregas (DoD)
- [ ] ...
- [ ] ...

## Plano de execução
1) ...
2) ...

## Como validar (comandos exatos)
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
./scripts/postgres_docker.sh
# + E2E
```

## Evidência (obrigatória)
- Arquivo: `evidence/<task_id>.md`
- Deve incluir: "o que mudou", "por que mudou", "como validar", outputs.
```

## Template de evidência (preencha no final)

Crie: `evidence/<task_id>.md`

```md
# Evidence: <task_id>

## O que mudou
- ...

## Por que mudou
- ...

## Como validar (comandos exatos)
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
./scripts/postgres_docker.sh
# introspect -> generate -> eval
```

## Resultado
- RUN_DIR=...
- OUT_DIR=...
- logs/erros relevantes: ...

## Notas/Riscos
- ...
```
