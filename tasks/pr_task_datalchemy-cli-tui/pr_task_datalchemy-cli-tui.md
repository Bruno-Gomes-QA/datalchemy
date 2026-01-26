# Plano final — Datalchemy Terminal App (TUI) “estilo Codex” (sem wireframe)

Data: 11/01/2026  
Plataforma alvo: **Linux**  
DB (MVP): **PostgreSQL-first**  
LLM (MVP): **Gemini (mock)** + modo **OFF**  
Objetivo: substituir a UI web por uma **experiência de terminal premium**, técnica, auditável, determinística e segura — com fluxo guiado **Introspect → Plan → Generate → Eval**.

> Este plano segue as regras do **AGENTS.md**: determinismo, separação de responsabilidades, sem logs de segredos, sem `unwrap()/expect()`, SQL concentrado no crate de introspect, e artefatos versionáveis/reprodutíveis.

---

## 0) O que mudou neste plano (vs rascunho anterior)

Este plano final adiciona itens críticos que normalmente ficam esquecidos:

1) **Profiles** (múltiplas conexões por workspace) + “active profile” no header  
2) **Approval policy real** (`ask_each_time`) com modal obrigatório antes de qualquer write  
3) **Contrato de cancelamento/erros** com artefatos parciais bem marcados (RUN/OUT/EVAL)  
4) **Artifact versioning** + detecção de incompatibilidade + comando `/doctor`  
5) **Plan edit** (via `$EDITOR`) + **Plan validate** (antes de gerar)  
6) **Logs viewer** e modo diagnóstico  
7) **Performance**: viewers paginados/streaming para JSON/CSV grandes  
8) **Multi-schema** e filtros no introspect (opções de introspecção do AGENTS)  
9) **Model list configurável** (sem recompilar)  
10) Opção de **privacy paranoid** (não exibir host/db no header, se necessário)

---

## 1) Objetivos e não objetivos

### 1.1 Objetivos (MVP)
- Entregar uma TUI bonita e usável, com:
  - onboarding (“login”) que configura workspace + DB + objetivo + LLM
  - command palette com `/` + atalhos (Enter, Ctrl+J, Ctrl+C)
  - histórico de runs/plans/outs/evals por workspace
  - criação de plan via chat (mock) **e** edição manual via `$EDITOR`
  - generate para CSV (INSERT preparado, mas ainda pode ser “not implemented”)
  - eval básico (consistência e sanity checks)
- Ser **determinístico** e auditável:
  - outputs ordenados e estáveis
  - artefatos com manifestos e versões
- Ser seguro:
  - segredos nunca em logs/artefatos
  - suporte `.env` + armazenamento criptografado opt-in

### 1.2 Não objetivos (por enquanto)
- UI web/API HTTP
- suporte a múltiplos bancos além de Postgres
- integração real com Gemini (fica mock, mas arquitetura pronta)
- “one-shot pipeline” (o fluxo é intencionalmente incremental e guiado)

---

## 2) Estética e UX (sem wireframe)

### 2.1 Estética (regras)
- **Tema escuro** (fundo quase preto), com acentos **azul/ciano**.
- Header “premium”: linha superior com logo “Datalchemy”, versão e estado.
- Componentes com bordas discretas (box drawing), sem poluição visual.
- Hierarquia visual:
  - Cyan: ações primárias, itens selecionados, badge de ativo
  - Verde: sucesso
  - Amarelo: warning
  - Vermelho: erro / bloqueio
  - Cinza: texto auxiliar/descrições
- Tipografia: monoespaçada padrão do terminal, mas com:
  - títulos curtos (caps/underline)
  - espaçamento consistente (padding interno nas boxes)
- Sempre mostrar:
  - workspace atual
  - profile ativo (redigido)
  - modo (readonly_csv / insert / explore)
  - LLM (provider/model ou OFF)
  - Active Schema (run_id) e Active Plan (plan_id)

