# Feature Specification: Rename to Disk Cleaner

**Feature Branch**: `012-disk-cleaner-rename`

**Created**: 2026-10-15

**Status**: Implemented

**Input**: User description (translated from Portuguese): "Let's migrate the name to Disk Cleaner"
(first step towards Windows and Linux support, where "macOS Cleaner" no longer fits).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Upgrade without losing anything (Priority: P1)

A user of macOS Cleaner 0.1.x installs Disk Cleaner. On first launch the previous catalogue,
analyses, settings and API keys are already there.

**Acceptance Scenarios**:

1. **Given** data under `~/Library/Application Support/com.marlonbochi.macoscleaner` and no data
   under the new identifier, **When** Disk Cleaner starts, **Then** the folder is moved to
   `com.marlonbochi.diskcleaner`; an existing new folder is never overwritten.
2. **Given** API keys in the Keychain under the old service, **When** Disk Cleaner starts,
   **Then** they are copied to the new service (a key already there wins) and the old entries are
   deleted.
3. **Given** leftovers of the old version (caches, web data, the old `macOS Cleaner.app`), **When**
   the user uninstalls Disk Cleaner, **Then** they are listed as "Previous version" and removed
   too. The old app is only targeted when its `CFBundleIdentifier` matches.

## Requirements *(mandatory)*

- **FR-1201**: Product name "Disk Cleaner", identifier `com.marlonbochi.diskcleaner`, crate and
  package `disk-cleaner`; the uninstall confirmation phrase becomes `Disk Cleaner`.
- **FR-1202**: Migration of data and keys MUST be automatic, idempotent and never overwrite data
  of the new identity.
- **FR-1203**: The GitHub repository keeps its name for now (`macos_cleaner`).
