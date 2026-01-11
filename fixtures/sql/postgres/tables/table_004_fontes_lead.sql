create table crm.fontes_lead (
  id uuid primary key default gen_random_uuid(),
  nome text not null,
  descricao text,
  ativo boolean not null default true,
  data_criacao timestamptz not null default now(),
  constraint fontes_lead_nome_unique unique (nome)
);
