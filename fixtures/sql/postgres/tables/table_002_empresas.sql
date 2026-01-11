create table crm.empresas (
  id uuid primary key default gen_random_uuid(),
  razao_social text not null,
  nome_fantasia text not null,
  cnpj text not null,
  email text,
  telefone text,
  site text,
  ativo boolean not null default true,
  data_criacao timestamptz not null default now(),
  data_atualizacao timestamptz not null default now(),
  constraint empresas_cnpj_unique unique (cnpj),
  constraint empresas_email_chk check (email is null or position('@' in email) > 1),
  constraint empresas_datas_chk check (data_atualizacao >= data_criacao)
);
