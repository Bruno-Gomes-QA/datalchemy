# Retomada do Projeto Datalchemy

> Documento gerado em 2026-04-05 para retomar o contexto do projeto.
> Branch atual: `feat/implement-faker-rs` (4 commits ahead de master)

---

## 1) O que é o Datalchemy

Biblioteca/ferramenta em **Rust** para:

1. **Introspecção** de bancos PostgreSQL → `schema.json`
2. **Planejamento** de dados sintéticos → `plan.json` (validado por JSON Schema)
3. **Geração** determinística de dados (CSV) com 219+ generators
4. **Avaliação** de qualidade (PK/FK/UNIQUE/CHECK coverage, métricas)

Pipeline completo: `Introspect → Plan → Generate → Eval`

---

## 2) Histórico de branches (cronológico)

| # | Branch | O que fez | Status |
|---|--------|-----------|--------|
| 1 | `feat/instrospect-database-schema` | Introspecção Postgres + `schema.json` + grafo FK | ✅ Merged |
| 2 | `feat/plan-schema-aware` | Modelo do plan, JSON Schema, validação schema-aware | ✅ Merged |
| 3 | `feat/generate-more-role-based` | Engine de geração: primitives, semantic, derive, domain packs, transforms, M0-M7 | ✅ Merged |
| 4 | `feat/datalchemy-cli-tui` | CLI + TUI com fluxo completo (introspect/plan/generate/eval) | ✅ Merged |
| 5 | **`feat/implement-faker-rs`** | **Integração fake-rs como backend baseline (F0-F7)** | **⬅ AQUI (não merged)** |

---

## 3) O que a branch atual (`feat/implement-faker-rs`) fez

### Objetivo
Substituir os geradores ad-hoc por **fake-rs** como backend unificado, com IDs estáveis, catálogo auto-gerado e suporte a locales.

### Milestones completados (F0-F7)

| Milestone | Descrição | Status |
|-----------|-----------|--------|
| **F0** | Decisões de escopo (fake-rs obrigatório, IDs estáveis, locales desde já) | ✅ |
| **F1** | Plan aceita `generator` como string ou objeto `{id, locale, params}`, plan_version → 0.2 | ✅ |
| **F2** | `FakeRsAdapter` centralizado em `faker_rs/adapter.rs`, migração rand gen_* → random_* | ✅ |
| **F3** | Catálogo auto-gerado (`catalog_gen.rs`, 219 IDs), aliases semânticos via `overrides.toml` | ✅ |
| **F4** | ParamSpec com validação de tipo + range (text: min_len/max_len/pattern/charset; int/float: min/max) | ✅ |
| **F5** | Locale global (`plan.global.locale`) + override por coluna, pt_BR + en_US | ✅ |
| **F6** | Remoção de heurísticas antigas, fallback por tipo (uuid→primitive.uuid, etc.) | ✅ |
| **F7** | Docs, `list_generators` example, testes de contrato | ✅ |

### Commits na branch
```
4c4062a checkpoint
753854b test(generate): add E2E tests for faker integration with Postgres
6440105 docs: add pending items task for faker integration
afc084e feat(faker-rs): implement full fake-rs integration (F0-F7)
```

### Arquivos-chave adicionados/alterados (70 files, +7935 -838 lines)

**Novos módulos:**
- `crates/datalchemy-generate/src/faker_rs/` (adapter, catalog_gen, locales)
- `crates/datalchemy-generate/src/generators/faker_rs.rs` (registro no GeneratorRegistry)
- `crates/datalchemy-generate/src/params.rs` (ParamSpec, validação forte)
- `tools/gen_faker_catalog.rs` (codegen do catálogo)

**Testes:**
- `tests/faker_catalog.rs` — contrato (IDs únicos/ordenados, rejeita IDs/params desconhecidos)
- `tests/faker_e2e.rs` — E2E com Postgres (6 tabelas, locale pt_BR, determinismo)
- 39 testes passando no total

**Plans de exemplo novos:**
- `faker_baseline.plan.json`, `faker_enus.plan.json`, `faker_ptbr.plan.json`

---

## 4) O que ficou pendente nesta branch

Documentado em `tasks/pr_task_implement-faker-rs/issue_task_20260128_faker_pending.md`:

| Item | Prioridade | Detalhes |
|------|-----------|----------|
| **19 Parametrized IDs** | Média | Generators que requerem params avançados (Geohash, DateTimeAfter, Password, Paragraph, etc.) retornam erro "not supported yet" |
| **E2E Postgres na CI** | Baixa | Requer `cargo sqlx prepare` ou Postgres no pipeline |
| **Clippy allows** | Baixa | 4 allows em crate-level (`result_large_err`, `large_enum_variant`, `too_many_arguments`, `type_complexity`) |
| **Sanitizers** | Baixa | Explicitamente adiado (fora do escopo original) |

**Nenhum bloqueia o merge** — os 219 generators cobrem a maioria dos cenários.

---

## 5) O que precisa ser feito para fechar esta feature

### 5.1 Ações imediatas

- [ ] Validar compilação (`cargo check`) e testes (`cargo test`)
- [ ] Rodar `cargo fmt` + `cargo clippy --all-targets -- -D warnings`
- [ ] Decidir se merge como está ou resolve os itens pendentes antes

### 5.2 Decisões antes do merge

- [ ] Fazer squash dos commits ou manter o histórico?
- [ ] Os 19 parametrized IDs entram nesta branch ou viram issue separada?
- [ ] Atualizar README com a nova seção de generators/faker?

