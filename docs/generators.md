# Catalogo de geradores

Este documento descreve os geradores disponiveis no datalchemy-generate e suas familias.

## 1. Primitives

- `primitive.uuid.v4`: UUID v4 deterministico pela seed.
- `primitive.bool`: true/false.
- `primitive.int.range`: inteiro entre `min` e `max`.
- `primitive.int.sequence_hint`: sequencia deterministica (`start`, `step`, `max`).
- `primitive.float.range`: float entre `min` e `max`.
- `primitive.decimal.numeric`: float com escala (usa `scale`).
- `primitive.text.pattern`: texto a partir de padrao (ex: `INV-####`).
- `primitive.text.lorem`: lorem curto deterministico.
- `primitive.date.range`: data entre `min`/`max`.
- `primitive.time.range`: hora entre `min`/`max`.
- `primitive.timestamp.range`: timestamp entre `min`/`max`.
- `primitive.enum`: usa labels do enum do schema.

## 2. Semanticos PT-BR

- `semantic.br.name`
- `semantic.br.email.safe`
- `semantic.br.phone`
- `semantic.br.cpf`
- `semantic.br.cnpj`
- `semantic.br.rg`
- `semantic.br.cep`
- `semantic.br.uf`
- `semantic.br.city`
- `semantic.br.address`
- `semantic.br.money.brl`
- `semantic.br.ip`
- `semantic.br.url`

## 3. Derive (RowContext)

Geradores que dependem de outras colunas da mesma linha (`params.input_columns`).

- `derive.email_from_name`: gera email a partir de nome(s).
- `derive.updated_after_created`: garante `updated >= created`.
- `derive.end_after_start`: garante `fim >= inicio`.
- `derive.money_total`: `total = price * qty - discount`.
- `derive.fk`: usa a FK do schema para selecionar valor valido.
- `derive.parent_value`: copia valor de tabela pai via FK.

Parametros comuns:
- `input_columns`: lista de colunas de entrada.
- `max_days` / `max_seconds`: limites para datas/horas.

## 4. Domain packs

### 4.1 CRM
- `domain.crm.lead_stage`
- `domain.crm.activity_type`
- `domain.crm.deal_value`
- `domain.crm.pipeline_name`

### 4.2 Finance
- `domain.finance.transaction_type`
- `domain.finance.payment_method`
- `domain.finance.invoice_status`
- `domain.finance.installments`

### 4.3 Logistics
- `domain.logistics.tracking_code`
- `domain.logistics.shipment_status`
- `domain.logistics.carrier`
- `domain.logistics.dimensions_cm`

## 5. Notas

- Todos os geradores sao deterministas com a mesma seed.
- `derive.*` requerem ordem correta das colunas e validacao de dependencias.
- `derive.fk` e `derive.parent_value` dependem do `ForeignContext` populado.
