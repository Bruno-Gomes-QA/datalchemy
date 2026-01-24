# Task: M3 — Common pt-BR + assets loader + mask

**Status:** Open  
**Milestone:** M3  
**Created:** 2026-01-24  
**Based on:** `tasks/pr_tasks_rule-based/milestones/M3_common_ptbr.md`

---

## 1. Contexto e Objetivo

Adicionar capacidades de geração de dados realistas para o contexto brasileiro (pt-BR) e fortalecer a privacidade com mascaramento de dados (LGPD). Isso envolve carregar assets estáticos (listas de nomes, cidades) de forma eficiente.

- **Assets:** Loader lazy com cache para arquivos `.txt`/`.json`.
- **Semantic:** Geradores de CPF, CNPJ, Nomes, Endereços.
- **LGPD:** Transform de mascaramento robusto.

## 2. Regras Inegociáveis (AGENTS.md)

- **Privacidade/LGPD:** PII tags devem ser rigorosas. Logs nunca devem mostrar valores gerados por estes generators.
- **Determinismo:** O loader de assets deve garantir ordem estável se houver iteração.
- **Performance:** Cache de assets é mandatório para não ler disco a cada linha.

## 3. Escopo de Trabalho

### 3.1 Assets Loader
- [ ] Criar estrutura `crates/datalchemy-generate/assets/pt_BR/`.
- [ ] Implementar `AssetsLoader` (Lazy static ou Arc/Mutex cache).
- [ ] Adicionar arquivos básicos: `names.txt`, `cities.json`, etc.

### 3.2 Semantic pt-BR
- [ ] Implementar em `generators/semantic/`:
    - Pessoas: `name`, `email.safe`, `phone.br`, `cpf`, `rg`.
    - Geo: `cep`, `uf`, `city`, `address`.
    - Finance: `money.brl`.
    - Net: `ip`, `url`.

### 3.3 LGPD & Masks
- [ ] Implementar PII Tagging na trait `Generator`.
- [ ] Implementar `transform.mask`:
    - `hash` (sha256)
    - `redact` (***)
    - `format_preserving` (CPF/Email mantendo estrutura).

## 4. Plano de validação

**Evidência:** `evidence/pr_task_M3_common_ptbr.md`

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

# 3. Teste Específico (M3 Plan)
# Criar `plans/examples/m3_ptbr.plan.json` usando CPF, Nomes e Masks.
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/m3_ptbr.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/

# 4. Validar LGPD
# Verificar se generation_report.json aponta "pii_columns_touched" corretamente.
# Verificar visualmente se os dados sensíveis outputados estão mascarados (se mask foi aplicado).
```

### 4.2 Critérios de Aceite
- [ ] Loader carrega assets corretamente (com fallback se arquivo sumir).
- [ ] Geradores pt-BR produzem dados válidos (algoritmo de CPF check digit, etc).
- [ ] `transform.mask` funciona de forma determinística.
- [ ] PII Tags são reportadas no JSON final.
