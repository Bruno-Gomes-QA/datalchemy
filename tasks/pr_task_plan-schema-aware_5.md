# plan_5.md — Avaliação e Métricas (Validação Pós-Geração + `metrics.json` + `report.md`) — “provar, não assumir”

> **Objetivo do Plan 5:** transformar “eu acho que os dados estão corretos” em **evidência objetiva**, comparável entre execuções.
>
> Entrada: `schema.json` + `plan.json` + dataset (CSV/JSONL)  
> Saída: `metrics.json` + `report.md` + lista estruturada de violações/warnings.

---

## 0) Resultado esperado (em 1 frase)

No final do Plan 5, existe um avaliador que valida automaticamente **PK/FK/UNIQUE/NOT NULL/CHECK subset** no dataset gerado e emite **métricas e relatório determinísticos** para comparar runs e diagnosticar problemas rapidamente.

---

## 1) Escopo e não-escopo

### 1.1 Escopo (entra)
- Loader de dataset (CSV no MVP, JSONL opcional).
- Validadores:
  - NOT NULL
  - PK/UNIQUE (simple + composite)
  - FK (referential integrity)
  - CHECK subset nível A (mesmo do Plan 4)
- Artefatos:
  - `metrics.json` (machine-readable)
  - `report.md` (humano)
- Integração com pipeline end-to-end:
  - introspect → validate plan → generate → evaluate
- Modo strict vs lenient:
  - `strict=true` por default (falha em violações críticas)

### 1.2 Não-escopo (não entra agora)
- Comparar com dados reais (LGPD + escopo).
- Métricas estatísticas avançadas (KL/JS, similaridade, privacidade diferencial).
- Benchmark/performance tuning pesado (pode vir depois).

---

## 2) Dependências (libs) recomendadas

### 2.1 MVP
- `csv`
- `serde`, `serde_json`
- `thiserror`

### 2.2 Opcional
- `rayon` (paralelismo por tabela, se necessário)
- `hashbrown` (performance em sets/maps)
- reutilizar `check.rs` do Plan 4 (ideal) para não duplicar lógica

---

## 3) Entradas e saídas (contratos)

### 3.1 Entradas
- `schema.json` (Plan 2)
- `plan.json` (Plan 3; inclui seed/policies)
- dataset gerado:
  - `out/<run_id>/<schema>.<table>.csv` (Plan 4)

### 3.2 Saídas
- `out/<run_id>/metrics.json`
- `out/<run_id>/report.md`
- (opcional) `out/<run_id>/violations.json` (lista completa estruturada, se quiser)

**Regra de determinismo:** `metrics.json` e `report.md` devem ser determinísticos para o mesmo dataset.

---

## 4) Loader e parsing (onde avaliação costuma quebrar)

### 4.1 Regras canônicas de parsing
Definir parsing por tipo, alinhado com a serialização do Plan 4:

- ints: parse estrito (`i64`)
- numeric/float: parse cuidadoso (sem scientific se possível)
- bool: `true/false` (opcional aceitar `0/1` se definido no contrato)
- date/timestamp: ISO 8601
- uuid: canonical string
- enum: string exata

### 4.2 Tratamento de null
Definir canonical (igual Plan 4):
- CSV vazio = null
- `NULL` literal: opcional, mas preferível não usar

### 4.3 Falhas de parsing
- `strict=true`: erro com path (tabela/linha/coluna)
- `strict=false`: warning + contar `invalid_values`

---

## 5) Validadores (regras e algoritmos)

### 5.1 Validador de consistência do dataset
Antes de constraints:
- arquivos existem para todas as tabelas target
- headers:
  - contém todas as colunas esperadas
  - não contém colunas surpresa (ou gera warning claro)
- número de linhas:
  - reportar `rows_expected` (do plan) vs `rows_found`

**Aceite**
- o avaliador não “quebra” com datasets incompletos; ele reporta com clareza.

### 5.2 NOT NULL
Para cada coluna NOT NULL:
- `null_count`
- `violations += null_count`

**Aceite**
- reporta por tabela/coluna com exemplos limitados.

### 5.3 PK/UNIQUE (simple + composite)
**Simple**
- `HashSet<Value>` por coluna constraint

**Composite**
- `HashSet<TupleKey>` onde `TupleKey` é:
  - serialização canônica (ex.: JSON compact do array de valores) **ou**
  - string com escaping seguro

Regras:
- PK: não-null e unique
- UNIQUE: unique (e permitir null conforme semântica do Postgres: múltiplos nulls são permitidos em UNIQUE; definir isso explicitamente)
  - **Recomendação:** modelar comportamento “Postgres-like”:
    - UNIQUE permite múltiplos NULLs
    - duplicidade só conta quando todos os campos do unique estão não-null e iguais

