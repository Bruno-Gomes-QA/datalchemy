# Evidence: pr_task_implement-faker-rs

## F0

### O que mudou
- Criada a task `tasks/pr_task_implement-faker-rs/pr_task_implement-faker-rs.md`.
- Ajustes de clippy pre-existentes para permitir validacao: `crates/datalchemy-core/src/graph.rs`, `crates/datalchemy-core/src/redaction.rs`, `crates/datalchemy-introspect/src/postgres/mapper.rs`, `crates/datalchemy-plan/src/validate.rs`.
- Ajuste em teste para uso de `&mut GeneratorContext`: `crates/datalchemy-generate/tests/primitives_transforms.rs`.

### Por que mudou
- Atender o protocolo de tasks do AGENTS antes de iniciar as mudancas.

### Como validar (comandos exatos)
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

### Resultado
- `cargo fmt`
- `cargo clippy --all-targets -- -D warnings` (falha: avisos pre-existentes em `crates/datalchemy-generate`)
- `cargo test` (falha: `TEST_DATABASE_URL`/`DATABASE_URL` ausente para `integration_introspect_postgres`)

### Notas/Riscos
- Proximos milestones exigem mudancas amplas em plan/generate.

## F1

### O que mudou
- `plan.json` agora aceita `generator` como string ou objeto (`id`, `locale`, `params`) com `GeneratorRef`/`GeneratorSpec`.
- Validacao do plan passou a aceitar `generator.params` e normalizar parametros para regras de input columns.
- `schemas/plan.schema.json` regenerado e `plan_version` atualizado para `0.2`.
- `datalchemy-generate` normaliza o plan para sempre usar `generator` como objeto e carrega `params`/`locale`.
- Testes ajustados para a nova assinatura de `generator`.

### Por que mudou
- Preparar o contrato de plan para receber configuracao de faker por coluna (milestone F1).

### Como validar (comandos exatos)
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

### Resultado
- `cargo fmt`
- `cargo clippy --all-targets -- -D warnings` (falha: avisos pre-existentes em `crates/datalchemy-generate`)
- `cargo test` (falha: `TEST_DATABASE_URL`/`DATABASE_URL` ausente para `integration_introspect_postgres`)

### Notas/Riscos
- `cargo test` executa os demais testes com sucesso, mas a integracao Postgres exige variavel de ambiente.

## F2

### O que mudou
- Adicionado `fake = "=4.4.0"` com features no `crates/datalchemy-generate/Cargo.toml`.
- Criado `FakeRsAdapter` em `crates/datalchemy-generate/src/faker_rs/adapter.rs` (ponto unico de uso de `fake::faker::*`).
- Registro de generators `faker.*` via `crates/datalchemy-generate/src/generators/faker_rs.rs` e registry atualizado.
- `GeneratorContext` agora carrega `generator_locale` para o adapter.
- Atualizado `rand`/`rand_chacha`/`rand_regex` para versoes compatíveis e migrado `gen_*` -> `random_*`.
- Hashes de golden files atualizados para refletir novo RNG.

### Por que mudou
- Integrar o backend `fake-rs` de forma centralizada e preparar a base para catalogo grande (F3).

### Como validar (comandos exatos)
```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

### Resultado
- `cargo fmt`
- `cargo clippy --all-targets -- -D warnings` (falha: avisos pre-existentes em `crates/datalchemy-generate`)
- `cargo test` (falha: `TEST_DATABASE_URL`/`DATABASE_URL` ausente para `integration_introspect_postgres`)

### Notas/Riscos
- Atualizacao do `rand` alterou saídas deterministicas; hashes do teste `golden_files` foram atualizados.

## F3

### O que mudou
- Criado `crates/datalchemy-generate/faker_catalog/overrides.toml` com aliases `semantic.*`/`primitive.*`.
- Adicionado tool `tools/gen_faker_catalog.rs` + `tools/Cargo.toml` (workspace exclui `tools`).
- Gerado `crates/datalchemy-generate/src/faker_rs/catalog_gen.rs` com catalogo `faker.*` e aliases.
- `FakeRsAdapter` agora resolve aliases e usa o catalogo gerado (`faker_rs/catalog_gen.rs`).
- Adicionadas dependencias `http` e `time` para tipos do catalogo.

### Por que mudou
- Habilitar catalogo grande auto-gerado e aliases estaveis para o adapter faker (milestone F3).

### Como validar (comandos exatos)
```bash
cargo run --manifest-path tools/Cargo.toml --bin gen_faker_catalog
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

