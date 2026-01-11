create table crm.itens_cotacao (
  id uuid primary key default gen_random_uuid(),
  cotacao_id uuid not null references crm.cotacoes(id) on delete cascade,
  produto_id uuid not null references crm.produtos(id),
  quantidade integer not null,
  preco_unitario numeric(12,2) not null,
  desconto_percentual numeric(5,2) not null default 0,
  constraint itens_cotacao_quantidade_chk check (quantidade > 0),
  constraint itens_cotacao_preco_chk check (preco_unitario >= 0),
  constraint itens_cotacao_desconto_chk check (desconto_percentual between 0 and 100),
  constraint itens_cotacao_unique unique (cotacao_id, produto_id)
);
