# Data Model: AI-Assisted Disk Space Analyzer and Cleaner

**Feature**: `001-disk-space-cleaner` | **Date**: 2026-10-14

Domain entities (structs in `cleaner-core::domain`) and their SQLite mapping
(`cleaner-infra::db`). Sizes in bytes (`u64` in Rust, `INTEGER` in SQLite, stored bit-for-bit so
values above `i64::MAX` round-trip); dates as Unix epoch seconds, UTC.

## Value Objects

| Type | Description | Rules |
|---|---|---|
| `ByteSize(u64)` | Size in bytes | Human formatting in base 1000, like Finder; saturating addition |
| `RiskLevel` | `Safe`, `Unknown`, `Caution`, `Dangerous` | Ordered by severity; `Caution`/`Dangerous` require confirmation and are never pre-selected |
| `RiskCategory` | `cache`, `build-artifact`, `installer`, `vm-image`, `media`, `archive`, `user-document`, `app-data`, `app-bundle`, `game`, `system`, `backup`, `log`, `other` | Unknown values degrade to `other` |
| `Recommendation` | `delete-safely`, `use-app-or-command`, `move-to-cloud-or-external`, `keep`, `review-manually` | — |
| `SuggestedCommand` | `{ command, description }` | Displayed and copyable only; never executed by the app |
| `AssessmentSource` | `ProtectedRule`, `Heuristic`, `Ai { provider, model }` | — |
| `FileKind` | `File`, `Directory`, `Package` | Packages (`.app`, `.photoslibrary`, ...) are treated as one unit |
| `FileFingerprint` | `(size_on_disk, mtime, inode)` | Equality validates cached assessments and pending actions |
| `AiProviderKind` | `anthropic`, `openai`, `gemini`, `ollama` | Each has a default model and official base URL |

## Entities

### Scan

| Field | Type | Rules |
|---|---|---|
| id | i64 PK | — |
| roots | JSON (`Vec<PathBuf>`) | ≥ 1 absolute root; nested/duplicate roots collapsed |
| min_file_size | u64 | default 100 MB |
| status | enum | see transitions |
| started_at / finished_at | i64 / i64? | `finished_at` only in terminal states |
| files_seen / bytes_seen | u64 | monotonic |
| inaccessible_count | u64 | — |
| error | text? | only when `Failed` |

**Transitions**:

```text
Running ──complete──▶ Completed
   │──cancel────────▶ Cancelled   (partial results visible, flagged incomplete)
   └──fatal error───▶ Failed
```

Only one `Scan` may be `Running` at a time; scans left `Running` by a crash are marked `Failed`
at startup. The UI shows the latest `Completed` or `Cancelled` scan. A completed scan replaces
older catalogues; a cancelled one does not.

### FileEntry

| Field | Type | Rules |
|---|---|---|
| id | i64 PK | — |
| scan_id | FK → Scan (cascade) | — |
| path | text, UNIQUE per scan | absolute, UTF-8 (APFS) |
| parent_path | text | index for folder browsing |
| name / extension | text | extension lower-cased; none for directories |
| kind | `FileKind` | — |
| size_on_disk | u64 | `st_blocks * 512`; aggregated for directories |
| apparent_size | u64 | `len()`; aggregated for directories |
| file_count | u64 | 1 for files; total below a directory |
| created_at / modified_at / accessed_at | i64? | — |
| inode / device | u64 | hard-link de-duplication |
| is_hardlink_dup | bool | reserved; duplicates are currently not persisted |
| is_protected | bool | result of `ProtectedPathPolicy` |

Indexes: `(scan_id, size_on_disk DESC)`, `(scan_id, parent_path)`, `(scan_id, extension)`.

Persisted: files ≥ `min_file_size` and directories whose total ≥ `min_file_size`.

### RiskAssessment

| Field | Type | Rules |
|---|---|---|
| id | i64 PK | — |
| path | text | logical key together with the fingerprint |
| size_on_disk / modified_at / inode | fingerprint | if the item changes, the assessment no longer applies |
| level | `RiskLevel` | protected items are always `Dangerous` |
| category | `RiskCategory` | — |
| explanation | text | ≤ 900 characters |
| details | JSON | `{ summary, recommendation, steps[], commands[] }` (V002) |
| source | JSON `AssessmentSource` | — |
| evaluator_version | text | `heuristic-vN:<locale>` or `<provider>:<model>:prompt-vN:<locale>`; changes force re-assessment |
| created_at | i64 | — |

