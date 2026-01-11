create table crm.etapas_funil (
  id uuid primary key default gen_random_uuid(),
  funil_id uuid not null references crm.funis(id) on delete cascade,
  nome text not null,
  ordem integer not null,
  probabilidade numeric(5,2) not null,
  constraint etapas_funil_ordem_chk check (ordem > 0),
  constraint etapas_funil_prob_chk check (probabilidade between 0 and 100),
  constraint etapas_funil_ordem_unique unique (funil_id, ordem),
  constraint etapas_funil_nome_unique unique (funil_id, nome)
);
