# Task — Contrato Estável do `schema.json` (Versionamento + JSON Schema + Golden Tests)

> **Objetivo central do Plan 2:** transformar o output da introspecção (Plan 1) em um **contrato estável, versionado, validável e testável**.  
> A partir daqui, qualquer evolução do projeto (planificação, geração, métricas) se apoia em um `schema.json` que:
> - muda **só quando intencional**,  
> - muda **com versionamento**,  
> - muda **com validação e testes**.

---

## 0) Resultado esperado (em 1 frase)

Ao final do Plan 2, o projeto gera um `schema.json` **determinístico** com `schema_version`, possui um **JSON Schema oficial** (`schema.schema.json`) e mantém **golden tests/snapshots** garantindo que mudanças no contrato sejam deliberadas.

---

## 1) Escopo e não-escopo

### 1.1. Escopo (entra)
- Definir e fixar o **formato canônico** do `schema.json`.
- Adicionar `schema_version` e (opcional) `schema_fingerprint`.
- Implementar **ordenação estável** de todos os arrays relevantes.
- Criar **JSON Schema oficial** do `schema.json` (gerado por `schemars` e commitado no repo).
- Criar **testes de contrato**:
  - validação do JSON gerado contra o JSON Schema,
  - golden files / snapshots para fixtures conhecidas.
- Documentação mínima do contrato (README ou `docs/schema_json.md`).

### 1.2. Não-escopo (fica fora)
- `plan.json` (isso é Plan 3).
- geração de dados (Plan 4).
- métricas avançadas (Plan 5).
- suporte multi-DB real (Plan 3+ / futuro).

---

## 2) Pré-requisitos e setup

### 2.1. Ferramentas
- Rust stable + `cargo` (fmt/clippy).
- Docker + Docker Compose (para fixtures de Postgres em integration tests).
- Git (para travar golden files e auditar mudanças).

### 2.2. Dependências Rust (sugeridas)
- `serde`, `serde_json` (já deve existir)
- `schemars` (gerar JSON Schema a partir de structs)
- `jsonschema` (validar o JSON gerado contra o JSON Schema)
- `insta` (opcional, para snapshot testing) **ou** golden files manuais em `tests/golden/`
- `sha2` (opcional, para fingerprint do schema)

> Observação: se você quiser manter o core super enxuto, `jsonschema` e `insta` podem ficar em `dev-dependencies`.

---

## 3) Decisões de contrato (as regras que impedem drift)

> **Regra-mãe:** contrato é mais importante que conveniência.

### 3.1. Campos obrigatórios do `schema.json`
Definir como **obrigatórios** (MVP do contrato):

- `schema_version: string`  
  Ex.: `"0.2"` (o Plan 2 define e começa a versionar de verdade)
- `engine: string`  
  Ex.: `"postgres"`
- `schemas: []`  
  Lista de schemas do usuário (por default exclui `pg_*` e `information_schema`)
- Para cada schema:
  - `name`
  - `tables` (inclui tabelas e, se habilitado, views/matviews/foreign tables)
- Para cada table:
  - `name`
  - `kind` (ex.: `table | view | matview | foreign_table`)
  - `columns`
  - `constraints` (PK/FK/UNIQUE/CHECK)
  - `indexes` (se `include_indexes=true`)
  - `enums` (no nível de schema ou global, conforme modelo atual)

> **Importante:** definir *onde* enums vivem no JSON (global vs por schema) e manter isso fixo.

### 3.2. Campos recomendados (mas opcionais)
- `generated_at` (opcional; se existir, deve ser **desabilitável** para não quebrar determinismo em diffs)
- `schema_fingerprint` (hash estável do conteúdo “sem timestamps”)
- `capabilities` (ex.: flags do engine e do introspector; útil para multi-DB futuro)

### 3.3. Proibição de campos instáveis
O `schema.json` **não pode** conter:
- timestamps por default (quebram diffs)
- ids aleatórios
- credenciais ou connection string

Se você quiser um timestamp, ele deve:
- estar fora do `schema.json` (ex.: `runs/config.json`)
- ou existir só quando `--include-meta` (opção explícita) e **não** usado em golden tests.

---

## 4) Determinismo: regras de ordenação (sem isso, nada fica confiável)

> Determinismo não é “nice-to-have”. É requisito.

### 4.1. Ordenação canônica
Definir e implementar ordenação estável para:

