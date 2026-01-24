# Guia de uso do plan.json (geradores)

Este guia mostra como declarar geradores e transforms no plan.json.

## 1. Estrutura basica

```json
{
  "type": "column_generator",
  "schema": "crm",
  "table": "usuarios",
  "column": "email",
  "generator": "semantic.br.email.safe"
}
```

## 2. Params

Cada gerador aceita params especificos. Exemplo:

```json
{
  "type": "column_generator",
  "schema": "crm",
  "table": "oportunidades",
  "column": "valor_estimado",
  "generator": "primitive.float.range",
  "params": { "min": 1000.0, "max": 90000.0 }
}
```

## 3. Derive (RowContext)

Geradores `derive.*` usam `input_columns` e leem valores ja gerados na linha.

```json
{
  "type": "column_generator",
  "schema": "crm",
  "table": "usuarios",
  "column": "email",
  "generator": "derive.email_from_name",
  "params": {
    "input_columns": ["nome"],
    "domain": "example.com"
  }
}
```

### 3.1 Inter-tabelas

```json
{
  "type": "column_generator",
  "schema": "crm",
  "table": "contatos",
  "column": "empresa_id",
  "generator": "derive.fk"
}
```

```json
{
  "type": "column_generator",
  "schema": "crm",
  "table": "contatos",
  "column": "data_criacao",
  "generator": "derive.parent_value",
  "params": {
    "input_columns": ["empresa_id"],
    "parent_schema": "crm",
    "parent_table": "empresas",
    "parent_column": "data_criacao"
  }
}
```

## 4. Transforms

Transforms sao aplicados depois da geracao da linha:

```json
{
  "type": "column_generator",
  "schema": "crm",
  "table": "usuarios",
  "column": "email",
  "generator": "semantic.br.email.safe",
  "transforms": [
    {
      "transform": "transform.mask",
      "params": { "mode": "format_preserving" }
    }
  ]
}
```

## 5. Opcoes do plan

```json
"options": {
  "strict": false,
  "allow_fk_disable": false
}
```

- `strict`: quando true, fallbacks viram erro.
- `allow_fk_disable`: permite `foreign_key_strategy: disable`.

## 6. Exemplos completos

- `plans/examples/m4_derives.plan.json`
- `plans/examples/m5_relationships.plan.json`
- `plans/examples/crm_domain.plan.json`
- `plans/examples/finance_domain.plan.json`
- `plans/examples/logistics_domain.plan.json`
- `plans/examples/full_stack_ptbr.plan.json`
