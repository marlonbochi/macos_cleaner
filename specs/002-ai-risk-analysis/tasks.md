# Tasks: AI-assisted risk analysis

**Input**: [spec.md](./spec.md); design in the [001 plan](../001-disk-space-cleaner/plan.md) and the
[ai-classification contract](../001-disk-space-cleaner/contracts/ai-classification.md).

## Core (`cleaner-core`)

- [X] A001 Domain: `Recommendation`, `SuggestedCommand`, `AiProviderKind`, `AiSettings`
- [X] A002 Ports: `AiProvider`, `AiProviderFactory`, `SecretStore`, `AssessmentRepository`, `SettingsRepository`
- [X] A003 Versioned prompt + JSON schema + response validation/bounds (tests)
- [X] A004 macOS local rules (Xcode, Docker, Photos, caches, iOS backups, cloud folders...) (tests)
- [X] A005 `AnalysisService` (protected > cache > AI > rules; safety floor) (tests)
- [X] A006 `AiSettingsService` (key only in the SecretStore; connection test) (tests)

## Infra (`cleaner-infra`)

- [X] A007 HTTP adapters: Claude, OpenAI, Gemini, Ollama + retries/backoff (contract tests against a mock server)
- [X] A008 `KeychainSecretStore` (real Keychain test on demand)
- [X] A009 V002 migration + SQLite assessment and settings repositories (tests)

## App (`src-tauri`) and UI

- [X] A010 Commands `analyze_file`, `get_analyses`, `get/save_ai_settings`, `test_ai_connection` + DTOs/TS types
- [X] A011 Settings screen (provider selector, key, model, URL, test, remove key)
- [X] A012 "Analyse" action, risk badge in the table and analysis panel with copyable commands
- [X] A013 Store, component and view tests

## Next steps

- [ ] A014 Manual validation with a real key for each provider
- [X] A015 Batch analysis of selected items (delivered in [003](../003-batch-ai-analysis/tasks.md)); labelled benchmark (SC-006) still pending as T035
