create table crm.funis (
  id uuid primary key default gen_random_uuid(),
  nome text not null,
  descricao text,
  ativo boolean not null default true,
  data_criacao timestamptz not null default now(),
  constraint funis_nome_unique unique (nome)
);
