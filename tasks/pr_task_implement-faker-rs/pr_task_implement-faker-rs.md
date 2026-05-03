# Task: Implementar suporte fake-rs (F0–F7)

- ID: pr_task_implement-faker-rs
- Owner: Codex
- Status: Done
- Milestone: F0–F7
- Crates: datalchemy-plan, datalchemy-generate, datalchemy-cli (examples), docs
- Risco: Alto

## Contexto
Plano de integracao fake-rs first com catalogo grande, IDs estaveis e suporte a locale/params.

## Objetivo
Implementar todos os milestones F0–F7 do plano e validar com testes e E2E reduzido.

## Nao-objetivos
- Sanitizers de output.
- Compatibilidade com outros backends que nao o fake-rs.

## Entregas (DoD)
- [x] Plan suporta generator.id/locale/params com compat layer.
- [x] Adapter fake-rs e catalogo faker.* gerado.
- [x] ParamSpec e validacao forte.
- [x] Locales pt_BR/en_US com override por coluna.
- [x] Migracao dos geradores antigos.
- [x] Docs + list_generators + testes de contrato.
- [x] Evidencia por milestone.

## Plano de execucao
1) F0 preparar task/evidence.
2) F1 atualizar plan e compat layer.
3) F2 integrar fake-rs e adapter.
4) F3 codegen catalogo faker.* + aliases.
5) F4 tipos completos + params.
6) F5 locales.
7) F6 migracao geradores.
8) F7 docs + testes.

## Como validar (comandos exatos)
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

## Evidencia (obrigatoria)
- Arquivo: evidence/pr_task_implement-faker-rs.md