### 2.2 UX (princípios)
- “Sem promessas vazias”: se algo não está implementado, aparece claramente como:
  - `Unsupported` ou `Not implemented (planned)`
- Mensagens de erro sempre incluem:
  - **o que faltou**
  - **por que**
  - **como corrigir** (próximo comando)
- Modos “prod”: evite confirmar demais; mas se `ask_each_time`, confirmar sempre.
- Navegação: command palette é a nave principal (usuário não “se perde”).

---

## 3) Arquitetura e separação (crates / módulos)

> Respeitar fronteiras do AGENTS.md.

### 3.1 Proposta de crates
1) `crates/datalchemy-cli`  
   - TUI (ratatui) + router de telas + command palette  
   - orquestrador do fluxo (gating)  
   - leitura/escrita de workspace local  
   - viewers (JSON/CSV/logs)  
2) `crates/datalchemy-secrets` (novo, leve)  
   - carregar `.env` (opt-in)  
   - armazenamento criptografado local (opt-in)  
   - redaction helpers (mas **contrato** de redaction fica no core, se já existir)  
3) `crates/datalchemy-llm` (novo)  
   - trait `LlmProvider`  
   - `GeminiMockProvider` (MVP)  
   - futuro: `GeminiProvider` real  
4) `crates/datalchemy-contracts` (opcional, recomendado cedo)  
   - tipos/DTOs de manifestos e metadados dos artefatos (run/plan/out/eval)  
   - ajuda a manter estabilidade e versionamento

### 3.2 Onde fica SQL (Postgres)
- SQL deve ficar concentrado no crate de introspect (conforme AGENTS.md).  
- O CLI **não** deve conter SQL; só chama o core/introspect.

---

## 4) Workspace local (fonte de verdade)

### 4.1 Estrutura (sempre local)
```
./datalchemy-cli/
  config/
    settings.toml
    profiles.toml                  # perfis sem senha (múltiplos)
    ui.toml                        # tema/atalhos (opcional)
    llm_models.toml                # lista de modelos (atualizável)
  secrets/
    vault.meta.json                # status do vault (locked/unlocked)
    db.enc                         # opt-in
    llm_gemini.enc                 # opt-in
  runs/
    <run_id>/
      run_manifest.json
      schema.json
      metrics.json
      logs.ndjson
      config.redacted.json
  plans/
    <plan_id>/
      plan.meta.json
      plan.json
      prompt.txt
      llm_transcript.jsonl
  out/
    <out_id>/
      out_manifest.json
      resolved_plan.json
      generation_report.json
      *.csv
  eval/
    <eval_id>/
      eval_manifest.json
      evaluation_report.json
  logs/
    cli.log
```

### 4.2 IDs determinísticos e estáveis
- `run_id`: `YYYY-MM-DD__run_<shortid>`
- `plan_id`: `YYYY-MM-DD__plan_<shortid>`
- `out_id`: `YYYY-MM-DD__out_<shortid>`
- `eval_id`: `YYYY-MM-DD__eval_<shortid>`

---

## 5) Estado do app (o que fica “ativo”)

### 5.1 Estado persistido (settings.toml)
- `approval_policy = always_allow | ask_each_time`
- `active_profile = "<profile_name>"`
- `mode = readonly_csv | insert | explore`
- `active_run_id = "<run_id>|none"`
- `active_plan_id = "<plan_id>|none"`
- `privacy = normal | paranoid`
- `llm_enabled = true|false`
- `llm_provider = gemini|off`
- `llm_model = "<model>"`

### 5.2 Profiles (múltiplos)
Arquivo: `config/profiles.toml`  
- suporta criar/duplicar/renomear/deletar
- perfil não guarda senha
- perfil tem “alias” curto (ex.: `local_dev`, `staging_ro`)

Comandos:
- `/profiles` (listar)
- `/profiles new`
- `/profiles set <name>`
- `/profiles edit <name>`
- `/profiles delete <name>`

---

## 6) Segurança, segredos e LGPD

