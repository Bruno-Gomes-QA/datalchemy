# Evidencia — issue_task_postgres_crm_fixtures

## O que mudou
- Fixtures antigas substituidas por schema CRM em portugues com 20 tabelas e constraints fortes.
- Script `scripts/postgres_docker.sh` padronizado para subir Postgres local e aplicar fixtures.
- `.env` atualizado com `DATABASE_URL` e `TEST_DATABASE_URL` padrao.
- Docs e regras atualizadas: `README.md`, `AGENTS.md`, `docs/fixtures.md`.

## Por que mudou
- Padronizar ambiente local e remover dependencia de bancos externos (ex.: mineer).
- Garantir fixtures mais realistas e deterministicas para testes de integracao.

## Como validar
```bash
./scripts/postgres_docker.sh
cargo test
```

## Testes executados
- `./scripts/postgres_docker.sh` (falhou: porta 5432 ja estava em uso)
