<p align="center"><img src="assets/icon/icon.png" alt="Disk Cleaner icon" width="128" /></p>

# Disk Cleaner

[![CI](https://github.com/marlonbochi/macos_cleaner/actions/workflows/ci.yml/badge.svg)](https://github.com/marlonbochi/macos_cleaner/actions/workflows/ci.yml)

A desktop app for **macOS, Windows and Linux (Debian)** that finds what is taking up disk space and
helps you decide, safely, what can go — with an AI assistant that explains each item and the right
way to clean it on your system.

- **Scan and rank** — read-only scan of the disk or chosen folders; files and folders listed from
  largest to smallest, with folder drill-down, filters and "Show in Finder / Explorer / file manager".
- **AI risk analysis** — each item gets a risk level (Safe / Caution / Dangerous / Unknown), what it
  is, why, and the best way to free the space, including official commands for each system, such
  as `xcrun simctl delete unavailable`, `Dism.exe /Online /Cleanup-Image /StartComponentCleanup` or
  `sudo apt clean` (shown and copyable, never executed by the app).
- **Bring your own AI** — Claude, OpenAI, Gemini or a local Ollama model, chosen from the models
  your account can actually use. Keys live in the system's secret store (macOS Keychain, Windows
  Credential Manager, Secret Service on Linux). Works offline with built-in rules for each system.
- **Batch analysis** — select many items and analyse them together, 20 per request, to save tokens.
- **AI diagnosis** — when a scan finishes, the 40 largest meaningful items are sent to the AI and
  listed in their own tab with what the AI thinks of each one and how much space is safe to free.
  AI features only appear once a provider is configured.
- **Clean uninstall** — Settings → Uninstall removes the app, its data and its stored keys after you
  type the app's name to confirm: to the Trash on macOS, through the official uninstaller on
  Windows, and with the `apt` command it shows you on Debian.
- **Safety first** — protected system paths are never deletion candidates, symlinks are never
  followed, only metadata (never file contents) is sent to the AI.

Planned: deletion to the Trash with review and history, applications & games by real total size,
and freeing space from files already synced to iCloud/OneDrive/Google Drive/Dropbox.

The project is also a Rust learning exercise: a hexagonal Rust core with a Vue 3 + TypeScript UI on
Tauri v2, built spec-first with [Spec Kit](https://github.com/github/spec-kit).

## Documentation

| Document                                                 | Contents                                                                                                                                                                                                           |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [Constitution](.specify/memory/constitution.md)          | Non-negotiable principles (safety, architecture, tests, AI)                                                                                                                                                        |
| [Specification](specs/001-disk-space-cleaner/spec.md)    | User stories, requirements and success criteria                                                                                                                                                                    |
| [Plan](specs/001-disk-space-cleaner/plan.md)             | Stack, structure and decisions ([research](specs/001-disk-space-cleaner/research.md))                                                                                                                              |
| [Data model](specs/001-disk-space-cleaner/data-model.md) | Entities and SQLite schema                                                                                                                                                                                         |
| [Contracts](specs/001-disk-space-cleaner/contracts/)     | IPC commands, core ports, AI contract                                                                                                                                                                              |
| [Tasks](specs/001-disk-space-cleaner/tasks.md)           | Backlog ordered by user story                                                                                                                                                                                      |
| Later stages                                             | [002 AI analysis](specs/002-ai-risk-analysis/spec.md), [003 batch analysis](specs/003-batch-ai-analysis/spec.md), [008 AI diagnosis](specs/008-ai-diagnosis/spec.md), [009 uninstall](specs/009-uninstall/spec.md) |

Development follows Spec Kit: `/speckit-specify` → `/speckit-plan` → `/speckit-tasks` →
`/speckit-implement`, one branch per stage.

## Architecture

```text
┌──────────────── Vue 3 (src/) ────────────────┐
│ views → stores (Pinia) → services/ipc (Api)  │
└──────────────────────┬───────────────────────┘
                       │ Tauri IPC (commands + Channel)
┌──────────── src-tauri (composition root) ────┐
│ commands → DTOs (ts-rs) → AppState           │
└──────────────────────┬───────────────────────┘
┌──────────── cleaner-core (no I/O) ───────────┐
│ domain · services (use cases) · analysis     │
│ ports (traits) ◀── implemented by ──┐        │
└─────────────────────────────────────│────────┘
┌──────────── cleaner-infra ──────────┴────────┐
│ fs · db (SQLite) · ai · secrets · system     │
└──────────────────────────────────────────────┘
```

Patterns: Repository (persistence), Strategy (scanner, local rules), Chain of Responsibility
(protected → cache → AI → rules), Observer (scan progress), Adapter + Factory (AI providers, IPC
transport), Facade (services and the frontend `Api`).

## Getting started

Requirements: [rustup](https://rustup.rs) (the toolchain is pinned in `rust-toolchain.toml`),
Node.js 22 and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) of your system:
Xcode Command Line Tools on macOS 13+, the WebView2 runtime and MSVC build tools on Windows 10/11,
and on Debian/Ubuntu the packages installed by `scripts/ci/linux-deps.sh`.

```bash
npm ci
```

```bash
npm run tauri dev
```

To build the installable package of your system (`.dmg` on macOS, NSIS `.exe` installer on Windows,
`.deb` on Debian/Ubuntu) under `target/release/bundle/`:

```bash
npm run tauri build
```

Builds are not signed by Apple or Microsoft yet, so the first launch shows a warning: **Open
Anyway** in macOS Privacy & Security, or **More info → Run anyway** in Windows SmartScreen.

### Full Disk Access (macOS)

Some folders (Mail, Safari, app containers) can only be read with Full Disk Access. The app tells
you which app macOS expects: the **Disk Cleaner** app when running the bundle, or the terminal/IDE
that launched `npm run tauri dev`. Grant it in System Settings → Privacy & Security → Full Disk
Access, then restart that app.

### AI providers

Open **Settings**, pick a provider, paste your API key (stored in the system's secret store), click **Load
models**, choose one and **Test connection**. With Ollama no key is needed — just a pulled model
(`ollama pull llama3.2`). Only metadata is sent: the path (with `~` instead of your home folder),
kind, sizes, dates and the names of the largest children of a folder.

## Quality gates (same as CI)

```bash
npm run check
```

```bash
npm run test:coverage
```

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo llvm-cov --workspace --ignore-filename-regex 'src-tauri/' --fail-under-lines 80
```

After changing DTOs in `src-tauri/src/dto.rs`, regenerate the TypeScript types:

```bash
npm run gen:types
```

A test tree for manual checks:

```bash
scripts/make-fixture-tree.sh /tmp/mc-fixture
```

## CI/CD

- `ci.yml` — on every push/PR: fmt, clippy and tests on macOS, Windows and Ubuntu (coverage ≥ 80%
  on macOS), ESLint, Prettier, vue-tsc, frontend tests (≥ 70%), generated-types drift check, and
  the `.dmg`, NSIS installer and `.deb` builds as downloadable artifacts.
- `release.yml` — on a `v*` tag: runs every CI gate, checks the tag matches the app version,
  drafts a GitHub Release with notes generated from the commits, builds a universal `.dmg`
  (macOS), an NSIS installer (Windows x64) and a `.deb` (Debian/Ubuntu amd64) on GitHub, attaches
  them and only then publishes the release.
- `audit.yml` — weekly dependency audit; Dependabot keeps dependencies current.

## Releases

Download the app from [Releases](https://github.com/marlonbochi/macos_cleaner/releases); every
version lists what changed ([CHANGELOG.md](CHANGELOG.md) has the full history). To cut a release
from an up-to-date `main`:

```bash
npm run release -- 0.2.0 --dry-run   # preview the release notes
npm run release -- 0.2.0             # bump versions, update CHANGELOG.md, commit and tag
git push origin main v0.2.0          # GitHub builds the app and publishes the release
```

Versions follow [SemVer](https://semver.org) and the notes come from
[Conventional Commits](https://www.conventionalcommits.org) through [git-cliff](https://git-cliff.org)
(`cliff.toml`).

## Uninstalling

Use **Settings → Uninstall Disk Cleaner**. If the app was already deleted, remove its leftovers by
hand (all named after the bundle identifier `com.marlonbochi.diskcleaner`):

- `~/Library/Application Support/com.marlonbochi.diskcleaner` (catalogue database)
- `~/Library/Caches/com.marlonbochi.diskcleaner` and `~/Library/WebKit/com.marlonbochi.diskcleaner`
- Keychain Access → search for `com.marlonbochi.diskcleaner` (API keys)

Versions up to 0.1.1 were called **macOS Cleaner** (`com.marlonbochi.macoscleaner`). Disk Cleaner
adopts their catalogue, settings and API keys on first launch, and its uninstaller also removes
anything the old version left behind, including the old app.

## License

MIT
