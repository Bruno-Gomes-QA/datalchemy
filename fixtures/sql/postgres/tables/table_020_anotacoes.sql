create table crm.anotacoes (
  id uuid primary key default gen_random_uuid(),
  atividade_id uuid not null references crm.atividades(id) on delete cascade,
  conteudo text not null,
  criado_em timestamptz not null default now(),
  constraint anotacoes_atividade_unique unique (atividade_id)
);
