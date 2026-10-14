# Feature Specification: AI-Assisted Disk Space Analyzer and Cleaner

**Feature Branch**: `001-disk-space-cleaner`

**Created**: 2026-10-14

**Status**: In progress (US1 delivered; US2 delivered in [002](../002-ai-risk-analysis/spec.md) and [003](../003-batch-ai-analysis/spec.md))

**Input**: User description (translated from Portuguese): "A desktop app that scans the computer,
finding and cataloguing the heaviest files as deletion candidates, using AI to assess how
important each file is so the user knows the risks before deleting. It lists files in descending
size order and the user decides what to delete. It also finds installed games and applications
whose total size (many small files) takes a lot of space. It checks files in cloud folders
(iCloud, OneDrive and the like) that still take local space and could live only in the cloud,
leaving just a placeholder on the machine."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Scan and rank the largest files (Priority: P1)

The user opens the app, chooses what to scan (the whole disk or specific folders) and starts the
scan. While it runs they see progress (items analysed, total volume found) and can cancel. At the
end they see the files ordered from largest to smallest, with name, location, size, kind and last
access/modification date. They can filter by minimum size, kind and folder, and reveal the file in
Finder. The catalogue is saved: when the app reopens the last result is still available.

**Why this priority**: it is the core of the product and delivers value on its own ("where is my
space?") without AI, deletion or cloud features.

**Independent Test**: scan a test folder with files of known sizes and check that the list shows
every file above the threshold, in descending order, with correct sizes.

**Acceptance Scenarios**:

1. **Given** a folder with 5 GB, 2 GB and 50 MB files and a 100 MB threshold, **When** the user
   scans it, **Then** the list shows the 5 GB file then the 2 GB file, and not the 50 MB one.
2. **Given** a running scan, **When** the user cancels, **Then** the scan stops within 2 seconds
   and partial results stay visible, flagged as incomplete.
3. **Given** a previously completed scan, **When** the user reopens the app, **Then** the last
   catalogue is shown with its scan date, without scanning again.
4. **Given** a folder without read permission, **When** the scan reaches it, **Then** it is
   skipped, recorded as "inaccessible", and the scan continues.
5. **Given** the app lacks Full Disk Access, **When** the user scans the whole disk, **Then** the
   app explains the permission, which app must be granted it, and lets the user continue with
   partial access.

---

### User Story 2 - AI-assisted risk assessment (Priority: P2)

For each item the user sees a deletion risk level (Safe, Caution, Dangerous, Unknown) and a short
explanation of what the item probably is and why it has that risk (e.g. "Xcode cache, can be
recreated — Safe"; "Photos library database — Dangerous"). Assessment happens on demand or in
batches. Without AI the app still shows a basic classification from local rules, labelled as such.

**Why this priority**: it is the product's differentiator — it turns a list of sizes into an
informed decision and reduces accidental deletions.

**Independent Test**: with a sample catalogue containing caches, personal documents and known
system files, request the assessment and check that every item gets a level and an explanation,
and that known system items are never "Safe".

**Acceptance Scenarios**:

1. **Given** a catalogued item, **When** the user requests an assessment, **Then** the item shows
   a risk level, a plain-language explanation and the assessment source (AI or local rules).
2. **Given** an item already assessed and unchanged, **When** the user sees it again, **Then** the
   previous assessment is reused without a new request.
3. **Given** AI unavailable or not configured, **When** the user requests an assessment, **Then**
   local rules are used and the user is told so.
4. **Given** an item in a protected system location, **When** any assessment runs, **Then** the
   risk is always "Dangerous" and the app cannot delete it.
5. **Given** the default configuration, **When** an AI assessment runs, **Then** only file
   metadata (name, path, extension, size, dates, kind) is shared, never contents.

---

### User Story 3 - Safe, informed deletion (Priority: P3)

The user selects one or more items and asks to delete them. The app shows a review screen with
the space to be freed and the item count, highlighting "Caution" or "Dangerous" items, which
require an extra confirmation. Items go to the Trash, where they can be restored. A history shows
everything the app removed.

**Why this priority**: it actually frees space; it depends on stories 1 and 2 to be safe and
informed.

**Independent Test**: select files from a test folder, confirm the deletion and check that they
are in the Trash, the catalogue was updated and the history records the action.

**Acceptance Scenarios**:

1. **Given** 3 selected items totalling 4 GB, **When** the user asks to delete, **Then** they see a
   review with "3 items — 4 GB" before confirming.
2. **Given** a confirmed review, **When** deletion finishes, **Then** the items are in the Trash,
   leave the list, and the freed space is shown.
3. **Given** a "Dangerous" item in the selection, **When** the user reviews, **Then** the item is
   highlighted and only included after a confirmation specific to it.
4. **Given** an item that changed or disappeared since the scan, **When** deletion runs, **Then**
   it is skipped and the user is told.
5. **Given** past deletions, **When** the user opens the history, **Then** they see date, path,
   size and result of each item.

---

### User Story 4 - Applications and games that take the most space (Priority: P4)

The user opens "Applications & Games" and sees each installed application or game with its real
total size — the app bundle plus associated data (support files, caches, containers, game store
libraries such as Steam or Epic) — largest first, with last-used date when available. They can
expand an item to see the size breakdown and get the AI risk/importance assessment (e.g. "not
used for 14 months").

**Why this priority**: it covers the case where thousands of small files never show up in the
per-file ranking but together take tens of GB.

**Independent Test**: with test applications of known bundle and data sizes, check that the
aggregated total and the breakdown match.

**Acceptance Scenarios**:

1. **Given** an app with a 1 GB bundle and 3 GB of associated data, **When** the view opens,
   **Then** the app shows 4 GB and the breakdown shows 1 GB + 3 GB.
2. **Given** a game installed through a known game store, **When** the view opens, **Then** the
   game is identified as a game, with the total size of its install folder.
3. **Given** a system application, **When** listed, **Then** it is marked as system and cannot be
   removed by the app.
4. **Given** an app selected for removal, **When** the user confirms, **Then** the bundle and the
   selected associated data go to the Trash through the story 3 flow.

---

### User Story 5 - Free space from files already synced to the cloud (Priority: P5)

The app detects the cloud service folders on the machine (iCloud Drive, OneDrive, Google Drive,
Dropbox) and lists files that are downloaded locally and already synced, largest first. The user
can choose "keep only in the cloud", freeing local space while leaving a placeholder (the file
stays visible and downloads on demand).

**Why this priority**: it frees space without losing anything, but depends on the user's cloud
services and their sync rules — so it comes after the core.

**Independent Test**: with a cloud folder containing synced, downloaded files, request "keep only
in the cloud" and check that local space is freed and the file is still listed as available in the
cloud.

**Acceptance Scenarios**:

1. **Given** iCloud Drive with 10 GB of downloaded, synced files, **When** the user opens the cloud
   view, **Then** they see those files, the provider and 10 GB reclaimable.
2. **Given** a file not yet synced (pending upload or error), **When** listed, **Then** it shows as
   "not synced" and "keep only in the cloud" is unavailable.
3. **Given** selected files, **When** the user confirms "keep only in the cloud", **Then** local
   space is freed and the files remain accessible through the system.
4. **Given** no cloud service installed, **When** the view opens, **Then** the app says no
   provider was detected.
5. **Given** large files outside cloud folders, **When** the view opens, **Then** they do not
   appear there; the app only acts on files already in cloud folders and never moves files into
   them.

---

### Edge Cases

- Disks with millions of files: the interface stays usable and the list is paginated.
- Hard links and APFS clones: the same content must not be counted twice in the reclaimable total;
  the app warns that deleting a clone may free less space than its apparent size.
- Symbolic links and mount points: not followed; external and network volumes are scanned only
  when explicitly chosen.
- Files in use or locked at deletion time: skipped and reported.
- Files that disappear or change between scan and action: re-validated before any action.
- Local Time Machine snapshots and "purgeable" space: the app explains that free space may not grow
  immediately.
- Disk almost full during the scan: the catalogue must not get corrupted; the scan fails with a
  clear message.
- AI returning invalid, slow or failed responses: the assessment falls back to local rules.
- The app's own files (catalogue, settings): never shown as deletion candidates.
- File names with special characters, Unicode or very long paths: displayed and handled correctly.

## Requirements *(mandatory)*

### Functional Requirements

**Scanning and catalogue**

- **FR-001**: The system MUST scan the startup disk or folders/volumes chosen by the user.
- **FR-002**: The system MUST catalogue, for each file above the minimum size, path, name, size on
  disk, kind, creation/modification/last-access dates and the source scan.
- **FR-003**: The system MUST compute aggregated folder sizes so the user can browse the folders
  that take the most space.
- **FR-004**: The system MUST show items in descending size order, filterable by minimum size
  (default 100 MB, configurable), file kind, folder and risk level.
- **FR-005**: The system MUST show scan progress and allow cancelling it.
- **FR-006**: The system MUST persist the catalogue locally and show it when the app reopens.
- **FR-007**: The system MUST avoid reprocessing unchanged items on later scans.
- **FR-008**: The system MUST skip and record inaccessible locations without stopping the scan.
- **FR-009**: The system MUST NOT follow symbolic links nor enter volumes not explicitly chosen.
- **FR-010**: The system MUST guide the user to grant Full Disk Access when needed (naming the app
  macOS expects), allowing partial use without it.
- **FR-011**: The user MUST be able to reveal any item in Finder.

**Risk assessment**

- **FR-012**: The system MUST give each assessed item a risk level (Safe, Caution, Dangerous,
  Unknown), a plain-language explanation and its source (AI or local rules).
- **FR-013**: The system MUST have local rules that work offline and without AI configured.
- **FR-014**: By default the system MUST share only file metadata with the AI; sharing contents
  requires an explicit user opt-in.
- **FR-015**: The system MUST store assessments and reuse them while the item is unchanged.
- **FR-016**: The system MUST classify as "Dangerous", and block from deletion, items in protected
  system locations regardless of the AI assessment.
- **FR-017**: The user MUST be able to configure the AI provider and its credentials, stored
  securely by the operating system. The user MUST be able to choose a cloud AI provider (with
  their own key), a local model running on the machine, or none (local rules only).

**Deletion**

- **FR-018**: The system MUST delete items only after a review screen with total, count and
  highlighted risky items, and the user's confirmation.
- **FR-019**: Unblocked "Caution"/"Dangerous" items MUST require an individual confirmation.
- **FR-020**: The system MUST move deleted items to the Trash by default.
- **FR-021**: The system MUST re-validate each item (existence, size, date) right before acting and
  skip changed items, informing the user.
- **FR-022**: The system MUST keep a local history of every destructive or space-freeing action
  (date, path, size, action, result).
- **FR-023**: The system MUST NOT pre-select "Caution" or "Dangerous" items.

**Applications and games**

- **FR-024**: The system MUST list installed applications with total size = bundle + associated
  data (support files, caches, containers, preferences, logs).
- **FR-025**: The system MUST identify games from known stores (e.g. Steam, Epic Games, GOG,
  Battle.net, App Store) and compute each game's total size.
- **FR-026**: The system MUST show each application/game's size breakdown and last-used date when
  available.
- **FR-027**: The system MUST mark system applications as non-removable.

**Cloud**

- **FR-028**: The system MUST detect iCloud Drive, OneDrive, Google Drive and Dropbox folders on
  the machine.
- **FR-029**: The system MUST list locally downloaded files in those folders with their sync state
  (synced, pending, error, cloud-only).
- **FR-030**: The system MUST allow "keep only in the cloud" only for fully synced files, using
  each provider's official mechanism.
- **FR-031**: The system MUST show the reclaimable total per provider.

### Key Entities

- **Scan**: one scan run; chosen roots, start/end, status (running, completed, cancelled, failed),
  totals and inaccessible items.
- **FileEntry**: a catalogued file or folder; path, size on disk, apparent size, kind, dates,
  content identity to detect hard links, scan.
- **RiskAssessment**: level, explanation, recommendation, steps, suggested commands, source
  (AI/rules), evaluator version, date; tied to a FileEntry state and invalidated when it changes.
- **InstalledApp**: name, identifier, kind (app, game, system), source store, last use, components
  (bundle + associated data) and total size.
- **CloudLocation**: provider, root folder, associated files and their sync states.
- **ActionRecord**: action (trash, permanent delete, keep in cloud), item, size, date, result and
  failure reason.
- **Settings**: minimum size, default roots, AI provider, content-sharing option, user-defined
  excluded paths.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A full scan of a disk with 1 million files finishes in under 5 minutes on an
  SSD-equipped Mac, and the interface responds to interactions in under 200 ms during the scan.
- **SC-002**: After scanning, the user identifies the 20 largest items on the disk in under
  30 seconds.
- **SC-003**: Re-assessing an unchanged catalogue sends no new AI requests.
- **SC-004**: 0 items in protected system locations can be deleted by the app (verified by
  automated tests).
- **SC-005**: 100% of items removed by the app are recoverable from the Trash in the default flow.
- **SC-006**: On a reference set of 200 labelled files, no item labelled essential is classified
  "Safe", and at least 85% of items get the expected level.
- **SC-007**: 90% of test users free at least 5 GB in their first session without help.
- **SC-008**: The space shown as reclaimable differs by at most 5% from the space actually freed
  (excluding local snapshots).

## Assumptions

- Users run macOS 13 or later on their own computer, with an administrator or standard account;
  no multi-user or cross-machine sync.
- The app is distributed outside the Mac App Store (needed for Full Disk Access and broad scans).
- Default scan scope: the home folder and `/Applications`; the whole disk is optional.
- Default minimum size for listing individual files: 100 MB (configurable).
- AI usage may cost the user money (their own key); the app does not provide AI credits.
- Permanent deletion is out of scope for v1 (Trash only); it may be added later per the
  constitution.
- Game detection covers popular macOS stores; unknown stores show up as regular apps.
- The UI ships in English (US) and Brazilian Portuguese, selectable in Settings; on first launch it
  follows the system language (English when unsupported).
- Non-goals for v1: Windows/Linux, scheduled automatic cleaning, content-based duplicate detection.

## Clarifications

### Session 2026-10-14

- Q: Should the AI be a cloud service, a local model, or both? → A: Both, chosen in the settings
  (plus a "local rules only" option).
- Q: Should the cloud offload suggest moving local files into cloud folders? → A: No; it only acts
  on files already in cloud folders.
