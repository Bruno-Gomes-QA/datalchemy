create table crm.atividades (
  id uuid primary key default gen_random_uuid(),
  usuario_id uuid not null references crm.usuarios(id),
  contato_id uuid references crm.contatos(id),
  oportunidade_id uuid references crm.oportunidades(id),
  tipo text not null,
  status crm.status_atividade not null default 'pendente',
  assunto text not null,
  descricao text,
  data_inicio timestamptz not null,
  data_fim timestamptz,
  constraint atividades_tipo_chk check (tipo in ('tarefa', 'reuniao', 'anotacao')),
  constraint atividades_periodo_chk check (data_fim is null or data_fim >= data_inicio)
);
