# faker integration

This document describes how to use faker generators in datalchemy.

## generator namespaces

- semantic.*: stable, human-friendly aliases mapped to faker.* ids via overrides.
- faker.*: raw catalog ids generated from fake-rs (auto-generated).
- primitive.*: typed generators (int/float/text/date/time/uuid) with strict params.

## locale

Locale can be defined in two places:

- plan.global.locale (default for all rules)
- generator.locale (per column override)

Supported locales: en_US, pt_BR.
If a locale is not supported by a generator id, generation fails with an error.

## params

Text params (where supported):
- min_len (int)
- max_len (int)
- allow_empty (bool)
- pattern (string)
- charset (string)

Int/Float params:
- min (int/float)
- max (int/float)

Date/Time/Timestamp params:
- min (string, ISO)
- max (string, ISO)

Invalid or unknown params always return an error.

## examples

### alias-based

```json
{
  "type": "column_generator",
  "schema": "crm",
  "table": "usuarios",
  "column": "nome",
  "generator": "semantic.person.name"
}
```

### direct faker id

```json
{
  "type": "column_generator",
  "schema": "crm",
  "table": "empresas",
  "column": "nome_fantasia",
  "generator": "faker.company.raw.CompanyName"
}
```

### with locale

```json
{
  "plan_version": "0.2",
  "seed": 42,
  "schema_ref": {
    "schema_version": "0.2",
    "engine": "postgres"
  },
  "global": {
    "locale": "pt_BR"
  },
  "targets": [
    {
      "schema": "crm",
      "table": "usuarios",
      "rows": 10
    }
  ],
  "rules": [
    {
      "type": "column_generator",
      "schema": "crm",
      "table": "usuarios",
      "column": "nome",
      "generator": "semantic.person.name"
    }
  ]
}
```
