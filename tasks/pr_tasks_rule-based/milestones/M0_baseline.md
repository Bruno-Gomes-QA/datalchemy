---
milestone: M0
title: "Baselines e guard rails (strict, warnings, coverage report)"
status: Draft
repo: https://github.com/Bruno-Gomes-QA/datalchemy
date: 2026-01-24
---


# M0 — Baselines e guard rails

> Estabilizar o terreno: strict mode, warnings padronizados, métricas/cobertura e regras LGPD by design.


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

Garantir que o pipeline atual (introspect → plan → generate → eval) continue funcionando **sem regressões**,
enquanto adicionamos os “guard rails” essenciais para escalar a complexidade dos generators:

- `--strict` (ou equivalente no plan/global) para transformar “fallback/heurística” em erro quando apropriado.
- Warnings padronizados (tracing) **e** persistidos no `generation_report.json`.
- `GenerationCoverageReport` (uso por generator_id, fallback_count, heurísticas acionadas, contagem de PII) com ordenação determinística.
- Política explícita de privacidade: logs sem PII/segredos e artefatos redigidos.


## Por que isso vem primeiro

Sem M0, qualquer evolução grande (refactors + novos generators + correlação) vira caos:

- Você perde observabilidade (não sabe quantos fallbacks aconteceram).
- “Funciona na minha máquina” aparece (não tem strict e validações repetíveis).
- Risco de LGPD (alguém loga PII por acidente).
- Regressões silenciosas (mudou output, ninguém percebe).

M0 cria um “sinal vital” do gerador.


## Escopo

**Dentro do escopo**
- Alterar/estender `generation_report.json` (campos novos, mantendo compatibilidade quando possível).
- Padronizar warnings (estrutura, categorias, contadores).
- Implementar strict mode (config global no plan).
- Garantir que PII **não aparece** em logs/artefatos.

**Fora do escopo**
- Refatorar pastas de generators (isso é M1).
- Adicionar novos generators (M2/M3/M6).
- Implementar RowContext/ForeignContext (M4/M5).


## Contratos e artefatos (run dir)

A “fonte de verdade do runtime” (AGENTS.md) é: `schema.json`, `plan.schema.json`, `plan.json`, e o diretório `runs/<...>`.
Para geração, os artefatos esperados em `out/<run>/` incluem `*.csv`, `generation_report.json`, `resolved_plan.json`.

**Meta do M0:** manter esses artefatos e enriquecer `generation_report.json` com campos novos sem quebrar consumers.


## Design — warnings e report

### Tipos propostos (no crate datalchemy-generate)

Crie (ou expanda) uma estrutura **determinística** (BTreeMap) para reportar cobertura:

```rust
#[derive(Serialize)]
pub struct GenerationCoverageReport {
  pub generator_usage: BTreeMap<String, u64>, // generator_id -> count
  pub transform_usage: BTreeMap<String, u64>, // transform_id -> count
  pub fallback_count: u64,
  pub heuristic_count: u64,
  pub unknown_generator_id_count: u64,
  pub pii_columns_touched: BTreeMap<String, u64>, // pii_tag -> count
  pub warnings_by_code: BTreeMap<String, u64>, // ex: "GEN_FALLBACK_USED" -> 23
}
```

Warning estruturado (nunca carrega valor gerado):

```rust
pub struct GenWarning {
  pub code: &'static str,
  pub severity: WarningSeverity, // Info/Warn/Error
  pub schema: String,
  pub table: String,
  pub column: String,
  pub generator_id: Option<String>,
  pub message: String,
}
```

### Persistência
- Warnings em `logs.ndjson` via `tracing` (sem PII).
- Contagens em `generation_report.json`.


## Strict mode — regras mínimas

Defina strict como controle global (com override futuro por tabela/coluna):

- `strict=true`:
  - `fallback_for_type` → **erro**.
  - heurística por nome → **erro** (ou warning “hard”; escolha e documente).
  - parâmetros inválidos (min>max etc.) → erro.
  - `null_rate > 0` em `NOT NULL` → erro.
  - generator_id desconhecido → erro.

- `strict=false`:
  - fallback permitido com warning + contador.
  - heurística permitida com warning + contador.


## PII/LGPD — safe by default

Crie um ponto único (ex.: `PrivacyConfig`) para guiar logging e report.

- `privacy.log_pii_values=false` (default).
- `privacy.pii_warning_level=warn` (default) — ou `error` em strict.
- Classificação de PII por heurística do nome da coluna + generator_id (ex.: `semantic.br.cpf` é PII).

Checklist LGPD:
- [ ] Nenhum log contém valores de colunas PII.
- [ ] `config.json` em runs está redigido (não regredir).
- [ ] `generation_report.json` armazena apenas contagens e IDs.


## Plano de implementação (passo a passo)

1) Mapear onde `generation_report.json` é construído hoje.
2) Implementar agregador de cobertura (BTreeMap) + API `record_*`.
3) Implementar strict (ler do plan/global; propagar).
4) Padronizar warnings (códigos) + persistência no report.
5) Testes: agregador, strict, “no PII logs” (mínimo: não logar `GeneratedValue`).
6) Criar task + evidence e registrar comandos/outputs.


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


## Critérios de aceite (DoD)

- [ ] `generation_report.json` inclui cobertura determinística (generator_id, fallback, warnings).
- [ ] Strict mode funciona (fallback/unknown generator/invalid params → erro).
- [ ] Nenhum log/artefato contém valores de PII.
- [ ] `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` passam.
- [ ] E2E Postgres passa e o determinismo (`diff`) é OK.


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

> **Modo de execução:** implemente **somente** esta milestone (M0).  
> **Não avance** para outras milestones.  
> **Não quebre compatibilidade** com os exemplos existentes.  
> **Siga AGENTS.md** (determinismo, privacidade, evidência, qualidade).

**Contexto disponível**
- Este arquivo em `plans/milestones/`
- `AGENTS.md`
- `end_to_end_postgres.md`
- Código atual do repo

**Tarefa**
- Implementar coverage + strict + warnings + privacidade conforme este documento.
- Manter outputs determinísticos.
- Entregar task+evidence.

**Saída obrigatória**
1) `tasks/issue_task_<YYYYMMDD>_<slug>.md`
2) Implementação em Rust + testes
3) `evidence/<task_id>.md` preenchido com comandos e resultados
