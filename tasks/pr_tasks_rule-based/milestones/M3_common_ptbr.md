---
milestone: M3
title: "Common pt-BR + assets loader + mask"
status: Draft
repo: https://github.com/Bruno-Gomes-QA/datalchemy
date: 2026-01-24
---


# M3 — Common pt-BR + assets loader + mask

> Pacote common é fundamental: nomes, documentos BR, geografia, dinheiro, e máscara LGPD.


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

Criar o pacote **common pt-BR** com assets em arquivo + cache, ≥15 generators semantic.* e `transform.mask` com LGPD by design.


## Escopo

**Dentro**
- `assets/pt_BR/*` + loader lazy com cache.
- ≥15 semantic generators (pessoa/docs/geo/finance/net).
- PII tagging: contagem no report e proibição de log de valores.
- `transform.mask` (hash/redact/format-preserving mínimo).

**Fora**
- Derivação (M4) e inter-tabelas (M5).


## Assets pt-BR — organização e regras

```
crates/datalchemy-generate/assets/pt_BR/
  person_first_names.txt
  person_last_names.txt
  cities_by_uf.json
  street_names.txt
  neighborhood_names.txt
  banks_br.json
```
- Conteúdo genérico (sem dados pessoais reais).
- Loader com env `DATALCHEMY_ASSETS_DIR` + fallback.
- Em erro: strict → erro; non-strict → warning + fallback mínimo.


## Catálogo semantic.* (pt-BR)

Pessoa:
- `semantic.person.name.pt_br`
- `semantic.person.first_name.pt_br`
- `semantic.person.last_name.pt_br`
- `semantic.person.email.safe`
- `semantic.person.phone.br`
- `semantic.br.cpf`
- `semantic.br.cnpj`
- `semantic.br.rg`

Geo:
- `semantic.geo.cep.br`
- `semantic.geo.uf.br`
- `semantic.geo.city.br`
- `semantic.geo.address.br`

Finance:
- `semantic.finance.money.brl`
- `semantic.finance.percentage`

Net:
- `semantic.net.ip.private`
- `semantic.net.url`


## LGPD — transform.mask

- `mask.hash` (sha256 + salt)
- `mask.redact` (`***`)
- `mask.format_preserving` (mínimo):
  - CPF: mantém máscara, troca números determinísticos
  - Email: mantém domínio, troca user
  - Telefone: mantém DDD, troca resto

Regras:
- não vazar valor original em logs
- determinístico por seed + salt


## Plano de implementação (passo a passo)

1) Criar assets e loader com cache.
2) Implementar semantic.* em módulos (`semantic/person`, `semantic/geo`, ...).
3) Integrar PII tagging no registry/engine/report.
4) Implementar `transform.mask`.
5) Criar examples de plan (ptbr_common e masked).
6) Testes unitários de formato/determinismo.


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


### Checks adicionais sugeridos (formato)

```bash
python - <<'PY'
import csv, re, glob, os
out_dir = sorted(glob.glob("out/*"))[-1]
path = os.path.join(out_dir, "crm.usuarios.csv")
rx = re.compile(r"^[^@]+@[^@]+\.[^@]+$")
bad = 0
with open(path, newline='', encoding="utf-8") as f:
  r = csv.DictReader(f)
  for row in r:
    if "email" in row and row["email"] and not rx.match(row["email"]):
      bad += 1
print("bad_emails=", bad)
PY
```


## Critérios de aceite (DoD)

- [ ] Assets pt-BR carregáveis com cache.
- [ ] ≥15 semantic generators registrados.
- [ ] PII tagging ativo e sem log de valores.
- [ ] `transform.mask` implementado e documentado.
- [ ] E2E + determinismo OK.


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

> **Modo de execução:** implemente **somente** esta milestone (M3).  
> **Não avance** para outras milestones.  
> **Não quebre compatibilidade** com os exemplos existentes.  
> **Siga AGENTS.md** (determinismo, privacidade, evidência, qualidade).

**Contexto disponível**
- Este arquivo em `plans/milestones/`
- `AGENTS.md`
- `end_to_end_postgres.md`
- Código atual do repo

**Tarefa**
- Criar assets pt-BR + loader.
- Implementar semantic.* + PII tags.
- Implementar transform.mask.
- Adicionar examples e testes.
- Entregar task+evidence.

**Saída obrigatória**
1) `tasks/issue_task_<YYYYMMDD>_<slug>.md`
2) Implementação em Rust + testes
3) `evidence/<task_id>.md` preenchido com comandos e resultados
