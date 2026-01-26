# Evidencia - pr_task_datalchemy-cli-tui

## O que mudou
- TUI MVP implementada no `datalchemy-cli` com workspace local, command palette e fluxo Introspect -> Plan -> Generate -> Eval.
- Workspace e manifestos versionados adicionados (run/plan/out/eval) com escrita atomica e /doctor.
- Suporte a secrets (`/secrets`) com vault criptografado (age) e import de `.env`.
- UI da TUI refinada para layout compacto estilo Codex CLI, com prompt visivel e cursor correto.
- Command palette agora aparece automaticamente ao digitar `/`, com input logo abaixo do header e barra cyan.
- Header exibido em linhas verticais com bordas arredondadas, sem timestamps nas mensagens, e setup guiado no primeiro uso.
- Docs atualizados (`README.md`, `docs/cli_commands.md`, `docs/cli_cargo_run.md`, `datalchemy_structure.md`).

## Por que mudou
Para entregar o plano completo da TUI com artefatos auditaveis, aprobacoes reais e workflow guiado, mantendo a documentacao alinhada.

## Como validar
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings

cargo run -p datalchemy-cli -- tui

rg -n "datalchemy tui" README.md docs/cli_commands.md
rg -n "cli_cargo_run" README.md docs/cli_cargo_run.md
```

## Evidencia
- Nao executei comandos neste ambiente. Os comandos acima reproduzem a validacao local.

## Update: Refatoração TUI

### O que mudou
- Refatoração completa do  (monolito) para estrutura modular:
    - : Definições de estado (, ).
    - : Lógica de renderização (, ) usando .
    - : Implementação dos comandos ().
    - : Tratamento de eventos de teclado ().
    - : Funções auxiliares (I/O, manipulação de strings).
    - : Criptografia e gestão de vault.

### Por que mudou
Para reduzir a complexidade do arquivo  (que ultrapassava 2000 linhas), separando responsabilidades de UI, lógica de estado, comandos e eventos, facilitando manutenção e leitura.

### Como validar
```bash
cargo check -p datalchemy-cli
```

## Update: UX comandos e resets

### O que mudou
- `/help` agora lista comandos e subcomandos de forma completa, com `/help` penultimo e `/exit` por ultimo.
- `/init` so aparece quando nao existe `datalchemy-cli/`; caso exista, o menu mostra `/reset` com confirmacao central.
- `resolve_connection_string` passa a ler `.env` automaticamente para evitar pedir a URL de novo.
- `/settings` virou menu com `show` e `set`.
- `/runs` ganhou `list`, `inspect`, `delete` e `set` com saidas formatadas.

### Por que mudou
Para melhorar o fluxo de uso, reduzir friccao em novas sessoes e deixar o menu consistente com a UX desejada.

### Como validar
```bash
cargo fmt
cargo check -p datalchemy-cli
```

### Evidencia
- `cargo fmt`
- `cargo check -p datalchemy-cli`

## Update: Comandos TUI e UX

### O que mudou
- Ajustei o fluxo de `/db session` e `/db change` para input dedicado (sem disparar introspecao).
- Corrigi o log de eventos async para aparecer fora da introspecao (ex.: `/db test` e `/db privileges`).
- Melhorei `/status` e `/help` com informacao clara e menos emojis.
- Expandido o command palette com comandos e subcomandos faltantes.
- Permiti `datalchemy introspect` usar `DATABASE_URL` quando nao houver flag/posicional.

### Por que mudou
Para tornar a UX mais previsivel, evitar logs de secrets e garantir que subcomandos e feedbacks funcionem corretamente.

### Como validar
```bash
cargo fmt
cargo check -p datalchemy-cli
cargo run -p datalchemy-cli -- tui
```

### Evidencia
- `cargo fmt`
- `cargo check -p datalchemy-cli`

## Update: Refatoração TUI

### O que mudou
- Refatoração completa do `crates/datalchemy-cli/src/tui/mod.rs` (monolito) para estrutura modular:
    - `state.rs`: Definições de estado (`App`, `UiState`).
    - `ui.rs`: Lógica de renderização (`draw_ui`, `render_*`) usando `ratatui`.
    - `commands.rs`: Implementação dos comandos (`cmd_*`).
    - `events.rs`: Tratamento de eventos de teclado (`handle_key`).
    - `utils.rs`: Funções auxiliares (I/O, manipulação de strings).
    - `secrets.rs`: Criptografia e gestão de vault.

### Por que mudou
Para reduzir a complexidade do arquivo `mod.rs` (que ultrapassava 2000 linhas), separando responsabilidades de UI, lógica de estado, comandos e eventos, facilitando manutenção e leitura.

### Como validar
```bash
cargo check -p datalchemy-cli
```
