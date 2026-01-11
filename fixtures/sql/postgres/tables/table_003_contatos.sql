create table crm.contatos (
  id uuid primary key default gen_random_uuid(),
  empresa_id uuid not null references crm.empresas(id) on delete cascade,
  nome text not null,
  sobrenome text,
  email text not null,
  telefone text,
  cargo text,
  data_nascimento date,
  data_criacao timestamptz not null default now(),
  constraint contatos_email_unique unique (email),
  constraint contatos_email_chk check (position('@' in email) > 1),
  constraint contatos_nascimento_chk check (data_nascimento is null or data_nascimento <= current_date)
);
