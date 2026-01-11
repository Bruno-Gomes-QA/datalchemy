# Contrato do schema.json

Este documento define o contrato estavel do `schema.json` gerado pela introspeccao.

---

## 1) Campos de alto nivel

- `schema_version` (string, obrigatorio)
  - Versao do contrato, ex.: `"0.2"`.
- `engine` (string, obrigatorio)
  - Engine de origem, ex.: `"postgres"`.
- `database` (string | null)
  - Nome do database quando disponivel.
- `schemas` (array, obrigatorio)
  - Lista de schemas do usuario.
- `enums` (array, obrigatorio)
  - Enums globais do database.
- `schema_fingerprint` (string | null)
  - Hash estavel opcional (quando habilitado no futuro).

---

## 2) Schema

Cada item de `schemas`:
- `name` (string)
- `tables` (array)

---

## 3) Table

Cada item de `tables`:
- `name` (string)
- `kind` (string)
  - `table` | `partitioned_table` | `view` | `materialized_view` | `foreign_table` | `other`
- `comment` (string | null)
- `columns` (array)
- `constraints` (array)
- `indexes` (array)

---

## 4) Column

Cada item de `columns`:
- `ordinal_position` (int)
- `name` (string)
- `column_type` (object)
  - `data_type`, `udt_schema`, `udt_name`
  - `character_max_length`, `numeric_precision`, `numeric_scale`, `collation`
- `is_nullable` (bool)
- `default` (string | null)
- `identity` (string | null)
  - `always` | `by_default`
- `generated` (object | null)
  - `kind` = `stored`
  - `expression` (string | null)
- `comment` (string | null)

---

## 5) Constraints

Todos os constraints sao serializados via enum tagged:

```json
{ "kind": "primary_key", ... }
{ "kind": "foreign_key", ... }
{ "kind": "unique", ... }
{ "kind": "check", ... }
```

Campos principais:
- **PrimaryKey**: `name` (opcional), `columns` (ordem preservada).
- **ForeignKey**: `name`, `columns`, `referenced_schema`, `referenced_table`, `referenced_columns`,
  `on_update`, `on_delete`, `match_type`, `is_deferrable`, `initially_deferred`.
- **Unique**: `name`, `columns`, `is_deferrable`, `initially_deferred`.
- **Check**: `name`, `expression`.

---

## 6) Index

Cada item de `indexes`:
- `name`
- `is_unique`
- `is_primary`
- `is_valid`
- `method`
- `definition`

---

## 7) Determinismo

- `schemas` ordenado por `name`.
- `tables` ordenado por `name`.
- `columns` por `attnum`.
- `constraints` ordenado por tipo + nome + colunas.
- `indexes` ordenado por `name`.
- `enums` ordenado por schema + name.

---

## 8) JSON Schema oficial

O contrato formal esta em:
- `schemas/schema.schema.json`

Para regenerar:
```bash
cargo run -p datalchemy-core --example emit_schema_json_schema > schemas/schema.schema.json
```
