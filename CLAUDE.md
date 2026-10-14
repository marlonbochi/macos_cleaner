# CLAUDE.md

Guidance for AI agents working in this repository.

## Read first

- `.specify/memory/constitution.md` — non-negotiable rules. Above all: scanning is read-only,
  deletion only after user review and only to the Trash, protected paths are blocked,
  tests come first.
- `specs/001-disk-space-cleaner/` — spec, plan, data model, contracts and `tasks.md` (mark tasks
  `[X]` when done).

## Layout

- `crates/cleaner-core` — domain, use cases (`services/`), ports (traits). No I/O, no Tauri.
  Fakes of every port live in `src/testing` behind the `testing` feature.
- `crates/cleaner-infra` — adapters: `fs` (jwalk scanner), `db` (SQLite + migrations), `macos`.
  Integration tests in `tests/` use real temp dirs and SQLite files.
- `src-tauri` — composition root (`state.rs`), IPC commands, DTOs exported to TS with ts-rs.
- `src/` — Vue 3 + TS. Components never call `invoke` directly: views → Pinia stores →
  `services/ipc` (`Api` over a `Transport`). Tests inject `FakeTransport` via `provideApi`.

## Commands

- `npm run tauri dev` — run the app
- `npm run check && npm test` — frontend gates
- `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` — Rust gates
- `npm run gen:types` — regenerate `src/types/generated` after changing `src-tauri/src/dto.rs`
- `npm run release -- x.y.z` — bump versions, regenerate `CHANGELOG.md`, commit and tag (from
  `main`, only when the user asks for a release); pushing the tag publishes it on GitHub

## Conventions

- Rust: `unwrap`/`expect`/`panic` are denied outside tests; errors use `thiserror`.
  Integration test files start with `#![allow(clippy::unwrap_used, clippy::expect_used)]`.
- Never edit an applied SQL migration; add `V00N__*.sql` and register it in `db/mod.rs`.
- UI strings live in `src/i18n/` (`en-US.ts` is the reference schema; keep `pt-BR.ts` in sync).
- Conventional Commits: they become the release notes (`cliff.toml`), so write them for users.
