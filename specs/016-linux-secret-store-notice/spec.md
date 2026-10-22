# Feature Specification: Warn when the system has no keyring

**Feature Branch**: `016-linux-secret-store-notice`

**Created**: 2026-10-21

**Status**: Implemented

**Input**: User description (translated from Portuguese): "Implement the Linux warning, so the user
is sure to know."

## Context

API keys are only ever stored in the system's secret store (Keychain, Windows Credential Manager,
Secret Service on Linux). Minimal Linux desktops may have no Secret Service (no GNOME Keyring or
KWallet). The app already keeps working without one, but the user only found out by trying to save
a key and reading a raw error.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The user knows why keys cannot be saved (Priority: P1)

On a system without a keyring, the AI settings explain that API keys cannot be saved, how to
install a keyring and that Ollama works without a key.

**Acceptance Scenarios**:

1. **Given** no secret store, **When** the user opens Settings, **Then** a warning is shown at the
   top of the AI section; on Linux it names GNOME Keyring / KWallet and the install command.
2. **Given** no secret store, **When** a provider that needs a key is selected, **Then** the key
   field is disabled and says why.
3. **Given** a working secret store, **When** the user opens Settings, **Then** no warning is shown.

## Requirements *(mandatory)*

- **FR-1601**: The `SecretStore` port MUST say whether it can store secrets (`is_available`,
  `true` by default); the null store used when the system has none returns `false`.
- **FR-1602**: The AI settings sent to the frontend MUST include `secretStoreAvailable`.
- **FR-1603**: The warning MUST be localised (en-US, pt-BR) and use the system's terms.
- **FR-1604**: Keys MUST still never be written anywhere else (no fallback to SQLite or files).
