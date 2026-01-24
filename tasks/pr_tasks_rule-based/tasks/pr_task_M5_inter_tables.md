# Task: M5 — Inter-tabelas (ForeignContext) + integração FK

**Status:** Open  
**Milestone:** M5  
**Created:** 2026-01-24  
**Based on:** `tasks/pr_tasks_rule-based/milestones/M5_inter_tables.md`

---

## 1. Contexto e Objetivo

Permitir a geração de dados consistentes entre tabelas (Pai -> Filho) sem duplicar lógica de chaves. O gerador deve ser capaz de "olhar" para tabela pai (Foreign Context) para selecionar chaves estrangeiras válidas ou herdar valores.

- **Feature:** Trait `ForeignContext` para acesso a dados de outras tabelas.
- **Provider:** Implementação que permite lookup eficiente (sem IO excessivo).
- **Derives:** Geradores que cruzam fronteiras de tabelas.

## 2. Regras Inegociáveis (AGENTS.md)

- **Sem SQL no Generate:** A resolução de chaves estrangeiras deve ocorrer via memória ou arquivos gerados previamente, jamais fazendo queries SQL ad-hoc no meio da geração.
- **Separação:** O gerador é agnóstico à persistência do Foreign Context.

## 3. Escopo de Trabalho

### 3.1 Contrato ForeignContext
- [ ] Definir trait `ForeignContext`:
```rust
pub trait ForeignContext {
  fn pick_fk(&mut self, schema: &str, table: &str, fk_column: &str) -> Result<GeneratedValue, GenerationError>;
  fn lookup_parent(&self, schema: &str, table: &str, pk: &GeneratedValue, col: &str) -> Option<GeneratedValue>;
}
```
- [ ] Integrar no `GenerationContext`.

### 3.2 Provider FK
- [ ] Implementar provider básico (ex: InMemory ou FileBased) que armazena PKs geradas das tabelas pai.
- [ ] Atualizar pipeline para popular esse provider após gerar uma tabela.

### 3.3 Derives Inter-tabelas
- [ ] Implementar:
    - `derive.fk`: Seleciona uma FK válida aleatória (pick_fk).
    - `derive.parent_value`: Copia valor da tabela pai (ex: `child.created_at >= parent.created_at`).

## 4. Plano de validação

**Evidência:** `evidence/pr_task_M5_inter_tables.md`

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

# 3. Teste Específico (Plan M5 - Multi-table)
# Criar `plans/examples/m5_relationships.plan.json` com Users -> Orders
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/m5_relationships.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/

# 4. Validação de Integridade Referencial
# Verificar se todos user_id em Orders existem em Users.csv
```

### 4.2 Critérios de Aceite
- [ ] Tabelas filhas geradas com FKs válidas (existentes na tabela pai).
- [ ] Derivação de valores do pai (ex: data) funciona.
- [ ] Execução multi-tabela coordenada (Pai gerado antes do Filho).
