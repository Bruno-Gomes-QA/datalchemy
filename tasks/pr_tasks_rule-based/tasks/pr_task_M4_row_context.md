# Task: M4 — RowContext + derive.* (intra-linha)

**Status:** Open  
**Milestone:** M4  
**Created:** 2026-01-24  
**Based on:** `tasks/pr_tasks_rule-based/milestones/M4_row_context.md`

---

## 1. Contexto e Objetivo

Introduzir consciência de linha (`RowContext`) no processo de geração, permitindo que o valor de uma coluna dependa de valores gerados previamente na mesma linha. Isso é crucial para coerência (ex: `email` derivado de `nome`, `data_fim` > `data_inicio`).

- **Feature:** Permitir que geradores leiam o `RowContext`.
- **Pipeline:** Geração em dois passos (Base -> Derivados).
- **Catálogo:** Implementar geradores do tipo `derive.*`.

## 2. Regras Inegociáveis (AGENTS.md)

- **Determinismo:** A ordem de geração das colunas deve ser estritamente definida (topológica ou fixa pelo schema) para garantir que a dependência já exista no `RowContext`.
- **Ciclos:** Detectar e rejeitar dependências cíclicas (A depende de B, B depende de A).

## 3. Escopo de Trabalho

### 3.1 RowContext & Pipeline
- [ ] Atualizar `GenerationContext` para conter `current_row: RowContext`.
- [ ] Alterar loop de geração:
    1. Gerar colunas independentes.
    2. Gerar colunas dependentes (com acesso aos valores do passo 1).
    3. Aplicar transforms finais.

### 3.2 Parsing do Plan (Derives)
- [ ] Suportar sintaxe para definir dependências (ex: `params: { input_columns: ["nm_cliente"] }`).
- [ ] Validar integridade referencial das colunas citadas.

### 3.3 Catálogo Derives
- [ ] Implementar em `generators/derive/`:
    - `derive.email_from_name`: Gera email sanitizado baseado em uma coluna de nome.
    - `derive.updated_after_created`: Garante `updated_at >= created_at`.
    - `derive.end_after_start`: Garante `dt_fim >= dt_inicio`.
    - `derive.money_total`: Calcula `total = price * qty - discount`.

## 4. Plano de validação

**Evidência:** `evidence/pr_task_M4_row_context.md`

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

# 3. Teste Específico (Plan M4)
# Criar `plans/examples/m4_derives.plan.json` com dependências intra-linha
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/m4_derives.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/

# 4. Verificação Manual (no CSV gerado)
# - Email corresponde ao nome?
# - Data Fim >= Data Inicio?
```

### 4.2 Critérios de Aceite
- [ ] Geradores `derive.*` funcionam corretamente acessando o RowContext.
- [ ] Pipeline de duas fases implementado e estável.
- [ ] Erro claro caso coluna de input não exista.
- [ ] E2E com plan complexo validado.