---

## 6) Para onde o projeto estava indo (roadmap inferido)

Baseado na estrutura de crates, tasks e docs:

| Fase | Descrição | Status |
|------|-----------|--------|
| Introspecção Postgres | schema.json + grafo FK | ✅ Concluído |
| Plan schema-aware | plan.json + JSON Schema + validação | ✅ Concluído |
| Engine de geração | primitives, semantic, derive, domain, transforms | ✅ Concluído |
| CLI + TUI | fluxo completo via terminal | ✅ Concluído |
| **Integração fake-rs** | **Backend baseline com 219+ generators** | **✅ Implementado, pendente merge** |
| Suporte a mais DBs | MySQL, SQLite (stubs em crates) | 🔜 Futuro |
| Sanitizers/masking | Anonimização de dados (LGPD) | 🔜 Futuro |
| Avaliação avançada | Métricas de fidelidade estatística | 🔜 Futuro |

---

## 7) Perguntas para refinamento

### Q1: Merge da branch faker-rs
**Pergunta:** Quer fazer merge desta branch como está (feature completa, 219 generators, itens pendentes como issues separadas) ou quer resolver algo antes?

**Resposta default:** Merge como está. Os 19 parametrized IDs viram uma issue separada — não bloqueiam nenhum uso real.

---

### Q2: Próximo foco após o merge
**Pergunta:** Qual deve ser o próximo foco? Opções:
- (A) Hardening: resolver clippy allows, melhorar error handling, CI/CD
- (B) Novos generators: implementar os 19 parametrized IDs pendentes
- (C) Funcionalidade nova: inserção direta no banco (não só CSV)
- (D) Mais bancos: MySQL/SQLite
- (E) Qualidade dos dados: métricas de fidelidade, distribuições estatísticas
- (F) Outro

**Resposta default:** (A) Hardening + CI, depois (C) inserção direta. O projeto tem features suficientes mas falta robustez operacional.

---

### Q3: Ambiente de desenvolvimento
**Pergunta:** O Rust/Cargo não está no PATH do terminal atual. Isso é um problema da sessão ou precisa instalar/configurar?

**Resposta default:** Provavelmente falta fazer `source ~/.cargo/env` ou equivalente. Verificar se `rustup` está instalado.

---

### Q4: Testes de integração com Postgres
**Pergunta:** Os testes E2E precisam de um Postgres rodando. Quer configurar o Docker para rodar testes automaticamente, ou mantém manual com `scripts/postgres_docker.sh`?

**Resposta default:** Manter manual por ora, mas adicionar instruções claras no README. CI com Postgres fica para depois.

---

### Q5: Escopo do projeto (acadêmico vs ferramenta real)
**Pergunta:** Este projeto é para a faculdade (TCC/PIT) ou pretende ser uma ferramenta open-source real? Isso altera prioridades (documentação acadêmica vs features).

**Resposta default:** É PIT/acadêmico, mas com ambição de ser utilizável. Priorizar o que demonstra valor no contexto acadêmico.

---

### Q6: Organização de tasks
**Pergunta:** Quer manter o padrão atual de tasks em `tasks/` com milestones por feature, ou simplificar para algo mais leve?

**Resposta default:** Manter o padrão atual — é pesado mas gera rastreabilidade que o PIT exige.

---

## 8) Resumo da arquitetura atual

```
datalchemy/
├── datalchemy-core          # Schema contract (DatabaseSchema, Table, Column, etc.)
├── datalchemy-introspect    # Postgres adapter + queries → schema.json
├── datalchemy-plan          # Plan model + JSON Schema + validation
├── datalchemy-generate      # Engine de geração
│   ├── generators/
│   │   ├── primitives/      # uuid, int, float, text, date, timestamp...
│   │   ├── semantic/        # pt-BR: name, cpf, cnpj, cep, phone...
│   │   ├── derive/          # email_from_name, fk, money_total...
│   │   ├── domain/          # CRM, Finance, Logistics packs
│   │   ├── transforms/      # mask, truncate, etc.
│   │   └── faker_rs.rs      # Registro dos 219 IDs do fake-rs
│   ├── faker_rs/
│   │   ├── adapter.rs       # FakeRsAdapter (único ponto de fake::*)
│   │   ├── catalog_gen.rs   # Auto-gerado via tools/gen_faker_catalog.rs
│   │   └── locales.rs       # LocaleKey: EnUs, PtBr
│   ├── params.rs            # ParamSpec + validação de parâmetros
│   └── engine.rs            # Orquestração: plan → CSV
├── datalchemy-eval           # Métricas + avaliação de datasets
├── datalchemy-cli            # CLI (introspect) + TUI (fluxo completo)
├── schemas/                  # plan.schema.json, schema.schema.json
├── plans/examples/           # Plans de exemplo (CRM, faker, domains)
└── fixtures/sql/postgres/    # SQL pra testes de integração
```

---

## 9) Números do projeto

| Métrica | Valor |
|---------|-------|
| Crates | 6 (core, introspect, plan, generate, eval, cli) |
| Generators disponíveis | 219+ |
| Locales | 2 (pt_BR, en_US) |
| Testes | 39 passando |
| Plan version | 0.2 |
| Schema version | 0.2 |
| Commits na branch | 4 |
| Linhas adicionadas | ~7935 |

---

> **Próximo passo:** Revise este documento, responda as perguntas da seção 7 e seguimos com um plano detalhado de ação.