- `schemas`: por `name`
- `tables`: por `name` (e/ou por `kind`, mas preferível por `name`)
- `columns`: por ordem natural do Postgres (`attnum`) **ou** por `name` (mas escolha 1 e fixe)
- `constraints`: por tipo + nome + colunas (ordem fixa)
- `indexes`: por nome
- `enums`: por nome, valores em ordem

### 4.2. “Ordem semântica” dentro de constraints
Para constraints multi-coluna (PK/FK/UNIQUE):
- preservar a ordem original (no Postgres: `WITH ORDINALITY`)
- serializar mantendo essa ordem (para evitar mudanças “invisíveis”)

---

## 5) Versionamento do contrato (`schema_version`)

### 5.1. Quando bump de versão é obrigatório
Bump de `schema_version` sempre que:
- um campo mudar de nome, tipo ou posição relevante,
- uma regra de ordenação mudar,
- uma entidade nova passar a ser emitida por default,
- a semântica de um campo mudar (ex.: `kind` antes era inferido, agora é explícito).

### 5.2. Estratégia recomendada de versão
- `"0.x"` enquanto estiver em PIT (iterando rápido)
- congelar `"1.0"` quando:
  - `schema.json` estável por múltiplos ciclos,
  - testes robustos,
  - planificação depende fortemente do contrato (Plan 3 consolidado).

---

## 6) JSON Schema oficial do `schema.json`

### 6.1. Objetivo do JSON Schema
- Validar estrutura e tipos
- Prevenir regressões (ex.: campo sumiu)
- Documentar contrato formalmente

### 6.2. Onde colocar
Escolha um local e padronize:
- `schemas/schema.schema.json` **(recomendado)**
ou
- `contracts/schema.schema.json`

### 6.3. Geração vs arquivo commitado
Recomendado:
- gerar via `schemars` num exemplo:
  - `cargo run --example emit_schema_json_schema > schemas/schema.schema.json`
- commitar o arquivo
- ter um teste que verifica que o arquivo commitado bate com a geração atual (para evitar “esquecer de atualizar”).

---

## 7) Testes de contrato: golden files / snapshots

### 7.1. Fixture Postgres (mínima e representativa)
Criar um schema “padrão de teste” com:
- PK composta
- FK composta
- UNIQUE multi-coluna
- CHECK simples
- DEFAULT
- enum
- índice simples e composto

Local sugerido:
- `fixtures/sql/postgres/00_schema.sql`

### 7.2. Golden file do schema.json
Gerar e versionar um `schema.json` de referência:
- `tests/golden/postgres_minimal.schema.json`

Esse arquivo deve:
- ser gerado por `dump_json` (ou função equivalente)
- ser estável (sem timestamps)

### 7.3. O que o teste valida
No mínimo:
1) introspecção roda sem erro
2) JSON gerado valida contra `schemas/schema.schema.json`
3) JSON gerado é idêntico ao golden file (diff é o “alarme” de mudança de contrato)

> Se você preferir `insta`, usar snapshot. Se preferir “puro git”, usar golden file e comparar strings.

---

## 8) Mudanças no repositório (sugestão prática)

### 8.1. Adicionar pasta de schemas
- `schemas/`
  - `schema.schema.json`

### 8.2. Adicionar docs mínimos
- `docs/`
  - `schema_json.md` (documenta campos e exemplos)

### 8.3. Adicionar fixtures e golden
- `fixtures/sql/postgres/00_schema.sql`
- `tests/golden/postgres_minimal.schema.json`

### 8.4. Exemplos
- `examples/emit_schema_json_schema.rs` (gera JSON Schema)
- `examples/dump_json.rs` (já existe; manter como padrão)

---


## 9) Critérios de aceitação finais (DoD do Plan 2)

O Plan 2 está concluído quando:

- [ ] `schema.json` possui `schema_version` e formato canônico estável
- [ ] existe `schemas/schema.schema.json` commitado
- [ ] `cargo run --example emit_schema_json_schema` gera o mesmo schema commitado (ou teste equivalente)
- [ ] existe fixture Postgres + teste de integração
- [ ] JSON gerado valida contra o JSON Schema
- [ ] existe golden file (ou snapshot) e comparação automática
- [ ] `cargo fmt`, `cargo test`, `cargo clippy` passam

---

## 10) Riscos e armadilhas (para não travar)

- Campos que variam por versão do Postgres (ex.: generated/identity)  
  → usar `Option<>` e documentar degrade gracioso.
- Comentários/collation podem variar conforme locale e settings  
  → manter “include_comments” configurável e ter fixture controlada.
- CHECK expressions podem vir com diferenças de formatação  
  → capturar raw SQL, mas normalizar whitespace se necessário (com cuidado).

---