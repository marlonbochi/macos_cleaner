# Research: AI-Assisted Disk Space Analyzer and Cleaner

**Feature**: `001-disk-space-cleaner` | **Date**: 2026-10-14

Each section follows the Decision / Rationale / Alternatives considered format.

## R1. Desktop shell and code organisation

- **Decision**: Tauri v2 with a three-crate Cargo workspace: `cleaner-core` (domain + use cases +
  ports, no I/O), `cleaner-infra` (adapters: filesystem, SQLite, AI, macOS) and `src-tauri`
  (composition, IPC commands). Vue 3 + TypeScript + Vite frontend at the root (`src/`), following
  the Tauri template.
- **Rationale**: satisfies Principle II (hexagonal). `cleaner-core` builds and tests without Tauri,
  which speeds up TDD and lets Rust be learnt in isolation from the UI.
- **Alternatives considered**: a single crate inside `src-tauri` (simpler, but couples the domain to
  the framework); Electron (heavier, and defeats the goal of learning Rust).

## R2. Filesystem scanning

- **Decision**: `jwalk` (rayon-based parallel walker) with:
  - `follow_links(false)`; never cross devices (compare `st_dev` with the root's).
  - Size on disk = `st_blocks * 512` (`MetadataExt::blocks`); apparent size = `len()`.
  - Hard-link de-duplication by `(st_dev, st_ino)` when `nlink > 1`.
  - When scanning `/`, skip `/System/Volumes` (firmlinks would duplicate the data volume),
    `/Volumes`, `/dev`, `/private/var/vm` and `/cores` unless explicitly chosen.
  - Folder totals aggregated bottom-up in memory during the walk and persisted only for folders
    above the threshold.
  - Files below the threshold are not persisted individually (they only count towards their
    folders), keeping the database small.
- **Rationale**: `jwalk` is the fastest parallel walker for large trees in Rust and allows
  per-directory pruning; `st_blocks` reflects the space actually reclaimable (sparse files, APFS
  compression).
- **Alternatives considered**: `walkdir` (sequential, slower); `ignore::WalkParallel` (geared to
  `.gitignore`); `getattrlistbulk` via FFI (faster but complex — a future optimisation behind the
  same `FileSystemScanner` port).
- **APFS clones**: there is no cheap API to detect shared blocks; the UI shows the warning described
  in the edge cases. Recorded as a known limitation.

## R3. Incremental rescans

- **Decision** (revised during implementation): each scan writes only entries above the threshold
  (a few thousand rows, cheap) and replaces the previous catalogue when it completes. The expensive
  reprocessing — risk assessment — is incremental: assessments are cached by
  `(path, fingerprint, evaluator version)` and reused while the item is unchanged. Cancelled scans
  never replace the last complete catalogue.
- **Rationale**: meets FR-007/SC-003 where it matters most without keeping two catalogues in sync;
  the walk itself is dominated by metadata I/O and has no reliable shortcut on APFS.
- **Alternatives considered**: diffing fingerprints and writing only the delta (complexity without
  measurable gain); FSEvents (`notify` crate) with a persisted journal — a future evolution.

## R4. Persistence

- **Decision**: `rusqlite` (`bundled` feature) + `rusqlite_migration` with embedded, versioned SQL
  migrations. WAL mode, `synchronous=NORMAL`, batched inserts of 5,000 rows per transaction. One
  writer connection and one reader connection, each behind a `Mutex`. Database at
  `~/Library/Application Support/<bundle-id>/catalog.sqlite3`.
- **Rationale**: a simple synchronous API (good for learning), no extra runtime; WAL lets the UI
  read while a scan writes.
- **Alternatives considered**: `sqlx` (async, compile-time checked macros, but needs
  `DATABASE_URL`/offline mode and adds complexity); `diesel` (a heavy ORM for this case).

## R5. Rust ↔ Vue communication

- **Decision**: Tauri commands (`#[tauri::command]`) for request/response and
  `tauri::ipc::Channel` for the scan progress stream (100 ms throttle). Shared types generated with
  `ts-rs` into `src/types/generated/`, checked in CI (empty diff).
- **Rationale**: `Channel` is the recommended Tauri v2 mechanism for ordered, high-volume streams;
  `ts-rs` is stable and prevents type drift between layers.
- **Alternatives considered**: global events (`emit`) — no ordering guarantee and more coupling;
  `tauri-specta` — also generates command wrappers, but its v2 line was still a release candidate.

## R6. Concurrency and cancellation

- **Decision**: the scan runs in `tauri::async_runtime::spawn_blocking`; cancellation through an
  `Arc<AtomicBool>` (`CancellationToken` in the core) checked by the walker for every entry.
  Long AI calls also run in `spawn_blocking` from `async` commands.
- **Rationale**: the walk is CPU/I/O bound and synchronous; async brings no benefit here.
- **Alternatives considered**: `tokio-util::CancellationToken` — would pull tokio into the core,
  which must stay runtime-free.

## R7. Deleting to the Trash (planned, US3)

- **Decision**: the `trash` crate (`NSFileManager trashItemAtURL` on macOS) behind a `TrashPort`.
  Before each item: re-validation of `(exists, size, mtime, inode)` and a `ProtectedPathPolicy`
  check.
- **Rationale**: the native Trash keeps Finder's "Put Back" (SC-005).
- **Alternatives considered**: `osascript` with Finder (slow, needs automation permission); moving
  to `~/.Trash` by hand (loses restore metadata).

## R8. Protected locations and Full Disk Access

- **Decision**: `ProtectedPathPolicy` in the core with protected trees (`/System`, `/usr` except
  `/usr/local`, `/bin`, `/sbin`, `/etc`, `/var`, `/private/var/db`, `/Library/Apple`,
  `/Applications/Safari.app`, the app's own data directory) and exact-path protection for top-level
  user folders (`~`, `~/Documents`, `~/Library`, cloud roots, ...). Relative paths and paths with
  `..` are treated as protected (fail safe). Full Disk Access is detected by trying to open the
  user's TCC database; the settings pane opens through
  `x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles`. The app also detects
  which app macOS credits with the permission (the enclosing `.app` bundle, or the terminal/IDE
  that launched an unbundled dev binary via `__CFBundleIdentifier` + Spotlight).
- **Rationale**: deterministic, testable rules (Principle I, SC-004); telling users exactly which
  app to enable removes the most common setup confusion.
- **Alternatives considered**: relying on the AI to block deletions (violates Principle IV).

## R9. AI providers (cloud + local)

- **Decision**: an `AiProvider` port (structured JSON completion) with adapters built by an
  `AiProviderFactory` from the user's settings:
  - **Anthropic (Claude)**: Messages API with structured outputs (`output_config.format` JSON
    schema); default model `claude-opus-5-5`; server-side refusal fallback
    (`fallbacks: "default"`) on models that support it.
  - **OpenAI**: Responses API (`POST /v1/responses`) with `text.format` = strict JSON schema;
    default model `gpt-6-astra` (OpenAI's current recommendation; `gpt-5-*` snapshots are being
    retired). *Revised 2026-10-14.*
  - **Gemini**: `generateContent` (still fully supported) in JSON mode, with the schema in the
    instructions and client-side validation; default model `gemini-3.8-flash` (2.5 models are only
    served to accounts that already used them). The Interactions API is recommended for new
    projects — a candidate for a future migration. *Revised 2026-10-14.*
  - **Ollama (local)**: `POST {base}/api/chat` with `format` = JSON schema.
  - HTTP through `reqwest` (blocking, rustls); 180 s timeout for analyses, 20 s for model listing;
    2 retries with exponential backoff on 429/5xx/529.
  - Answers validated against the schema (serde + enum checks); invalid or missing items fall
    back to local rules.
  - Models are chosen from each provider's "list models" endpoint (text models only), so users
    cannot type an invalid id. Base URLs default to the official endpoints; custom URLs are
    validated and must use HTTPS when a key is sent, unless the server is local.
  - Batching: items that still need AI are sent in requests of up to 20 items (compact JSON, ids
    echoed back), so the system prompt and schema are sent once per batch.
- **Classification pipeline** (Chain of Responsibility + Strategy):
  `protected rule → cached assessment → AI (if configured) → local heuristics`, with the heuristics
  also acting as a safety floor (an item they know is essential never drops below "Caution").
- **Rationale**: satisfies clarification Q1 (both cloud and local), Principle IV (fallback, schema,
  cache) and allows switching providers without touching the domain.
- **Alternatives considered**: cloud-only or local-only (rejected by the clarification); official
  SDKs per provider — there is no official Rust SDK for all of them; plain HTTP is simple and easy
  to test against a local mock server (`mockito`).

## R10. Secrets

- **Decision**: `security-framework` (macOS generic passwords) behind the `SecretStore` port,
  service name = the app's bundle identifier.
- **Rationale**: the macOS Keychain, as required by the constitution; a small, macOS-only
  dependency.
- **Alternatives considered**: `keyring` (cross-platform; v4 pulls Linux/Windows store crates that
  this macOS-only app does not need); a custom encrypted file (reinvents the Keychain).

## R11. Installed applications and games (planned, US4)

- **Decision**: an `AppDetector` Strategy with implementations:
  - `BundleAppDetector`: `/Applications`, `~/Applications`, `/System/Applications` (marked system);
    reads `Info.plist` (`plist` crate): `CFBundleIdentifier`, name, `LSApplicationCategoryType`
    (`*-games` category → game).
  - Associated data by bundle id under `~/Library/{Application Support, Caches, Containers,
    Group Containers, Preferences, Logs, Saved Application State, HTTPStorages, WebKit}` (exact
    bundle-id match and, with lower confidence, app-name match — flagged in the UI).
  - `SteamDetector`: `~/Library/Application Support/Steam/steamapps/appmanifest_*.acf` (VDF) →
    `installdir` under `steamapps/common`.
  - `EpicDetector`: `~/Library/Application Support/Epic/EpicGamesLauncher/Data/Manifests/*.item`
    (JSON, `InstallLocation`).
  - `GogDetector`, `BattleNetDetector`: default install folders; directory-based detection.
  - Last use: `kMDItemLastUsedDate` through `mdls -raw` (Spotlight), optional.
- **Rationale**: each store has its own format; Strategy allows adding stores without changing the
  service.
- **Alternatives considered**: `system_profiler SPApplicationsDataType` (slow, no associated data,
  no store games).

## R12. Cloud: detection, sync state and "keep only in the cloud" (planned, US5)

- **Decision**:
  - Detection: iCloud Drive at `~/Library/Mobile Documents/com~apple~CloudDocs`; File Provider
    clients (OneDrive, Google Drive, Dropbox) at `~/Library/CloudStorage/<Provider>-*`.
  - State: resource keys `NSURLUbiquitousItemDownloadingStatusKey`,
    `NSURLUbiquitousItemIsUploadedKey`, `NSURLUbiquitousItemIsUploadingKey`,
    `NSURLUbiquitousItemUploadingErrorKey` through `objc2-foundation`; cloud-only files also
    detected through the `SF_DATALESS` flag in `st_flags`.
  - Evict: `NSFileManager evictUbiquitousItemAtURL:error:` (iCloud and File Provider on macOS 13+).
    Fallback: `brctl evict` (iCloud); if a provider fails, the UI points to Finder's
    "Remove Download".
  - Only files with state `Current` + `IsUploaded = true` are eligible (FR-030).
- **Rationale**: official system mechanisms (constitution); scope limited to existing cloud folders
  (clarification Q2).
- **Risk**: evict behaviour for third-party File Providers varies → a **technical spike** as the
  first US5 task, with a manual per-provider test documented in the quickstart.
- **Alternatives considered**: each provider's REST API (needs OAuth, out of scope);
  `fileproviderctl` (developer tool, not guaranteed on user machines).

## R13. Frontend

- **Decision**: Vue 3 (Composition API, `<script setup>`), strict TypeScript, Pinia, Vue Router,
  database-side pagination (100 items per page) instead of list virtualisation, `vue-i18n` (pt-BR
  first), no heavy UI kit (own CSS with variables + dark mode). A `Transport` port (Tauri or fake)
  under a typed `Api` facade wraps `invoke` and `Channel`; stores consume the API; components only
  present.
- **Rationale**: business logic outside components (Principle II) and minimal dependencies (VI).
- **Alternatives considered**: Naive UI / Vuetify (faster to build screens, but heavier and harder
  to test); Nuxt (unnecessary on desktop).

## R14. Testing strategy

- **Decision**:
  - Rust unit: `cargo test` on `cleaner-core` with in-memory fakes of every port; `proptest` for
    `ProtectedPathPolicy`.
  - Rust integration: `cleaner-infra/tests/` with `tempfile` (real trees, hard links, symlinks,
    denied permissions), real SQLite in a temporary file, `mockito` for AI providers and model
    listing; the real Keychain test is `#[ignore]` and run on demand.
  - Frontend: Vitest + Vue Test Utils with a `FakeTransport` injected through `provideApi`, and
    `@tauri-apps/api/mocks` for the Tauri transport itself.
  - A labelled reference set of 200 paths (`fixtures/risk-benchmark.json`) for SC-006 (pending).
  - Coverage: `cargo llvm-cov --fail-under-lines 80` (core + infra), Vitest v8 with a 70% threshold.
- **E2E**: Tauri's WebDriver (`tauri-driver`) does not support macOS; automated E2E is out of v1.
  Acceptance scenarios are covered by integration tests plus the manual quickstart script.
- **Alternatives considered**: Playwright against the dev server with mocked IPC — a possible
  evolution for UI smoke tests.

## R15. CI/CD

- **Decision**: GitHub Actions:
  - `ci.yml` (push and PR): `rust` job (macos-latest: fmt, clippy `-D warnings`, tests, llvm-cov),
    `frontend` job (ubuntu-latest: `npm ci`, ESLint, Prettier check, vue-tsc, Vitest with
    coverage), `types` job (regenerates ts-rs bindings and fails on diff), `build` job
    (macos-latest, unsigned `tauri build --bundles dmg`, artifact attached to the run). Caching with
    `Swatinem/rust-cache` and npm cache.
  - `release.yml` (`v*` tags): `tauri-apps/tauri-action` builds a draft release with a universal
    `.dmg` (aarch64 + x86_64).
  - Dependabot for cargo, npm and actions; weekly `cargo audit` + `npm audit`.
- **Rationale**: satisfies the constitution's quality gates.
- **Alternatives considered**: Apple signing and notarisation in v1 — needs a paid developer
  account; prepared through optional secrets and documented as a future step.

## R16. Package manager and toolchain

- **Decision**: npm (Node 22 LTS); Rust stable through `rustup` with `rust-toolchain.toml` pinning
  the `stable` channel and the `rustfmt`, `clippy` and `llvm-tools-preview` components.
- **Rationale**: fewer tools to install; reproducible toolchain in CI.
- **Alternatives considered**: pnpm/bun (no meaningful gain for this project).