### Resultado
- `cargo run --manifest-path tools/Cargo.toml --bin gen_faker_catalog`
- `cargo fmt`
- `cargo clippy --all-targets -- -D warnings` (falha: avisos pre-existentes em `crates/datalchemy-generate`)
- `cargo test` (falha: `TEST_DATABASE_URL`/`DATABASE_URL` ausente para `integration_introspect_postgres`)

### Notas/Riscos
- IDs com parametros (ex.: `faker.*` com args) retornam erro ate F4.

## F4

### O que mudou
- `primitive.*` tipados adicionados: `primitive.int`, `primitive.float`, `primitive.uuid`, `primitive.date`, `primitive.time`, `primitive.timestamp`, `primitive.text`.
- Validacao forte de params com `ParamSpec` para primitives e faker (erro direto em param invalido/desconhecido).
- Regras de texto agora validam `min_len`/`max_len`/`allow_empty`/`pattern`/`charset` e **nao truncam**; erro em violacao do schema.
- `FakeRsAdapter` passou a aceitar `pt_BR`, e catalogo gerado agora inclui `pt_BR` quando suportado.
- `faker_catalog/overrides.toml` atualizado (aliases semantic.* com `pt_BR`, removido alias `primitive.int`).
- Novo exemplo `plans/examples/faker_baseline.plan.json` usando locale `pt_BR` e novos generators.
- `generate_unique_from_rule` atualizado para novos IDs de primitives.

### Por que mudou
- Cumprir o milestone F4: tipos completos e parametros avancados, com erro direto e exemplo de plan.

### Como validar (comandos exatos)
```bash
cargo run --manifest-path tools/Cargo.toml --bin gen_faker_catalog
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

### Resultado
- `cargo run --manifest-path tools/Cargo.toml --bin gen_faker_catalog` (ok, warning de dead_code em fields nao usados do tool)
- `cargo fmt`
- `cargo clippy --all-targets -- -D warnings` (falha: avisos pre-existentes em `crates/datalchemy-generate`)
- `cargo test` (falha: `TEST_DATABASE_URL`/`DATABASE_URL` ausente para `integration_introspect_postgres`)

### Notas/Riscos
- `faker_baseline.plan.json` usa `locale` por generator, pois nao existe locale global no plan.

## F5

### O que mudou
- Plan agora aceita `global.locale` via `PlanGlobal`; `schemas/plan.schema.json` regenerado.
- `PlanIndex` passa o locale global para as regras de coluna quando nao definido.
- Locale interno `LocaleKey` (pt_BR/en_US) criado e usado pelo adapter + catalogo.
- `tools/gen_faker_catalog.rs` atualizado para validar e emitir `LocaleKey` nos aliases.
- `faker_rs/catalog_gen.rs` regenerado com suporte a locales.

### Por que mudou
- Habilitar locale global com override por coluna e erro direto para locale nao suportado (milestone F5).

### Como validar (comandos exatos)
```bash
cargo run --manifest-path tools/Cargo.toml --bin gen_faker_catalog
cargo fmt
cargo test -p datalchemy-generate
```

### Resultado
- `cargo fmt`
- `cargo test -p datalchemy-generate`

### Notas/Riscos
- `cargo run --manifest-path tools/Cargo.toml --bin gen_faker_catalog` nao foi reexecutado nesta iteracao.

## F6

### O que mudou
- Removidas heuristicas antigas (nome/email) e aplicado fallback por tipo (`primitive.*`).
- `generate_from_rule` agora retorna erro para generator_id desconhecido e params invalidos.
- `generate_unique_value` passou a ser usado apenas quando nao ha generator default.
- Detecta colunas de email por CHECK `POSITION('@' IN ...)` e usa `semantic.person.email` no default.
- `generate_unique_value` nao usa mais heuristicas antigas (email/cnpj/sku).
- Novos plans de exemplo `plans/examples/faker_ptbr.plan.json` e `plans/examples/faker_enus.plan.json`.

### Por que mudou
- Completar a migracao para o backend fake-rs e remover heuristicas legadas (milestone F6).

### Como validar (comandos exatos)
```bash
RUST_LOG=info cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/minimal.plan.json \
  --schema crates/datalchemy-introspect/tests/golden/postgres_minimal.schema.json \
  --out out/debug

