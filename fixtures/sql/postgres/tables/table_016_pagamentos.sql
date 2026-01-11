create table crm.pagamentos (
  id uuid primary key default gen_random_uuid(),
  fatura_id uuid not null references crm.faturas(id) on delete cascade,
  valor numeric(12,2) not null,
  data_pagamento date not null,
  status crm.status_pagamento not null default 'pendente',
  metodo text not null,
  constraint pagamentos_valor_chk check (valor > 0)
);
