---
milestone: M1
title: "Refactor de pastas + registry por IDs + compat layer"
status: Draft
repo: https://github.com/Bruno-Gomes-QA/datalchemy
date: 2026-01-24
---


# M1 — Refactor de pastas + registry por IDs

> Organização híbrida + resolver generator por `id` string sem explodir enums.


## Guardrails (AGENTS.md) — inegociáveis

- **Determinismo**: outputs estáveis e ordenados (evitar `HashMap` em output; usar `BTreeMap`/ordenação explícita).
- **Separação de responsabilidades**: contratos e validação no core; geração não “invade” introspecção/SQL.
- **Rust idiomático e seguro**: sem `unsafe` (a menos que documentado e justificado).
- **Sem promessas vazias**: o que não for suportado deve ser explicitamente `Unsupported`/warning.
- **Privacidade/LGPD**: nunca logar credenciais nem valores de PII; artefatos devem ser redigidos.
- **Reprodutibilidade**: cada execução gera artefatos versionáveis (run dir).
- **Proibições**: sem `main()` em lib; sem `src/bin/`; sem `unwrap()`/`expect()` em caminho de produção; sem `println!` em lib.
- **Erros e logs**: `thiserror` + `tracing` (logs estruturados).
- **Qualidade**: `cargo fmt`; `cargo clippy --all-targets -- -D warnings`; versões fixas no `Cargo.toml` (sem `^`/`~`).
- **Evidência obrigatória**: toda mudança precisa de `tasks/issue_task_*.md` e `evidence/<task_id>.md` com o que mudou / por quê / como validar.


## Objetivo

Refatorar a organização do código e a forma de selecionar generators para permitir expansão rápida e modular:

- Estrutura de pastas em `crates/datalchemy-generate/src/generators/`.
- Introduzir `GeneratorId` (string) e um `Registry` (mapa id → factory/handler).
- Atualizar `plan.schema.json` para usar IDs string (breaking change permitida na fase beta).
- Criar contratos internos claros: generator base + transforms (M2) + derivação (M4).


## Estrutura de pastas (proposta)

```text
crates/datalchemy-generate/src/generators/
├─ mod.rs
├─ primitives/   (M2)
├─ semantic/     (M3)
├─ domain/       (M6)
└─ transforms/   (M2/M3)
```


## Design — Registry e resolução de generator

### Interfaces recomendadas

```rust
pub trait Generator {
  fn id(&self) -> &'static str;
  fn generate(&self, col: &Column, row: &RowContext, ctx: &GenerationContext, rng: &mut impl Rng)
    -> Result<GeneratedValue, GenerationError>;
  fn pii_tags(&self) -> &'static [PiiTag] { &[] }
}

pub struct Registry {
  by_id: BTreeMap<&'static str, Box<dyn Generator + Send + Sync>>,
}
```

### Breaking Change (Beta)
Atualizar `plan.schema.json` trocando os enums hardcoded por strings livres (padrão `category.name.variant`).
O `resolved_plan.json` já refletirá os IDs novos.

## Plano de implementação (passo a passo)

1) Criar a nova árvore de módulos e mover o código existente.
2) Introduzir `Registry` e registrar os generators atuais.
3) Atualizar `plan.schema.json` e o parser para esperar IDs string.
4) Normalizar sempre para IDs em `resolved_plan.json`.
5) Testes: resolução por ID + determinismo.
6) Docs: `docs/generators.md` e `docs/plan_generators.md`.


## Como validar