Unique index: `(path, size_on_disk, COALESCE(modified_at, -1), inode, evaluator_version)` — acts
as the cache (FR-015). Badges show the newest assessment matching the current fingerprint, from
any evaluator.

### InstalledApp (planned, US4)

| Field | Type | Rules |
|---|---|---|
| id | i64 PK | — |
| bundle_id | text? | absent for games without a bundle |
| name | text | — |
| kind | `App`, `Game`, `System` | `System` ⇒ not removable (FR-027) |
| store | `None`, `AppStore`, `Steam`, `Epic`, `Gog`, `BattleNet` | — |
| last_used_at | i64? | Spotlight, when available |
| total_size | u64 | sum of components |
| detected_at | i64 | — |

### AppComponent (planned, US4)

| Field | Type | Rules |
|---|---|---|
| id | i64 PK | — |
| app_id | FK → InstalledApp | — |
| role | `Bundle`, `InstallDir`, `AppSupport`, `Cache`, `Container`, `GroupContainer`, `Preferences`, `Logs`, `SavedState`, `Other` | — |
| path | text | — |
| size_on_disk | u64 | — |
| match_confidence | `Exact`, `Heuristic` | `Heuristic` shows a warning and is never pre-selected |

### CloudLocation (planned, US5)

| Field | Type | Rules |
|---|---|---|
| id | i64 PK | — |
| provider | `ICloudDrive`, `OneDrive`, `GoogleDrive`, `Dropbox`, `OtherFileProvider` | — |
| root_path | text UNIQUE | — |
| detected_at | i64 | — |

### CloudFileState (planned, US5 — derived, queried on demand)

| Field | Type | Rules |
|---|---|---|
| file_entry_id | FK → FileEntry | — |
| status | `Downloaded`, `CloudOnly`, `UploadPending`, `Error` | only `Downloaded` + uploaded is eligible (FR-030) |
| is_uploaded | bool | — |
| error | text? | — |

### ActionRecord (audit log — append-only)

| Field | Type | Rules |
|---|---|---|
| id | i64 PK | — |
| batch_id | text (UUID) | groups one user confirmation |
| action | `MoveToTrash`, `EvictToCloud` (`DeletePermanently` reserved) | — |
| path | text | — |
| size_on_disk | u64 | size at action time |
| risk_level | `RiskLevel` | level shown at confirmation |
| status | `Succeeded`, `Skipped`, `Failed` | — |
| reason | text? | e.g. `changed-since-scan`, `protected`, `missing`, `risk-not-confirmed`, `not-uploaded` |
| executed_at | i64 | — |

Never updated nor deleted — enforced by SQLite triggers (Principle I).

### Settings (key/value, JSON values)

| Key | Value | Default |
|---|---|---|
| `ai` | `{ provider, model, base_url }` (`provider: null` = local rules only) | `{ provider: null }` |
| `ui.locale` | `"en-US"` / `"pt-BR"` | unset: the UI follows the system language (Portuguese → pt-BR, anything else → en-US) |

API keys are **not** stored here: they live in the Keychain (`SecretStore`), account
`ai.<provider>.api_key`, service = the app's bundle identifier.

## Relationships

```text
Scan 1──* FileEntry
FileEntry (path + fingerprint) 1──0..* RiskAssessment (one per evaluator version)
InstalledApp 1──* AppComponent                      (planned)
FileEntry 1──0..1 CloudFileState                     (planned)
ActionRecord (independent; references a path, not a FK — survives catalogue replacement)
```

## Migrations

`crates/cleaner-infra/src/db/migrations/`:

- `V001__initial.sql` — scans, file entries, risk assessments, append-only action records,
  settings.
- `V002__assessment_details.sql` — `details` JSON column on `risk_assessments`.

Every schema change is a new migration registered in `db/mod.rs`; migrations are never edited
after being merged.
