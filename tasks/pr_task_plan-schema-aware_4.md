# plan_4.md — Engine de Geração (Rule-based) Reprodutível + Cobertura de Constraints (MVP forte)

> **Objetivo do Plan 4:** implementar a primeira engine de geração de dados sintéticos **reprodutível (seed)**, guiada por `schema.json` + `plan.json`, com foco em **correção antes de realismo**.
>
> “Rule-based” aqui = regras explícitas + heurísticas determinísticas + validação forte com retry/backtracking.
> IA/LLM *não* entra para gerar valores (isso é Plan 6 e mesmo assim só como planner).

---

## 0) Resultado esperado (em 1 frase)

No final do Plan 4, eu rodo: `schema.json` + `plan.json` → gera dataset (CSV no MVP) para tabelas alvo, **respeitando PK/FK/UNIQUE/NOT NULL/DEFAULT** e um subset útil de **CHECK**, com saída determinística (mesma seed → mesmo output) e relatório de execução.

---

## 1) Escopo e não-escopo

### 1.1 Escopo (entra)
- Engine de geração:
  - Entrada: `schema.json` (Plan 2) + `plan.json` (Plan 3, validado)
  - Saída: dataset + artefatos (relatório, logs estruturados, metadata do run)
- Determinismo:
  - `seed` obrigatório
  - RNG seedado (ex.: ChaCha)
  - ordenação canônica de tabelas/colunas/entidades
- Constraints (MVP obrigatório):
  - **NOT NULL** (sempre preencher)
  - **DEFAULT** (aplicar quando a coluna não tiver regra no plan)
  - **PK** e **UNIQUE** (simples e composto)
  - **FK** (integridade referencial)
  - **CHECK subset nível A**: validar pós-geração + retry/backtracking controlado
- Formato de saída (MVP):
  - **CSV** (um arquivo por tabela)
  - JSONL/SQL ficam como opcionais (feature ou tasks futuras)

### 1.2 Não-escopo (não entra agora)
- Cobrir 100% de CHECK possíveis (vamos por níveis)
- Gerar diretamente no Postgres (pode vir depois)
- Realismo estatístico avançado (distribuições complexas, privacy, etc.)
- LLM gerando valores (somente planner no Plan 6)

---

## 2) Dependências (libs) recomendadas

### 2.1 Geração (core)
- `rand`, `rand_chacha` (RNG determinístico seedável)
- `rand_distr` (opcional: normal/lognormal/zipf)
- `fake` (semântica básica: nome, email, endereço)
- `uuid`
- `chrono`
- `regex`

### 2.2 IO e modelos
- `serde`, `serde_json`
- `csv` (MVP)
- (opcional) `indexmap` se precisar manter ordem estável de mapas

### 2.3 Erros e logs
- `thiserror`
- (opcional) `tracing` atrás de feature flag (lib não deve printar por padrão)

### 2.4 Avançado (NÃO no MVP)
- `z3` (solver SMT) atrás de feature flag para CHECK complexo
  - Só entra se subset+retry não for suficiente.

---

## 3) Entradas e saídas (contratos)

### 3.1 Entradas
- `schema.json` (contrato estável: schemas/tables/columns/constraints/indexes/enums)
- `plan.json` (targets + rules + policies + seed)
- (opcional) `GenerateOptions` (CLI/config):
  - `out_dir`
  - `strict: bool`
  - `max_attempts_cell`
  - `max_attempts_row`
  - `max_attempts_table`
  - `auto_generate_parents: bool`
  - `output_format: csv|jsonl|sql`

### 3.2 Saídas (MVP)
Diretório por run:
- `out/<run_id>/`
  - `<schema>.<table>.csv` (um arquivo por tabela)
  - `generation_report.json`
  - `resolved_plan.json` (plan “final” após defaults/policies normalizados)
  - (opcional) `schema_ref.json` (fingerprint/versão)

**Regra de determinismo**: outputs devem ser reproduzíveis byte-a-byte (para fixtures).

---

## 4) Arquitetura da engine (módulos / tools internas)

> “Tools” aqui = componentes internos bem definidos (padrão AGENTS: separação clara, determinismo e auditabilidade).

### 4.1 Componentes principais

#### A) `TablePlanner`
Responsável por:
- Ler `plan.targets`
- Construir grafo de dependências por FK (pai → filho)
- Definir ordem topológica de geração
- Validar dependências:
  - `auto_generate_parents=true`: inclui pais automaticamente (rows mínimas ou derivadas)
  - `strict=true`: falha se faltarem pais e existir FK obrigatória

Saída:
- lista ordenada de “tarefas de geração”:
  - `{schema, table, rows, generation_mode}`

#### B) `RowBuilder`
Responsável por:
- Montar uma linha respeitando:
  - ordem canônica do schema
  - NOT NULL
  - DEFAULT
  - tipos e serialização
- Delegar geração ao `GeneratorRegistry`
- Aplicar pós-processamento (quando necessário)

#### C) `GeneratorRegistry`
Responsável por:
- Resolver gerador por coluna:
  1) regra explícita do plan
  2) heurística por nome (se habilitada)
  3) fallback por tipo SQL
- Interface uniforme:
  - `generate_value(ctx) -> Value`

**MVP de generators**
- `uuid`
- `int_range`
- `float_range`
- `date_range`
- `timestamp_range`
- `string`
- `email` (fake)
- `name` (fake)
- `enum_pick`
- `bool`

