create table crm.faturas (
  id uuid primary key default gen_random_uuid(),
  cotacao_id uuid references crm.cotacoes(id),
  responsavel_id uuid not null references crm.usuarios(id),
  numero text not null,
  data_emissao date not null default current_date,
  data_vencimento date not null,
  status crm.status_fatura not null default 'aberta',
  valor_total numeric(12,2) not null,
  constraint faturas_numero_unique unique (numero),
  constraint faturas_valor_chk check (valor_total >= 0),
  constraint faturas_vencimento_chk check (data_vencimento >= data_emissao)
);
