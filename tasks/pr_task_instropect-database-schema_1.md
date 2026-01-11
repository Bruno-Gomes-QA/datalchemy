#  MVP — Fundação + Introspecção (PostgreSQL) + Registry de runs

> **Contexto**: este plano é o primeiro “bloco executável” da Etapa 2 do Datalchemy. O objetivo aqui é **colocar a mão na massa** e chegar num estado em que **um comando único de CLI consegue introspectar um banco PostgreSQL e gerar um `schema.json` padronizado**, com **logs e artefatos reprodutíveis por execução** (pasta `runs/`), preparando o terreno para multi-DB e para o contrato de `GenerationPlan` nas próximas entregas.

---

## 0) Objetivo, escopo e não-escopo

### Objetivo do Plan 1 (o que “funciona” ao final)
1) **CLI funcional**: executar `datalchemy introspect <CONNECTION_STRING>` e obter um `schema.json`.
2) **Modelo interno de schema** (tipos Rust) estável o suficiente para:
   - representar tabelas, colunas, tipos e constraints (mínimo: **NOT NULL, PK, FK, UNIQUE, CHECK, DEFAULT**);
   - serializar para JSON em um formato único (`schema.json`);
   - calcular **grafo de dependências por FK** e fornecer uma **ordenação topológica** (ou relatório de ciclo).
3) **Registry de runs**: cada execução cria uma pasta `runs/<timestamp>__run_<id>/` contendo:
   - `schema.json`
   - `config.json` (com connection string **redigida**)
   - `logs.ndjson` (logs estruturados)
   - `metrics.json` (métricas mínimas do schema extraído)
4) **Base para multi-DB**: existir um contrato (trait) de adapter e detecção por URL scheme, mesmo que **MySQL/SQLite entrem no Plan 2**.

> Nota: o baseline atual do projeto (conforme o registro de 10/01/2026) é “introspect só de PostgreSQL”. Este plan assume isso e foca em **fazer o PostgreSQL ficar robusto + padronizado + auditável** antes de expandir.

### Escopo (entra)
- Refatoração incremental da introspecção atual de Postgres para produzir o `DatabaseSchema` padronizado.
- Criação do “core” mínimo: tipos de schema + adapter trait + introspector + registry.
- CLI `datalchemy` com subcomando `introspect`.
- Testes mínimos:
  - unitários (serialização, redaction, grafo FK);
  - integração (Postgres via Docker) para garantir “não quebrou”.

### Não-escopo (fica fora)
- Geração real de dados (insert em lote, engine estatística, LLM, etc.).
- Suporte completo a MySQL/SQLite (isso vira Plan 2).
- DSL própria para regras de negócio (apenas representação mínima, e/ou guardar `raw_sql` quando necessário).

---

## 1) Definition of Done (DoD) do Plan 1

O Plan 1 está “done” quando TODOS os itens abaixo forem verdadeiros:

### 1.1. CLI e artefatos
- [ ] Consigo rodar **localmente**:  
  `cargo run -- datalchemy introspect "<CONN>" --run-dir runs/`  
  e isso gera uma pasta de run com os arquivos esperados.
- [ ] O `schema.json` gerado tem **estrutura estável** e contém, no mínimo:
  - [ ] tabelas e colunas (nome, tipo, nullable, default quando existir)
  - [ ] PK (incluindo composta)
  - [ ] FK (incluindo composta)
  - [ ] UNIQUE (single e multi-coluna)
  - [ ] CHECK (capturando expressão e/ou `raw_sql`)
- [ ] A pasta de run contém:
  - [ ] `schema.json`
  - [ ] `config.json` com **redaction obrigatório** (não pode vazar senha/token)
  - [ ] `logs.ndjson`
  - [ ] `metrics.json`

### 1.2. Qualidade mínima e reprodutibilidade
- [ ] Há um **identificador de run** e um timestamp padronizado no nome da pasta.
- [ ] O `config.json` registra:
  - engine detectada (postgres)
  - flags (ex.: strict mode)
  - commit hash (quando disponível) OU “dirty state” (quando não estiver em git)
  - versões do contrato (ex.: `schema_version`).
