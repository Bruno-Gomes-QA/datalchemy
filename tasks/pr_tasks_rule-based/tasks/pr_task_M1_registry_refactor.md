# Task: M1 — Refactor de pastas + registry por IDs

**Status:** Open  
**Milestone:** M1  
**Created:** 2026-01-24  
**Based on:** `tasks/pr_tasks_rule-based/milestones/M1_registry_refactor.md`

---

## 1. Contexto e Objetivo

Refatorar a arquitetura interna do gerador para suportar extensibilidade via IDs (strings) em vez de Enums hardcoded. Isso prepara o terreno para a explosão de geradores nos próximos milestones.

- **Refactor estrutural:** Organizar `src/generators/` em módulos.
- **Registry dinâmico:** Mapear `id string -> Generator factory`.
- **Breaking Change:** Atualizar `plan.schema.json` para aceitar strings no campo de generator.

## 2. Regras Inegociáveis (AGENTS.md)

- **Determinismo:** A resolução de IDs e a inicialização do Registry devem ser determinísticas.
- **Qualidade:** `cargo fmt`, `cargo clippy` sem warnings.
- **E2E:** Não quebrar o exemplo `minimal.plan.json` (atualizá-lo para a nova sintaxe).

## 3. Escopo de Trabalho

### 3.1 Reorganização de Pastas
- [ ] Criar estrutura em `crates/datalchemy-generate/src/generators/`:
    - `mod.rs` (Registry)
    - `primitives/` (Base para M2)
    - `transforms/` (Base para M2)
    - `semantic/` (Base para M3)

### 3.2 Registry e Traits
- [ ] Definir trait `Generator` com método `id() -> &'static str`.
- [ ] Implementar `Registry` (`BTreeMap<&'static str, Box<dyn Generator>>`).
- [ ] Registrar os geradores atuais (ex: `Uuid`, `Name`) com IDs novos (ex: `primitive.uuid.v4`).

### 3.3 Breaking Change no Plan
- [ ] Alterar `crates/datalchemy-plan/src/model.rs`:
    - `ColumnGenerator` de `Enum` para `String` (ou struct wrapper).
- [ ] Atualizar `plan.schema.json` (rodar script de geração).
- [ ] Atualizar `plans/examples/minimal.plan.json` para usar os novos IDs.

## 4. Plano de validação

**Evidência:** `evidence/pr_task_M1_registry_refactor.md`

### 4.1 Comandos
```bash
# 1. Qualidade
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

# 2. Setup E2E
./scripts/postgres_docker.sh

# 3. Introspecção
cargo run -p datalchemy-cli -- introspect --conn "$DATABASE_URL" --run-dir runs/
RUN_DIR=$(ls -1d runs/* | sort | tail -n 1)

# 4. Validar Schema do Plan (Atualizado)
cargo run -p datalchemy-plan --example validate_plan -- \
  plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json"

# 5. Geração e Determinismo
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/

# Validar output
cat out/generation_report.json
```

### 4.2 Critérios de Aceite
- [ ] `minimal.plan.json` usa strings (ex: `"primitive.uuid.v4"`) ao invés de enums.
- [ ] Registry resolve corretamente os IDs.
- [ ] Pipeline E2E funciona do início ao fim com a nova estrutura.
