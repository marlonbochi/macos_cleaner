# Tasks: GitHub Releases built in CI

- [X] R001 Version helpers (parse, compare, read/update `package.json`, `tauri.conf.json`, `Cargo.toml`) (tests)
- [X] R002 `check-version.mjs` used by the workflow to match the tag with the app version
- [X] R003 `npm run release -- x.y.z`: guards (newer, `main`, clean, new tag), bump, CHANGELOG, commit, tag (verified in a scratch clone)
- [X] R004 `cliff.toml`: notes grouped by type with commit links
- [X] R005 `release.yml`: CI gates (`workflow_call`), version check, notes + install steps, universal `.dmg`, tauri-action v1
- [X] R006 README "Releases" section and CLAUDE.md command
- [ ] R007 First release `v0.1.0` (after merging 009 and 010 into `main`, on the user's approval)
