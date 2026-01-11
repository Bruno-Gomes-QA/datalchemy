create table crm.itens_lista_precos (
  id uuid primary key default gen_random_uuid(),
  lista_precos_id uuid not null references crm.listas_precos(id) on delete cascade,
  produto_id uuid not null references crm.produtos(id),
  preco numeric(12,2) not null,
  constraint itens_lista_precos_preco_chk check (preco >= 0),
  constraint itens_lista_precos_unique unique (lista_precos_id, produto_id)
);