#### D) `UniqueManager`
Responsável por:
- Garantir unicidade para PK/UNIQUE simples e composta
- Estratégias:
  - `sequence`
  - `uuid`
  - `set+retry` (fallback)

#### E) `ForeignKeyResolver`
Responsável por:
- Preencher colunas FK usando pool de chaves de pais gerados
- Estratégias:
  - `uniform` (padrão)
  - `zipf` (opcional)

#### F) `CheckEvaluator` (subset Nível A)
Responsável por:
- Avaliar subset após montar linha
- CHECK desconhecido:
  - `policy=enforce` → erro `Unsupported`
  - `policy=warn/ignore` → warning “not evaluated”

#### G) `RetryEngine`
Responsável por:
- Limites de tentativas por célula/linha/tabela
- Estratégias:
  - UNIQUE → regenerar colunas participantes
  - FK → resample de chave pai
  - CHECK simples → regenerar colunas envolvidas
- Ao exceder tentativas:
  - `strict=true`: fail
  - `strict=false`: reporta e segue (não recomendado no MVP)

#### H) `OutputWriter` (CSV)
Responsável por:
- Escrever CSV por tabela
- Garantir cabeçalho e ordem determinísticos

---

## 5) Cobertura de constraints (o que é obrigatório no Plan 4)

### 5.1 NOT NULL (obrigatório)
- Nunca emitir null em colunas NOT NULL.
- Falha de generator → retry → excedeu → fail.

### 5.2 DEFAULT (obrigatório)
Se não há regra no plan:
- aplica default quando possível (literal simples)
- para defaults complexos (funções):
  - preferir equivalência via generator (ex.: `now()` → timestamp_range)
  - se não der e `strict=true` → exigir regra no plan (falhar com hint)

### 5.3 PK/UNIQUE (obrigatório)
- PK simples: não-null + unique
- PK/UNIQUE composta: unique da tupla
- Se plan configurar generator “ruim” (constante):
  - em `strict=true` deve falhar (idealmente já no validador schema-aware)

### 5.4 FK (obrigatório)
- Gerar pais antes de filhos (toposort)
- Para cada FK: amostrar de pool do pai
- Se houver ciclo de FK:
  - MVP: retornar `Unsupported("cyclic FK graph")` com hint

### 5.5 CHECK subset Nível A (obrigatório)
Suportar:
- comparações: `> >= < <=`
- `IN (...)`
- `BETWEEN a AND b`
- `IS NOT NULL`
- `AND` simples

Não suportar (por enquanto):
- `OR`, `NOT` complexos, funções/casts avançados, subqueries

---

## 6) Determinismo (pontos críticos)

Checklist:
- RNG sempre derivado de seed + contexto estável:
  - `rng_table = H(seed, schema, table)`
  - `rng_row = H(rng_table, row_index)`
- Ordem estável:
  - schemas/tables/columns/constraints ordenados canonicamente
- Serialização:
  - timestamps em ISO 8601
  - floats com formato fixo (se possível)
- Output:
  - mesma ordem de escrita sempre

---

## 7) Estrutura no repo (sugestão)

- `src/generate/`
  - `engine.rs`
  - `planner.rs`
  - `row_builder.rs`
  - `registry.rs`
  - `unique.rs`
  - `fk.rs`
  - `check.rs`
  - `retry.rs`
  - `output/csv.rs`
  - `generators/`
    - `uuid.rs`, `int_range.rs`, `email.rs`, `name.rs`, `enum_pick.rs`, `datetime.rs`, etc.
- `examples/`
  - `generate_csv.rs` (obrigatório)

---

## 8) Plano de execução (tarefas)

### T1 — Esqueleto da engine + dry run
- `GenerationEngine::run(schema, plan, opts)`
- valida inputs e prepara job list

**Aceite**
- compila e roda e imprime um plano de execução (sem gerar dados)

### T2 — OutputWriter CSV
- gerar CSV de tabela fake

**Aceite**
- arquivo gerado com header e ordem estável

### T3 — Registry + fallback por tipo
- implementar generators mínimos por tipo

**Aceite**
- gera N linhas sem null em colunas NOT NULL

### T4 — UniqueManager (simple + composite)
**Aceite**
- 10k linhas sem duplicidade em PK/UNIQUE

### T5 — FKResolver + TablePlanner
**Aceite**
- fixture pai/filho sem FK quebrada

### T6 — CHECK subset + RetryEngine
**Aceite**
- fixture com CHECK simples passa 100%
- CHECK desconhecido respeita policy

### T7 — generation_report.json
Campos mínimos:
- rows requested vs generated
- retries total
- warnings/unsupported (com paths/hints)

**Aceite**
- relatório existe, ordenado e útil para debug

### T8 — Testes
- unit: unique, fk, retry
- integration: fixture → plan mínimo → geração

**Aceite**
- `cargo test` passa

---

## 9) Critérios finais (Definition of Done) — Plan 4

- [ ] Reprodutível: mesma seed → mesmo output (byte-to-byte) para fixture
- [ ] 0 violações PK/UNIQUE/FK/NOT NULL para fixture
- [ ] CHECK subset avaliado ou bloqueado conforme policy
- [ ] Relatório `generation_report.json` com warnings/unsupported
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` passando

---

## 10) Ponte para Plan 5
Plan 5 lê o dataset e gera métricas e relatórios para provar qualidade e comparar runs.
