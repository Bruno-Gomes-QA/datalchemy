# Task: M2 — Primitives + transforms base

**Status:** Open  
**Milestone:** M2  
**Created:** 2026-01-24  
**Based on:** `tasks/pr_tasks_rule-based/milestones/M2_primitives_transforms.md`

---

## 1. Contexto e Objetivo

Implementar o catálogo "core" de geradores primitivos (tipos básicos) e transformações de dados. O objetivo é cobrir 80% das necessidades de geração simples sem depender de dados semânticos complexos.

- **Primitives:** Inteiros, Floats, Strings (Patterns), Datas, UUIDs.
- **Transforms:** Pipeline de pós-processamento (Null rate, Casing, Format).

## 2. Regras Inegociáveis (AGENTS.md)

- **Determinismo:** A ordem de aplicação de transforms deve ser fixa (conforme array no plan).
- **Inputs:** Respeitar limites (min/max) do plan e do schema.
- **Segurança:** Validar parâmetros de entrada para evitar panic (ex: regex inválido).

## 3. Escopo de Trabalho

### 3.1 Catálogo Primitives
- [ ] Implementar em `generators/primitives/`:
    - `primitive.bool`
    - `primitive.int.range`, `primitive.int.sequence_hint`
    - `primitive.float.range`, `primitive.decimal.numeric`
    - `primitive.text.pattern` (regex basic), `primitive.text.lorem`
    - `primitive.uuid.v4`
    - `primitive.date.range`, `primitive.time.range`, `primitive.timestamp.range`

### 3.2 Catálogo Transforms
- [ ] Implementar em `generators/transforms/`:
    - `transform.null_rate`
    - `transform.truncate`
    - `transform.format`
    - `transform.prefix_suffix`
    - `transform.casing`
    - `transform.weighted_choice`

### 3.3 Integração no Plan/Engine
- [ ] Atualizar `Rule` no plan para aceitar lista de `transforms` por coluna.
- [ ] Atualizar loops de geração: `Generator -> [Transforms] -> Output`.

## 4. Plano de validação

**Evidência:** `evidence/pr_task_M2_primitives_transforms.md`

### 4.1 Comandos
```bash
# 1. Qualidade
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

# 2. Setup E2E
./scripts/postgres_docker.sh
cargo run -p datalchemy-cli -- introspect --conn "$DATABASE_URL" --run-dir runs/
RUN_DIR=$(ls -1d runs/* | sort | tail -n 1)

# 3. Teste Específico (Criar Plan de Teste M2)
# Criar um `plans/examples/m2_primitives.plan.json` que exercite os novos tipos
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/
```

### 4.2 Critérios de Aceite
- [ ] Todos os primitives listados registrados e funcionais.
- [ ] Todos os transforms listados registrados e funcionais.
- [ ] `resolved_plan.json` reflete a aplicação de transforms.
- [ ] Testes unitários cobrindo boundaries (min/max) e determinismo.