- [ ] Logs são **estruturados** e suficientes para debugar (pelo menos: start/end, engine, contagens, erros).

### 1.3. Testes
- [ ] `cargo test` passa em ambiente limpo.
- [ ] Existe ao menos 1 teste de integração que:
  - sobe um Postgres com um schema mínimo (docker compose),
  - roda a introspecção,
  - e valida que o JSON gerado contém PK/FK/UNIQUE em um caso simples.

---

## 2) Pré-requisitos (o que instalar e configurar)

> A ideia aqui é reduzir “tempo perdido” com setup.

### 2.1. Ferramentas obrigatórias
- **Rust toolchain (stable)**:
  - `rustc`, `cargo`, `rustfmt`, `clippy`
- **Docker + Docker Compose** (para testes de integração)
- **Git** (para registrar commit hash no `config.json`)

### 2.2. Dependências de sistema (para compilar crates comuns)
> Dependendo do stack (especialmente SQLx e TLS), pode ser necessário instalar libs nativas.

**Ubuntu/Debian (exemplo)**
- `build-essential`
- `pkg-config`
- `libssl-dev`
- `ca-certificates`

**macOS**
- Xcode Command Line Tools
- `pkg-config` (brew)
- OpenSSL (brew) se necessário

**Windows**
- Visual Studio Build Tools (C++)
- Atenção com SSL/TLS; preferir runtime `rustls` no Rust para reduzir dependências de OpenSSL.

---

## 3) Mudanças propostas no repositório (estrutura)

> Objetivo: separar responsabilidades sem “re-arquitetar demais”.

### 3.1. Opção A (mínima): manter 1 crate e organizar módulos
Recomendado se você quer mover rápido no início.

**Estrutura sugerida**
- `src/`
  - `lib.rs`
  - `main.rs` (CLI)
  - `core/`
    - `schema.rs` (tipos do modelo interno)
    - `constraints.rs` (enum/structs para PK/FK/UNIQUE/CHECK/DEFAULT)
    - `graph.rs` (grafo FK + toposort)
  - `adapters/`
    - `mod.rs`
    - `postgres.rs`
    - `mysql.rs` (stub, Plan 2)
    - `sqlite.rs` (stub, Plan 2)
  - `introspector/`
    - `mod.rs`
    - `introspect.rs` (orquestra: adapter -> DatabaseSchema -> artefatos)
  - `registry/`
    - `mod.rs`
    - `run.rs` (cria pasta, escreve config/logs/metrics)
    - `redaction.rs` (remove segredos de conn string)
  - `metrics/`
    - `mod.rs`
    - `schema_metrics.rs`
- `examples/`
  - `introspect_postgres.rs` (exemplo rápido)
- `tests/`
  - `integration_introspect_postgres.rs`
- `docker/`
  - `compose.postgres.yml`
  - `init/` (SQL de schema mínimo para testes)

### 3.2. Opção B (mais “profissional”): workspace com `core` + `cli`
Recomendado se você já sabe que vai crescer rápido.

- `crates/datalchemy-core/` (schema, adapters, introspector, registry)
- `crates/datalchemy-cli/` (CLI, parsing de args, UX)
- `Cargo.toml` na raiz como workspace

> Neste Plan 1: **fazer Opção A** é suficiente. A migração para workspace pode acontecer no Plan 2/3 se a base estabilizar.

---

## 4) Stack de crates (dependências) — sugeridas para o Plan 1

> O registro de 10/01/2026 cita explicitamente alguns crates e referências. Aqui vai a lista “mínima necessária”.

### 4.1. Runtime / CLI / Serialização
- `tokio` (runtime async)
- `clap` (CLI)
- `serde`, `serde_json` (JSON)
- `thiserror` (erros)
- `chrono` (timestamp de run)
- `uuid` (run_id) **ou** contador incremental simples (você escolhe)

### 4.2. Banco e introspecção (Postgres)
- `sqlx` com features:
  - `postgres`
  - `runtime-tokio-rustls` (preferível para evitar OpenSSL)
  - **(opcional já no Plan 1)** `any` (para detecção multi-DB via URL scheme)