### 6.1 Fontes de segredo (ordem de prioridade)
1) sessão (input do usuário, não persiste)  
2) `.env` (opt-in do usuário)  
3) `secrets/*.enc` (opt-in, criptografado)

### 6.2 Vault criptografado (opt-in)
- Padrão: **não salvar** segredos.
- Se salvar:
  - criptografar (preferência: `age` com passphrase)  
  - arquivo com permissão **0600**
  - o usuário precisa “unlock” ao abrir workspace (se vault existe)
- Mensagem clara: “sem vault, use `.env` ou sessão”.

Comandos:
- `/secrets status`
- `/secrets import-env`
- `/secrets store-session` (gera vault)
- `/secrets unlock` (se vault locked)
- `/secrets delete`

### 6.3 Redaction obrigatório
- `config.redacted.json` sempre presente em run/out/eval
- logs (`cli.log` e `logs.ndjson`) **nunca** imprimem:
  - password
  - API key
  - connection string completa
- Modo `privacy=paranoid`: header não mostra host/db (apenas alias do profile).

---

## 7) Approval policy (ask_each_time) — precisa ser real

### 7.1 Regra
Se `approval_policy = ask_each_time`, **qualquer** operação que escreve algo fora de `logs/cli.log` deve:
- gerar um “write intent” (lista de arquivos/pastas e motivo)
- pedir confirmação numa modal padrão:
  - “Vou criar X arquivos em `runs/<id>` … Confirmar?”

### 7.2 Onde aplicar
- criar workspace
- introspect (runs)
- criar plan (plans)
- generate (out)
- eval (eval)
- deletar artefatos

---

## 8) Fluxo do usuário (end-to-end)

### 8.1 Start
1) splash (curto)
2) se workspace não existe → permission gate + cria `datalchemy-cli/`
3) se não há profile ativo → wizard `/db` (cria profile e define ativo)
4) root check + seleção de modo (readonly/insert/explore)
5) setup de LLM (opcional) — Gemini mock / OFF
6) entra na home (prompt + palette)

### 8.2 Gating (dependências)
- `/plan new` exige `active_run_id`
- `/generate` exige `active_run_id` + `active_plan_id`
- `/eval` exige `out_id` selecionado (ou escolher na hora)

Sempre que faltar, mostrar:
- “Missing dependencies”
- “How to fix” (comandos sugeridos)
- atalho para abrir a tela certa (ex.: “Press Enter to open /runs”)

---

## 9) Comandos (MVP + essenciais)

### 9.1 Navegação e estado
- `/help`
- `/status`
- `/doctor`
- `/logs` (viewer)
- `/open <path|artifact>`
- `/exit`

### 9.2 Workspace e approvals
- `/init` (cria estrutura)
- `/settings` (approval_policy, privacy, etc.)

### 9.3 DB e profiles
- `/profiles`
- `/db` (wizard para criar/editar profile ativo)
- `/db test`
- `/db privileges` (root check, permissões)
- `/introspect` (wizard com filtros)

### 9.4 LLM
- `/llm` (wizard provider/model, liga/desliga, mock)
- `/llm models` (mostrar modelos carregados de `llm_models.toml`)

### 9.5 Runs / Plans / Out / Eval
- `/runs` (listar + set active)
- `/plans` (listar + set active)
- `/plan new` (chat mock)
- `/plan edit` (abre `$EDITOR` no plan.json)
- `/plan validate` (valida contra schema e contrato)
- `/generate` (CSV; INSERT preparado)
- `/out` (listar + preview)
- `/eval` (executa e lista)
- `/eval list`

---

## 10) Artefatos: manifestos, status e cancelamento

### 10.1 Regras gerais
- Sempre escrever de forma **atômica**:
  - escrever em arquivo temp → fsync → rename
- Sempre incluir:
  - `artifact_version`
  - `cli_version`
  - `created_at`, `finished_at` (se aplicável)
  - `status = RUNNING|OK|ERROR|CANCELLED`

