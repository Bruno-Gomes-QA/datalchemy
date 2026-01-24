---
milestone: M5
title: "Inter-tabelas (ForeignContext) + integração FK"
status: Draft
repo: https://github.com/Bruno-Gomes-QA/datalchemy
date: 2026-01-24
---


# M5 — Inter-tabelas (ForeignContext)

> Correlação pai→filho sem duplicar FK/PK: ForeignContext como contrato.


## Guardrails (AGENTS.md) — inegociáveis

- **Determinismo**: outputs estáveis e ordenados (evitar `HashMap` em output; usar `BTreeMap`/ordenação explícita).
- **Separação de responsabilidades**: contratos e validação no core; geração não “invade” introspecção/SQL.
- **Rust idiomático e seguro**: sem `unsafe` (a menos que documentado e justificado).
- **Sem promessas vazias**: o que não for suportado deve ser explicitamente `Unsupported`/warning.
- **Privacidade/LGPD**: nunca logar credenciais nem valores de PII; artefatos devem ser redigidos.
- **Reprodutibilidade**: cada execução gera artefatos versionáveis (run dir).
- **Proibições**: sem `main()` em lib; sem `src/bin/`; sem `unwrap()`/`expect()` em caminho de produção; sem `println!` em lib.
- **Erros e logs**: `thiserror` + `tracing` (logs estruturados).
- **Qualidade**: `cargo fmt`; `cargo clippy --all-targets -- -D warnings`; versões fixas no `Cargo.toml` (sem `^`/`~`).
- **Evidência obrigatória**: toda mudança precisa de `tasks/issue_task_*.md` e `evidence/<task_id>.md` com o que mudou / por quê / como validar.


## Objetivo

Permitir correlação inter-tabelas (pai→filho) consumindo um provider via trait (`ForeignContext`), sem SQL no generate.


## Contrato ForeignContext

```rust
pub trait ForeignContext {
  fn pick_fk(&mut self, schema: &str, table: &str, fk_column: &str) -> Result<GeneratedValue, GenerationError>;
  fn lookup_parent(
    &self,
    parent_schema: &str,
    parent_table: &str,
    parent_pk: &GeneratedValue,
    parent_column: &str,
  ) -> Option<GeneratedValue>;
}
```


## Plano de implementação (passo a passo)

1) Introduzir trait + plugar em `GenerationContext`.
2) Usar provider para colunas FK (generator interno ou derive).
3) Implementar 1–2 derives inter-tabelas (ex.: child_date >= parent_date).
4) Example plan com tabelas relacionadas.
5) Evidence com SQL provando 0 violações.


## Como validar

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


### Check adicional (SQL — exemplo genérico)

```bash
docker exec -e PGPASSWORD=datalchemy datalchemy-postgres   psql -U datalchemy -d datalchemy_crm   -c "select count(*) from crm.oportunidades o join crm.contatos c on o.contato_id=c.id where o.created_at < c.created_at;"
```


## Critérios de aceite (DoD)

- [ ] ForeignContext integrado e determinístico.
- [ ] Derive inter-tabelas mínimo implementado.
- [ ] Evidence com queries SQL (0 violações).
- [ ] E2E + determinismo OK.


## Templates (tasks/evidence)

## Template de task (crie antes de codar)

Crie um arquivo: `tasks/issue_task_<YYYYMMDD>_<slug>.md`

```md
# Task: <título curto>

- ID: issue_task_<YYYYMMDD>_<slug>
- Owner: <nome>
- Status: Draft | In Progress | Done
- Scope: <M0 | M1 | ...>
- Crates: <lista>
- Risco: Baixo | Médio | Alto

## Contexto
<por que esta task existe>

## Objetivo
<resultado final e observável>

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
# + comandos E2E se aplicável
```

## Evidência (obrigatória)
- Arquivo: `evidence/issue_task_<YYYYMMDD>_<slug>.md`
- Deve incluir: "o que mudou", "por que mudou", "como validar", resultados/outputs.
```


## Template de evidência (preencha no final)

Crie um arquivo: `evidence/<task_id>.md`

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
# E2E (se aplicável)
./scripts/postgres_docker.sh
cargo run -p datalchemy-cli -- introspect --conn "$DATABASE_URL" --run-dir runs/
cargo run -p datalchemy-generate --example generate_csv -- --plan plans/examples/minimal.plan.json --schema "$RUN_DIR/schema.json" --out out/
cargo run -p datalchemy-eval --example evaluate_run -- --plan plans/examples/minimal.plan.json --schema "$RUN_DIR/schema.json" --run "$OUT_DIR"
```

## Resultado
- `RUN_DIR=...`
- `OUT_DIR=...`
- `generation_report.json`: ...
- `metrics.json`: ...
- `diff` determinismo: (sem diferenças)

## Notas/Riscos
- ...
```


## Prompt sugerido para Codex (execução “sem medo”, mas com trilhos)

> **Modo de execução:** implemente **somente** esta milestone (M5).  
> **Não avance** para outras milestones.  
> **Não quebre compatibilidade** com os exemplos existentes.  
> **Siga AGENTS.md** (determinismo, privacidade, evidência, qualidade).

**Contexto disponível**
- Este arquivo em `plans/milestones/`
- `AGENTS.md`
- `end_to_end_postgres.md`
- Código atual do repo

**Tarefa**
- Implementar ForeignContext + integração.
- Criar derives inter-tabelas mínimas.
- Adicionar example e validações SQL.
- Entregar task+evidence.

**Saída obrigatória**
1) `tasks/issue_task_<YYYYMMDD>_<slug>.md`
2) Implementação em Rust + testes
3) `evidence/<task_id>.md` preenchido com comandos e resultados
