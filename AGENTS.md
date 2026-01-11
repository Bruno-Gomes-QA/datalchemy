# AGENTS.md — Datalchemy (PIT + legado valido)

> Este arquivo e a **fonte de verdade de contribuicao**.  
> Ele mescla as regras do PIT com regras validas do legado.  
> **Prioridade sempre do PIT**; o legado so entra quando nao conflita.

---

## 1) Visao do projeto (PIT)

O Datalchemy e uma biblioteca/ferramenta em Rust para:
- Introspeccao de bancos relacionais (Postgres-first) -> `schema.json`.
- Planejamento (`plan.json`) validado por contrato.
- Geracao de dados sinteticos reprodutiveis.
- Avaliacao e metricas.

**Fonte de verdade do runtime**
1. `schema.json`
2. `plan.schema.json`
3. `plan.json`
4. `runs/<...>/` (logs + metrics)

IAs sugerem, o core valida.

---

## 2) Principios inegociaveis

1. **Determinismo**: output estavel e ordenado (sem HashMap no output).
2. **Separacao de responsabilidades**: contratos separados de adapters/queries.
3. **Rust seguro e idiomatico**: sem `unsafe` (a menos que documentado).
4. **Sem promessas vazias**: o que nao e suportado deve ser `Unsupported`.
5. **Privacidade/LGPD**: nunca logar credenciais ou dados reais.
6. **Reprodutibilidade**: execucoes geram artefatos versionaveis.

---

## 3) Protocolo de trabalho (IA e humano)

Quando alterar algo, entregar sempre:
1) **O que mudou** (lista curta)
2) **Por que mudou**
3) **Como validar** (comandos exatos)
4) **Evidencia** (arquivo em `evidence/` com ID da task)

> Sem evidencia, a mudanca e considerada incompleta.

### 3.1 Regra de tasks (PIT)
- **IA so atua quando existir** `tasks/issue_task_*.md` ou `tasks/pr_task_*.md`.
- Fora disso, abrir task ou solicitar contexto.

---

## 4) Estrutura e fronteiras (workspace)

- **`datalchemy-core`**: contratos do schema, validacao, redaction, grafo FK.
- **`datalchemy-introspect`**: adapters e queries (Postgres-first).
- **`datalchemy-cli`**: CLI e registry de runs.
- **`datalchemy-eval`**: metricas do schema.
- **`datalchemy-plan`/`datalchemy-generate`**: stubs Plan 2+.

SQL deve ficar concentrado em:
- `crates/datalchemy-introspect/src/postgres/queries.rs`

Conversoes/normalizacoes em:
- `crates/datalchemy-introspect/src/postgres/mapper.rs`
- `crates/datalchemy-introspect/src/postgres/utils.rs`

---

## 5) Regras Postgres (introspeccao)

- Preferir `pg_catalog` para constraints/indices.
- `information_schema.columns` pode complementar metadados.
- Campos `char` do catalogo chegam como `i8` via `sqlx::query!`.
  - Converter: `relkind`, `confdeltype`, `confupdtype`, `confmatchtype`.
  - `attidentity` deve ser normalizado no SQL:
    - `'a'` -> `ALWAYS`
    - `'d'` -> `BY DEFAULT`
    - `''` -> `NULL`
- Ordem e critica:
  - PK/FK/UNIQUE com `unnest(... WITH ORDINALITY)`.
  - Colunas por `attnum`.

---

## 6) Proibicoes

- Proibido `main()` em lib.
- Proibido `src/bin/` (use exemplos ou CLI no crate dedicado).
- Proibido `unwrap()`/`expect()` em caminhos de producao.
- Proibido repetir alias no mesmo `SELECT` quando usar `sqlx::query!`.
- Proibido logs de segredos.

---

## 7) API e contratos

- Tipos publicos minimos: `DatabaseSchema`, `Schema`, `Table`, `Column`,
  `PrimaryKey`, `ForeignKey`, `UniqueConstraint`, `CheckConstraint`, `Index`, `EnumType`.
- Opcoes de introspeccao (default seguras):
  - `include_system_schemas`, `include_views`, `include_materialized_views`,
    `include_foreign_tables`, `include_indexes`, `include_comments`, `schemas`.

Qualquer mudanca no contrato `schema.json` deve:
- Bump de `schema_version`.
- Atualizar testes e evidencia.

---

## 8) Erros e logging

- Erros com `thiserror`.
- Logs com `tracing` (nunca `println!` em lib).
- Redaction obrigatoria em `config.json` e `logs.ndjson`.

---

## 9) Qualidade

- `cargo fmt` obrigatorio.
- `cargo clippy --all-targets -- -D warnings` sem warnings.
- Sem dependencias pesadas sem justificativa.
- Versoes fixas no `Cargo.toml` (sem `^` ou `~`).

---

## 10) Exemplos obrigatorios

- `crates/datalchemy-introspect/examples/dump_json.rs`
  - le `DATABASE_URL`
  - chama introspeccao
  - imprime JSON

---

## 11) Testes

- Unit tests: redaction, grafo FK, serializacao deterministica.
- Integration tests: Postgres via Docker, valida PK/FK/UNIQUE/CHECK.
- Fixtures em `fixtures/sql/postgres/`.

---

## 12) Checklist de PR

- [ ] Testes atualizados
- [ ] `cargo fmt` + `cargo clippy` ok
- [ ] `evidence/<task_id>.md` atualizado
- [ ] Contratos versionados quando necessario
