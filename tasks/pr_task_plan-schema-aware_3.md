# plan_3.md — Etapa 2 (PIT) — `plan.json` + `plan.schema.json` + Validação Schema-aware

> **Objetivo central do Plan 3:** criar o contrato do **plano de geração** (`plan.json`) e um validador forte, incluindo:
> - validação estrutural (JSON Schema)
> - validação “schema-aware” (o plano referencia tabelas/colunas reais do `schema.json`)
>
> Este Plan 3 é o “cinto de segurança” para usar IA como assistente sem alucinação: a IA pode sugerir um plano, mas o core rejeita o que não é válido.

---

## 0) Resultado esperado (em 1 frase)

Ao final do Plan 3, existe um `plan.json` versionado e validável, um `plan.schema.json` oficial, e um validador que garante que o plano **faz sentido** para um `schema.json` real (sem references quebradas, sem regras impossíveis, sem promessas).

---

## 1) Escopo e não-escopo

### 1.1. Escopo (entra)
- Definir o contrato canônico de `plan.json` (`plan_version`, `seed`, targets, regras).
- Gerar e commitar `plan.schema.json` (JSON Schema oficial).
- Implementar validação:
  1) JSON Schema validation (estrutura)
  2) schema-aware validation (referências e compatibilidade com constraints)
- Criar exemplos e fixtures:
  - `plans/examples/*.plan.json`
  - validador rodando via `examples/validate_plan.rs`
- Definir catálogo de regras suportadas (MVP) e mecanismo de `unsupported`.

### 1.2. Não-escopo (fica fora)
- geração efetiva de dados (Plan 4)
- métricas pós-geração (Plan 5)
- integração “real” com LLM (Plan 6) — aqui só garantimos o pipeline de validação para suportar isso.

---

## 2) Pré-requisitos e setup

### 2.1. Dependências Rust (sugeridas)
- `serde`, `serde_json` (modelo e IO)
- `schemars` (emitir JSON Schema do plan)
- `jsonschema` (validar instâncias do plan)
- `thiserror` (erros)
- `regex` (opcional, para validações simples de padrões)
- (opcional) `indexmap` para preservar ordem em mapas (se necessário)

> Assim como no Plan 2, `jsonschema` pode ficar em `dev-dependencies` se você quiser “core minimalista”.

---

## 3) O contrato do `plan.json` (decisões essenciais)

> **Regra-mãe:** `plan.json` deve ser claro, versionado e validável.  
> Ele descreve **o que gerar** e **como respeitar regras/semântica**, sem depender de “magia”.

### 3.1. Campos obrigatórios (MVP)
Sugestão de top-level:

- `plan_version: string`  
  Ex.: `"0.1"`
- `seed: integer`  
  Ex.: `42` (reprodutibilidade)
- `schema_ref` (obrigatório por robustez):
  - `schema_version: string`
  - `schema_fingerprint: string` (se existir no Plan 2; recomendado)
  - `engine: string` (ex.: `"postgres"`)
- `targets: []` (o que gerar)
  - cada target referencia `schema/table` e define volume
- `rules: []` (regras por tabela/coluna e semânticas de domínio)
- `options` (opcional): flags globais do gerador (quando existir)

### 3.2. Modelagem recomendada de `targets`
Cada item:
- `schema: string` (ex.: `"public"`)
- `table: string` (ex.: `"orders"`)
- `rows: integer` (ex.: `10000`)
- `strategy` (opcional):
  - `"insert_order": "fk_toposort"` (futuro)
  - `"batch_size"` (futuro)
- `overrides` (opcional): ajustes específicos

> O importante é: o target identifica **tabela** e **volume** de forma inequívoca.

### 3.3. Modelagem recomendada de regras (`rules`)
Aqui o objetivo é MVP **orientado por semântica**, mas com validação forte.

Sugestão de rule “tagged union” (tipo + payload):