### 10.2 Run manifest (`run_manifest.json`)
Campos mínimos:
- `run_id`
- `status`
- `db_profile` (alias, redigido)
- `introspect_options` (schemas, include_views etc.)
- `schema_fingerprint` (hash do schema.json quando OK)
- `artifact_version`, `cli_version`

### 10.3 Plan meta (`plan.meta.json`)
- `plan_id`
- `status`
- `schema_run_id`
- `schema_fingerprint`
- `provider/model`
- `mock=true|false`
- `artifact_version`, `cli_version`

### 10.4 Out manifest (`out_manifest.json`)
- `out_id`
- `status`
- `schema_run_id`
- `plan_id`
- `mode=csv|insert`
- `seed`, `scale`
- `artifact_version`, `cli_version`

### 10.5 Eval manifest (`eval_manifest.json`)
- `eval_id`
- `status`
- `out_id`
- `checks_enabled`
- `artifact_version`, `cli_version`

### 10.6 Cancelamento (Ctrl+C)
- Introspect cancelado:
  - cria run parcial com `status=CANCELLED` (ou `ERROR` se inconsistência)
- Generate cancelado:
  - cria out parcial com `status=CANCELLED`
- Eval cancelado:
  - cria eval parcial com `status=CANCELLED`
- UI deve avisar: “partial artifact created” e permitir deletar.

---

## 11) Versionamento e compatibilidade

### 11.1 version fields
- `artifact_version`: versão do formato dos manifestos/metas
- `schema_version`: do `schema.json` (se já existe no core)
- `plan_version`: do contrato `plan.json` (se existir)
- `cli_version`: do binário atual

### 11.2 /doctor (obrigatório)
`/doctor` deve checar:
- estrutura do workspace
- permissões de pasta/arquivo (inclui 0600 em secrets)
- presença de profile ativo
- compatibilidade de artefatos:
  - run/plan/out/eval com `artifact_version` incompatível
- sugestões:
  - “regen plan”
  - “re-introspect”
  - “migrate (future)”

No MVP: migração automática só se for trivial; caso contrário, sugerir regerar.

---

## 12) Introspect (Postgres-first) — opções e multi-schema

Wizard `/introspect` deve permitir:
- selecionar schemas (lista + filtro)
- opções (default seguras, conforme AGENTS):
  - `include_system_schemas=false`
  - `include_views=false` (MVP; opcional)
  - `include_materialized_views=false`
  - `include_foreign_tables=false`
  - `include_indexes=true|false`
  - `include_comments=true|false`
- resultado: `schema.json` determinístico (ordenado)

---

## 13) Plan: criação, edição e validação

### 13.1 /plan new (chat mock)
- exige `active_run_id`
- gera `plan.json` + `plan.meta.json` + transcript/prompt
- define `active_plan_id`

### 13.2 /plan edit (MVP obrigatório)
- abre `plan.json` no `$EDITOR` (fallback `nano`/`vi`)
- ao voltar, rodar validação mínima (JSON parse + campos obrigatórios)
- se inválido: não deixa setar como active até corrigir

### 13.3 /plan validate (antes de generate)
- valida:
  - schema-aware (tabelas/colunas existentes)
  - constraints suportadas
  - contrato/semântica do generator
- saída amigável: erros com path + dica

---

## 14) Generate: CSV agora, INSERT preparado

### 14.1 CSV (MVP)
- exige `active_run_id` + `active_plan_id` + plan validado
- escreve:
  - CSVs
  - resolved_plan.json
  - generation_report.json
  - out_manifest.json

### 14.2 INSERT (preparado)
- só habilitar se:
  - modo do workspace = insert
  - superuser = true
  - confirmação “type INSERT”
  - “dry-run summary” exibido antes (tabelas/linhas)
- no MVP pode ser “Not implemented”, mas o gating e UX devem existir.

---

## 15) Eval: mínimo útil, determinístico

