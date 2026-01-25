# Task: M6 — Domain packs (CRM/Finance/Logística)

**Status:** Open  
**Milestone:** M6  
**Created:** 2026-01-24  
**Based on:** `tasks/pr_tasks_rule-based/milestones/M6_domains.md`

---

## 1. Contexto e Objetivo

Expandir a biblioteca de geradores com pacotes de domínio específicos (CRM, Financeiro, Logística) para suportar casos de uso reais de QA e Demo, além de criar planos de exemplo complexos que demonstrem todo o poder da ferramenta.

- **Packs:** Coleção de geradores temáticos.
- **Examples:** Planos que simulam sistemas reais completos.

## 2. Regras Inegociáveis (AGENTS.md)

- **Determinismo:** Mesmo em domínios complexos, a seed deve garantir reproducibilidade total.
- **Modularidade:** Manter os domínios isolados em sub-módulos.

## 3. Escopo de Trabalho

### 3.1 Pack: CRM
- [ ] Implementar em `generators/domain/crm/`:
    - `domain.crm.lead_stage` (New, Contacted, Qualified...)
    - `domain.crm.activity_type` (Call, Email, Meeting)
    - `domain.crm.deal_value`
    - `domain.crm.pipeline_name`

### 3.2 Pack: Finance
- [ ] Implementar em `generators/domain/finance/`:
    - `domain.finance.transaction_type` (Debit, Credit, Pix)
    - `domain.finance.payment_method`
    - `domain.finance.invoice_status`
    - `domain.finance.installments`

### 3.3 Pack: Logística
- [ ] Implementar em `generators/domain/logistics/`:
    - `domain.logistics.tracking_code`
    - `domain.logistics.shipment_status`
    - `domain.logistics.carrier`
    - `domain.logistics.dimensions_cm`

### 3.4 Plans de Exemplo
- [ ] Criar plans robustos em `plans/examples/`:
    - `crm_domain.plan.json`
    - `finance_domain.plan.json`
    - `logistics_domain.plan.json`
    - `full_stack_ptbr.plan.json` (Integrando Common PT-BR + 3 Domínios + 10k rows)

## 4. Plano de validação

**Evidência:** `evidence/pr_task_M6_domains.md`

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

# 3. Gerar todos os exemplos de domínio
for plan in crm_domain finance_domain logistics_domain full_stack_ptbr; do
  echo "Generating $plan..."
  cargo run -p datalchemy-generate --example generate_csv -- \
    --plan "plans/examples/${plan}.plan.json" \
    --schema "$RUN_DIR/schema.json" \
    --out "out/${plan}/"
done
```

### 4.2 Critérios de Aceite
- [ ] Todos os novos geradores implementados e funcionais.
- [ ] Plans de exemplo válidos (schema check passa).
- [ ] Geração "Full Stack" roda sem erros e produz dados coerentes semanticamente.