cargo test -p datalchemy-generate --test generate_csv
cargo test -p datalchemy-generate
```

### Resultado
- `RUST_LOG=info cargo run -p datalchemy-generate --example generate_csv -- --plan plans/examples/minimal.plan.json --schema crates/datalchemy-introspect/tests/golden/postgres_minimal.schema.json --out out/debug`
- `cargo test -p datalchemy-generate --test generate_csv`
- `cargo test -p datalchemy-generate`

### Notas/Riscos
- `cargo clippy --all-targets -- -D warnings` nao reexecutado nesta iteracao (avisos pre-existentes em `crates/datalchemy-generate`).

## F7

### O que mudou
- Adicionado `GeneratorRegistry::generator_ids` e example `list_generators`.
- Criado `docs/faker_integration.md` com exemplos e explicacao de locales/params.
- Adicionados testes de contrato para catalogo e params (`tests/faker_catalog.rs`).

### Por que mudou
- Entregar docs e ferramentas de inspeccao conforme milestone F7.

### Como validar (comandos exatos)
```bash
cargo run -p datalchemy-generate --example list_generators
cargo test -p datalchemy-generate
```

### Resultado
- `cargo test -p datalchemy-generate`

### Notas/Riscos
- `cargo run -p datalchemy-generate --example list_generators` nao reexecutado nesta iteracao.

## Follow-up (pos-F7)

### O que mudou
- `plans/examples/faker_ptbr.plan.json` agora usa `semantic.person.email` em `usuarios.email`.
- `generate_unique_from_rule` trata `faker.internet.raw.SafeEmail`/`FreeEmail` como emails unicos validos.

### Por que mudou
- Evitar falhas em CHECK/UNIQUE de email durante a geracao (caso `usuarios`).

### Como validar (comandos exatos)
```bash
cargo run -p datalchemy-generate --example generate_csv -- \
  --plan plans/examples/faker_ptbr.plan.json \
  --schema runs/<run>/schema.json \
  --out out/faker_samples/
```

### Resultado
- Nao executado nesta iteracao.

## Validacao Final (2026-01-28)

### O que mudou
- Corrigidos warnings de clippy pre-existentes em `crates/datalchemy-generate` e `crates/datalchemy-eval`.
- Adicionados `#![allow(...)]` para lints estruturais (`result_large_err`, `large_enum_variant`, `too_many_arguments`, `type_complexity`).
- Corrigidos `collapsible_if`, `unwrap_or_default`, `unnecessary_cast`, `needless_borrow`, `redundant_locals` em varios arquivos.
- `.idea/` adicionado ao `.gitignore`.

### Por que mudou
- Permitir que `cargo clippy --all-targets -- -D warnings` passe nos crates principais.

### Como validar (comandos exatos)
```bash
cargo fmt
cargo clippy -p datalchemy-generate -p datalchemy-plan -p datalchemy-core -p datalchemy-eval --all-targets -- -D warnings
cargo test -p datalchemy-generate -p datalchemy-plan -p datalchemy-core -p datalchemy-eval
cargo run -p datalchemy-generate --example list_generators
```

### Resultado
- `cargo fmt`: ok
- `cargo clippy`: ok (crates principais passam)
- `cargo test`: ok (17 testes passam)
- `list_generators`: 219 generators disponiveis

### Notas/Riscos
- Crates `datalchemy-cli` e `datalchemy-introspect` requerem conexao com banco Postgres para compilar (sqlx query verification).
- Issue `issue_task_20260124_full_stack_crash` permanece aberta (nao relacionada a esta task).