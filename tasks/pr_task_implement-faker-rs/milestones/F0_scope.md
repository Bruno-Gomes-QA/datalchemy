# F0 — Decisões de produto e escopo (fake-rs first)

> Consolidar decisões que guiam TODAS as tarefas seguintes.



**Repo:** https://github.com/Bruno-Gomes-QA/datalchemy  
**Data:** 2026-01-25

## Trilhos obrigatórios (AGENTS.md)

- **Tasks + Evidence**: toda mudança precisa de `tasks/issue_task_*.md` e `evidence/<task_id>.md`.
- **Qualidade**: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- **Logs**: `tracing` (nunca `println!` em libs).
- **Erros**: `thiserror`.
- **Dependências pesadas**: só com justificativa clara (este plano inclui a justificativa).
- **Determinismo** (do AGENTS): output ordenado/estável (sem HashMap em output).  
  **Nota deste plano:** você pediu “determinismo fora do escopo”, então aqui tratamos como *não-objetivo de produto*, mas **mantemos ordenação estável** e **passagem de RNG** para não violar AGENTS e para manter a porta aberta.

## Decisões (com base nas suas respostas)

1) **Cobertura alvo:** queremos contemplar `fake-rs` de forma ampla:  
   - (a) 80% do uso comum, (b) mapeamento “quase completo” do catálogo, e (c) mecanismo de expansão fácil.

2) **Locales desde já:** `pt_BR` e `en_US` (com default configurável).  
   - Locale vira parte do contexto de geração (`GenerationContext`).

3) **IDs estáveis do Datalchemy:** o plan usa **IDs estáveis** (ex.: `semantic.person.name`, `semantic.address.city`).  
   - O `fake-rs` fica **encapsulado** em `datalchemy-generate` (sem dependência no plan).
   - Para “cobrir tudo”, adicionaremos também IDs “espelho” gerados automaticamente do tipo `faker.<mod>.<Struct>`.
     - Esses IDs “espelho” contam como IDs do Datalchemy (porque o contrato é nosso), mas são gerados a partir do upstream.

4) **Tipos completos:** o motor deve gerar **todos** os tipos suportados hoje pelo `GeneratedValue`:  
   Bool, Int, Float, Text, Uuid, Date, Time, Timestamp (+ JSON/Decimal se existir no core).

5) **Sem sanitizers por enquanto:** a saída do faker entra “como veio”.  
   - Ainda assim: **logs sem valores sensíveis**, e nenhum `println!`.

6) **Erro direto:**  
   - generator_id desconhecido → **ERRO** (parar).  
   - parâmetro inválido/desconhecido → **ERRO** (parar).  
   - locale não suportado para um generator → **ERRO**.

7) **Catálogo grande + parâmetros avançados:**  
   - catálogo gerado automaticamente com “defaults” (param_spec vazio)  
   - + overrides manuais para IDs estáveis (param_spec avançado onde faz sentido).

8) **Dependência fake-rs NÃO será opcional** (decisão sua).  
   - Precisamos justificar no repo (AGENTS pede justificativa para deps pesadas).

9) **Geradores atuais podem ser descartados**:  
   - vamos migrar para fake-rs como backend “baseline” e manter camadas autorais depois.
   - compatibilidade: manter `plan.json` atual funcionando (ou migrar com compat layer), mas a implementação passa a ser fake-based.

## Artefatos e prova de sucesso

No final (após F6/F7), você deve conseguir:
- rodar E2E Postgres e gerar CSVs coerentes (mesmo sem determinismo como requisito);
- trocar locale global e ver dados mudarem de pt_BR para en_US;
- usar IDs estáveis (semantic.*) e também IDs “espelho” (faker.*);
- passar parâmetros avançados (min/max/len/pattern etc.) com validação forte.

## Template (tasks/evidence)
## Template de task (crie antes de codar)

Crie: `tasks/issue_task_<YYYYMMDD>_<slug>.md`

```md
# Task: <título curto>

- ID: issue_task_<YYYYMMDD>_<slug>
- Owner: <nome>
- Status: Draft | In Progress | Done
- Milestone: <F0 | F1 | ...>
- Crates: <lista>
- Risco: Baixo | Médio | Alto

## Contexto
<por que esta task existe>

## Objetivo
<resultado final observável>

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
./scripts/postgres_docker.sh
# + E2E
```

## Evidência (obrigatória)
- Arquivo: `evidence/<task_id>.md`
- Deve incluir: "o que mudou", "por que mudou", "como validar", outputs.
```

## Template de evidência (preencha no final)

Crie: `evidence/<task_id>.md`

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
./scripts/postgres_docker.sh
# introspect -> generate -> eval
```

## Resultado
- RUN_DIR=...
- OUT_DIR=...
- logs/erros relevantes: ...

## Notas/Riscos
- ...
```
