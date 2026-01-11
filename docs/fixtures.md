# Fixtures de Postgres (CRM)

Este repositorio usa fixtures em portugues para o schema CRM.

## Estrutura
- `fixtures/sql/postgres/tables/`
  - SQL por tabela, com prefixo numerico (`table_001_*.sql`).
- `fixtures/sql/postgres/data/`
  - Cargas de dados de teste (`data_001.sql`).

## Regras
- Nomes de tabelas e colunas em portugues, sem acentos.
- Constraints devem ser explicitas (PK, FK, UNIQUE, CHECK).
- Datas devem ter checks de consistencia (ex.: fim >= inicio).
- Para novos bancos, manter:
  - `scripts/<db>_docker.sh`
  - `docker/compose.<db>.yml`

## Execucao rapida
```bash
./scripts/postgres_docker.sh
```
