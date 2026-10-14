---
description: "Task list for 001-disk-space-cleaner"
---

# Tasks: AI-Assisted Disk Space Analyzer and Cleaner

**Input**: Design documents from `/specs/001-disk-space-cleaner/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: REQUIRED (Constitution, Principle III — TDD). In each story, test tasks come before the
implementation and must fail before they are satisfied.

**Organization**: tasks grouped by user story for incremental delivery.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependencies)
- **[Story]**: US1..US5

## Path Conventions

- Rust core: `crates/cleaner-core/`, adapters: `crates/cleaner-infra/`, app: `src-tauri/`
- Frontend: `src/` (tests next to the code as `*.test.ts`)

---

## Phase 1: Setup (Shared Infrastructure)

- [X] T001 Install the Rust toolchain and create `rust-toolchain.toml` (stable + rustfmt, clippy, llvm-tools-preview)
- [X] T002 Scaffold Tauri v2 + Vue 3 + TS (Vite) at the root: `package.json`, `vite.config.ts`, `index.html`, `src/`, `src-tauri/`
- [X] T003 Create the Cargo workspace in `Cargo.toml` with members `crates/cleaner-core`, `crates/cleaner-infra`, `src-tauri`
- [X] T004 [P] Configure workspace Rust lints (`[workspace.lints]`: `clippy::unwrap_used`, `expect_used`, `panic`, ...) and `rustfmt.toml`
- [X] T005 [P] Configure ESLint (flat config, `eslint-plugin-vue`, `typescript-eslint`), Prettier and strict `tsconfig`
- [X] T006 [P] Configure Vitest + Vue Test Utils + v8 coverage (70% threshold) in `vite.config.ts`
- [X] T007 [P] Create `.github/workflows/ci.yml` (rust, frontend, types, build jobs) and `.github/dependabot.yml`
- [X] T008 [P] Create `README.md` and `CLAUDE.md` with development commands and a reference to the constitution

---

## Phase 2: Foundational (Blocking Prerequisites)

**⚠️ CRITICAL**: no user story starts before this phase.

### Tests

- [X] T009 [P] Tests for `ByteSize` (base-1000 formatting) and `RiskLevel` (ordering) in `crates/cleaner-core/src/domain/size.rs` and `risk.rs`
- [X] T010 [P] Tests (including `proptest`) for `ProtectedPathPolicy` in `crates/cleaner-core/src/policy/protected_paths.rs`
- [X] T011 [P] SQLite migration tests (apply migrations on a temporary database) in `crates/cleaner-infra/src/db/mod.rs` and `tests/sqlite_catalog.rs`

### Implementation

- [X] T012 [P] Domain value objects and entities in `crates/cleaner-core/src/domain/` (scan, file_entry, risk, action, size, package)
- [X] T013 [P] Ports (traits) per `contracts/core-ports.md` in `crates/cleaner-core/src/ports/`
- [X] T014 Domain errors with `thiserror` in `crates/cleaner-core/src/error.rs`
- [X] T015 `ProtectedPathPolicy` in `crates/cleaner-core/src/policy/protected_paths.rs`
- [X] T016 In-memory port fakes (`testing` feature) in `crates/cleaner-core/src/testing/`
- [X] T017 SQLite connection (WAL, pragmas) + `V001__initial.sql` + migration runner in `crates/cleaner-infra/src/db/`
- [X] T018 `AppError` + IPC conversions in `src-tauri/src/error.rs`; `tracing` set up in `src-tauri/src/lib.rs`
- [X] T019 Composition root and `AppState` in `src-tauri/src/state.rs`
- [X] T020 [P] `ts-rs` type generation into `src/types/generated/` + `npm run gen:types` + diff check in CI
- [X] T021 [P] Base UI layout: router, side navigation, light/dark theme, pt-BR `vue-i18n` in `src/`
- [X] T022 [P] `src/services/ipc/` layer (`Transport` port + typed `Api` facade over `invoke` and `Channel`) with tests

**Checkpoint**: foundation ready; `cargo test`, `npm test` and CI green.

---

## Phase 3: User Story 1 - Scan and rank the largest files (P1) 🎯 MVP

**Goal**: scan folders, catalogue and list files/folders by size, with progress and cancellation.

**Independent Test**: scan a fixture with known sizes and check order, filters and totals.

### Tests for User Story 1 ⚠️

- [X] T023 [P] [US1] Fixture script `scripts/make-fixture-tree.sh` (real data files, hard link, symlink, locked folder)
- [X] T024 [P] [US1] `JwalkScanner` integration tests (size on disk, hard link reported, symlink ignored, inaccessible reported, exclusions, cancellation) in `crates/cleaner-infra/tests/scanner_fs.rs`
- [X] T025 [P] [US1] `Scan`/`FileEntry` repository integration tests (batch insert, ordered pagination, filters) in `crates/cleaner-infra/tests/sqlite_catalog.rs`
- [X] T026 [P] [US1] `ScanService` unit tests with fakes (folder aggregation, threshold, state transitions, recovery) in `crates/cleaner-core/src/services/scan_service.rs`
- [X] T027 [P] [US1] Tests for the `scan`/`catalog` stores and the `ScanProgress`/`FileTable` components

### Implementation for User Story 1

- [X] T028 [US1] `JwalkScanner` + `StdFileInspector` in `crates/cleaner-infra/src/fs/`
- [X] T029 [US1] `SqliteScanRepository` and `SqliteFileEntryRepository` in `crates/cleaner-infra/src/db/`
- [X] T030 [US1] `ScanService` (bottom-up aggregation, batches, cancellation) and `CatalogService` in `crates/cleaner-core/src/services/`
- [X] T031 [US1] Commands `start_scan`, `cancel_scan`, `get_latest_scan`, `list_files`, `list_folder_children`, `reveal_in_finder` in `src-tauri/src/commands/`
- [X] T032 [US1] Permissions: `get_permissions_status` (including the app macOS expects), `open_full_disk_access_settings` in `crates/cleaner-infra/src/macos/` and `src-tauri/src/commands/system.rs`
- [X] T033 [US1] `ScanView` (roots, progress, cancel, Full Disk Access guidance) and `FilesView` (paginated list, filters, folder drill-down) in `src/views/`
- [X] T034 [US1] Pinia stores `scan` and `catalog` in `src/stores/`

**Checkpoint**: functional MVP — scan and ranking end to end.

---

## Phase 4: User Story 2 - AI-assisted risk assessment (P2)

> Delivered on branches `002-ai-risk-analysis` and `003-batch-ai-analysis` — see
> [specs/002](../002-ai-risk-analysis/tasks.md) and [specs/003](../003-batch-ai-analysis/tasks.md).
> Pending: T035 (labelled benchmark).

**Goal**: risk level + explanation for each item, with cloud/local AI or local rules.

**Independent Test**: sample catalogue assessed; system items never "Safe"; cache reused.

### Tests for User Story 2 ⚠️

- [ ] T035 [P] [US2] Labelled benchmark `fixtures/risk-benchmark.json` (200 items) + test `crates/cleaner-core/tests/risk_benchmark.rs`
- [X] T036 [P] [US2] Classification chain tests (protected > cache > AI > rules; fallback on error/invalid JSON; safety floor) in `crates/cleaner-core/src/services/analysis_service.rs`
- [X] T037 [P] [US2] Contract tests for the Claude/OpenAI/Gemini/Ollama adapters against a mock server (metadata only, `~` for home, retries, schema) in `crates/cleaner-infra/tests/ai_providers.rs`
- [X] T038 [P] [US2] Assessment repository tests (cache by fingerprint + version) in `crates/cleaner-infra/tests/sqlite_assessments.rs`
- [X] T039 [P] [US2] Tests for `RiskBadge`, `AnalysisPanel`, the `analysis`/`aiSettings` stores and `SettingsView`

### Implementation for User Story 2

- [X] T040 [P] [US2] Local rules (`HeuristicAnalyzer`) in `crates/cleaner-core/src/analysis/heuristics.rs`
- [X] T041 [P] [US2] Prompt, schema and response validation in `crates/cleaner-core/src/analysis/prompt.rs` and `verdict.rs`
- [X] T042 [US2] `AnalysisService` and `AiSettingsService` in `crates/cleaner-core/src/services/`
- [X] T043 [P] [US2] AI adapters + `HttpAiProviderFactory` in `crates/cleaner-infra/src/ai/`
- [X] T044 [P] [US2] `KeychainSecretStore` in `crates/cleaner-infra/src/macos/keychain.rs` and `SqliteSettingsRepository`/`SqliteAssessmentRepository`
- [X] T045 [US2] Commands `analyze_file`, `analyze_files`, `get_analyses`, `get_ai_settings`, `save_ai_settings`, `list_ai_models`, `test_ai_connection` in `src-tauri/src/commands/analysis.rs`
- [X] T046 [US2] UI: `RiskBadge`, analysis panel with copyable commands, multi-select batch analysis, `SettingsView` (provider, key, model list, URL)

**Checkpoint**: US1 + US2 independent and functional.

---

## Phase 5: User Story 3 - Safe, informed deletion (P3)

**Goal**: review → confirmation → Trash → history.

**Independent Test**: delete fixture files; check the Trash, the catalogue and the history.

### Tests for User Story 3 ⚠️

- [ ] T047 [P] [US3] `ActionExecutor` tests (protected skipped, changed fingerprint skipped, unconfirmed risky skipped, audit always recorded) in `crates/cleaner-core/src/services/cleanup_service.rs`
- [ ] T048 [P] [US3] Integration tests for the real `TrashPort` and the append-only `ActionLogRepository` in `crates/cleaner-infra/tests/cleanup.rs`
- [ ] T049 [P] [US3] Tests for `CleanupReviewDialog` (totals, risk highlighting, individual confirmation) and `HistoryView`

### Implementation for User Story 3

- [ ] T050 [US3] `MoveToTrash` command + `ActionExecutor` + `CleanupService` (preview with a 10-minute token, execute) in `crates/cleaner-core/src/services/cleanup_service.rs`
- [ ] T051 [P] [US3] `MacTrash` (`trash` crate) and `SqliteActionLogRepository` in `crates/cleaner-infra/`
- [ ] T052 [US3] Commands `preview_cleanup`, `execute_cleanup`, `list_action_history` in `src-tauri/src/commands/cleanup.rs`
- [ ] T053 [US3] UI: "Delete selected" on the existing selection bar (no risky pre-selection), `CleanupReviewDialog`, `HistoryView`, `cleanup` store

**Checkpoint**: full cycle scan → assess → delete.

---

## Phase 6: User Story 4 - Applications and games (P4)

**Goal**: apps/games with total size (bundle + data) and breakdown.

**Independent Test**: fixture apps with known sizes; correct totals and breakdown.

### Tests for User Story 4 ⚠️

- [ ] T054 [P] [US4] Detector tests with fixtures (`Info.plist`, `appmanifest_*.acf`, Epic manifests) in `crates/cleaner-infra/tests/app_detectors.rs`
- [ ] T055 [P] [US4] `AppsService` tests (aggregation, system not removable, heuristic matches not pre-selected)
- [ ] T056 [P] [US4] Tests for `AppsView` and the `apps` store

### Implementation for User Story 4

- [ ] T057 [P] [US4] `BundleAppDetector` + associated data by bundle id in `crates/cleaner-infra/src/apps/bundle.rs`
- [ ] T058 [P] [US4] `SteamDetector` (VDF parser), `EpicDetector`, `GogDetector`, `BattleNetDetector` in `crates/cleaner-infra/src/apps/`
- [ ] T059 [P] [US4] Last use through Spotlight in `crates/cleaner-infra/src/apps/spotlight.rs`
- [ ] T060 [US4] `AppsService` + `SqliteAppRepository` and commands `list_installed_apps`, `get_app_components`, `preview_app_removal`
- [ ] T061 [US4] `AppsView` UI (list, expandable breakdown, removal through the US3 flow)

---

## Phase 7: User Story 5 - Free space from cloud files (P5)

**Goal**: list downloaded files in cloud folders and "keep only in the cloud".

**Independent Test**: synced files in a cloud folder; evicting frees local space.

- [ ] T062 [US5] Spike: validate `evictUbiquitousItemAtURL` and state keys per provider; record in `docs/cloud-provider-matrix.md`
- [ ] T063 [P] [US5] `CloudService` tests with fakes (only `Downloaded` + uploaded eligible, reclaimable total per provider)
- [ ] T064 [P] [US5] Cloud folder detection tests with a temporary tree mimicking `~/Library/CloudStorage` and `Mobile Documents`
- [ ] T065 [US5] `CloudLocationDetector`, `FoundationCloudState`, `FoundationEvict` (objc2-foundation, `brctl` fallback) in `crates/cleaner-infra/src/cloud/`
- [ ] T066 [US5] `CloudService` + `EvictToCloud` command + IPC commands `list_cloud_locations`, `list_cloud_files`, `preview_evict`, `execute_evict`
- [ ] T067 [US5] `CloudView` UI (per provider, sync state, action unavailable when not synced)

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T068 [P] Scan performance benchmark (1M synthetic files) and batch/throttle tuning — SC-001
- [X] T069 [P] `.github/workflows/release.yml` with `tauri-action` (universal dmg) and a weekly audit job (`audit.yml`)
- [X] T070 [P] App icon (source `assets/icon/icon.svg`, generated with `tauri icon`) and bundle metadata; `Info.plist` usage descriptions still pending
- [ ] T071 Run the `quickstart.md` script and record the results
- [ ] T072 [P] Architecture documentation (hexagonal diagram and patterns) in `docs/architecture.md`
- [X] T073 [P] Translate all documentation and specs to English (repository language rule) — branch `004-english-docs`

---

## Dependencies & Execution Order

- Setup (1) → Foundational (2) → US1 (3) → {US2, US3, US4, US5}
- US3 depends on the catalogue (US1); it uses assessments (US2) when present but works with
  `Unknown`.
- US4 and US5 reuse the US3 review/execution flow for destructive actions.
- Polish after the desired stories.

### Parallel Opportunities

- Phase 1: T004–T008 in parallel after T002/T003.
- Phase 2: T009–T011 (tests) in parallel; T012/T013 in parallel; T020–T022 in parallel.
- US2: T035–T039 in parallel; T040, T041, T043, T044 in parallel.
- US4 and US5 can be developed in parallel after US3.

## Implementation Strategy

1. **MVP**: phases 1–3 (scan + ranking). Validate with quickstart scenarios 1–3.
2. **+US2**: risk with heuristics first, then AI.
3. **+US3**: safe deletion — the first version that actually frees space.
4. **+US4, +US5**: specialised views.
5. Small commits per task/group, CI green on every push.
