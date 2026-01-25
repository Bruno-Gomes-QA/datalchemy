# Datalchemy

Datalchemy e um pipeline em Rust para **introspeccao de schema**, **planificacao**, **geracao** e **avaliacao** de dados sinteticos.  
O foco atual (PIT) e **Postgres-first** com output deterministico e artefatos reprodutiveis por run.

---

## 1) O que esta pronto hoje

- **Introspeccao Postgres** -> `schema.json` deterministico.
- **CLI** `datalchemy introspect` gerando uma pasta de run com logs e metricas.
- **Plan** com JSON Schema oficial + validacao schema-aware.
- **Geracao** com registry por IDs, primitives/transforms, RowContext (derive) e ForeignContext (inter-tabelas).
- **Domain packs** (CRM, Finance, Logistica) para geracao semantica.
- **Relatorio de geracao** com contadores de cobertura e metricas de throughput.
- **Fixtures + Docker** para testes de integracao.

---

## 2) Estrutura do repositorio (workspace)

```
datalchemy/
├─ crates/
│  ├─ datalchemy-core/        # Contratos do schema + validacao + redaction + grafo FK
│  ├─ datalchemy-introspect/  # Adapters + queries (Postgres-first)
│  ├─ datalchemy-cli/         # CLI e registry de runs
│  ├─ datalchemy-eval/        # Metricas do schema
│  ├─ datalchemy-plan/        # Contrato do plan + JSON Schema
│  └─ datalchemy-generate/    # Engine de geracao (Plan 4)
├─ fixtures/sql/postgres/     # Fixtures SQL para testes (tables/ + data/)
├─ docker/compose.postgres.yml
├─ scripts/                   # Scripts para subir banco e aplicar fixtures
├─ tasks/                     # issue_task_*.md / pr_task_*.md
├─ evidence/                  # Evidencias por task
├─ schemas/                   # JSON Schema oficial do contrato
├─ runs/                      # Artefatos gerados pelo CLI (gitignored)
└─ datalchemy_structure.md    # Estrutura atual detalhada
```

---

## 3) Como executar (estado atual)

### 3.1 CLI: introspect
```bash
cargo run -p datalchemy-cli -- introspect \
  --conn "postgres://user:pass@localhost:5432/db" \
  --run-dir runs/
```

**Saida por run**
- `schema.json`
- `config.json` (connection redigida)
- `logs.ndjson`
- `metrics.json`

### 3.2 Exemplo: dump_json
```bash
export DATABASE_URL="postgres://user:pass@localhost:5432/db"
cargo run -p datalchemy-introspect --example dump_json > schema.json
```

### 3.3 Exemplo: gerar CSV (plan)
```bash
cargo run -p datalchemy-generate --example generate_csv -- \\
  --plan plans/examples/minimal.plan.json \\
  --schema runs/<run>/schema.json \\
  --out out/
```

---

## 4) Ambiente local Postgres (docker)

### 4.1 Subir container e aplicar fixtures
```bash
./scripts/postgres_docker.sh
```

**Container padrao**
- Nome: `datalchemy-postgres`
- Porta: `5432`
- Usuario: `datalchemy`
- Senha: `datalchemy`
- Database: `datalchemy_crm`

Para novos bancos, manter o padrao `scripts/<db>_docker.sh`.
Para docker compose, manter `docker/compose.<db>.yml`.

### 4.2 .env recomendado
```bash
DATABASE_URL=postgres://datalchemy:datalchemy@localhost:5432/datalchemy_crm
TEST_DATABASE_URL=postgres://datalchemy:datalchemy@localhost:5432/datalchemy_crm
```

---

## 5) Testes

### 5.1 Subir Postgres local via Docker
```bash
docker compose -f docker/compose.postgres.yml up -d
export TEST_DATABASE_URL="postgres://datalchemy:datalchemy@localhost:5432/datalchemy_crm"
```

### 5.2 Rodar testes
```bash
cargo test
```

