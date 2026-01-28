# Issue Task: Itens pendentes da integração fake-rs

- **ID:** issue_task_20260128_faker_pending
- **Status:** Open
- **Severity:** Low
- **Related:** pr_task_implement-faker-rs
- **Created:** 2026-01-28

## 1. Contexto

Durante a validação final da task `pr_task_implement-faker-rs`, identificamos alguns itens que foram propositalmente adiados ou que surgiram durante a implementação.

## 2. Itens Pendentes

### 2.1 Parâmetros para IDs parametrizados (Prioridade: Média)

O catálogo gerado marca os seguintes IDs como `PARAMETERIZED_IDS`:

```
faker.address.raw.Geohash
faker.boolean.raw.Boolean
faker.chrono.raw.DateTimeAfter
faker.chrono.raw.DateTimeBefore
faker.chrono.raw.DateTimeBetween
faker.internet.raw.Password
faker.lorem.raw.Paragraph
faker.lorem.raw.Paragraphs
faker.lorem.raw.Sentence
faker.lorem.raw.Sentences
faker.lorem.raw.Words
faker.markdown.raw.BlockQuoteMultiLine
faker.markdown.raw.BlockQuoteSingleLine
faker.markdown.raw.BulletPoints
faker.markdown.raw.Code
faker.markdown.raw.ListItems
faker.time.raw.DateTimeAfter
faker.time.raw.DateTimeBefore
faker.time.raw.DateTimeBetween
```

Atualmente, usar esses IDs retorna erro:
```
faker id 'faker.lorem.raw.Paragraph' requires params (not supported yet)
```

**Ação necessária:** Implementar suporte a parâmetros para esses IDs no adapter.

### 2.2 Validação E2E com Postgres (Prioridade: Baixa)

O runbook de validação inclui testes E2E com Postgres, mas a compilação do CLI e introspect requer conexão ativa com o banco (sqlx query verification).

**Ação necessária:** 
- Executar `cargo sqlx prepare` para gerar cache offline
- Ou configurar CI com banco Postgres

### 2.3 Clippy permite em nível de crate (Prioridade: Baixa)

Foram adicionados os seguintes allows em `datalchemy-generate/src/lib.rs`:

```rust
#![allow(clippy::result_large_err)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
```

**Ação necessária:** Refatorar o código para remover a necessidade desses allows:
- Boxar `GenerationReport` dentro de `GenerationError::Failed`
- Refatorar funções com muitos argumentos usando structs de contexto

### 2.4 Sanitizers de output (Prioridade: Baixa - fora do escopo original)

Conforme decidido no plano F0, sanitizers de output foram explicitamente adiados. Exemplos:
- Truncar strings que excedam `character_max_length`
- Normalizar formatos de telefone/CPF/CNPJ

## 3. Critérios de Conclusão

- [ ] IDs parametrizados funcionam com params corretos
- [ ] Testes E2E com Postgres passam
- [ ] Clippy passa sem allows de crate
- [ ] (Opcional) Sanitizers implementados

## 4. Notas

Estes itens não bloqueiam o uso da integração fake-rs. A implementação atual (219 generators) cobre a maioria dos casos de uso.
