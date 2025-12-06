# Regras no projeto `datalchemy`

Este arquivo define **regras, padrões e critérios de aceitação** para quem for implementar mudanças no repositório `datalchemy`.

> O `datalchemy_structure.md` é o **roadmap/arquitetura alvo**.
> Este `AGENTS.md` é o **playbook de execução**: como mexer no repo sem bagunçar.

---

## 1) Objetivo do projeto

- Biblioteca Rust para **introspecção de schema do Postgres** com output determinístico.
- O resultado deve capturar, no mínimo:
  - schemas do usuário (excluindo `pg_*` e `information_schema` por default)
  - tabelas / views / matviews / foreign tables (configurável por opções)
  - colunas com metadados relevantes (tipo, nullability, default, identity, generated, collation, comentários)
  - constraints: PK, FK, UNIQUE, CHECK
  - índices
  - enums
- A lib deve ser **robusta**, com **erros bem definidos** e **API pública estável**.

---

## 2) Princípios gerais

1. **Rust idiomático e seguro**: nada de `unsafe` a menos que seja absolutamente necessário (e documentado).
2. **Determinismo**: output deve ser estável entre execuções (ordenar coleções e evitar `HashMap` no output).
3. **Separação de responsabilidades**: não colocar SQL espalhado; não misturar modelo com queries.
4. **Public API mínima**: exponha apenas o necessário; o resto fica interno.
5. **Zero promessas que não cumprem**: se algo não está implementado, retornar `Error::Unsupported(...)` ou omitir com opção explícita.
6. **Sem bin em `src/bin/`**: usar **`examples/`** para executáveis de demonstração.
7. **Compatibilidade**: suportar Postgres moderno (>= 12 é um alvo razoável por causa de generated columns), mas não falhar feio em versões ligeiramente mais antigas—retornar degrade gracioso se necessário.

---

## 3) Regras de arquitetura (obrigatórias)

### 3.1 Módulos e fronteiras
- `src/model/*` é a **verdade** sobre como representamos schema em memória.
- `src/introspect/postgres/*` contém **toda a lógica específica do Postgres**.
- SQL deve ficar concentrado em `src/introspect/postgres/queries.rs`.
- Conversões e normalizações (ex.: `i8` → enum) ficam em `mapper.rs` e/ou `utils/pg.rs`.

### 3.2 Proibido
- Proibido adicionar `main()` em `src/lib.rs`.
- Proibido colocar executáveis fora de `examples/`
- Proibido “engolir” erro com `unwrap()`/`expect()` em caminho de produção.
- Proibido duplicar aliases no `SELECT` quando usar `sqlx::query!`.

---

## 4) Banco e introspecção (Postgres)

### 4.1 Fonte de verdade
- Constraints e índices: preferir `pg_catalog` (`pg_constraint`, `pg_index`, `pg_class`, `pg_attribute` etc.).
- Metadados padronizados: `information_schema.columns` pode complementar (ex. precision/scale/collation).

### 4.2 Tipos chatos do catálogo (regra do `char`)
Campos `char` no Postgres frequentemente chegam como **`i8`** via `sqlx::query!`.
Deve:
- Converter códigos (`relkind`, `confdeltype`, `confupdtype`, `confmatchtype`) para enums/strings no Rust.
- Para `attidentity`, **normalizar para texto no SQL**:
  - `'a'` → `ALWAYS`
  - `'d'` → `BY DEFAULT`
  - `''`  → `NULL`
- Não usar casts inválidos (ex.: nunca tentar `attgenerated::pg_node_tree`).

### 4.3 Ordem é crítica
- PK/FK/UNIQUE: preservar ordem original usando `unnest(... WITH ORDINALITY)` e ordenar por `ordinality`.
- Colunas: ordenar por `attnum`.

---

## 5) API pública (contrato)

A lib deve expor (no mínimo):

