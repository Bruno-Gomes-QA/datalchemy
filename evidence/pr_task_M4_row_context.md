# Evidence: pr_task_M4_row_context

## Changes
- Pipeline de geracao com RowContext em duas fases (base + derive) e transforms finais.
- Ordenacao topologica de derives com deteccao de ciclos.
- Geradores derive.* implementados e integrados.

## Checks
- cargo fmt
- cargo test -p datalchemy-generate --test golden_files -- --nocapture

## Notes
- Tests completos (cargo test / clippy) nao executados aqui.