- Alternativa: usar `sqlx::AnyConnection` já agora; mas pode trazer complexidade inicial. O essencial do Plan 1 é: **Postgres robusto + contrato de adapter**.

### 4.3. Grafo FK e utilitários
- `petgraph` (toposort / detecção de ciclos)
- `tracing` + `tracing-subscriber` (logs estruturados)
- `schemars` (JSON Schema gerado a partir de structs — aqui pode ser usado já para o contrato de `plan.schema.json` ou para validar `schema.json` no futuro)
- `jsonschema` (validação de JSON Schema; pode entrar ainda no Plan 1 como “esqueleto” — a validação forte de `plan.json` é Plan 2)

### 4.4. CHECK constraints (apenas captura / parsing mínimo)
- `sqlparser` (opcional nesta entrega; use se você quiser começar a transformar CHECK em AST mínimo. Caso contrário, guardar `raw_sql` e marcar como “não interpretado”.)

---

## 5) Contratos e modelos (o coração do Plan 1)

### 5.1. SchemaVersion e compatibilidade
Defina uma constante (ex.: `schema_version = "0.1"`) e inclua no JSON gerado.
- Isso evita drift e facilita futuras migrações.

### 5.2. Modelo interno de schema (tipos Rust)
Meta: um modelo interno consistente, serializável e testável.

**Estruturas sugeridas (mínimo)**
- `DatabaseSchema`
  - `database: String` (nome do DB, quando disponível)
  - `engine: String` (ex.: "postgres")
  - `schemas: Vec<Schema>`
  - `schema_version: String`
  - `fingerprint: Option<String>` (hash do schema, opcional Plan 1)
- `Schema`
  - `name: String` (ex.: "public")
  - `tables: Vec<Table>`
- `Table`
  - `name: String`
  - `columns: Vec<Column>`
  - `constraints: Vec<Constraint>`
  - `indexes: Vec<Index>` (pode ficar vazio no Plan 1)
  - `comment: Option<String>`
- `Column`
  - `name: String`
  - `data_type: String`
  - `nullable: bool`
  - `default: Option<String>`
  - `is_generated: bool` (default false no Plan 1)
  - `comment: Option<String>`

**Constraint enum (mínimo)**
- `PrimaryKey { name: Option<String>, columns: Vec<String> }`
- `ForeignKey { name: Option<String>, columns: Vec<String>, ref_table: String, ref_columns: Vec<String>, on_delete: Option<String>, on_update: Option<String>, deferrable: Option<bool> }`
- `Unique { name: Option<String>, columns: Vec<String> }`
- `Check { name: Option<String>, raw_sql: Option<String>, expr: Option<RuleExpr> }`
- `NotNull { column: String }` (opcional: pode ser derivado de `Column.nullable == false`, mas manter separado ajuda em uniformização)
- `Default { column: String, value: String }` (opcional: pode ser derivado de `Column.default`)

> Observação prática: NOT NULL e DEFAULT podem ficar apenas como propriedades da coluna no Plan 1. O importante é não perder informação.

### 5.3. Trait do adapter (multi-DB-friendly)
Mesmo que só Postgres implemente de verdade agora, já deixe o contrato.

**Exemplo de responsabilidades**
- `fn engine(&self) -> &'static str`
- `async fn introspect(&self) -> Result<DatabaseSchema>`
- `async fn ping(&self) -> Result<()>` (opcional)
- `fn capabilities(&self) -> AdapterCapabilities` (opcional)

### 5.4. Introspector (orquestrador)
Responsável por:
- escolher o adapter (por enquanto: Postgres; com stub para o resto),
- executar introspecção,
- validar consistência interna mínima (`SchemaValidator` simples),
- gerar grafo de dependência,
- enviar tudo para o `Registry` gravar.

---

## 6) Pipeline do comando `datalchemy introspect` (passo a passo)

### 6.1. Inputs do CLI
**Obrigatórios**
- `--conn <CONNECTION_STRING>` (ex.: `postgres://user:pass@localhost:5432/db`)

