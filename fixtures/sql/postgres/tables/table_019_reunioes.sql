create table crm.reunioes (
  id uuid primary key default gen_random_uuid(),
  atividade_id uuid not null references crm.atividades(id) on delete cascade,
  local text not null,
  link_reuniao text,
  duracao_minutos integer not null,
  constraint reunioes_duracao_chk check (duracao_minutos > 0),
  constraint reunioes_atividade_unique unique (atividade_id)
);
