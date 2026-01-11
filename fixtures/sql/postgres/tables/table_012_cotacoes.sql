create table crm.cotacoes (
  id uuid primary key default gen_random_uuid(),
  oportunidade_id uuid not null references crm.oportunidades(id) on delete cascade,
  responsavel_id uuid not null references crm.usuarios(id),
  codigo text not null,
  data_emissao date not null default current_date,
  validade date not null,
  constraint cotacoes_codigo_unique unique (codigo),
  constraint cotacoes_validade_chk check (validade >= data_emissao)
);
