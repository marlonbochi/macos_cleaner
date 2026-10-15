# Tasks: Windows and Linux support

- [X] X001 `Platform` and path normalisation for rule matching (tests)
- [X] X002 Protected-path policies for Windows (case-insensitive) and Linux; per-system scan exclusions (tests)
- [X] X003 Windows and Linux local rules; platform-aware AI prompt (tests)
- [X] X004 Uninstall by system: Trash (macOS), NSIS uninstaller with an app-data hook (Windows), apt command (Linux) (tests)
- [X] X005 Infra: Windows file metadata, Credential Manager / Secret Service stores (tested with the keyring mock), cross-platform Trash and launcher
- [X] X006 App: per-system composition, scan presets, reveal through tauri-plugin-opener, Full Disk Access only on macOS
- [X] X007 UI: system terms in both languages, drive buttons, Windows paths (tests)
- [X] X008 CI matrix (macOS, Windows, Ubuntu 22.04) building `.dmg`, NSIS and `.deb`
- [ ] X009 Manual validation on Windows (user) and Debian
