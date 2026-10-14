# Implementation Plan: Analisador e Limpador de Espaço em Disco com IA

**Branch**: `001-disk-space-cleaner` | **Date**: 2026-10-14 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-disk-space-cleaner/spec.md`

## Summary

App desktop macOS que varre o disco (somente leitura), cataloga em SQLite os maiores arquivos,
pastas, aplicações/jogos e arquivos de nuvem baixados, classifica o risco de exclusão com uma
cadeia de classificadores (regras protegidas → cache → IA em nuvem ou local → heurística) e
executa ações auditáveis (Lixeira, "manter só na nuvem") somente após revisão do usuário.
Arquitetura hexagonal em Rust (workspace com `cleaner-core`, `cleaner-infra`, `src-tauri`) e
frontend Vue 3 + TypeScript. Detalhes das decisões em [research.md](./research.md).

## Technical Context

**Language/Version**: Rust stable (edição 2021, toolchain fixada em `rust-toolchain.toml`);
TypeScript 5.x (strict); Node 22 LTS

**Primary Dependencies**: Tauri 2.x, `jwalk`, `rusqlite` (bundled) + `rusqlite_migration`,
`serde`/`serde_json`, `thiserror`, `tracing`, `reqwest` (rustls), `trash`, `keyring`
(apple-native), `plist`, `objc2-foundation`, `ts-rs`, `crossbeam-channel`; Vue 3, Pinia,
Vue Router, vue-i18n, `@tauri-apps/api` v2, `tauri-plugin-dialog`

**Storage**: SQLite (WAL) em `~/Library/Application Support/<bundle-id>/catalog.sqlite3`;
segredos no Keychain

**Testing**: `cargo test`, `proptest`, `insta`, `tempfile`, `wiremock`, `cargo-llvm-cov`;
Vitest + Vue Test Utils + `@tauri-apps/api/mocks`

**Target Platform**: macOS 13+ (aarch64 e x86_64), distribuição fora da App Store (.dmg)

**Project Type**: desktop-app (Tauri: núcleo Rust + webview Vue)

**Performance Goals**: varredura de 1 M de arquivos < 5 min em SSD; re-varredura sem mudanças
< 25% do tempo; UI responde < 200 ms durante a varredura; listas paginadas ≤ 500 itens

**Constraints**: somente leitura na varredura; nenhum envio de conteúdo à IA sem opt-in;
funcionamento offline (heurística); memória do processo < 500 MB durante varredura de 1 M itens

**Scale/Scope**: 1 usuário local; catálogos de até ~5 M entradas vistas (persistidas só as
≥ limite); ~8 telas (Varredura, Arquivos, Pastas, Aplicações e Jogos, Nuvem, Revisão, Histórico,
Configurações)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Princípio | Como o plano atende | Status |
|---|---|---|
| I. Segurança dos dados | Scanner sem operações de escrita; `ProtectedPathPolicy` + `ActionExecutor` com revalidação por fingerprint; Lixeira padrão; sem exclusão permanente na v1; `ActionRecord` append-only; symlinks não seguidos | ✅ |
| II. Arquitetura limpa | Hexagonal com `cleaner-core` sem I/O; ports em [core-ports.md](./contracts/core-ports.md); Repository, Strategy, Chain of Responsibility, Observer, Command, Factory/Adapter; `thiserror`; clippy proíbe `unwrap` fora de testes (`clippy::unwrap_used`) | ✅ |
| III. Testes primeiro | Fakes de todos os ports; integração com FS/SQLite reais; `wiremock` para IA; cobertura 80%/70% no CI; testes dedicados para cada regra do Princípio I | ✅ |
| IV. IA consultora | Só metadados, `~` no lugar do home; schema validado; fallback heurístico; cache por fingerprint+versão; IA nunca pré-seleciona | ✅ |
| V. Desempenho | `jwalk` paralelo, `spawn_blocking`, cancelamento, `Channel` com throttle, inserções em lote, paginação no banco, re-varredura incremental | ✅ |
| VI. Simplicidade | SQLite único, sem servidor; `rusqlite` em vez de ORM; sem kit de UI; E2E e FSEvents adiados | ✅ |
| Stack & Restrições | Tauri v2, Vue 3 + TS, SQLite, macOS 13+, mecanismos oficiais de nuvem, Keychain | ✅ |
| Quality Gates | CI com fmt, clippy, test, cobertura, eslint, prettier, vue-tsc, vitest, build dmg; release por tag | ✅ |

**Re-check pós-design (Phase 1)**: ✅ sem violações. Ver Complexity Tracking para os pontos
que adicionam estrutura.

## Project Structure

### Documentation (this feature)

```text
specs/001-disk-space-cleaner/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── tauri-commands.md
│   ├── core-ports.md
│   └── ai-classification.md
├── checklists/requirements.md
└── tasks.md               # /speckit-tasks
```

### Source Code (repository root)

```text
Cargo.toml                      # workspace
rust-toolchain.toml
crates/
├── cleaner-core/               # domínio + casos de uso + ports (sem I/O, sem Tauri)
│   ├── src/
│   │   ├── domain/             # scan.rs, file_entry.rs, risk.rs, app.rs, cloud.rs, action.rs, size.rs
│   │   ├── ports/              # filesystem.rs, repository.rs, ai.rs, actions.rs, apps.rs, cloud.rs, secrets.rs, clock.rs
│   │   ├── policy/             # protected_paths.rs, heuristics.rs
│   │   ├── classify/           # chain.rs, protected.rs, cached.rs, ai.rs, heuristic.rs
│   │   ├── services/           # scan_service.rs, catalog_service.rs, assessment_service.rs,
│   │   │                       # cleanup_service.rs, apps_service.rs, cloud_service.rs
│   │   ├── testing/            # fakes in-memory (feature "testing")
│   │   └── lib.rs
│   └── tests/                  # risk_benchmark.rs, service-level tests
├── cleaner-infra/              # adapters
│   ├── src/
│   │   ├── fs/                 # jwalk_scanner.rs, inspector.rs
│   │   ├── db/                 # connection.rs, migrations/V001__initial.sql, *_repository.rs
│   │   ├── ai/                 # factory.rs, anthropic.rs, openai.rs, ollama.rs, prompt.rs, schema.rs
│   │   ├── apps/               # bundle.rs, steam.rs, epic.rs, gog.rs, battlenet.rs, spotlight.rs
│   │   ├── cloud/              # detector.rs, icloud.rs, file_provider.rs, evict.rs
│   │   └── macos/              # trash.rs, keychain.rs, permissions.rs, finder.rs
│   └── tests/                  # integração: scanner_fs.rs, sqlite_repos.rs, ai_providers.rs, ...
src-tauri/
├── src/
│   ├── main.rs, lib.rs         # composition root (injeção de dependências)
│   ├── state.rs                # AppState com serviços
│   ├── error.rs                # AppError → IPC
│   ├── dto.rs                  # DTOs com #[derive(TS)]
│   └── commands/               # scan.rs, catalog.rs, assessment.rs, cleanup.rs, apps.rs, cloud.rs, settings.rs
├── capabilities/default.json
└── tauri.conf.json
src/                            # frontend Vue
├── main.ts, App.vue
├── router/
├── stores/                     # scan.ts, catalog.ts, assessment.ts, cleanup.ts, apps.ts, cloud.ts, settings.ts
├── services/ipc/               # wrappers tipados de invoke/Channel (Adapter)
├── views/                      # ScanView, FilesView, AppsView, CloudView, HistoryView, SettingsView
├── components/                 # FileTable, RiskBadge, SizeBar, CleanupReviewDialog, ScanProgress...
├── composables/                # useFormatBytes, usePagination, ...
├── i18n/pt-BR.json
├── types/generated/            # ts-rs (não editar)
└── **/__tests__/               # vitest
scripts/make-fixture-tree.sh
fixtures/risk-benchmark.json
.github/
├── workflows/ci.yml
├── workflows/release.yml
└── dependabot.yml
```

**Structure Decision**: workspace Cargo com três crates para isolar domínio (testável sem
Tauri) de adapters e da camada de composição; frontend Vue na raiz conforme convenção do Tauri.

## Implementation Phases (resumo para /speckit-tasks)

1. **Setup**: toolchain, scaffold Tauri+Vue, workspace, lint/format, CI esqueleto.
2. **Foundational**: tipos de domínio, ports, fakes, SQLite + migrações, `ProtectedPathPolicy`,
   `AppError`, ts-rs, layout base da UI.
3. **US1 (MVP)**: scanner jwalk, `ScanService`, repositórios, comandos de varredura/listagem,
   telas de varredura e lista.
4. **US2**: heurística, cadeia de classificação, provedores de IA + Factory, Keychain, tela de
   configurações, badges de risco.
5. **US3**: `ActionExecutor`, Lixeira, preview/execute, histórico.
6. **US4**: detectores de apps/jogos, agregação, tela de aplicações.
7. **US5**: spike de nuvem, detectores, estado, evict, tela de nuvem.
8. **Polish**: benchmark de risco, performance, release workflow, docs.

## Complexity Tracking

| Ponto de complexidade | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| 3 crates no workspace | Domínio testável sem Tauri/I/O (Princípio II/III) e didático | Crate único acopla domínio ao framework e dificulta fakes |
| Chain of Responsibility na classificação | Garante precedência das regras protegidas, cache e fallback sem `if`s espalhados | Um único classificador com condicionais mistura responsabilidades e é difícil de testar |
| `objc2-foundation` (FFI) para nuvem | Estado de sincronização e evict só existem via Foundation | `brctl` cobre apenas iCloud e é frágil para parse |
| `ts-rs` + checagem de diff no CI | Evita divergência de tipos entre Rust e TS | Tipos duplicados à mão quebram silenciosamente |
