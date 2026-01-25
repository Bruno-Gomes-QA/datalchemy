# Evidence: issue_task_20260124_full_stack_crash

## Changes
- Guardrail para colunas com CHECK envolvendo `current_date` (clamp para `base_date` quando sem rule).
- Report sempre escrito mesmo em erro/panic, com issue `generation_failed`.
- Logs de progresso no engine e inicializacao do tracing no example `generate_csv`.

## Checks
- `cargo run -p datalchemy-generate --example generate_csv -- --plan out/full_stack_ptbr_200.plan.json --schema runs/2026-01-24T14-59-05Z__run_ad39c7ad-c6e2-4272-9114-c96fd29456d9/schema.json --out out/full_stack_smoke/`

## Result
- CSVs gerados para todas as tabelas do plano reduzido (incluindo auto-generated `crm.cotacoes`).
- `generation_report.json` presente no run.

## Notes
- Plano reduzido criado localmente em `out/full_stack_ptbr_200.plan.json` apenas para reproducao.
- Execucao full_stack 10k nao foi rodada nesta maquina.
