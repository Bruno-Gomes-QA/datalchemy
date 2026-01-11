create table crm.oportunidades (
  id uuid primary key default gen_random_uuid(),
  empresa_id uuid not null references crm.empresas(id),
  contato_id uuid references crm.contatos(id),
  etapa_id uuid not null references crm.etapas_funil(id),
  responsavel_id uuid not null references crm.usuarios(id),
  titulo text not null,
  valor_estimado numeric(12,2) not null,
  status crm.status_oportunidade not null default 'aberta',
  data_abertura date not null default current_date,
  data_fechamento date,
  constraint oportunidades_valor_chk check (valor_estimado >= 0),
  constraint oportunidades_fechamento_chk check (data_fechamento is null or data_fechamento >= data_abertura)
);
