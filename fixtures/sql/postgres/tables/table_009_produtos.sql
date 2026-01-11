create table crm.produtos (
  id uuid primary key default gen_random_uuid(),
  sku text not null,
  nome text not null,
  descricao text,
  ativo boolean not null default true,
  preco_base numeric(12,2) not null,
  data_criacao timestamptz not null default now(),
  constraint produtos_sku_unique unique (sku),
  constraint produtos_preco_chk check (preco_base >= 0)
);
