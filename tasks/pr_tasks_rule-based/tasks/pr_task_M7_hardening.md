# Task: M7 — Hardening (perf 10k, docs, golden files)

**Status:** Open  
**Milestone:** M7  
**Created:** 2026-01-24  
**Based on:** `tasks/pr_tasks_rule-based/milestones/M7_hardening.md`

---

## 1. Contexto e Objetivo

Fase final de estabilização ("Hardening"). Garantir que o sistema seja performático, estável para grandes volumes (10k+ linhas), bem documentado e protegido contra regressões futuras através de "Golden Files".

- **Performance:** Medir e otimizar throughput.
- **Streaming:** Garantir uso baixo de memória (BufWriter real).
- **Docs:** Documentação final de todos os geradores.
- **Regressão:** Testes de Snapshot (Golden Files).

## 2. Regras Inegociáveis (AGENTS.md)

- **Determinismo:** O teste de regressão (Golden File) depende inteiramente do determinismo. Se o hash mudar sem motivo, o teste deve falhar.
- **Resources:** O streaming não pode carregar o CSV inteiro em memória antes de escrever.

## 3. Escopo de Trabalho

### 3.1 Performance & Streaming
- [ ] Instrumentar métricas de tempo (Instant) e vazão (bytes/sec) no report.
- [ ] Revisar implementação do Writer para garantir `BufWriter` e flushing correto sem bufferização excessiva.
- [ ] Benchmark: Gerar 10k e 100k linhas, medir tempo.

### 3.2 Golden Files / Regressão
- [ ] Criar utilitário de hash para CSVs gerados.
- [ ] Estabelecer "Golden Set": um plan fixo + seed fixa + hashes esperados dos CSVs.
- [ ] Adicionar teste de integração que falha se o hash mudar.

### 3.3 Documentação
- [ ] Escrever/Atualizar:
    - `docs/generators.md` (Catálogo completo).
    - `docs/plan_generators.md` (Guia de uso do Plan).
    - `docs/privacy_lgpd.md` (Guia de privacidade e mascaramento).
    - Atualizar `README.md` principal com novas capacidades.

## 4. Plano de validação

**Evidência:** `evidence/pr_task_M7_hardening.md`

### 4.1 Comandos
```bash
# 1. Qualidade
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

# 2. Teste de Regressão Golden File
# (Deve ser um comando automatizado criado nesta milestone, ex: cargo test --test golden_files)

# 3. Benchmark 10k
# Rodar generation do full_stack_ptbr e capturar métricas do report
./scripts/postgres_docker.sh
cargo run -p datalchemy-cli -- introspect --conn "$DATABASE_URL" --run-dir runs/
RUN_DIR=$(ls -1d runs/* | sort | tail -n 1)

time cargo run -p datalchemy-generate --release --example generate_csv -- \
  --plan plans/examples/full_stack_ptbr.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/perf_test/
```

### 4.2 Critérios de Aceite
- [ ] Streaming confirmado (monitorar RAM durante geração de 100k linhas).
- [ ] Teste de Golden File implementado e passando.
- [ ] Documentação completa e revisada.
- [ ] README atualizado.
