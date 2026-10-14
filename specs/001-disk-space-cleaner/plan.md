# Implementation Plan: AI-Assisted Disk Space Analyzer and Cleaner

**Branch**: `001-disk-space-cleaner` | **Date**: 2026-10-14 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-disk-space-cleaner/spec.md`

## Summary

A macOS desktop app that scans the disk (read-only), catalogues the largest files, folders,
applications/games and downloaded cloud files in SQLite, classifies deletion risk through a
classifier chain (protected rules → cache → cloud or local AI → heuristics) and runs auditable
actions (Trash, "keep only in the cloud") only after the user's review. Hexagonal Rust
architecture (workspace with `cleaner-core`, `cleaner-infra`, `src-tauri`) and a Vue 3 +
TypeScript frontend. Decisions are detailed in [research.md](./research.md).

## Technical Context

**Language/Version**: Rust stable (edition 2024, toolchain pinned in `rust-toolchain.toml`);
TypeScript (strict); Node 22 LTS

**Primary Dependencies**: Tauri 2.x (+ `tauri-plugin-dialog`), `jwalk`, `rusqlite` (bundled) +
`rusqlite_migration`, `serde`/`serde_json`, `thiserror`, `tracing`, `reqwest` (blocking, rustls),
`security-framework` (Keychain), `ts-rs`; planned for later stories: `trash`, `plist`,
`objc2-foundation`. Frontend: Vue 3, Pinia, Vue Router, vue-i18n, `@tauri-apps/api` v2

**Storage**: SQLite (WAL) at `~/Library/Application Support/<bundle-id>/catalog.sqlite3`;
secrets in the Keychain

**Testing**: `cargo test`, `proptest`, `tempfile`, `mockito` (local mock HTTP server for AI
providers), `cargo-llvm-cov`; Vitest + Vue Test Utils + `@tauri-apps/api/mocks`

**Target Platform**: macOS 13+ (aarch64 and x86_64), distributed outside the App Store (.dmg)

**Project Type**: desktop app (Tauri: Rust core + Vue webview)

**Performance Goals**: 1M-file scan < 5 min on SSD; UI responds < 200 ms during scans; paginated
lists ≤ 500 items per request

**Constraints**: read-only scanning; no file contents sent to AI without opt-in; offline
operation (heuristics); process memory < 500 MB while scanning 1M items

**Scale/Scope**: single local user; catalogues of up to ~5M entries seen (only entries above the
threshold persisted); ~8 screens (Scan, Largest items, Folders, Applications & Games, Cloud,
Review, History, Settings)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How the plan complies | Status |
|---|---|---|
| I. User data safety | Scanner performs no writes; `ProtectedPathPolicy` + `ActionExecutor` with fingerprint re-validation; Trash by default; no permanent deletion in v1; append-only `ActionRecord`; symlinks not followed | ✅ |
| II. Clean architecture | Hexagonal, I/O-free `cleaner-core`; ports in [core-ports.md](./contracts/core-ports.md); Repository, Strategy, Chain of Responsibility, Observer, Command, Factory/Adapter; `thiserror`; clippy denies `unwrap` outside tests | ✅ |
| III. Test first | Fakes for every port; integration tests on real FS/SQLite; mock HTTP server for AI; 80%/70% coverage gates in CI; dedicated tests for each Principle I rule | ✅ |
| IV. AI advises | Metadata only, home shown as `~`; schema-validated answers; heuristic fallback; cache keyed by fingerprint + evaluator version; AI never pre-selects | ✅ |
| V. Performance | Parallel `jwalk`, `spawn_blocking`, cancellation, throttled `Channel`, batched inserts, database pagination | ✅ |
| VI. Simplicity | Single SQLite file, no server; `rusqlite` instead of an ORM; no UI kit; E2E and FSEvents deferred | ✅ |
| Stack & constraints | Tauri v2, Vue 3 + TS, SQLite, macOS 13+, official cloud mechanisms, Keychain | ✅ |
| Quality gates | CI with fmt, clippy, tests, coverage, ESLint, Prettier, vue-tsc, Vitest, dmg build; tag-based release | ✅ |

**Post-design re-check (Phase 1)**: ✅ no violations. See Complexity Tracking for the points that
add structure.

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
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml                      # workspace
rust-toolchain.toml
crates/
├── cleaner-core/               # domain + use cases + ports (no I/O, no Tauri)
│   └── src/
│       ├── domain/             # scan, file_entry, risk, ai, action, size, package
│       ├── ports/              # filesystem, repository, ai (provider, factory, secrets), Clock
│       ├── policy/             # protected_paths
│       ├── analysis/           # heuristics, prompt (+ batch), verdict
│       ├── services/           # scan, catalog, analysis, ai_settings (later: cleanup, apps, cloud)
│       └── testing/            # in-memory fakes ("testing" feature)
├── cleaner-infra/              # adapters
│   ├── src/
│   │   ├── fs/                 # jwalk_scanner, inspector, metadata
│   │   ├── db/                 # migrations/V00N__*.sql, *_repository.rs
│   │   ├── ai/                 # factory, http, providers (Claude/OpenAI/Gemini/Ollama), models
│   │   └── macos/              # Full Disk Access, Finder, Keychain
│   └── tests/                  # integration: scanner, SQLite, AI providers, model listing
src-tauri/
├── src/
│   ├── lib.rs, main.rs         # app setup
│   ├── state.rs                # composition root (dependency injection by hand)
│   ├── error.rs                # AppError → IPC
│   ├── dto.rs                  # DTOs with #[derive(TS)]
│   └── commands/               # scan, catalog, analysis, system
├── capabilities/default.json
└── tauri.conf.json
src/                            # Vue frontend
├── services/ipc/               # Transport port (Tauri / fake) + typed Api facade
├── stores/                     # scan, catalog, analysis, aiSettings
├── views/                      # ScanView, FilesView, SettingsView
├── components/                 # FileTable, AnalysisPanel, RiskBadge, ScanProgress, ...
├── composables/                # formatting helpers
├── i18n/pt-BR.ts               # UI strings
├── types/generated/            # ts-rs output (do not edit)
└── test/                       # fakes, factories, mount helper
scripts/make-fixture-tree.sh
.github/workflows/              # ci.yml, release.yml, audit.yml
```

