# Plan 4+ — Expansão de generators (Master Plan)

> Visão + regras imutáveis + como executar milestones com Codex (com trilhos).


**Repo:** https://github.com/Bruno-Gomes-QA/datalchemy  
**Branch:** master
**Data:** 2026-01-24

Este documento é a **constituição** do trabalho de expansão de generators.  
Ele depende dos detalhes de execução por milestone (M0–M7) e consolida as regras imutáveis e o plano de ataque.


## Como usar (modo rápido)

1) Leia `AGENTS.md` (regras do projeto).
2) Escolha **uma** milestone por vez em `milestones/`.
3) Antes de codar: crie `tasks/issue_task_*.md` para habilitar a IA (regra PIT).
4) Execute a milestone, gere `evidence/<task_id>.md`, rode os comandos e registre resultados.
5) Só avance para a próxima milestone quando a anterior estiver “Done” e com evidência.


## Regras imutáveis (não negociar)

### Determinismo
- Outputs devem ser estáveis e ordenados.
- Não iterar `HashMap` para gerar output serializado.
- Seed global define o run; é aceitável que adicionar tabelas altere o dataset (decisão do projeto), mas isso deve ser registrado em evidence/golden.

### Privacidade/LGPD
- Dados gerados são **100% sintéticos**.
- Nunca logar: credenciais, tokens, valores de colunas PII.
- Artefatos de run precisam estar redigidos.
- `transform.mask` deve existir e ser documentado.

### Separação de responsabilidades
- SQL fica na introspecção; o gerador não faz query SQL direta.
- Contratos e validação ficam no core/plan.
- O que não é suportado vira `Unsupported`/warning (sem promessas vazias).

### Qualidade e evidência
- `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` sempre.
- Toda mudança precisa de task e evidence.


## Arquitetura alvo (alto nível)

Pipeline:
- (Schema + Plan) → `resolved_plan.json`
- Engine determinística → `*.csv` (streaming)
- Report → `generation_report.json` (coverage + warnings + métricas)

Camadas:
- `primitives.*`
- `semantic.*` (common pt-BR)
- `domain.*` (CRM/Finance/Logística)
- `transform.*`
- `derive.*` (RowContext/ForeignContext)


## Milestones (mapa)

- M0: baselines/strict/warnings/coverage
- M1: refactor + registry IDs + compat
- M2: primitives + transforms
- M3: common pt-BR + assets + mask
- M4: RowContext + derives intra-linha
- M5: ForeignContext + derives inter-tabelas
- M6: domain packs + examples
- M7: hardening (perf 10k, docs, golden)


## Estrutura de arquivos

```
tasks/pr_tasks_rule_based/
    M0_baseline.md
    M1_registry_refactor.md
    M2_primitives_transforms.md
    M3_common_ptbr.md
    M4_row_context.md
    M5_inter_tables.md
    M6_domains.md
    M7_hardening.md
**  execute.md  <-- você está aqui
```


## Runbook E2E (comandos)

```bash
# Qualidade
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

# E2E Postgres (Plan 4 + Plan 5)
./scripts/postgres_docker.sh

# Introspecção (schema.json) — usa DATABASE_URL
cargo run -p datalchemy-cli -- introspect \
  --conn "$DATABASE_URL" \
  --run-dir runs/

# Localize o RUN_DIR mais recente (ajuste se necessário)
RUN_DIR=$(ls -1d runs/* | sort | tail -n 1)
echo "RUN_DIR=$RUN_DIR"

# Validar plan (se você estiver alterando schema/plan)
cargo run -p datalchemy-plan --example validate_plan -- \
  plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json"

# Gerar CSV (Plan 4)
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/

OUT_DIR=$(ls -1d out/* | sort | tail -n 1)
echo "OUT_DIR=$OUT_DIR"

# Avaliar (Plan 5) — gera metrics.json + report.md
cargo run -p datalchemy-eval --example evaluate_run -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --run "$OUT_DIR"

# Determinismo: segunda geração com a mesma seed deve produzir CSV idêntico
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/

OUT_DIR_2=$(ls -1d out/* | sort | tail -n 1)
diff -u "$OUT_DIR/crm.usuarios.csv" "$OUT_DIR_2/crm.usuarios.csv"
```


## Artefatos esperados

Em `out/<run>/`:
- `*.csv`
- `generation_report.json`
- `resolved_plan.json`
- `metrics.json`
- `report.md`

Em `runs/<run>/`:
- `schema.json`
- `config.json` (redigido)
- `logs.ndjson`


## Links rápidos

- `milestones/M0_baseline.md`
- `milestones/M1_registry_refactor.md`
- `milestones/M2_primitives_transforms.md`
- `milestones/M3_common_ptbr.md`
- `milestones/M4_row_context.md`
- `milestones/M5_inter_tables.md`
- `milestones/M6_domains.md`
- `milestones/M7_hardening.md`
