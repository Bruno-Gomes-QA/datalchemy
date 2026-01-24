# datalchemy_structure.md — Estrutura atual do Datalchemy

> Fonte de verdade para a organizacao atual do repositorio (workspace).  
> Este documento descreve as pastas, crates e responsabilidades **como estao hoje**.

---

## 1) Visao geral (workspace)

```
datalchemy/
├─ Cargo.toml                 # Workspace (membros e deps fixas)
├─ Cargo.lock                 # Lockfile do workspace
├─ README.md                  # Guia do projeto
├─ AGENTS.md                  # Regras unificadas (PIT + legado valido)
├─ datalchemy_structure.md    # Estrutura atual (este arquivo)
├─ crates/
│  ├─ datalchemy-core/        # Contratos do schema + validacao + redaction
│  ├─ datalchemy-introspect/  # Adapters + queries (Postgres-first)
│  ├─ datalchemy-cli/         # CLI e registry de runs
│  ├─ datalchemy-eval/        # Metricas do schema
│  ├─ datalchemy-plan/        # Contrato do plan + JSON Schema
│  └─ datalchemy-generate/    # Engine de geracao (Plan 4)
├─ fixtures/
│  └─ sql/
│     └─ postgres/            # Fixtures SQL para testes (schema + data)
│        ├─ tables/           # SQL por tabela (ordenado por prefixo)
│        └─ data/             # Carga de dados de teste
├─ docker/
│  └─ compose.postgres.yml    # Postgres para testes de integracao (opcional)
├─ scripts/                   # Scripts de infraestrutura/teste (ex.: postgres_docker.sh)
├─ docs/                      # Documentacao adicional
├─ schemas/                   # JSON Schema oficial do contrato
├─ plans/                     # Exemplos de plan.json (Plan 3)
├─ evidence/                  # Evidencias por task/issue
├─ tasks/                     # Tasks (issue_task_*.md, pr_task_*.md)
├─ runs/                      # Artefatos gerados pelo CLI (gitignored)
├─ target/                    # Build artifacts (gitignored)
└─ schema.json                # Exemplo local/artefato (gitignored)
```

---

## 2) Crates e responsabilidades

### 2.1 `crates/datalchemy-core`
**Responsavel por:**
- Contratos canonicos do schema: `DatabaseSchema`, `Schema`, `Table`, `Column`.
- Constraints: PK/FK/UNIQUE/CHECK e `Index`.
- Tipos e metadados de coluna (identity, generated, enum).
- Validacao interna (`validate_schema`).
- Redaction de connection string.
- Grafo de dependencias por FK (toposort/ciclo).

**Arquivos principais**
- `crates/datalchemy-core/src/schema.rs`
- `crates/datalchemy-core/src/constraints.rs`
- `crates/datalchemy-core/src/types.rs`
- `crates/datalchemy-core/src/validation.rs`
- `crates/datalchemy-core/src/redaction.rs`
- `crates/datalchemy-core/src/graph.rs`

### 2.2 `crates/datalchemy-introspect`
**Responsavel por:**
- Adapters de banco (Postgres-first).
- Queries SQL concentradas em `postgres/queries.rs`.
- Mapeamento e normalizacao de tipos/constraints.

**Arquivos principais**
- `crates/datalchemy-introspect/src/postgres/mod.rs`
- `crates/datalchemy-introspect/src/postgres/queries.rs`
- `crates/datalchemy-introspect/src/postgres/mapper.rs`
- `crates/datalchemy-introspect/src/postgres/utils.rs`
- `crates/datalchemy-introspect/examples/dump_json.rs`

### 2.3 `crates/datalchemy-cli`
**Responsavel por:**
- CLI `datalchemy` com o comando `introspect`.
- Registry de runs (`runs/<timestamp>__run_<id>/`).
- Logs estruturados (`logs.ndjson`) e artefatos (`schema.json`, `config.json`, `metrics.json`).

**Arquivos principais**
- `crates/datalchemy-cli/src/main.rs`
- `crates/datalchemy-cli/src/registry/run.rs`
- `crates/datalchemy-cli/src/registry/logging.rs`

### 2.4 `crates/datalchemy-eval`
**Responsavel por:**
- Metricas do schema (contagens, cobertura, grafo de FK).
- Avaliacao de datasets (PK/FK/UNIQUE/NOT NULL/CHECK subset) + `metrics.json`/`report.md`.

**Arquivos principais**
- `crates/datalchemy-eval/src/schema_metrics.rs`
- `crates/datalchemy-eval/src/engine.rs`
- `crates/datalchemy-eval/src/metrics.rs`
- `crates/datalchemy-eval/src/report.rs`

### 2.5 `crates/datalchemy-plan`
**Responsavel por:**
- Contrato do `plan.json`, JSON Schema e validacao schema-aware (Plan 3).

**Arquivos principais**
- `crates/datalchemy-plan/src/model.rs`
- `crates/datalchemy-plan/src/validate.rs`
- `crates/datalchemy-plan/examples/emit_plan_json_schema.rs`
- `crates/datalchemy-plan/examples/validate_plan.rs`

### 2.6 `crates/datalchemy-generate`
**Responsavel por:**
- Engine de geracao deterministica (CSV) guiada por `schema.json` + `plan.json`.
- Registry de generators/transforms por ID (primitives, transforms, semantic).
- Assets estaticos para pt-BR (nomes, cidades, ruas).

**Arquivos principais**
- `crates/datalchemy-generate/src/engine.rs`
- `crates/datalchemy-generate/src/generators/mod.rs`
- `crates/datalchemy-generate/src/generators/primitives/mod.rs`
- `crates/datalchemy-generate/src/generators/transforms/mod.rs`
- `crates/datalchemy-generate/src/generators/semantic/mod.rs`
- `crates/datalchemy-generate/src/assets.rs`
- `crates/datalchemy-generate/src/checks.rs`
- `crates/datalchemy-generate/examples/generate_csv.rs`
- `crates/datalchemy-generate/assets/pt_BR/`

---

## 3) Fixtures e testes

### 3.1 Fixtures
- `fixtures/sql/postgres/tables/000_schema.sql` (schema CRM + enums)
- `fixtures/sql/postgres/tables/table_XXX_*.sql` (tabelas CRM, ordem deterministica)
- `fixtures/sql/postgres/data/data_XXX.sql` (cargas minimas de dados)
- Nomes em portugues, sem acentos

### 3.2 Docker
- `docker/compose.postgres.yml` sobe o Postgres para testes de integracao.
- Script recomendado: `scripts/postgres_docker.sh` (sobe o container e aplica fixtures).
- Para novos bancos: `scripts/<db>_docker.sh` + `docker/compose.<db>.yml`.

### 3.3 Plan (Plan 3)
- `schemas/plan.schema.json` (JSON Schema oficial do plan).
- `plans/examples/minimal.plan.json` (exemplo minimo valido).
- `plans/examples/m2_primitives.plan.json` (primitives + transforms).
- `plans/examples/m3_ptbr.plan.json` (pt-BR + masks).

---

## 4) Artefatos de execucao

### 4.1 `runs/`
Cada execucao do CLI gera:
- `schema.json`
- `config.json` (connection redigida)
- `logs.ndjson`
- `metrics.json`

---

## 5) Convenções importantes

- Sem `src/bin/`. Executaveis ficam em `examples/` (introspect) e no crate CLI.
- SQL de Postgres centralizado em `crates/datalchemy-introspect/src/postgres/queries.rs`.
- Output deterministico: ordenacao explicita em colecoes.
- Nada de `unwrap()`/`expect()` em caminhos de producao.