Os testes de integracao fazem:
- Validacao do `schema.json` contra `schemas/schema.schema.json`.
- Comparacao com o golden file `crates/datalchemy-introspect/tests/golden/postgres_minimal.schema.json`.

### 5.3 Encerrar Docker
```bash
docker compose -f docker/compose.postgres.yml down
```

---

## 6) Regras essenciais (resumo)

- **Determinismo**: output ordenado e estavel.
- **SQL centralizado** em `crates/datalchemy-introspect/src/postgres/queries.rs`.
- **Nada de `unwrap()`/`expect()`** em producao.
- **Sem vazamento de segredos**: redaction obrigatoria.
- **Versoes fixas** no `Cargo.toml` (sem `^` ou `~`).

Veja `AGENTS.md` para o guia completo.

---

## 7) Contrato do schema.json

- O contrato e versionado por `schema_version` (atual: `0.2`).
- O JSON Schema oficial fica em `schemas/schema.schema.json`.
- Documentacao detalhada: `docs/schema_json.md`.

### Regenerar o JSON Schema
```bash
cargo run -p datalchemy-core --example emit_schema_json_schema > schemas/schema.schema.json
```

---

## 8) Plan + geracao

- `schemas/plan.schema.json` define o contrato do plan.
- `plans/examples/minimal.plan.json` usa generators por ID (`primitive.*`, `semantic.br.*`).
- `plans/examples/m2_primitives.plan.json` cobre primitives/transforms.
- `plans/examples/m3_ptbr.plan.json` cobre semantic pt-BR + masks.
- `plans/examples/m4_derives.plan.json` cobre RowContext (`derive.*`).
- `plans/examples/m5_relationships.plan.json` cobre ForeignContext (inter-tabelas).
- `plans/examples/crm_domain.plan.json` cobre domain pack CRM.
- `plans/examples/finance_domain.plan.json` cobre domain pack Finance.
- `plans/examples/logistics_domain.plan.json` cobre domain pack Logistica.
- `plans/examples/full_stack_ptbr.plan.json` integra pt-BR + domains (10k rows).

Docs adicionais:
- `docs/generators.md` (catalogo de geradores)
- `docs/plan_generators.md` (guia de uso do plan)
- `docs/privacy_lgpd.md` (privacidade e mascaramento)

### Regenerar o plan.schema.json
```bash
cargo run -p datalchemy-plan --example emit_plan_json_schema > schemas/plan.schema.json
```

---

## 9) Fluxo de contribuicao (branches, commits e evidencia)

### 8.1 Tasks obrigatorias (PIT)
- Toda mudanca deve ter um arquivo em `tasks/`:
  - `issue_task_<id>.md` ou `pr_task_<id>.md`.
- A evidencia deve ir em `evidence/<id>.md`.

### 8.2 Branches
- Use uma branch por task.
- Nomes sugeridos:
  - `feat/<id>-descricao`
  - `fix/<id>-descricao`
  - `docs/<id>-descricao`
  - `chore/<id>-descricao`

### 8.3 Commits
- Um commit por unidade logica de mudanca.
- Mensagens claras e curtas, citando o ID da task.

**Exemplo**
```
feat(pr_task_instrospect-database-schema): add cli registry artifacts
```

### 8.4 Checklist de PR
- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `evidence/<id>.md` atualizado
- [ ] Contrato versionado (se mudou `schema.json`)

---

## 10) Como o CLI gera uma run

1) Detecta engine pela URL.
2) Cria `runs/<timestamp>__run_<id>/`.
3) Introspecta o DB e valida o schema.
4) Escreve `schema.json` e `metrics.json`.
5) Registra logs em `logs.ndjson`.

---

## 11) Arquivos chave

- `crates/datalchemy-core/src/schema.rs` — contrato do schema.
- `crates/datalchemy-introspect/src/postgres/queries.rs` — SQL do Postgres.
- `crates/datalchemy-cli/src/main.rs` — CLI.
- `crates/datalchemy-cli/src/registry/run.rs` — registry de runs.
