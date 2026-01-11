# Comandos do CLI (estado atual)

Este documento descreve **todos os comandos existentes no CLI hoje**, sua finalidade, saida esperada e como executar. Tambem detalha qual crate e funcoes sao chamadas por cada comando.

> CLI atual: `datalchemy` (crate `datalchemy-cli`).

---

## 1) Comando: `datalchemy introspect`

### 1.1 Objetivo
Executa a introspeccao do banco (Postgres) e cria uma **run completa** no diretorio indicado, com artefatos versionaveis:
- `schema.json`
- `config.json` (connection redigida)
- `logs.ndjson`
- `metrics.json`

Este e o **comando oficial para usuarios finais**.

### 1.2 Sintaxe
```bash
datalchemy introspect \
  --conn "postgres://user:pass@host:5432/db" \
  --run-dir runs/
```

### 1.3 Argumentos e flags
- `--conn <CONNECTION_STRING>`
  - **Obrigatorio** (a nao ser que a string seja passada como argumento posicional).
  - Ex.: `postgres://datalchemy:datalchemy@localhost:5432/datalchemy_crm`
- `<CONNECTION_STRING>` (posicional)
  - Alternativa a `--conn`.
  - **Nao** pode ser usada junto com `--conn`.
- `--run-dir <PATH>`
  - Diretorio onde a run sera criada.
  - Default: `runs`
- `--out <PATH>`
  - Caminho extra para escrever uma copia do `schema.json`.
  - O arquivo principal sempre e escrito dentro da run.
- `--schema <SCHEMA>` (multi-uso)
  - Filtra schemas por nome (whitelist).
  - Pode ser usado varias vezes.
- `--strict`
  - Se `true`, falha quando houver ciclos no grafo de FKs.
  - Default: `false`
- `--redact`
  - Redacao de credenciais em `config.json`.
  - Default: `true`
  - **Nao pode ser desabilitado** (o CLI falha se `--redact=false`).
- `--include-system-schemas`
  - Inclui schemas do sistema (`pg_*`, `information_schema`).
  - Default: `false`
- `--include-views`
  - Inclui views na introspeccao.
  - Default: `true`
- `--include-materialized-views`
  - Inclui materialized views.
  - Default: `true`
- `--include-foreign-tables`
  - Inclui foreign tables.
  - Default: `true`
- `--include-indexes`
  - Inclui indexes.
  - Default: `true`
- `--include-comments`
  - Inclui comentarios.
  - Default: `true`

### 1.4 Saida esperada
Dentro de `--run-dir`, o CLI cria uma pasta:
```
<timestamp>__run_<uuid>/
  schema.json
  config.json
  logs.ndjson
  metrics.json
```

- `schema.json` segue o contrato em `schemas/schema.schema.json`.
- `config.json` contem a conexao **redigida** (nao ha credenciais).
- `logs.ndjson` registra eventos do processo.
- `metrics.json` contem metricas calculadas a partir do schema.

### 1.5 Exemplo real (com o CRM local)
```bash
cargo run -p datalchemy-cli -- introspect \
  --conn "postgres://datalchemy:datalchemy@localhost:5432/datalchemy_crm" \
  --run-dir runs/
```

### 1.6 O que ele chama (cadeia de funcoes/crates)
- Crate principal: `crates/datalchemy-cli`
- Funcao de entrada: `crates/datalchemy-cli/src/main.rs`:
  - `main()` -> `run_introspect()`
- Introspeccao:
  - Crate: `crates/datalchemy-introspect`
  - Funcao: `introspect_postgres_with_options()`
- Validacao do contrato:
  - Crate: `crates/datalchemy-core`
  - Funcao: `validate_schema()`
- Metricas:
  - Crate: `crates/datalchemy-eval`
  - Funcao: `collect_schema_metrics()`
- Registry de run:
  - Crate: `crates/datalchemy-cli` (modulo `registry`)
  - Funcoes: `start_run()`, `init_run_logging()`, `write_schema()`, `write_metrics()`

### 1.7 Erros comuns
- **Conexao invalida**: retorna erro de banco (`sqlx::Error`).
- **Engine nao suportado**: apenas `postgres://` e `postgresql://` sao aceitos.
- **Redaction desabilitada**: o CLI falha com erro de configuracao.
- **Ciclos de FK com `--strict`**: falha se o grafo tem ciclos.

---

## 2) Comandos de teste (nao sao do CLI)

Estes **nao** fazem parte do CLI oficial, mas sao usados em desenvolvimento/testes.

### 2.1 Exemplo: `dump_json`
- **Crate**: `crates/datalchemy-introspect`
- **Comando**:
  ```bash
  cargo run -p datalchemy-introspect --example dump_json > schema.json
  ```
- **Objetivo**: gerar `schema.json` deterministico via stdout.
- **Saida esperada**: apenas JSON no stdout (nao cria `runs/`).
- **Uso tipico**: gerar golden files e snapshots de contrato.
- **Funcao chamada**: `introspect_postgres()` / `introspect_postgres_with_options()` dentro do exemplo.

---

## 3) Estado atual do CLI

- **Comando oficial para usuario final**: `datalchemy introspect`.
- **Outros comandos**: nao existem hoje.
- **Comandos de teste**: apenas exemplos (`--example`) dentro de crates.
