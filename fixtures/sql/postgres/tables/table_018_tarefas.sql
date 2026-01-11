create table crm.tarefas (
  id uuid primary key default gen_random_uuid(),
  atividade_id uuid not null references crm.atividades(id) on delete cascade,
  status crm.status_tarefa not null default 'aberta',
  prioridade integer not null,
  data_limite date,
  constraint tarefas_prioridade_chk check (prioridade between 1 and 5),
  constraint tarefas_atividade_unique unique (atividade_id),
  constraint tarefas_data_limite_chk check (data_limite is null or data_limite >= current_date)
);
