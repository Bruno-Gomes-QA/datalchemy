# Issue Task: Crash/Incomplete Generation on Full Stack Plan

- **ID:** issue_task_20260124_full_stack_crash
- **Status:** Open
- **Severity:** Critical
- **Affected Milestone:** M6 (Domains) / M7 (Hardening) / Core Engine
- **Created:** 2026-01-24

## 1. Contexto do Problema

Durante a validação manual do plano `plans/examples/full_stack_ptbr.plan.json` (10k linhas, múltiplas tabelas, generators variados), o processo de geração apresentou comportamento anômalo:
1. **Geração Incompleta:** O plano define 12 tabelas alvo, mas apenas **uma** (`crm.empresas.csv`) foi encontrada no diretório de saída.
2. **Ausência de Relatório:** O arquivo `generation_report.json` não foi gerado, indicando que o processo não finalizou graciosamente (panic, OOM killed, ou silent crash).
3. **Performance Degradada:** O usuário relatou que a execução "demorou muito" antes de (presumivelmente) falhar.

## 2. Evidências

**Comando Executado:**
```bash
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/full_stack_ptbr.plan.json \
  --schema "runs/<latest>/schema.json" \
  --out out/full_stack/
```

**Resultado Observado (`ls -la out/full_stack/<run_id>/`):**
```
total 1836
drwxrwxr-x 2 user user    4096 Jan 24 12:13 .
drwxrwxr-x 3 user user    4096 Jan 24 12:04 ..
-rw-rw-r-- 1 user user 1868865 Jan 24 12:13 crm.empresas.csv
```
- Faltam tabelas: `usuarios`, `contatos`, `leads`, `oportunidades`, `atividades`, etc.
- Falta: `generation_report.json`.

**Plano de Origem:**
- `plans/examples/full_stack_ptbr.plan.json`
- Targets: 10.000 linhas para tabelas principais.
- Generators: `semantic.br.*`, `domain.*`, `derive.*`.

## 3. Investigação Necessária

Precisamos entender por que o gerador parou após uma tabela e por que não reportou erro.

### Hipóteses
1. **Panic Silencioso:** Algum generator (possivelmente em `crm.empresas` ou na próxima tabela da fila `crm.contatos`?) pode estar causando `panic!`.
2. **Memory Leak / OOM:** Se o gerador mantém estado excessivo em memória (ex: RowContext acumulando indefinidamente ou Assets carregados incorretamente), o processo pode ter sido morto pelo SO.
3. **Deadlock:** Se houver concorrência (embora o engine atual pareça serial), ou se o `AssetsLoader` tiver locking incorreto.
4. **Performance Exponencial:** Algum generator pode estar com complexidade O(N^2) ou pior, fazendo o processo parecer travado.

## 4. Plano de Ação (Correção)

1. **Reprodução Isolada:**
   - Rodar o plano com `RUST_BACKTRACE=1` e logs em nível `trace`.
   - Reduzir o número de linhas (ex: 100) para ver se o erro persiste ou é volume-dependente.

2. **Profiling:**
   - Monitorar uso de CPU e Memória durante a execução.
   - Identificar qual tabela estava sendo processada no momento da falha (logs).

3. **Fixing:**
   - Se for panic: Corrigir o bug no generator ou engine.
   - Se for performance: Otimizar o hot-path (provavelmente interações de IO ou clones excessivos).
   - **Garantia:** Implementar catch de panic na engine para garantir que o `generation_report.json` seja escrito (com status `FAILED`) mesmo em caso de crash.

## 5. Critérios de Conclusão
- [ ] O plano `full_stack_ptbr.plan.json` executa do início ao fim com 10k linhas.
- [ ] Todas as 12 tabelas geram arquivos CSV.
- [ ] `generation_report.json` é gerado sempre.
- [ ] Tempo de execução aceitável (< 2min para 100k linhas totais, ou definir baseline).