```bash
# Qualidade
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

# E2E Postgres (Plan 4 + Plan 5)
./scripts/postgres_docker.sh

# Introspecção (schema.json) — usa DATABASE_URL
cargo run -p datalchemy-cli -- introspect \
  --conn "$DATABASE_URL" \
  --run-dir runs/

# Localize o RUN_DIR mais recente (ajuste se necessário)
RUN_DIR=$(ls -1d runs/* | sort | tail -n 1)
echo "RUN_DIR=$RUN_DIR"

# Validar plan (se você estiver alterando schema/plan)
cargo run -p datalchemy-plan --example validate_plan -- \
  plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json"

# Gerar CSV (Plan 4)
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/

OUT_DIR=$(ls -1d out/* | sort | tail -n 1)
echo "OUT_DIR=$OUT_DIR"

# Avaliar (Plan 5) — gera metrics.json + report.md
cargo run -p datalchemy-eval --example evaluate_run -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --run "$OUT_DIR"

# Determinismo: segunda geração com a mesma seed deve produzir CSV idêntico
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/

OUT_DIR_2=$(ls -1d out/* | sort | tail -n 1)
diff -u "$OUT_DIR/crm.usuarios.csv" "$OUT_DIR_2/crm.usuarios.csv"
```


## Critérios de aceite (DoD)

- [ ] E2E Postgres sem regressões.
- [ ] `Registry` resolve por ID de forma determinística.
- [ ] Compat layer existe e normaliza para IDs.
- [ ] `resolved_plan.json` contém IDs normalizados.
- [ ] `cargo fmt/clippy/test` OK + evidence.


## Templates (tasks/evidence)

## Template de task (crie antes de codar)

Crie um arquivo: `tasks/issue_task_<YYYYMMDD>_<slug>.md`

```md
# Task: <título curto>

- ID: issue_task_<YYYYMMDD>_<slug>
- Owner: <nome>
- Status: Draft | In Progress | Done
- Scope: <M0 | M1 | ...>
- Crates: <lista>
- Risco: Baixo | Médio | Alto

## Contexto
<por que esta task existe>

## Objetivo
<resultado final e observável>

## Não-objetivos
<o que explicitamente não será feito>

## Entregas (DoD)
- [ ] ...
- [ ] ...

## Plano de execução
1) ...
2) ...

## Como validar (comandos exatos)
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
# + comandos E2E se aplicável
```

## Evidência (obrigatória)
- Arquivo: `evidence/issue_task_<YYYYMMDD>_<slug>.md`
- Deve incluir: "o que mudou", "por que mudou", "como validar", resultados/outputs.
```


## Template de evidência (preencha no final)

Crie um arquivo: `evidence/<task_id>.md`

```md
# Evidence: <task_id>

## O que mudou
- ...

## Por que mudou
- ...

## Como validar (comandos exatos)
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
# E2E (se aplicável)
./scripts/postgres_docker.sh
cargo run -p datalchemy-cli -- introspect --conn "$DATABASE_URL" --run-dir runs/
cargo run -p datalchemy-generate --example generate_csv -- --plan plans/examples/minimal.plan.json --schema "$RUN_DIR/schema.json" --out out/
cargo run -p datalchemy-eval --example evaluate_run -- --plan plans/examples/minimal.plan.json --schema "$RUN_DIR/schema.json" --run "$OUT_DIR"
```

## Resultado
- `RUN_DIR=...`
- `OUT_DIR=...`
- `generation_report.json`: ...
- `metrics.json`: ...
- `diff` determinismo: (sem diferenças)

## Notas/Riscos
- ...
```


## Prompt sugerido para Codex (execução “sem medo”, mas com trilhos)

> **Modo de execução:** implemente **somente** esta milestone (M1).  
> **Não avance** para outras milestones.  
> **Não quebre compatibilidade** com os exemplos existentes.  
> **Siga AGENTS.md** (determinismo, privacidade, evidência, qualidade).

**Contexto disponível**
- Este arquivo em `plans/milestones/`
- `AGENTS.md`
- `end_to_end_postgres.md`
- Código atual do repo

**Tarefa**
- Refatorar generators em módulos.
- Implementar registry por ID + compat layer.
- Atualizar parsing/normalização do plan.
- Adicionar docs e testes.
- Entregar task+evidence.

**Saída obrigatória**
1) `tasks/issue_task_<YYYYMMDD>_<slug>.md`
2) Implementação em Rust + testes
3) `evidence/<task_id>.md` preenchido com comandos e resultados
