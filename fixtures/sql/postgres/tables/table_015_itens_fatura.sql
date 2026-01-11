create table crm.itens_fatura (
  id uuid primary key default gen_random_uuid(),
  fatura_id uuid not null references crm.faturas(id) on delete cascade,
  produto_id uuid not null references crm.produtos(id),
  quantidade integer not null,
  preco_unitario numeric(12,2) not null,
  constraint itens_fatura_quantidade_chk check (quantidade > 0),
  constraint itens_fatura_preco_chk check (preco_unitario >= 0),
  constraint itens_fatura_unique unique (fatura_id, produto_id)
);
