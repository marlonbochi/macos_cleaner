# Tasks: Warn when the system has no keyring

- [X] K001 `SecretStore::is_available` (default `true`) and `AiSettingsService::can_store_api_keys`
- [X] K002 Move the null store to `cleaner-infra::secrets::UnavailableSecretStore` with tests
- [X] K003 `AiSettingsDto.secretStoreAvailable` and regenerated TS bindings
- [X] K004 Settings: warning banner, disabled key field, en-US and pt-BR strings, view tests
- [ ] K005 Manual check on a Debian install without GNOME Keyring (user)
