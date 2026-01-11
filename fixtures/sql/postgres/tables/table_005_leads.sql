create table crm.leads (
  id uuid primary key default gen_random_uuid(),
  fonte_lead_id uuid references crm.fontes_lead(id),
  contato_id uuid references crm.contatos(id),
  empresa_id uuid references crm.empresas(id),
  responsavel_id uuid references crm.usuarios(id),
  status crm.status_lead not null default 'novo',
  score integer not null default 0,
  data_criacao timestamptz not null default now(),
  data_qualificacao timestamptz,
  constraint leads_score_chk check (score between 0 and 100),
  constraint leads_qualificacao_chk check (data_qualificacao is null or data_qualificacao >= data_criacao)
);