- `type: "column_generator"`
  - `schema`, `table`, `column`
  - `generator`: `"uuid" | "email" | "name" | "int_range" | "date_range" | "regex" | ...`
  - `params`: objeto com parâmetros (depende do generator)
- `type: "column_distribution"`
  - `schema/table/column`
  - `distribution`: `"uniform" | "normal" | "categorical"`
  - `params`
- `type: "foreign_key_strategy"`
  - `schema/table`
  - `mode`: `"respect"` (default) | `"disable"` (não recomendado; deve exigir flag)
- `type: "constraint_policy"`
  - `schema/table`
  - `constraint`: `"check" | "unique" | "not_null" | ...`
  - `mode`: `"enforce" | "warn" | "ignore"` (default enforce)

> **MVP realista**: comece com `column_generator` + `constraint_policy` + `foreign_key_strategy`.
> O resto pode ser adicionado gradualmente.

### 3.4. `unsupported` como primeira-classe
Para evitar “promessas que não cumprem”, permitir:

- `rules_unsupported: []`
  - cada item contém:
    - descrição
    - motivo (ex.: “generator not implemented”)
    - referência (schema/table/column)

Isso é importante para:
- IA propor coisas que ainda não existem sem quebrar o fluxo
- registrar intenção de pesquisa sem mentir para o runtime

---

## 4) `plan.schema.json` (JSON Schema oficial)

### 4.1. Onde colocar
- `schemas/plan.schema.json` (recomendado, junto do `schema.schema.json` do Plan 2)

### 4.2. Como gerar
Criar exemplo:
- `cargo run --example emit_plan_json_schema > schemas/plan.schema.json`

Commita o arquivo e cria um teste que garante que o schema commitado bate com o gerado.

---

## 5) Validação: duas camadas (estrutural + schema-aware)

> Esta é a parte mais importante do Plan 3.

### 5.1. Validação 1 — Estrutural (JSON Schema)
Entrada: `plan.json`  
Regra: se não passar no `plan.schema.json`, falha com erro claro.

Exemplos de erros:
- campo obrigatório faltando
- tipo errado (string vs number)
- enum inválido

### 5.2. Validação 2 — Schema-aware (contra `schema.json`)
Entrada: `plan.json` + `schema.json`  
Regras mínimas:

**Referências**
- `targets[].schema` existe no `schema.json`
- `targets[].table` existe no schema
- qualquer `rules[].(schema/table/column)` existe
- coluna existe e é compatível com generator (ex.: `uuid` em coluna string/uuid)

**Compatibilidade de constraints (MVP)**
- se o schema tem FK: o plano não pode “desrespeitar” sem flag explícita
- se o schema tem NOT NULL: o plano deve prover generator compatível (ou uma policy explicitando comportamento)
- se UNIQUE/PK: evitar generators que gerem constantes (se `strict=true`, falhar; senão warn)
- CHECK: se a engine de geração ainda não suporta interpretar check, deve:
  - manter `constraint_policy` (enforce/warn/ignore),
  - e registrar warnings/unsupported quando “enforce” for impossível.

**Coerência do plano**
- `rows > 0`
- não duplicar o mesmo target (ou consolidar)
- regras conflitantes (ex.: dois generators diferentes para mesma coluna) devem ser erro.

### 5.3. Saída do validador
O validador deve produzir:
- `Ok(ValidatedPlan)` **ou**
- uma lista de erros/warnings estruturados:
  - `code`
  - `path` (ex.: JSON pointer `/targets/0/table`)
  - `message`
  - `hint` (como corrigir)

> Isso melhora MUITO a UX e evita “debug por adivinhação”.

---

## 6) Integração no repositório (sem bagunçar)

### 6.1. Organização sugerida (single-crate)
Se você ainda não migrou para workspace, dá para manter simples:

- `src/plan/`
  - `mod.rs`
  - `model.rs` (structs do Plan, serde)
  - `schema.rs` (emitir JSON Schema via schemars)
  - `validate.rs` (validação estrutural + schema-aware)
  - `errors.rs`
