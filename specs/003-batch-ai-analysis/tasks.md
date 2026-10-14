# Tasks: Batch analysis of selected items

- [X] B001 Batch prompt: compact message with ids, `analyses[]` schema, per-item parsing (tests)
- [X] B002 `AnalysisService::analyze_many`: protected/cached items left out, batches of 20, per-item fallback (tests)
- [X] B003 Programmable AI fake for tests (`FakeAiFactory::responding`)
- [X] B004 IPC command `analyze_files` (max 200 ids)
- [X] B005 Selection in the list (per-row and per-page checkboxes; protected items disabled) (tests)
- [X] B006 Selection bar with "Analyse selected with AI", progress and batch summary (tests)
- [ ] B007 Manual validation with a real key and token measurement (single vs batch)

## AI settings adjustments (2026-10-14)

- [X] B008 Updated default models (Gemini `gemini-3.8-flash`, OpenAI `gpt-6-astra`) and OpenAI through the Responses API
- [X] B009 Model listing per provider (Anthropic, OpenAI, Gemini, Ollama) with a text-model filter (contract tests)
- [X] B010 Model and URL validation in the domain (HTTPS required for providers that receive a key, unless local)
- [X] B011 Screen: account model selector, "Load/Refresh list", official URL by default and a custom URL under "Advanced" (tests)
