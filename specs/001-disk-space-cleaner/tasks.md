---
description: "Task list for 001-disk-space-cleaner"
---

# Tasks: Analisador e Limpador de Espaço em Disco com IA

**Input**: Design documents from `/specs/001-disk-space-cleaner/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: OBRIGATÓRIOS (Constituição, Princípio III — TDD). Em cada história, as tarefas de teste
vêm antes da implementação e devem falhar antes de serem satisfeitas.

**Organization**: tarefas agrupadas por história para entrega incremental.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência)
- **[Story]**: US1..US5

## Path Conventions

- Rust core: `crates/cleaner-core/`, adapters: `crates/cleaner-infra/`, app: `src-tauri/`
- Frontend: `src/` (testes em `src/**/__tests__/`)

---

## Phase 1: Setup (Shared Infrastructure)

- [X] T001 Instalar toolchain Rust e criar `rust-toolchain.toml` (stable + rustfmt, clippy, llvm-tools-preview)
- [X] T002 Scaffold Tauri v2 + Vue 3 + TS (Vite) na raiz: `package.json`, `vite.config.ts`, `index.html`, `src/`, `src-tauri/`
- [X] T003 Criar workspace Cargo em `Cargo.toml` com membros `crates/cleaner-core`, `crates/cleaner-infra`, `src-tauri`
- [X] T004 [P] Configurar lints Rust no workspace (`[workspace.lints]`: `clippy::unwrap_used`, `expect_used`, `pedantic` seletivo) e `rustfmt.toml`
- [X] T005 [P] Configurar ESLint (flat config, `eslint-plugin-vue`, `typescript-eslint`), Prettier e `tsconfig` strict
- [X] T006 [P] Configurar Vitest + Vue Test Utils + cobertura v8 (limite 70%) em `vitest.config.ts`
- [X] T007 [P] Criar `.github/workflows/ci.yml` (jobs rust, frontend, build) e `.github/dependabot.yml`
- [X] T008 [P] Criar `README.md` e `CLAUDE.md` com comandos de desenvolvimento e referência à constituição

---

## Phase 2: Foundational (Blocking Prerequisites)

**⚠️ CRITICAL**: nenhuma história começa antes desta fase.

### Tests

- [X] T009 [P] Testes de `ByteSize` (formatação base 1000) e `RiskLevel` (ordenação) em `crates/cleaner-core/src/domain/size.rs` e `risk.rs`
- [X] T010 [P] Testes (incl. `proptest`) da `ProtectedPathPolicy` em `crates/cleaner-core/src/policy/protected_paths.rs`
- [X] T011 [P] Testes de integração de migrações SQLite (aplica V001 em banco temporário) em `crates/cleaner-infra/tests/db_migrations.rs`

### Implementation

- [X] T012 [P] Value objects e entidades de domínio em `crates/cleaner-core/src/domain/` (scan, file_entry, risk, app, cloud, action, size)
- [X] T013 [P] Ports (traits) conforme `contracts/core-ports.md` em `crates/cleaner-core/src/ports/`
- [X] T014 Erros de domínio com `thiserror` em `crates/cleaner-core/src/error.rs`
- [X] T015 `ProtectedPathPolicy` em `crates/cleaner-core/src/policy/protected_paths.rs`
- [X] T016 Fakes in-memory dos ports (feature `testing`) em `crates/cleaner-core/src/testing/`
- [X] T017 Conexão SQLite (WAL, pragmas) + `V001__initial.sql` + runner de migrações em `crates/cleaner-infra/src/db/`
- [X] T018 `AppError` + conversões para IPC em `src-tauri/src/error.rs`; `tracing` configurado em `src-tauri/src/lib.rs`
- [X] T019 Composition root e `AppState` em `src-tauri/src/lib.rs` e `src-tauri/src/state.rs`
- [X] T020 [P] Geração de tipos `ts-rs` para `src/types/generated/` + script `npm run gen:types` + checagem de diff no CI
- [X] T021 [P] Layout base da UI: router, navegação lateral, tema claro/escuro, `vue-i18n` pt-BR em `src/`
- [X] T022 [P] Camada `src/services/ipc/` (wrapper tipado de `invoke` e `Channel`) com testes usando `mockIPC`

**Checkpoint**: fundação pronta; `cargo test`, `npm test` e CI verdes.

---

## Phase 3: User Story 1 - Varredura e ranking dos maiores arquivos (P1) 🎯 MVP

**Goal**: varrer pastas, catalogar e listar arquivos/pastas por tamanho, com progresso e cancelamento.

**Independent Test**: varrer fixture com tamanhos conhecidos e verificar ordem, filtros e totais.

### Tests for User Story 1 ⚠️

- [X] T023 [P] [US1] Script de fixture `scripts/make-fixture-tree.sh` (arquivos esparsos, hard link, symlink, pasta sem permissão)
- [X] T024 [P] [US1] Teste de integração do `JwalkScanner` (tamanho em disco, hard link uma vez, symlink ignorado, inacessível reportado, não cruza volume, cancelamento) em `crates/cleaner-infra/tests/scanner_fs.rs`
- [X] T025 [P] [US1] Teste de integração dos repositórios `Scan`/`FileEntry` (batch insert, paginação ordenada, filtros) em `crates/cleaner-infra/tests/sqlite_catalog.rs`
- [X] T026 [P] [US1] Testes unitários do `ScanService` com fakes (agregação de pastas, limite mínimo, transições de estado, incremental) em `crates/cleaner-core/src/services/scan_service.rs`
- [X] T027 [P] [US1] Testes dos stores `scan`/`catalog` e componentes `ScanProgress`/`FileTable` com `mockIPC` em `src/**/__tests__/`

### Implementation for User Story 1

- [X] T028 [US1] `JwalkScanner` + `FileInspector` em `crates/cleaner-infra/src/fs/`
- [X] T029 [US1] `SqliteScanRepository` e `SqliteFileEntryRepository` em `crates/cleaner-infra/src/db/`
- [X] T030 [US1] `ScanService` (agregação bottom-up, batches, cancelamento, incremental) e `CatalogService` em `crates/cleaner-core/src/services/`
- [X] T031 [US1] Comandos `start_scan`, `cancel_scan`, `get_latest_scan`, `list_files`, `list_folder_children`, `reveal_in_finder` em `src-tauri/src/commands/scan.rs` e `catalog.rs`
- [X] T032 [US1] Permissões: `get_permissions_status`, `open_full_disk_access_settings` em `crates/cleaner-infra/src/macos/permissions.rs` e `src-tauri/src/commands/settings.rs`
- [X] T033 [US1] Telas `ScanView` (escolha de raízes, progresso, cancelar, aviso de Acesso Total) e `FilesView` (lista virtualizada, filtros, navegação por pasta) em `src/views/`
- [X] T034 [US1] Stores Pinia `scan` e `catalog` em `src/stores/`

**Checkpoint**: MVP funcional — varredura e ranking ponta a ponta.

---

## Phase 4: User Story 2 - Avaliação de risco assistida por IA (P2)

> Implementada na branch `002-ai-risk-analysis` — ver [specs/002-ai-risk-analysis](../002-ai-risk-analysis/tasks.md). Pendente: T035 (benchmark) e análise em lote (`assess_top`).

**Goal**: nível de risco + explicação para cada item, com IA em nuvem/local ou regras locais.

**Independent Test**: catálogo de exemplo avaliado; itens de sistema nunca "Seguro"; cache reaproveitado.

### Tests for User Story 2 ⚠️

- [ ] T035 [P] [US2] Benchmark rotulado `fixtures/risk-benchmark.json` (200 itens) + teste `crates/cleaner-core/tests/risk_benchmark.rs`
- [X] T036 [P] [US2] Testes da `ClassifierChain` (precedência protegido > cache > IA > heurística; fallback em erro/JSON inválido) em `crates/cleaner-core/src/classify/chain.rs`
- [X] T037 [P] [US2] Testes de contrato dos provedores Anthropic/OpenAI/Ollama com `wiremock` (payload só metadados, `~` no home, retries, schema) em `crates/cleaner-infra/tests/ai_providers.rs`
- [X] T038 [P] [US2] Teste do repositório de avaliações (cache por fingerprint + versão) em `crates/cleaner-infra/tests/sqlite_assessments.rs`
- [X] T039 [P] [US2] Testes de `RiskBadge`, store `assessment` e `SettingsView` em `src/**/__tests__/`

### Implementation for User Story 2

- [X] T040 [P] [US2] `HeuristicClassifier` (regras por caminho/extensão/categoria) em `crates/cleaner-core/src/policy/heuristics.rs` e `classify/heuristic.rs`
- [X] T041 [P] [US2] `ProtectedPathClassifier`, `CachedClassifier`, `AiClassifier`, `ClassifierChain` em `crates/cleaner-core/src/classify/`
- [X] T042 [US2] `AssessmentService` em `crates/cleaner-core/src/services/assessment_service.rs`
- [X] T043 [P] [US2] Adapters de IA + `AiProviderFactory` + prompt versionado + schema em `crates/cleaner-infra/src/ai/`
- [X] T044 [P] [US2] `KeychainSecretStore` em `crates/cleaner-infra/src/macos/keychain.rs` e `SqliteSettingsRepository`/`SqliteAssessmentRepository`
- [X] T045 [US2] Comandos `assess_files`, `assess_top`, `get_ai_settings`, `save_ai_settings`, `test_ai_connection` em `src-tauri/src/commands/assessment.rs` e `settings.rs`
- [X] T046 [US2] UI: `RiskBadge` com explicação/origem, filtro por risco, botão "Avaliar maiores", `SettingsView` (provedor, modelo, chave, compartilhar conteúdo)

**Checkpoint**: US1 + US2 independentes e funcionais.

---

## Phase 5: User Story 3 - Exclusão segura e consciente (P3)

**Goal**: revisão → confirmação → Lixeira → histórico.

**Independent Test**: excluir arquivos de fixture; verificar Lixeira, catálogo e histórico.

### Tests for User Story 3 ⚠️

- [ ] T047 [P] [US3] Testes do `ActionExecutor` (protegido pulado, fingerprint divergente pulado, arriscado sem confirmação pulado, auditoria sempre registrada) em `crates/cleaner-core/src/services/cleanup_service.rs`
- [ ] T048 [P] [US3] Teste de integração do `TrashPort` real e do `ActionLogRepository` append-only em `crates/cleaner-infra/tests/cleanup.rs`
- [ ] T049 [P] [US3] Testes do `CleanupReviewDialog` (total, destaque de risco, confirmação individual) e `HistoryView`

### Implementation for User Story 3

- [ ] T050 [US3] Commands `MoveToTrash` e `ActionExecutor` + `CleanupService` (preview com token de 10 min, execute) em `crates/cleaner-core/src/services/cleanup_service.rs`
- [ ] T051 [P] [US3] `MacTrash` (crate `trash`) e `SqliteActionLogRepository` em `crates/cleaner-infra/`
- [ ] T052 [US3] Comandos `preview_cleanup`, `execute_cleanup`, `list_action_history` em `src-tauri/src/commands/cleanup.rs`
- [ ] T053 [US3] UI: seleção na `FilesView` (sem pré-seleção de risco), `CleanupReviewDialog`, `HistoryView`, store `cleanup`

**Checkpoint**: ciclo completo varrer → avaliar → excluir.

---

## Phase 6: User Story 4 - Aplicações e jogos (P4)

**Goal**: apps/jogos com tamanho total (pacote + dados) e composição.

**Independent Test**: apps de fixture com tamanhos conhecidos; totais e composição corretos.

### Tests for User Story 4 ⚠️

- [ ] T054 [P] [US4] Testes dos detectores com fixtures (`Info.plist`, `appmanifest_*.acf`, manifests Epic) em `crates/cleaner-infra/tests/app_detectors.rs`
- [ ] T055 [P] [US4] Testes do `AppsService` (agregação, sistema não removível, confiança heurística não pré-selecionada)
- [ ] T056 [P] [US4] Testes da `AppsView` e store `apps`

### Implementation for User Story 4

- [ ] T057 [P] [US4] `BundleAppDetector` + resolução de dados associados por bundle id em `crates/cleaner-infra/src/apps/bundle.rs`
- [ ] T058 [P] [US4] `SteamDetector` (parser VDF), `EpicDetector`, `GogDetector`, `BattleNetDetector` em `crates/cleaner-infra/src/apps/`
- [ ] T059 [P] [US4] Último uso via Spotlight em `crates/cleaner-infra/src/apps/spotlight.rs`
- [ ] T060 [US4] `AppsService` + `SqliteAppRepository` e comandos `list_installed_apps`, `get_app_components`, `preview_app_removal`
- [ ] T061 [US4] UI `AppsView` (lista, composição expansível, remoção via fluxo US3)

---

## Phase 7: User Story 5 - Liberar espaço de arquivos da nuvem (P5)

**Goal**: listar arquivos baixados em pastas de nuvem e "manter somente na nuvem".

**Independent Test**: arquivos sincronizados na pasta de nuvem; evict libera espaço local.

- [ ] T062 [US5] Spike: validar `evictUbiquitousItemAtURL` e chaves de estado por provedor; registrar em `docs/cloud-provider-matrix.md`
- [ ] T063 [P] [US5] Testes do `CloudService` com fakes (elegibilidade só `Downloaded`+uploaded, total liberável por provedor)
- [ ] T064 [P] [US5] Testes de detecção de pastas de nuvem com árvore temporária simulando `~/Library/CloudStorage` e `Mobile Documents`
- [ ] T065 [US5] `CloudLocationDetector`, `FoundationCloudState`, `FoundationEvict` (objc2-foundation, fallback `brctl`) em `crates/cleaner-infra/src/cloud/`
- [ ] T066 [US5] `CloudService` + command `EvictToCloud` + comandos `list_cloud_locations`, `list_cloud_files`, `preview_evict`, `execute_evict`
- [ ] T067 [US5] UI `CloudView` (por provedor, estado de sincronização, ação indisponível para não sincronizados)

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T068 [P] Benchmark de desempenho da varredura (1 M arquivos sintéticos) e ajuste de batch/throttle — SC-001/SC-003
- [ ] T069 [P] `.github/workflows/release.yml` com `tauri-action` (dmg universal) e job semanal `cargo deny`
- [ ] T070 [P] Ícone, metadados do bundle, `Info.plist` com descrições de uso
- [ ] T071 Rodar roteiro do `quickstart.md` e registrar resultados
- [ ] T072 [P] Documentação de arquitetura (diagrama hexagonal e padrões) em `docs/architecture.md`

---

## Dependencies & Execution Order

- Setup (1) → Foundational (2) → US1 (3) → {US2, US3, US4, US5}
- US3 depende do catálogo (US1); usa avaliações (US2) quando existirem, mas funciona com
  `Unknown`.
- US4 e US5 reutilizam o fluxo de revisão/execução de US3 para ações destrutivas.
- Polish após as histórias desejadas.

### Parallel Opportunities

- Fase 1: T004–T008 em paralelo após T002/T003.
- Fase 2: T009–T011 (testes) em paralelo; T012/T013 em paralelo; T020–T022 em paralelo.
- US2: T035–T039 em paralelo; T040, T041, T043, T044 em paralelo.
- US4 e US5 podem ser desenvolvidas em paralelo após US3.

## Implementation Strategy

1. **MVP**: Fases 1–3 (varredura + ranking). Validar com quickstart cenários 1–3.
2. **+US2**: risco com heurística primeiro, depois IA.
3. **+US3**: exclusão segura — primeira versão que realmente libera espaço.
4. **+US4, +US5**: visões especializadas.
5. Commits pequenos por tarefa/grupo, CI verde a cada push.
