# Feature Specification: Complete Uninstall

**Feature Branch**: `009-uninstall`

**Created**: 2026-10-14

**Status**: Implemented

**Input**: User description (translated from Portuguese): "The SQLite database stays behind when the
app is deleted from Applications, becoming junk itself. Add a way to remove the app completely, in
the app's settings, with a dialog where a text must be typed to confirm, so the user is fully aware
of the uninstall."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Remove the app and everything it created (Priority: P1)

In Settings, the user clicks "Uninstall…". A dialog lists every item that will be removed (data,
caches, web view data, preferences, the app itself) with sizes, plus the API keys stored in the
Keychain. The "Uninstall" button stays disabled until the user types the exact phrase
`Disk Cleaner`. After confirming, the keys are deleted, everything goes to the Trash and the app
quits.

**Acceptance Scenarios**:

1. **Given** the dialog is open, **When** the user types anything other than the exact phrase,
   **Then** nothing can be removed (the backend checks the phrase again).
2. **Given** the exact phrase, **When** confirmed, **Then** the Keychain entries are deleted, every
   existing location is moved to the Trash (the app bundle last) and the app quits.
3. **Given** one item cannot be moved, **When** uninstalling, **Then** the others are still
   removed and the failure is listed.
4. **Given** a development build (not running from a `.app`), **When** the dialog opens, **Then**
   it says the app itself will not be removed, only its data.
5. **Given** the user cancels, **When** the dialog closes, **Then** nothing changed.

## Requirements *(mandatory)*

- **FR-901**: The uninstaller MUST only touch the running `.app` bundle (and only when its
  `CFBundleIdentifier` matches the app's) and locations named after the bundle identifier:
  `~/Library/{Application Support, Caches, WebKit, HTTPStorages, Logs}/<id>`,
  `~/Library/Preferences/<id>.plist`, `~/Library/Saved Application State/<id>.savedState`.
- **FR-902**: Removal MUST go to the Trash (Constitution I), never delete outright.
- **FR-903**: The user MUST type the confirmation phrase exactly; the backend MUST reject any
  other text.
- **FR-904**: API keys of every provider MUST be deleted from the Keychain.
- **FR-905**: The dialog MUST show each item with its size before anything happens.

## Success Criteria *(mandatory)*

- **SC-901**: After uninstalling and emptying the Trash, no file or Keychain entry of the app
  remains (verified: web view data is not recreated after the app quits).
