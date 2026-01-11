# Task — Ambiente local Postgres (CRM) + Fixtures fortes

## Objetivo
Padronizar o ambiente local de testes com Postgres e substituir fixtures antigas por um schema CRM em portugues, com constraints fortes e pelo menos 20 tabelas relacionadas.

## Entregaveis
- Script `scripts/postgres_docker.sh` para subir o Postgres local e aplicar fixtures.
- Fixtures em `fixtures/sql/postgres/`:
  - `tables/` com SQL por tabela (prefixo numerico e ordem deterministica).
  - `data/` com carga minima de dados.
- `.env` com `DATABASE_URL` e `TEST_DATABASE_URL` padrao.
- Documentacao atualizada em `README.md`, `AGENTS.md` e `docs/fixtures.md`.

## Criterios de aceitacao
- Container sobe na porta 5432 com nome fixo e credenciais padrao.
- Fixtures criam schema CRM com >= 20 tabelas e constraints (PK/FK/UNIQUE/CHECK).
- Nomes de tabelas/colunas em portugues (sem acentos).
- Docs e regras refletindo o novo padrao.
