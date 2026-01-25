# Privacidade e LGPD

Este documento descreve boas praticas de privacidade no Datalchemy.

## 1. Principios

- Nao use dados reais em planos ou fixtures.
- Os geradores usam dados sinteticos deterministas.
- Nao registrar credenciais ou dados sensiveis nos logs.

## 2. PII e mascaramento

- O relatorio de geracao registra `pii_columns_touched`.
- Use `transform.mask` para mascarar valores sensiveis.

Exemplo:

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

## 3. Redaction

- Configuracoes e logs devem aplicar redaction.
- Nunca grave tokens, senhas ou dados reais em `runs/`.

## 4. Auditoria

- Os CSVs gerados sao deterministas pela seed.
- O relatorio inclui contadores de warn/fallback para rastreio.
