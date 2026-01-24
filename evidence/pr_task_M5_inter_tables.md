# Evidence: pr_task_M5_inter_tables

## Changes
- ForeignContext + provider em memoria para FKs.
- Integracao de derives inter-tabelas (derive.fk, derive.parent_value).
- Exemplo `plans/examples/m5_relationships.plan.json`.

## Checks
- cargo fmt
- cargo test -p datalchemy-generate --test golden_files -- --nocapture

## Notes
- E2E com Postgres nao executado (requer DATABASE_URL).
