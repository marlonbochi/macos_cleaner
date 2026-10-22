# Changelog

All notable changes to Disk Cleaner. Generated from the commit history by
[git-cliff](https://git-cliff.org); each version is also published as a
[GitHub Release](https://github.com/marlonbochi/macos_cleaner/releases) with the app attached.

## [0.2.2](https://github.com/marlonbochi/macos_cleaner/releases/tag/v0.2.2) — 2026-10-22

### Features

- **settings:** Warn when the system has no keyring to store API keys ([139e225](https://github.com/marlonbochi/macos_cleaner/commit/139e225a58dd687fcdfde6b398e3b5a7facc4f97))

## [0.2.1](https://github.com/marlonbochi/macos_cleaner/releases/tag/v0.2.1) — 2026-10-19

### Bug fixes

- **deps:** Update quick-xml, time and inout to clear security advisories ([d383b6f](https://github.com/marlonbochi/macos_cleaner/commit/d383b6f1b5645da43f49e53df34113c7860ac79a))

## [0.2.0](https://github.com/marlonbochi/macos_cleaner/releases/tag/v0.2.0) — 2026-10-15

### Features

- Rename the app to Disk Cleaner and migrate data from macOS Cleaner ([25db7c4](https://github.com/marlonbochi/macos_cleaner/commit/25db7c447e7a0d2d7e348dbf32bc0a08186f8063))
- **core:** Platform-aware protected paths, local rules and AI prompt ([6c23381](https://github.com/marlonbochi/macos_cleaner/commit/6c233810ddd235b680c90def475e356c3f3b5e22))
- Run on Windows and Linux (Debian) as well as macOS ([394a802](https://github.com/marlonbochi/macos_cleaner/commit/394a8027978867c70a1ed4b84e68481b4a0ce5e2))

### Bug fixes

- **infra:** Select the COM threading model the trash crate needs on Windows ([9335ed9](https://github.com/marlonbochi/macos_cleaner/commit/9335ed97875546c1e3735fd0230576d90f0295f2))

### Tests

- Run POSIX-path fixtures only on Unix, add Windows path tests ([dc3dbe6](https://github.com/marlonbochi/macos_cleaner/commit/dc3dbe6b56862af9ced46302b15fb34c37ae1b7a))
- Keep the proptest import inside the Unix-only properties module ([35f0d25](https://github.com/marlonbochi/macos_cleaner/commit/35f0d25d9d7d9655fab69c1f5aac56f44f0222da))

### Maintenance

- **release:** Build and publish the .dmg, Windows installer and .deb together ([8eeb6ff](https://github.com/marlonbochi/macos_cleaner/commit/8eeb6ff587f75dcab2ff8e8b6a13ec979eef3b08))
- Report every clippy and test failure in a single run ([f426ee6](https://github.com/marlonbochi/macos_cleaner/commit/f426ee643282330409c17292567ea1a61b48d0d5))

## [0.1.1](https://github.com/marlonbochi/macos_cleaner/releases/tag/v0.1.1) — 2026-10-15

### Bug fixes

- **release:** Ad-hoc sign the whole app bundle so macOS offers "Open Anyway" ([4dc7659](https://github.com/marlonbochi/macos_cleaner/commit/4dc7659fdc3e76ff46bf8418af9f91d5207ebaf5))

## [0.1.0](https://github.com/marlonbochi/macos_cleaner/releases/tag/v0.1.0) — 2026-10-15

### Features

- **core,infra:** Domain model, ports, protected-path policy and SQLite catalog ([66f75d4](https://github.com/marlonbochi/macos_cleaner/commit/66f75d4f649111cb500fe6493de569ef0aea799b))
- **scan:** Parallel jwalk scanner and ScanService catalog builder (US1) ([c5a4e75](https://github.com/marlonbochi/macos_cleaner/commit/c5a4e75923c070022d20081e8cdd9b92e9efff14))
- **app:** Tauri IPC layer for scanning and catalog browsing (US1) ([8634787](https://github.com/marlonbochi/macos_cleaner/commit/86347872e0c639f6a802be61de58f9582b344029))
- **ui:** Vue frontend for scanning and browsing the largest items (US1) ([4cb684c](https://github.com/marlonbochi/macos_cleaner/commit/4cb684c904dba720860f6a485c95cca5d5d89eb9))
- **permissions:** Tell the user which app needs Full Disk Access ([56f257b](https://github.com/marlonbochi/macos_cleaner/commit/56f257bd0880c3a3c986399c3d1ff386fb267a01))
- **core:** AI-assisted risk analysis use cases with local-rule fallback (US2) ([809c065](https://github.com/marlonbochi/macos_cleaner/commit/809c06522da1a4ca058c81d8b48ac0caa07c4372))
- **infra:** Claude, OpenAI, Gemini and Ollama adapters, Keychain secrets, assessment cache ([0f12bf7](https://github.com/marlonbochi/macos_cleaner/commit/0f12bf7d183f968476673806a286a3019b0725fb))
- **app:** IPC commands for AI analysis and provider settings (US2) ([112a1f8](https://github.com/marlonbochi/macos_cleaner/commit/112a1f8207e720c588ac462a196eb213ca3c75bd))
- **ui:** AI settings page, per-item analysis panel and risk badges (US2) ([be48171](https://github.com/marlonbochi/macos_cleaner/commit/be48171fe4575aef1c00360e4207d24114808071))
- **analysis:** Batch AI analysis of many items in a single request ([5a9c62b](https://github.com/marlonbochi/macos_cleaner/commit/5a9c62bcf46bde57bb52a5944859f8983285ef11))
- **ui:** Select items in the list and analyse them together with AI ([acf9aef](https://github.com/marlonbochi/macos_cleaner/commit/acf9aef68b9a0a92b5bb6145eb694d1b62abcd2b))
- **ai:** List available models per provider and validate model ids and server URLs ([ae88c64](https://github.com/marlonbochi/macos_cleaner/commit/ae88c64848187003fbd83da85f4fe158d5fc804f))
- **ui:** Pick the AI model from the provider's list; official URL by default ([65ba67d](https://github.com/marlonbochi/macos_cleaner/commit/65ba67d51459ddbd93574c3165e3ab217d3f4111))
- **brand:** Add the app icon (disk with sparkles) ([a5248ea](https://github.com/marlonbochi/macos_cleaner/commit/a5248eaa9b31d661da5e1c48e43186a2e37879d7))
- **i18n:** Language preference and bilingual analyses in the backend ([44b46e7](https://github.com/marlonbochi/macos_cleaner/commit/44b46e7bd5ccea65f489a784d45fca2bcd8a17d9))
- **i18n:** English (US) and Brazilian Portuguese UI with a saved language choice ([aa47b19](https://github.com/marlonbochi/macos_cleaner/commit/aa47b19d92547c5b5aef4da7f3e2e8ba6c0cec5c))
- **diagnosis:** AI diagnosis of the largest meaningful items of a scan ([0fd4458](https://github.com/marlonbochi/macos_cleaner/commit/0fd4458318a4e07c76602a5444152a5a6022dfbd))
- **diagnosis:** AI diagnosis tab, auto-run after a scan and AI-only features gated ([9484087](https://github.com/marlonbochi/macos_cleaner/commit/9484087c5b80711e64699da979a8e28e3dcf4db9))
- **uninstall:** Complete uninstall of the app's data, Keychain entries and bundle ([0da1a59](https://github.com/marlonbochi/macos_cleaner/commit/0da1a59f479d45c3b8e7f727cfc6896df754e0fd))
- **uninstall:** Settings card and typed-confirmation dialog ([5eb5f1b](https://github.com/marlonbochi/macos_cleaner/commit/5eb5f1bb7dd7d75c732b2ec2ee4cc1342386e07a))

### Bug fixes

- **ui:** Keep file table actions inside the table and let names use the free width ([5ac50d2](https://github.com/marlonbochi/macos_cleaner/commit/5ac50d26711173deded24b8aee54565e921387a3))
- **ai:** Update default Gemini and OpenAI models; use OpenAI Responses API ([a54793a](https://github.com/marlonbochi/macos_cleaner/commit/a54793ac49b4170321c4eca9e9ecbf322c7c2a89))

### Documentation

- **spec:** Clarify AI providers and cloud scope; add implementation plan and design artifacts ([78a58b5](https://github.com/marlonbochi/macos_cleaner/commit/78a58b519efd9532ac8276f29efd6dc2d1b4272c))
- **tasks:** Generate dependency-ordered task list for 001 ([4f85f07](https://github.com/marlonbochi/macos_cleaner/commit/4f85f0763409d56183e5db042ad238e613664eec))
- Translate constitution, specs, plans and README to English ([484c3d0](https://github.com/marlonbochi/macos_cleaner/commit/484c3d053b6dc20ada99b070c004ac1b47a74174))
- Format README with Prettier ([961a564](https://github.com/marlonbochi/macos_cleaner/commit/961a564877c43905b1131ec67807799737cd0ee6))
- **diagnosis:** Spec, tasks and IPC contract for the AI diagnosis ([f04eb12](https://github.com/marlonbochi/macos_cleaner/commit/f04eb126c4f91908131aa293ddeb45b9395d3ad3))

### Maintenance

- Bootstrap Spec Kit, constitution v1.0.0 and spec 001-disk-space-cleaner ([da191e5](https://github.com/marlonbochi/macos_cleaner/commit/da191e54af2dff3893a81c9c46f2d46160215b2d))
- GitHub Actions for quality gates, dmg build, release and audit; docs ([99e531a](https://github.com/marlonbochi/macos_cleaner/commit/99e531afb289c1e63cc0125401d924c6f1418ae0))
- Keep ts-rs output canonical (excluded from Prettier) ([4222a43](https://github.com/marlonbochi/macos_cleaner/commit/4222a43edb3c89764790ef0ccb9f34a9a33f1f93))
- Align tauri crates with @tauri-apps npm packages (2.12 / dialog 2.8) ([af74b87](https://github.com/marlonbochi/macos_cleaner/commit/af74b8778f700da3c5d3ad31d71478223ae1f107))
- **deps:** Bump actions/checkout from 4 to 7 (#5) ([3b9da34](https://github.com/marlonbochi/macos_cleaner/commit/3b9da348f6ed9b5286c813fb2f250f036490ed36))
- **deps:** Bump proptest (#1) ([641ac9b](https://github.com/marlonbochi/macos_cleaner/commit/641ac9b54174477b5327f1cf71d5e7d8d53a8103))
- **deps-dev:** Bump jsdom from 29.1.1 to 30.1.2 in the npm-deps group (#6) ([71a906d](https://github.com/marlonbochi/macos_cleaner/commit/71a906d0cc6c889d2b08509bb81af8b3cf52c9d1))
- Bump actions/setup-node and actions/upload-artifact to v7 ([5af54ce](https://github.com/marlonbochi/macos_cleaner/commit/5af54ce3ce9509da8bb08c7caa7baac04b28fd7d))
- **release:** Build the app on GitHub and publish releases with generated notes ([04d2a96](https://github.com/marlonbochi/macos_cleaner/commit/04d2a963dfdcb79e4d46a5e2a60718d7e88a4ef2))
