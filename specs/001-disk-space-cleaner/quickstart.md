# Quickstart: validating feature 001-disk-space-cleaner

## Prerequisites

- macOS 13+, Xcode Command Line Tools (`xcode-select --install`)
- Rust stable through rustup (the repository pins the toolchain in `rust-toolchain.toml`)
- Node.js 22 LTS + npm
- Optional: Ollama (`ollama pull llama3.2`) or an API key for a cloud AI provider

## Setup

```bash
npm ci
cargo build --workspace
```

## Run the app in development

```bash
npm run tauri dev
```

## Run the quality gates locally (same as CI)

```bash
npm run check          # eslint + prettier --check + vue-tsc
npm run test:coverage  # vitest with coverage
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo llvm-cov --workspace --ignore-filename-regex 'src-tauri/' --fail-under-lines 80
```

## Manual validation scenarios

Create a test tree with `scripts/make-fixture-tree.sh /tmp/mc-fixture` (500 MB / 200 MB / 50 MB
files with real data, 150 small files, a hard link, a symlink, an `.app` bundle and a folder
without permissions).

| # | Story | Steps | Expected result |
|---|---|---|---|
| 1 | US1 | Scan `/tmp/mc-fixture` with a 100 MB threshold | List: 500 MB, 200 MB and the `game` folder (~150 MB); no 50 MB file; locked folder listed as inaccessible; hard link counted once |
| 2 | US1 | Start a scan of `~` and cancel it | Stops within 2 s; partial results flagged "incomplete" |
| 3 | US1 | Close and reopen the app | Last catalogue shown with its date |
| 4 | US2 | AI = none; analyse the 20 largest items | All have a level + explanation, source "local rules" |
| 5 | US2 | AI = Ollama/cloud; analyse | Source "AI"; analysing again sends no new request (cache) |
| 6 | US2 | Analyse an item under `/System` | Always "Dangerous", never sent to the AI |
| 7 | US2 | Select 25 items and "Analyse selected with AI" | 2 AI requests (20 + 5); every row gets a badge |
| 8 | US3 | Select 3 fixture files → Delete | Review with totals; after confirming, items in the Trash and in the History |
| 9 | US3 | Change a file after the scan and try to delete it | Item skipped with reason "changed since scan" |
| 10 | US4 | Open "Applications & Games" | Apps sorted by total size with breakdown; system apps not removable |
| 11 | US5 | With iCloud Drive enabled, open "Cloud" | Downloaded files listed by size; "keep only in the cloud" frees local space |
| 12 | US5 | File with a pending upload | Action unavailable, state "not synced" |

### Per cloud provider validation (spike R12)

For each installed provider (iCloud, OneDrive, Google Drive, Dropbox), record in
`docs/cloud-provider-matrix.md`: detection ok?, state ok?, evict ok?, notes.

## Risk benchmark (SC-006, pending)

```bash
cargo test -p cleaner-core --test risk_benchmark
```

Fails if any item labelled essential is classified `Safe` or if accuracy is below 85%.