**Structure Decision**: a three-crate Cargo workspace isolates the domain (testable without
Tauri) from adapters and from the composition layer; the Vue frontend sits at the repository root
following the Tauri convention.

## Implementation Phases (summary for /speckit-tasks)

1. **Setup**: toolchain, Tauri + Vue scaffold, workspace, lint/format, CI skeleton.
2. **Foundational**: domain types, ports, fakes, SQLite + migrations, `ProtectedPathPolicy`,
   `AppError`, ts-rs, base UI layout.
3. **US1 (MVP)**: jwalk scanner, `ScanService`, repositories, scan/listing commands, scan and
   list screens.
4. **US2**: heuristics, classifier chain, AI providers + factory, Keychain, settings screen, risk
   badges (delivered in 002 and 003).
5. **US3**: `ActionExecutor`, Trash, preview/execute, history.
6. **US4**: application/game detectors, aggregation, applications screen.
7. **US5**: cloud spike, detectors, sync state, evict, cloud screen.
8. **Polish**: risk benchmark, performance, release workflow, docs.

## Complexity Tracking

| Complexity point | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| 3 crates in the workspace | Domain testable without Tauri/I/O (Principles II/III) and instructive | A single crate couples the domain to the framework and makes fakes harder |
| Chain of Responsibility for classification | Guarantees precedence of protected rules, cache and fallback without scattered `if`s | One classifier with conditionals mixes responsibilities and is hard to test |
| `objc2-foundation` (FFI) for cloud (planned) | Sync state and evict only exist through Foundation | `brctl` covers only iCloud and is brittle to parse |
| `ts-rs` + diff check in CI | Prevents type drift between Rust and TypeScript | Hand-duplicated types break silently |