**Opcionais recomendados**
- `--schema <name>` (default: `public`)
- `--run-dir <path>` (default: `runs/`)
- `--out <path>` (se quiser salvar `schema.json` fora da pasta de run)
- `--strict` (default: false)
  - strict = falhar se não conseguir capturar algum tipo de constraint que “prometemos” suportar no mínimo.
- `--redact` (default: true; e não permitir desligar em modo CI)

### 6.2. Fluxo de execução
1) CLI lê args e cria `RunContext`.
2) `Registry::start_run(ctx)` cria:
   - pasta `runs/<timestamp>__run_<id>/`
   - `config.json` inicial (com redaction)
   - inicializa `logs.ndjson`
3) `Introspector` chama `PostgresAdapter::introspect()`.
4) Resultado (`DatabaseSchema`) passa pelo `SchemaValidator`:
   - valida que todas FKs referenciam tabela/colunas existentes (consistência interna)
   - valida que PK columns existem na tabela
   - valida duplicidades de nomes
5) `GraphBuilder` constrói o grafo FK:
   - gera `dependency_order` por schema (toposort)
   - se houver ciclo, registra relatório no `metrics.json` e no log (não falha necessariamente, a menos que `--strict`)
6) `MetricsCollector` calcula:
   - contagem de schemas/tabelas/colunas
   - contagem de constraints por tipo
   - cobertura (% de tabelas com PK, % de colunas not null etc.)
7) `Registry::write_schema(schema.json)`
8) `Registry::write_metrics(metrics.json)`
9) `Registry::finish_run(status)` escreve “end marker” no log e finaliza.

---

## 7) Introspecção PostgreSQL (MVP robusto)

> O Roadmap indica que o caminho mais robusto é combinar `information_schema` + `pg_catalog`. No Plan 1, foque em capturar o mínimo com confiança.

### 7.1. Checklist de extração (por tabela/coluna)
- [ ] Nome da tabela
- [ ] Colunas:
  - [ ] nome
  - [ ] tipo SQL (texto padronizado)
  - [ ] nullable
  - [ ] default
  - [ ] comentários (se existirem; opcional)
- [ ] Constraints:
  - [ ] PK
  - [ ] FK (incluindo ref_table/ref_columns)
  - [ ] UNIQUE (incluindo compostas)
  - [ ] CHECK (capturar raw_sql ou expressão quando possível)
  - [ ] NOT NULL (pode vir pelo `nullable=false`)
  - [ ] DEFAULT (pode vir pelo `default`)

### 7.2. “Provas” de que está correto (como validar sem adivinhar)
- Compare:
  - `information_schema.columns` vs o que você está serializando
  - `pg_constraint`, `pg_class`, `pg_attribute`, `pg_namespace` para constraints
- Faça um schema mínimo controlado (docker init SQL) e use como “golden sample” no teste.

---

## 8) Registry de runs (auditável e sem vazar segredo)

### 8.1. Nome de pasta e layout
Padrão sugerido:
- `runs/2026-01-10T12-34-56Z__run_00001/`

Conteúdo mínimo do Plan 1:
- `schema.json`
- `config.json` (connection redacted + flags)
- `logs.ndjson`
- `metrics.json`

### 8.2. Redaction de connection string
Regras:
- Nunca armazenar `password`, `token`, `api_key`.
- Pode armazenar: engine, host, port, dbname, user (opcional), e uma versão “mask” do user.

Teste obrigatório:
- um teste unitário que garante que uma string contendo `:senha@` vira `:***@` ou similar.

### 8.3. Logs NDJSON
Cada linha é um JSON.
Eventos mínimos:
- `run_started`
- `engine_detected`
- `introspection_started`
- `introspection_finished`
- `schema_written`
- `metrics_written`
- `run_finished` (com status + duração)

---

## 9) Métricas mínimas (Plan 1)

### 9.1. O que medir
- contagem:
  - schemas, tabelas, colunas
  - constraints por tipo (pk/fk/unique/check)
- cobertura:
  - % tabelas com PK
  - % tabelas com pelo menos 1 FK
  - % colunas NOT NULL
