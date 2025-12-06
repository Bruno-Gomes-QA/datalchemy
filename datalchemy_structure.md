# datalchemy

Objetivo: biblioteca Rust para **introspecção de schema do Postgres** (e, no futuro, geração/validação de dados sintéticos orientados por semântica).  
Preferência: **usar `examples/`** para executáveis de demonstração.

---

## Estrutura de diretórios

```
datalchemy/
├─ Cargo.toml
├─ README.md
├─ LICENSE
├─ .gitignore
├─ src/
│  ├─ lib.rs
│  ├─ error.rs
│  ├─ model/
│  │  ├─ mod.rs
│  │  ├─ schema.rs
│  │  ├─ constraints.rs
│  │  └─ types.rs
│  ├─ introspect/
│  │  ├─ mod.rs
│  │  └─ postgres/
│  │     ├─ mod.rs
│  │     ├─ queries.rs
│  │     └─ mapper.rs
│  └─ utils/
│     ├─ mod.rs
│     └─ pg.rs
├─ examples/
│  ├─ introspect.rs
│  └─ dump_json.rs
└─ tests/
   ├─ integration_introspect.rs
   └─ fixtures/
      └─ sql/
         ├─ 001_schema.sql
         └─ 002_data.sql
```

---

## Responsabilidades

### `src/lib.rs` — API pública da biblioteca
Responsável por:
- Exportar os tipos públicos (por exemplo `DatabaseSchema`, `Table`, `Column`, etc.).
- Expor funções públicas e estáveis, ex.:
  - `pub async fn introspect_postgres(pool: &PgPool, opts: IntrospectOptions) -> Result<DatabaseSchema>`

Deve conter:
- `pub use` reexportando structs e funções principais.
- Documentação (`///`) do que é estável e do que é interno.

---

### `src/error.rs` — Erros e Result
Responsável por:
- Centralizar erros da lib (conexão, query, parse, inconsistência).
- Unificar retorno em algo do tipo:
  - `pub type Result<T> = std::result::Result<T, Error>;`
- Envelopar `sqlx::Error` e outros em `Error::Db(...)` etc.

---

## Camada de domínio

### `src/model/mod.rs` — Módulo raiz do modelo
Responsável por:
- Reexportar submódulos do modelo.
- Ser a “porta de entrada” para `schema.rs`, `constraints.rs`, `types.rs`.

---

### `src/model/schema.rs` — Estrutura do schema
Responsável por:
- Definir structs que descrevem o schema.
- Exemplos:
  - `DatabaseSchema { schemas: BTreeMap<String, Schema> }`
  - `Schema { tables: BTreeMap<String, Table> }`
  - `Table { columns, primary_key, ... }`
- Garantir consistência interna (ex.: ordem de colunas, nomes, invariantes simples).

Decisões:
- Preferir `BTreeMap` para output determinístico.
- Colocar IDs/oid do Postgres como **opcional** e separado (evitar acoplar o modelo ao SGBD).

---

### `src/model/constraints.rs` — Constraints e relacionamentos
Responsável por:
- Tipos de constraint:
  - PK, FK, UNIQUE, CHECK
- Regras que vão ser críticas no futuro para geração sintética consistente:
  - integridade referencial
  - ranges/expressões
  - deferrable/deferred
- Representar ações de FK como enums (mais seguro):
  - `enum FkAction { NoAction, Restrict, Cascade, SetNull, SetDefault, Unknown }`

---

### `src/model/types.rs` — Tipos e metadados de coluna
Responsável por:
- Tipo “amigável” e metadados úteis:
  - `data_type` (formatado), `udt_schema`, `udt_name`
  - nullability, default, identity, generated, collation
- Modelar enums:
  - `EnumType { schema, name, labels }`
- (Futuro) mapear para um type-system próprio (ex.: `DataType::Int`, `DataType::Text`, `DataType::Enum(...)`).

---

## Camada de introspecção (Postgres-first)

### `src/introspect/mod.rs` — Traits e opções genéricas
Responsável por:
- Definir traits/interfaces:
  - `trait Introspector { async fn introspect(&self) -> Result<DatabaseSchema>; }`
