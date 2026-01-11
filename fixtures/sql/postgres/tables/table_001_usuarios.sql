create table crm.usuarios (
  id uuid primary key default gen_random_uuid(),
  nome text not null,
  email text not null,
  telefone text,
  ativo boolean not null default true,
  data_criacao timestamptz not null default now(),
  data_atualizacao timestamptz not null default now(),
  constraint usuarios_email_unique unique (email),
  constraint usuarios_email_chk check (position('@' in email) > 1),
  constraint usuarios_datas_chk check (data_atualizacao >= data_criacao)
);