- dependências:
  - total de arestas FK no grafo
  - resultado de toposort (ok/ciclo)
  - se ciclo: listar ciclo detectado (melhor esforço)

### 9.2. Formato do metrics.json (sugestão)
- `schema_version`
- `engine`
- `counts`
- `coverage`
- `fk_graph`
- `warnings` (lista)

---

## 10) Testes (o mínimo para não quebrar amanhã)

### 10.1. Unit tests (sem Docker)
- [ ] `redaction.rs`: remove segredos corretamente
- [ ] `schema.rs`: serialização JSON é estável (snapshot test opcional)
- [ ] `graph.rs`: toposort e detecção de ciclos (grafo pequeno em memória)

### 10.2. Integration test (com Docker Compose)
Estratégia:
- `docker/compose.postgres.yml` sobe um Postgres com:
  - user/pass fixos (somente local/CI)
  - volume init com `docker/init/schema.sql` criando:
    - 3–5 tabelas
    - PK composta em uma tabela
    - FK composta em outra
    - UNIQUE em coluna e multi-coluna
    - 1 CHECK simples (ex.: `qtd > 0`)
- Teste:
  1) sobe docker
  2) roda `datalchemy introspect ...`
  3) parseia `schema.json` e verifica que:
     - existe pelo menos 1 PK, 1 FK, 1 UNIQUE, 1 CHECK
     - FK referencia tabela/colunas existentes
  4) derruba docker

> Dica: em CI, use `docker compose up -d` antes do `cargo test` e `down` ao final.

---

## 11) Lista de tarefas (sequência recomendada)

> Ordem pensada para reduzir retrabalho e manter “sempre verde”.

### Tarefa 1 — Criar o modelo interno de schema (core/types)
**Entregáveis**
- Tipos Rust do schema + serde
- Serialização JSON

**Critérios de aceite**
- Compila
- Há teste unitário simples serializando um schema fake

### Tarefa 2 — Implementar `Registry` (runs/)
**Entregáveis**
- Criação de pasta run
- Escrita de config/logs
- Redaction testado

**Critérios de aceite**
- Ao rodar um “dry run” em memória, cria pasta e arquivos sem segredos

### Tarefa 3 — Adapter trait + PostgresAdapter (refatorar o que já existe)
**Entregáveis**
- `PostgresAdapter::introspect()` retornando `DatabaseSchema`

**Critérios de aceite**
- Consegue extrair tabelas e colunas do Postgres no docker de teste

### Tarefa 4 — `Introspector` + `SchemaValidator` mínimo
**Entregáveis**
- Orquestração: adapter -> valida -> registry
- Validações mínimas internas

**Critérios de aceite**
- Falhas úteis (mensagem clara) quando FK aponta para algo inexistente (simulado)

### Tarefa 5 — Grafo FK + `metrics.json`
**Entregáveis**
- build do grafo
- toposort (ou ciclo)
- métricas básicas

**Critérios de aceite**
- `metrics.json` existe e tem contagens coerentes

### Tarefa 6 — CLI `datalchemy introspect`
**Entregáveis**
- `src/main.rs` com `clap`
- flags definidas em 6.1

**Critérios de aceite**
- `cargo run -- datalchemy introspect ...` roda e produz run completo

### Tarefa 7 — Testes de integração (Docker)
**Entregáveis**
- compose + init SQL
- integration test rodando

**Critérios de aceite**
- `cargo test` passa e valida o conteúdo do `schema.json`

---

## 12) Critérios de aceite finais (resumo)

Para considerar **Plan 1 concluído**, você deve conseguir demonstrar:

1) **Comando**  
   `datalchemy introspect "<CONN_POSTGRES>" --run-dir runs/`  
   gera uma run com todos os arquivos.

2) **Conteúdo do schema.json**  
   inclui tabelas/colunas e constraints mínimas (PK/FK/UNIQUE/CHECK) em um schema de teste.

3) **Registro reprodutível**  
   `config.json` não vaza segredo, `logs.ndjson` tem começo/fim, `metrics.json` tem contagens.

4) **Testes passando**  
   `cargo test` passa (unit + integração).

---