- checks:
  - FK consistency
  - nullability
  - uniqueness (quando aplicável)
  - sanity de distribuições (básico)
- outputs:
  - evaluation_report.json
  - eval_manifest.json

---

## 16) Viewers (JSON/CSV/logs) — performance

Regras:
- nunca carregar arquivos grandes inteiros por padrão
- JSON viewer:
  - colapsável e com paginação (render lazy)
- CSV preview:
  - read “head N” e depois pagina
  - busca incremental (não O(n) no arquivo todo de primeira)
- logs viewer:
  - tail das últimas N linhas
  - filtro por nível (INFO/WARN/ERROR)

---

## 17) Determinismo e ordem (não negociável)

- JSON:
  - serialização determinística (ordenar coleções; evitar HashMap no output)
- listagens:
  - ordenar por `created_at` derivado do id + fallback por nome
- manifestos:
  - campos sempre na mesma ordem (ser/de com struct, sem maps)

---

## 18) Dependências Rust (pin exato)

- TUI: `ratatui`, `crossterm`
- config: `serde`, `toml`, `serde_json`
- logs: `tracing`, `tracing-subscriber`
- ids/time: `uuid`, `chrono`
- hash: `blake3`
- csv preview: `csv`
- errors: `thiserror`
- crypto (opt-in):
  - preferir `age` (mais simples) **ou** `aes-gcm` + `argon2`
- Regra: versões **fixas** no Cargo.toml (sem `^`/`~`) conforme AGENTS.md.

---

## 19) Testes e evidências (AGENTS.md)

### 19.1 Unit tests
- redaction
- status/manifest writing atômico
- determinismo de serialização
- gating de dependências

### 19.2 Integration tests
- Postgres via Docker (`scripts/postgres_docker.sh`)
- introspect cobre PK/FK/UNIQUE/CHECK
- generate CSV smoke + eval smoke

### 19.3 Evidência (obrigatória por task)
- cada PR/task deve gerar `evidence/<task_id>.md` com:
  - o que mudou
  - por que mudou
  - como validar (comandos exatos)
  - links/paths para artefatos gerados

---

## 20) Fases de implementação (ordem recomendada)

### Fase 1 — Base TUI + workspace + palette
- splash + header + input
- permission gate + approval policy
- `/help`, `/status`, `/open`, `/exit`

### Fase 2 — Profiles + DB wizard + privileges
- `/profiles` + `/db`
- `/db test` + `/db privileges`
- modo (readonly/insert/explore)

### Fase 3 — ask_each_time real (write intent)
- modal padrão antes de qualquer write
- delete com confirmação

### Fase 4 — Introspect + runs viewer
- `/introspect` wizard com filtros multi-schema
- `/runs` list + set active + details
- run_manifest + cancelamento OK

### Fase 5 — Plans (mock + editor + validate)
- `/plan new` (mock)
- `/plan edit` ($EDITOR)
- `/plan validate`
- plans list + active

### Fase 6 — Generate CSV + outputs viewer
- `/generate` CSV
- `/out` list + preview paginado
- out_manifest + cancelamento OK

### Fase 7 — Eval + reports
- `/eval` + viewer
- eval_manifest

### Fase 8 — Doctor + logs + hardening
- `/doctor` completo
- `/logs` viewer
- performance e edge cases

---

## 21) Critérios de aceite (MVP)

- workspace local criado e respeita approval policy
- profiles funcionam, active profile aparece no header
- segredos nunca vazam; `.env` suportado; vault opt-in criptografado
- `/introspect` gera run com run_manifest e schema.json determinístico
- `/plan new` cria plan mock; `/plan edit` permite ajustar; `/plan validate` funciona
- `/generate` gera CSV + out_manifest; preview funciona sem travar
- `/eval` gera report + eval_manifest
- cancelamento cria artefatos parciais com status correto
- `/doctor` detecta inconsistências e sugere correções
- `cargo fmt` + `cargo clippy -D warnings` + testes passam

