# Task: M0 — Baselines e guard rails

**Status:** Open  
**Milestone:** M0  
**Created:** 2026-01-24  
**Based on:** `tasks/pr_tasks_rule-based/milestones/M0_baseline.md`

---

## 1. Contexto e Objetivo

Esta task executa a **Milestone M0**, focada em estabilizar o pipeline de geração com "guard rails":
- Sistema de warnings estruturado.
- Relatório de cobertura (`generation_report.json`).
- Strict Mode.
- Privacidade (LGPD) by design.

**Referência principal:** `tasks/pr_tasks_rule-based/milestones/M0_baseline.md`

## 2. Regras Inegociáveis (AGENTS.md)

- **Determinismo:** Output estável (BTreeMap, ordenação explícita). Nada de iterar HashMap em output.
- **Privacidade:** NUNCA logar valores gerados para colunas PII. Sem `println`/`unwrap`/`expect` em produção.
- **Qualidade:** `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- **Compatibilidade:** Não quebrar o fluxo E2E Postgres atual.

## 3. Escopo de Trabalho

### 3.1 Warnings e Report (Estrutura)
- [ ] Implementar sistema de warnings via `tracing` e coleta para report.
- [ ] Estender `GenerationReport` (determinístico):
    - `generator_usage` (id -> count)
    - `transform_usage` (id -> count)
    - `fallback_count`
    - `heuristic_count`
    - `unknown_generator_id_count`
    - `pii_columns_touched` (tag -> count)
    - `warnings_by_code`

### 3.2 Strict Mode
- [ ] Implementar verificação de `strict` (config global no plan).
- [ ] Comportamento Strict=True:
    - Fallback de tipo -> **Erro**
    - Params inválidos -> **Erro**
    - Null rate > 0 em colunas Not Null -> **Erro**
    - Generator ID desconhecido -> **Erro**
- [ ] Comportamento Strict=False (Default):
    - Fallback de tipo -> Warning + Heurística
    - Params inválidos -> Warning + Default seguro
    - Null rate > 0 em colunas Not Null -> Warning + Force Null=0
    - Generator ID desconhecido -> Warning + Fallback

### 3.3 Privacidade
- [ ] Garantir que logs e artefatos não contenham valores gerados (apenas metadados/counts).
- [ ] Revisar implementação atual de logs no crate `generate`.

## 4. Plano de validação

**Evidência:** `evidence/pr_task_M0_baseline.md`

### 4.1 Comandos
```bash
# 1. Qualidade
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

# 2. Setup E2E
./scripts/postgres_docker.sh

# 3. Introspecção (Gera schema.json em runs/)
cargo run -p datalchemy-cli -- introspect --conn "$DATABASE_URL" --run-dir runs/
RUN_DIR=$(ls -1d runs/* | sort | tail -n 1)

# 4. Geração (Testar Strict e Non-Strict)
# 4.1 Strict=false (default)
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/

# 4.2 Validar outputs
OUT_DIR=$(ls -1d out/* | sort | tail -n 1)
cat "$OUT_DIR/generation_report.json"

# 5. Eval (Validar que métricas de schema ainda funcionam)
cargo run -p datalchemy-eval --example evaluate_run -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --run "$OUT_DIR"
```

### 4.2 Critérios de Aceite
- [ ] `generation_report.json` contém os novos campos preenchidos.
- [ ] Execução com `strict=true` falha adequadamente em cenários de erro (criar caso de teste se necessário).
- [ ] Logs limpos de PII.
- [ ] Diff de execução repetida é vazio (determinismo).
