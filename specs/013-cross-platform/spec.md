# Feature Specification: Windows and Linux Support

**Feature Branch**: `013-cross-platform`

**Created**: 2026-10-15

**Status**: Implemented (manual validation on Windows pending)

**Input**: User description (translated from Portuguese): "Let's go multiplatform: Windows and
Linux, focusing on Debian and generating a Debian package in the releases. I will test it manually
on my Windows machine."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Same app on Windows (Priority: P1)

A Windows 10/11 user installs Disk Cleaner with the NSIS installer, scans their home folder or
whole drives (`C:\`, `D:\`), sees the largest items with rules and AI advice written for Windows
(temporary files, `Windows.old`, `hiberfil.sys`, WinSxS, Program Files, WSL disks, OneDrive) and
opens items in File Explorer. API keys are kept in the Windows Credential Manager.

### User Story 2 - Same app on Debian (Priority: P1)

A Debian/Ubuntu user installs the `.deb`, scans `~` or `/`, gets Linux advice (journald, APT cache,
snap, flatpak, Docker, `~/.cache`) and keys are stored through the Secret Service (GNOME Keyring,
KWallet).

### User Story 3 - Uninstall on every system (Priority: P2)

**Acceptance Scenarios**:

1. **Given** Windows, **When** the user confirms the uninstall, **Then** the keys are deleted from
   the Credential Manager, `uninstall.exe /P /DELETEAPPDATA` is started (the NSIS hook ticks
   "delete application data", since the data folders are locked while the app runs) and the app
   quits.
2. **Given** a `.deb` install, **When** the user confirms, **Then** the data goes to the Trash, the
   keys are deleted and the app shows `sudo apt remove disk-cleaner` to finish.
3. **Given** macOS, **When** the user confirms, **Then** everything goes to the Trash as before.

## Requirements *(mandatory)*

- **FR-1301**: Protected paths MUST exist for each system; Windows paths are compared ignoring case.
- **FR-1302**: Local rules and the AI prompt MUST target the user's system (commands for
  PowerShell on Windows, bash on Linux, zsh on macOS); cached analyses are invalidated
  (`heuristic-v3`, `prompt-v3`).
- **FR-1303**: On Windows the size on disk of online-only cloud files (OneDrive placeholders) MUST
  be zero, and junctions MUST never be followed.
- **FR-1304**: The scan screen MUST offer the system's locations (drives and program folders on
  Windows; `/`, `/usr` and `/opt` on Linux; `/` and `/Applications` on macOS).
- **FR-1305**: UI wording MUST name the system's tools (Finder / File Explorer / file manager, Trash
  / Recycle Bin, Keychain / Credential Manager / keyring, Terminal / PowerShell / terminal).
- **FR-1306**: CI MUST lint and test Rust on macOS, Windows and Ubuntu 22.04 and build the `.dmg`,
  the NSIS installer and the `.deb`.
