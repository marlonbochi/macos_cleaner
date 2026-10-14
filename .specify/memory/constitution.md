<!--
Sync Impact Report
- Version change: 1.0.0 → 1.1.0
- Modified principles: none (wording translated to English, meaning unchanged)
- Modified sections:
  - Development Workflow & Quality Gates: one branch per stage, merged into `main` only after
    the owner's approval (no pull requests required for owner work).
- Added sections: Governance → "Repository language" rule (all repository content in English).
- Removed sections: none
- Templates requiring updates: none (plan/spec/tasks templates read the constitution at runtime)
- Deferred TODOs: none
-->

# macOS Cleaner Constitution

## Core Principles

### I. User Data Safety (NON-NEGOTIABLE)

- Scanning MUST be strictly read-only: no scanning, cataloguing or AI analysis operation may
  modify, move or delete files.
- Every deletion MUST be explicitly initiated by the user, item by item or as a reviewed batch,
  through a confirmation screen showing total size, item count and risk level.
- Deletion MUST move items to the macOS Trash by default. Permanent deletion may only exist as
  an explicit option, disabled by default, with an additional confirmation.
- Protected paths (system directories such as `/System`, `/usr`, `/bin`, `/sbin`,
  `/private/var/db`, Apple-signed bundles, the app itself and its database) MUST be blocked
  from deletion by a tested blocklist.
- Symbolic links MUST NOT be followed while scanning, to avoid double counting and actions
  outside the chosen scope.
- Every destructive action MUST be recorded in a local audit log (what, when, size, destination).

Rationale: the product handles irreplaceable data; a single false positive costs more than any
amount of reclaimed space.

### II. Clean, Domain-Oriented Architecture

- The Rust core MUST follow a hexagonal architecture (ports & adapters): the domain crate does
  not depend on Tauri, SQLite, the real filesystem or AI providers.
- External dependencies MUST be reached through traits (ports) with injectable implementations
  (adapters), so they can be replaced by fakes in tests.
- Design patterns MUST be applied where they solve a real problem, and documented in code when
  not obvious: Repository (persistence), Strategy (detectors and risk classifiers),
  Observer/Event (scan progress), Command (auditable delete/offload actions), Factory/Adapter
  (AI and cloud providers).
- Object orientation is applied through structs + traits in Rust and typed classes/composables
  in TypeScript; composition is preferred over inheritance.
- The Vue frontend MUST use strict TypeScript and the Composition API, and keep business logic
  out of components (stores/composables/services).
- Functions MUST have a single responsibility; `unwrap()`/`expect()` are forbidden outside tests
  and documented invariants; errors use dedicated types (`thiserror`).

Rationale: layer separation keeps the code testable, replaceable and instructive for learning
Rust.

### III. Test First (NON-NEGOTIABLE)

- TDD flow: tests are written and failing before the implementation (Red-Green-Refactor).
- Unit tests MUST cover the domain and use cases; minimum line coverage of 80% for the Rust core
  and 70% for the frontend, enforced in CI.
- Integration tests MUST exercise real adapters: filesystem in temporary directories
  (`tempfile`), real SQLite in a temporary file, and Tauri commands.
- AI and cloud providers MUST have contract tests with fakes or local mock servers; no CI test
  depends on the network or on credentials.
- Every safety rule of Principle I MUST have a dedicated test (blocklist, Trash, symlinks,
  confirmation).

Rationale: an app that deletes files needs executable guarantees, not good intentions.

### IV. AI Advises, Never Decides

- AI MUST only classify and explain (risk level + human-readable rationale); it never triggers
  deletions nor pre-selects high-risk items.
- By default only metadata (path, name, extension, size, dates, kind, bundle id) may be sent to
  an AI provider. Sending file contents requires an explicit opt-in.
- The app MUST work without AI (offline or without a key): a local heuristic classifier provides
  the baseline risk classification.
- AI responses MUST be validated against a schema; invalid responses are discarded and fall back
  to the heuristics.
- Classifications MUST be cached in SQLite to avoid cost and re-sending data.

Rationale: privacy and predictability; the final decision always belongs to a well-informed
user.

### V. Performance and Responsiveness

- Scanning MUST run off the UI thread, be cancellable, and emit throttled progress events to the
  interface.
- Scanning MUST be parallel and persist to SQLite in batches (transactions).
- Rescans MUST be incremental where possible (compare mtime/size).
- The UI MUST stay responsive with catalogues of millions of entries (pagination and ordered
  queries in the database; never load everything into frontend memory).

Rationale: modern disks hold millions of files; a frozen UI destroys trust.

### VI. Simplicity and Learning

- Prefer the fewest dependencies: local SQLite as the only persistence, no servers.
- YAGNI: no speculative features; extra complexity requires a justification in the plan's
  "Complexity Tracking" section.
- Rust code MUST be idiomatic and commented where it teaches something (ownership, lifetimes,
  concurrency), since the project is also a learning vehicle.

Rationale: simplicity reduces bugs and keeps the project understandable.

## Stack and Technical Constraints

- Desktop: Tauri v2; Rust stable core (edition 2021+), organised as a Cargo workspace.
- Frontend: Vue 3 + TypeScript (strict) + Vite; state with Pinia; tests with Vitest + Vue Test
  Utils.
- Persistence: SQLite (via `rusqlite`) with versioned migrations.
- Target platform: macOS 13+ (Apple Silicon and Intel). The app requests and explains the
  "Full Disk Access" permission when needed. Other platforms are a non-goal.
- Cloud integrations (iCloud Drive, OneDrive, Google Drive, Dropbox) MUST use official
  system/client mechanisms (e.g. File Provider, `brctl evict` for iCloud), never undocumented
  APIs that manipulate data directly.
- Secrets (AI API keys) MUST be stored in the macOS Keychain, never in SQLite nor in plain-text
  configuration files.

## Development Workflow and Quality Gates

- Spec-Driven Development with Spec Kit: each feature goes through specify → clarify (when
  needed) → plan → tasks → implement.
- Each stage lives on its own branch (`NNN-short-name`) created from `main`. Branches are pushed
  freely; they are merged into `main` only after the project owner approves the stage.
  Contributions from other people go through pull requests.
- Commits follow Conventional Commits.
- CI (GitHub Actions) MUST run on every push and pull request, and every gate MUST pass:
  - Rust: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, coverage
    (`cargo llvm-cov`) with a minimum threshold.
  - Frontend: lint (ESLint), type checking (`vue-tsc`), formatting (Prettier), `vitest` with
    coverage.
  - Build: `tauri build` on a macOS runner.
- CD: `v*` tags produce a GitHub release with the `.dmg` as an artifact.
- Code review MUST check compliance with this constitution.

## Governance

- This constitution supersedes any other project practice.
- Amendments require a dedicated change with a rationale, an updated Sync Impact Report and a
  version bump.
- Semantic versioning: MAJOR for removing/redefining principles, MINOR for new principles or
  sections, PATCH for clarifications.
- Justified violations MUST be recorded in the feature plan's "Complexity Tracking".
- Repository language: everything committed to the repository (code, comments, README,
  CLAUDE.md, specs, plans, tasks, this constitution and commit messages) MUST be written in
  English. User-facing UI strings live in `src/i18n/` and follow the app's locale (pt-BR first).
- Runtime development guidance lives in `CLAUDE.md` / `README.md`.

**Version**: 1.1.0 | **Ratified**: 2026-10-14 | **Last Amended**: 2026-10-14