- Tipos públicos: `DatabaseSchema`, `Schema`, `Table`, `Column`, `PrimaryKey`, `ForeignKey`,
  `UniqueConstraint`, `CheckConstraint`, `Index`, `EnumType`
- Opções:
  - `IntrospectOptions` com defaults, ex.:
    - `include_system_schemas: bool` (default false)
    - `include_views: bool` (default true)
    - `include_materialized_views: bool` (default true)
    - `include_foreign_tables: bool` (default true)
    - `include_indexes: bool` (default true)
    - `include_comments: bool` (default true)
    - `schemas: Option<Vec<String>>` (whitelist; default None)
- Funções:
  - `pub async fn introspect_postgres(pool: &sqlx::PgPool) -> Result<DatabaseSchema>`
  - `pub async fn introspect_postgres_with_options(pool: &sqlx::PgPool, opts: IntrospectOptions) -> Result<DatabaseSchema>`

Se algo não for suportado por enquanto (ex.: domains, sequences), documentar e/ou retornar `Unsupported`.

---

## 6) Erros e logging

### 6.1 Erros
- Deve existir `src/error.rs` com:
  - `pub enum Error { Db(sqlx::Error), InvalidSchema(String), Unsupported(String), Other(anyhow::Error) }`
  - `pub type Result<T> = std::result::Result<T, Error>;`
- Converter `sqlx::Error` via `From` para `Error::Db`.

### 6.2 Logging
- Preferir não logar por padrão na lib.
- Se necessário, usar `tracing` como dependência opcional (feature flag), nunca `println!` em código de lib.

---

## 7) Qualidade do código (padrões)

### 7.1 Estilo e lints
Toda mudança deve manter:
- `cargo fmt` limpo
- `cargo clippy --all-targets -- -D warnings` passando (quando possível)

### 7.2 Docs
- Funções públicas e structs públicas devem ter doc comment `///` explicando:
  - o que fazem
  - invariantes
  - limitações conhecidas
  - exemplo mínimo de uso

### 7.3 Sem gambiarras no SQL
- `sqlx::query!` exige aliases consistentes com nomes válidos.
- Evitar nomes reservados (se usar `def`, use `as "def!"` e acessar `r.def`).
- Nunca repetir o mesmo alias no mesmo SELECT.

---

## 8) Exemplos (obrigatório)

Deve manter **no mínimo** um exemplo funcional:

- `examples/dump_json.rs`:
  - lê `DATABASE_URL`
  - chama `introspect_postgres(...)`
  - imprime JSON no stdout

Comando esperado:
```bash
cargo run --release --example dump_json > schema.json
```

---

## 9) Testes (quando adicionados)

Quando mexer na introspecção, preferir adicionar/atualizar testes em `tests/`:

- `tests/integration_introspect.rs`:
  - cria schema de fixture
  - roda introspecção
  - valida: PK/FK/UNIQUE/CHECK, enums, colunas e tipos

Se não houver infraestrutura de Postgres de teste ainda:
- Pelo menos criar “unit tests” para conversores (ex.: char codes → enums).

---

## 10) Critérios de aceitação (Definition of Done)

Uma alteração feita só é considerada pronta quando:

- [ ] `cargo build` passa
- [ ] `cargo test` passa (se houver testes)
- [ ] `cargo fmt` aplicado
- [ ] `cargo clippy` sem warnings relevantes (idealmente `-D warnings`)
- [ ] `cargo run --release --example dump_json`
- [ ] Output JSON contém, no mínimo:
  - schemas/tables/columns
  - PK/FK/UNIQUE/CHECK
  - indexes
  - enums
- [ ] Nenhum `main()` em `lib.rs`
- [ ] Sem `unwrap()`/`expect()` em código de lib

---

## 11) Notas de contexto
- Este projeto está começando **Postgres-first**. Não tentar suportar “todos os bancos”.
- O arquivo `datalchemy_structure.md` é a visão/roadmap. Este arquivo é o rulebook.
- Prioridade atual: **introspecção completa e confiável**.

