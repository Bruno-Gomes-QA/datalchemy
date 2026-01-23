# Guia End-to-End (Postgres + fixtures CRM)

Este guia mostra o fluxo completo do Datalchemy **ponta a ponta** usando o Postgres
com as fixtures atuais em `fixtures/sql/postgres/`.
O objetivo e executar cada etapa, validar os artefatos gerados e entender o que verificar.

---

## 0) Pre-requisitos

- Rust + Cargo instalados.
- Docker instalado e rodando.
- Porta 5432 livre (ou defina `POSTGRES_PORT`).

Opcional:
- `psql` local (nao obrigatorio; usaremos `docker exec` quando preciso).

---

## 1) Subir Postgres com fixtures

Esse script cria (ou inicia) o container e aplica as fixtures de tabelas e dados.

```bash
./scripts/postgres_docker.sh
```

Saida esperada:
- Uma mensagem com `Postgres pronto em localhost:5432`
- Uma linha com `DATABASE_URL=postgres://...`

Validacoes recomendadas:

1) Confirmar container:
```bash
docker ps --format 'table {{.Names}}\t{{.Status}}' | rg datalchemy-postgres
```

2) Ver tabelas do schema `crm`:
```bash
docker exec -e PGPASSWORD=datalchemy datalchemy-postgres \
  psql -U datalchemy -d datalchemy_crm -c "\\dt crm.*"
```

3) Conferir contagem basica:
```bash
docker exec -e PGPASSWORD=datalchemy datalchemy-postgres \
  psql -U datalchemy -d datalchemy_crm -c "select count(*) from crm.usuarios;"
```

---

## 2) Introspect (Plan 1)

Gera uma run com `schema.json`, `metrics.json`, `logs.ndjson`, `config.json`.

```bash
cargo run -p datalchemy-cli -- introspect \
  --conn "postgres://datalchemy:datalchemy@localhost:5432/datalchemy_crm" \
  --run-dir runs/
```

Localizar a run gerada:
```bash
RUN_DIR=$(ls -1d runs/* | sort | tail -n 1)
echo "$RUN_DIR"
```

Validar artefatos:
```bash
ls -la "$RUN_DIR"
```

Esperado:
- `schema.json` (contrato do schema)
- `metrics.json` (metricas de schema)
- `logs.ndjson` (logs)
- `config.json` (connection redigida)

Checks rapidos:
```bash
sed -n '1,40p' "$RUN_DIR/schema.json"
sed -n '1,40p' "$RUN_DIR/metrics.json"
```

---

## 3) Validar `plan.json` contra o schema (Plan 3)

Use o plan de exemplo e o schema gerado no passo anterior.

```bash
cargo run -p datalchemy-plan --example validate_plan -- \
  plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json"
```

Saida esperada:
- `plan validated successfully`

Se houver warnings/erros, eles aparecem no stdout/stderr com o path do problema.

---

## 4) Gerar dataset (Plan 4)

Gera CSVs deterministas usando `schema.json` + `plan.json`.

```bash
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/
```

Localizar o output:
```bash
OUT_DIR=$(ls -1d out/* | sort | tail -n 1)
echo "$OUT_DIR"
```

Artefatos esperados em `out/<run>/`:
- `crm.<tabela>.csv` (um CSV por tabela)
- `generation_report.json`
- `resolved_plan.json`

Validacoes recomendadas:

1) Ver arquivos CSV:
```bash
ls -la "$OUT_DIR" | rg '\.csv$'
```

2) Contagem de linhas (lembre: CSV tem header, entao linhas = rows + 1):
```bash
wc -l "$OUT_DIR/crm.usuarios.csv"
wc -l "$OUT_DIR/crm.contatos.csv"
wc -l "$OUT_DIR/crm.oportunidades.csv"
```

3) Inspecionar report:
```bash
sed -n '1,80p' "$OUT_DIR/generation_report.json"
```

---

## 5) Avaliar dataset (Plan 5)

Valida PK/FK/UNIQUE/NOT NULL/CHECK subset e gera `metrics.json` + `report.md`.

```bash
cargo run -p datalchemy-eval --example evaluate_run -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --run "$OUT_DIR"
```

Artefatos esperados em `out/<run>/`:
- `metrics.json`
- `report.md`

Validacoes recomendadas:

1) Checar metricas:
```bash
sed -n '1,120p' "$OUT_DIR/metrics.json"
```

2) Ler o report:
```bash
sed -n '1,120p' "$OUT_DIR/report.md"
```

Notas:
- O evaluator roda em modo `strict=true` por default.  
  Se houver violacoes, o comando pode retornar erro, mas `metrics.json`
  e `report.md` **ja estao gravados** no diretorio.

---

## 6) Validacoes adicionais (determinismo)

Executar a geracao novamente com a mesma seed deve produzir CSV identico.

1) Gerar um segundo run:
```bash
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema "$RUN_DIR/schema.json" \
  --out out/
```

2) Comparar CSVs (ajuste os paths):
```bash
OUT_DIR_2=$(ls -1d out/* | sort | tail -n 1)
diff -u "$OUT_DIR/crm.usuarios.csv" "$OUT_DIR_2/crm.usuarios.csv"
```

Sem diferencas = determinismo OK.

---

## 7) Checklist final (o que deve estar OK)

- Postgres subiu com fixtures aplicadas.
- `schema.json` foi gerado pelo CLI.
- `plan.json` validou contra o schema.
- Dataset CSV foi gerado (Plan 4).
- `metrics.json` e `report.md` foram gerados (Plan 5).
- Nao houve violacoes criticas (ou foram investigadas no report).

---

## 8) Encerrar ambiente (opcional)

```bash
docker stop datalchemy-postgres
```
