# Evidence: pr_task_M7_hardening

## Changes
- Streaming com BufWriter e metricas de throughput no report.
- Teste de golden files (hash dos CSVs).
- Documentacao: generators, plan_generators, privacy_lgpd.

## Checks
- cargo fmt
- cargo test -p datalchemy-generate --test golden_files -- --nocapture

## Notes
- cargo clippy / cargo test completo nao executados aqui.