- Definir `IntrospectOptions`:
  - schemas a incluir/excluir
  - incluir views? materialized views?
  - incluir comentários?
  - incluir índices?
  - etc.

---

### `src/introspect/postgres/mod.rs` — Introspector do Postgres
Responsável por:
- Implementar o introspector do Postgres.
- Expor entrypoints internos:
  - `pub async fn introspect(pool: &PgPool, opts: &IntrospectOptions) -> Result<DatabaseSchema>`

---

### `src/introspect/postgres/queries.rs` — SQL
Responsável por:
- Conter *somente* strings SQL e funções que chamam `sqlx::query!`.
- Uma função por query, ex.:
  - `list_schemas(...)`
  - `list_tables_in_schema(...)`
  - `list_columns(...)`
  - `list_primary_key(...)`
  - `list_foreign_keys(...)`
  - `list_unique_constraints(...)`
  - `list_check_constraints(...)`
  - `list_indexes(...)`
  - `list_enums(...)`

Boas práticas:
- **Nunca repetir aliases** (o macro do SQLx quebra).
- Normalizar campos `char` do catálogo:
  - `relkind`, `confdeltype`, etc. chegam como `i8` — converter em enums/strings no Rust
  - `attidentity` converter para texto no SQL (ALWAYS/BY DEFAULT) para simplificar.
- Preferir `pg_catalog` para constraints/índices, e `information_schema` apenas para campos mais padronizados quando útil.

---

### `src/introspect/postgres/mapper.rs` — Conversões e normalização
Responsável por:
- Converter tipos crus do `sqlx::query!` (record structs) para o modelo:
  - char codes (`i8`) → enums/strings
  - arrays → `Vec<String>`
- Regras de normalização:
  - ordenar colunas por `attnum`
  - manter ordem do `unnest(... with ordinality)`
  - filtrar schemas do sistema por default

---

## Utilidades

### `src/utils/mod.rs`
- Reexports e helpers usados por mais de um módulo.

### `src/utils/pg.rs`
Responsável por:
- Helpers específicos do Postgres:
  - conversões de códigos para enums (FK actions, match types, relkind)
  - funções pequenas para lidar com `i8` → `char` com segurança

---

## `examples/` — demos e “binários” de uso
Objetivo: permitir testar a lib sem criar `src/bin`.

### `examples/introspect.rs`
Responsável por:
- Conectar no DB via `DATABASE_URL`
- Chamar a API pública da lib
- Mostrar resultado

### `examples/dump_json.rs`
Responsável por:
- Idem, mas imprime JSON completo (para `> schema.json`)

Comandos:
```bash
cargo run --release --example introspect
cargo run --release --example dump_json > schema.json
```

---

## `tests/` — testes de integração
Responsável por:
- Subir Postgres via Docker e aplicar fixtures SQL.
- Rodar introspecção e validar invariantes:
  - PK/FK detectadas
  - CHECKs presentes
  - enums capturados
  - índices retornados
- Garantir que output é determinístico.

---

## API pública sugerida

### Tipos
- `DatabaseSchema`, `Schema`, `Table`, `Column`, `PrimaryKey`, `ForeignKey`, `UniqueConstraint`, `CheckConstraint`, `Index`, `EnumType`
- `IntrospectOptions` (com defaults sensatos)

### Funções
- `pub async fn introspect_postgres(pool: &sqlx::PgPool) -> Result<DatabaseSchema>`
- `pub async fn introspect_postgres_with_options(pool: &sqlx::PgPool, opts: IntrospectOptions) -> Result<DatabaseSchema>`

### Erros
- `Error` com variantes:
  - `Db(sqlx::Error)`
  - `InvalidSchema(String)` (quando invariantes falham)
  - `Unsupported(String)` (quando algo não for suportado ainda)
  - `Other(anyhow::Error)` (opcional)

---

## Decisões de design
- Output determinístico (`BTreeMap`, ordenação estável).
- Evitar acoplamento precoce ao Postgres nos types públicos (mas ok ter módulos internos Postgres).
- Normalizar `char` codes e representar ações (FK) como enums.
- Manter SQL isolado em `queries.rs` para facilitar manutenção.
- `examples/` como “binários de referência” (sem `src/bin`).

---
