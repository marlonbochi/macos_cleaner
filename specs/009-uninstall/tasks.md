# Tasks: Complete uninstall

- [X] U001 Domain `FootprintLocation` validation: only the `.app` or paths named after the bundle id (tests)
- [X] U002 Ports `Trash` and `DiskUsage`; `UninstallService` (footprint, exact phrase, keys, Trash, bundle last, failures reported) (tests)
- [X] U003 Adapters `StdDiskUsage` (no symlink following) and `MacTrash` (`NSFileManager`); bundle id check of the running app (integration tests)
- [X] U004 Commands `get_app_footprint` and `uninstall_app` (quits afterwards); Keychain service = bundle id
- [X] U005 Settings "Uninstall" card and a confirmation dialog that requires typing the phrase (tests)
- [X] U006 Verified with an isolated build that the web view does not recreate its data after quitting
- [ ] U007 Manual validation of the full flow on an installed copy