**Aceite**
- detecta duplicidades com:
  - contagem
  - até K exemplos (ex.: 20) com `row_index`

### 5.4 FK (integridade referencial)
Para cada FK:
1) construir set das chaves do pai (simple/composite)
2) iterar linhas do filho:
   - se qualquer campo FK é null → tratar como “sem referência” (Postgres não valida FK quando qualquer parte é NULL)
   - caso contrário, checar existência no set pai

**Aceite**
- reporta `broken_refs` e exemplos.

### 5.5 CHECK subset nível A
Reusar o mesmo subset do Plan 4:
- comparações: `> >= < <=`
- `IN (...)`
- `BETWEEN`
- `IS NOT NULL`
- `AND` simples

CHECK desconhecido:
- `policy=enforce`: erro/violação com hint “não suportado”
- `policy=warn`: warning + `not_evaluated++`
- `policy=ignore`: `not_evaluated++`

**Aceite**
- `check.not_evaluated` sempre presente e explica o “porquê”.

---

## 6) Métricas (`metrics.json`) — contrato sugerido

### 6.1 Estrutura mínima recomendada
- `metrics_version`
- `run_id`
- `schema_ref`:
  - `schema_version`
  - `schema_fingerprint` (se existir)
- `plan_ref`:
  - `plan_version`
  - `seed`
  - `plan_hash` (opcional)
- `tables[]`:
  - `{schema, table, rows_found, rows_expected}`
- `column_stats[]` (opcional, pode ser grande):
  - `{schema, table, column, null_count, distinct_count?}`
- `constraints` (resumo global):
  - `not_null {checked, violations}`
  - `pk {checked, violations}`
  - `unique {checked, violations}`
  - `fk {checked, violations}`
  - `check {checked, violations, not_evaluated}`
- `warnings[]`:
  - `{code, path, message, hint}`
- `performance`:
  - `{load_ms, validate_ms, total_ms}`

### 6.2 Determinismo
- Ordenar arrays sempre por `(schema, table, column, constraint_name)`
- Limitar exemplos para tamanho controlado

---

## 7) Relatório humano (`report.md`) — formato recomendado

Seções:
1) **Run summary**
   - seed, plan_version, schema_version, run_id
2) **Targets e contagem de linhas**
3) **Resumo de constraints**
   - uma tabela com violações por tipo
4) **Detalhes por tabela**
   - nulls, unique, fk, check
5) **Warnings e not evaluated**
6) **Top exemplos (limitados)**
7) **Recomendações**
   - ex.: “aumentar max_attempts_row”, “definir generator para coluna X”, etc.

---

## 8) Integração no repo (sugestão)

- `src/eval/`
  - `engine.rs` (orquestração)
  - `load_csv.rs`
  - `parse.rs`
  - `dataset_checks.rs` (headers/rows)
  - `not_null.rs`
  - `unique.rs`
  - `fk.rs`
  - `check.rs` (reuso do Plan 4, se possível)
  - `metrics.rs`
  - `report.rs`
- `examples/`
  - `evaluate_run.rs` (obrigatório)

---

## 9) Plano de execução (tarefas)

### T1 — Loader CSV + validação de headers
**Aceite**
- detecta colunas ausentes/sobrando e reporta claramente
- parsing determinístico

### T2 — NOT NULL
**Aceite**
- gera `null_count` por coluna e violações

### T3 — PK/UNIQUE
**Aceite**
- detecta duplicidade simple/composta
- respeita semântica de UNIQUE com NULLs (Postgres-like)

### T4 — FK
**Aceite**
- detecta referências quebradas
- lida com FKs compostas e nulls corretamente

### T5 — CHECK subset
**Aceite**
- avalia subset e marca `not_evaluated`

### T6 — `metrics.json` e `report.md`
**Aceite**
- arquivos gerados, ordenados e comparáveis

### T7 — Integração end-to-end
Pipeline:
1) introspect fixture → `schema.json`
2) validar plan → `plan.json`
3) gerar dataset (Plan 4)
4) avaliar dataset (Plan 5)

**Aceite**
- pipeline roda em CI/local e produz evidências

---

## 10) Critérios finais (Definition of Done) — Plan 5

- [ ] `examples/evaluate_run.rs` executável
- [ ] `metrics.json` + `report.md` gerados para uma run
- [ ] valida PK/UNIQUE/FK/NOT NULL corretamente na fixture
- [ ] CHECK subset avaliado ou registrado como not-evaluated conforme policy
- [ ] resultados determinísticos e ordenados
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` passando

---

## 11) Ponte para Plan 6

Com métricas, dá para medir cientificamente “sem IA vs com IA (planner)” e provar ganhos/limitações com evidências reprodutíveis.
