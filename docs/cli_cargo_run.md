# CLI via cargo run (sem build separado)

Este guia mostra como usar o CLI com `cargo run` para testar rapidamente,
sem executar `cargo build` manualmente.

---

## 1) Pre-requisitos

- Rust toolchain instalado (`cargo`, `rustc`).
- Postgres local (Docker recomendado).
- Variavel de ambiente `DATABASE_URL` com a string de conexao.

---

## 2) Subir o Postgres local (Docker)

```bash
./scripts/postgres_docker.sh
```

Em seguida, exporte a conexao:
```bash
export DATABASE_URL="postgres://datalchemy:datalchemy@localhost:5432/datalchemy_crm"
```

Se voce usa arquivo `.env`, carregue no shell:
```bash
set -a
source .env
set +a
```

---

## 3) Rodar o CLI com cargo run

```bash
cargo run -p datalchemy-cli -- introspect \
  --conn "$DATABASE_URL" \
  --run-dir runs/
```

---

## 3.1) Rodar a TUI (MVP)

```bash
cargo run -p datalchemy-cli -- tui
```

Dentro da TUI:
- execute `/init` para criar o workspace local
- use `/profiles new <nome> <conn_string>` ou `/db session <conn_string>`
- rode `/introspect`, `/plan new`, `/generate`, `/eval`

---

## 4) Conferir os artefatos gerados

```bash
ls runs/
rg --files runs/<timestamp>__run_<id>/
```

Arquivos esperados por run:
- `schema.json`
- `config.json` (redigido)
- `logs.ndjson`
- `metrics.json`

---

## 5) Validacao rapida (opcional)

```bash
cargo run -p datalchemy-cli -- --help
```

Se houver erro de conexao:
- confirme `DATABASE_URL`
- valide se o Postgres esta de pe
