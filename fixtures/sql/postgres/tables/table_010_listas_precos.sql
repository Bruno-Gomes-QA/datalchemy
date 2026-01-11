create table crm.listas_precos (
  id uuid primary key default gen_random_uuid(),
  nome text not null,
  moeda char(3) not null default 'BRL',
  ativo boolean not null default true,
  data_inicio date not null,
  data_fim date,
  constraint listas_precos_nome_unique unique (nome),
  constraint listas_precos_periodo_chk check (data_fim is null or data_fim >= data_inicio)
);