- `schemas/`
  - `schema.schema.json` (Plan 2)
  - `plan.schema.json` (Plan 3)
- `plans/examples/`
  - `minimal.plan.json`
  - `ecommerce.plan.json` (opcional)
- `examples/`
  - `emit_plan_json_schema.rs`
  - `validate_plan.rs`

### 6.2. Organização sugerida (workspace)
Se você já decidiu migrar:
- `crates/datalchemy-plan/` concentra tudo do Plan
- `datalchemy-core` contém tipos compartilhados e validators comuns

> Recomendação prática: no PIT, só migre para workspace se estiver atrapalhando.  
> O Plan 3 funciona em single-crate desde que haja fronteiras claras.

---

## 7) Fixtures e exemplos (para reduzir alucinação e drift)

### 7.1. Exemplo mínimo de plan (obrigatório)
`plans/examples/minimal.plan.json` deve:
- apontar para 1–2 tabelas do fixture do Postgres
- definir `rows`
- definir 2–3 regras de generators
- ter `seed` fixo

### 7.2. Exemplo “quase real” (recomendado)
`plans/examples/ecommerce.plan.json` (ou similar):
- 4–6 tabelas com dependências
- regras semânticas simples (ex.: e-mail, datas, status)
- mostra como lidar com constraints

### 7.3. Exemplo de validação
`cargo run --example validate_plan -- plans/examples/minimal.plan.json --schema tests/golden/postgres_minimal.schema.json`

> Mesmo que a CLI ainda não exista como binário, exemplos em Rust já funcionam como “comando”.

---

## 8) Sequência de tarefas (execução recomendada)

### T1 — Definir modelo Rust do Plan (`Plan`, `Target`, `Rule`)
- structs `serde` com enums bem definidos
- `plan_version`, `seed`, `schema_ref`

**Aceite T1**
- consegue carregar um plan JSON em memória sem gambiarra

---

### T2 — Emitir `plan.schema.json`
- criar exemplo para gerar JSON Schema via `schemars`
- commitar em `schemas/plan.schema.json`

**Aceite T2**
- schema gerado é estável e versionado

---

### T3 — Validação estrutural (JSON Schema)
- usar `jsonschema` para validar `plan.json` contra `plan.schema.json`

**Aceite T3**
- erros apontam paths claros

---

### T4 — Validação schema-aware (contra `schema.json`)
- indexar `schema.json` em memória (mapas/índices internos)
- validar targets, rules e compatibilidade básica de tipos

**Aceite T4**
- plano com referência quebrada falha com erro claro

---

### T5 — Catálogo de suportado vs unsupported
- lista mínima de `generator` suportados no contrato
- mecanismo para `rules_unsupported` e warnings

**Aceite T5**
- plano pode registrar intenção sem quebrar o core

---

### T6 — Testes (unit + integração)
- unit: validação de conflitos e tipos
- integração: introspect fixture → gerar schema.json → validar plan exemplo

**Aceite T6**
- `cargo test` cobre o pipeline real

---

## 9) Critérios de aceitação finais (DoD do Plan 3)

O Plan 3 está concluído quando:

- [ ] existe `schemas/plan.schema.json` commitado
- [ ] existe `plans/examples/minimal.plan.json` commitado
- [ ] `cargo run --example validate_plan` valida o plan mínimo contra o schema do fixture
- [ ] validação estrutural (JSON Schema) funciona com mensagens úteis
- [ ] validação schema-aware impede referências quebradas e conflitos básicos
- [ ] existe mecanismo de `unsupported`/warnings (sem mentir)
- [ ] `cargo fmt`, `cargo test`, `cargo clippy` passam

---

## 10) Ponte para Plan 4

Com Plan 3 pronto, o Plan 4 pode:
- implementar geração rule-based mínima guiada por `plan.json`
- sem medo de “plano inválido” ou “referência quebrada”
- com seed e reprodutibilidade desde o início
