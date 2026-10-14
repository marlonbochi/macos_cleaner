# Feature Specification: AI-Assisted Risk Analysis

**Feature Branch**: `002-ai-risk-analysis`

**Created**: 2026-10-14

**Status**: Implemented and merged into `main`

**Input**: User description (translated from Portuguese): "Show in the app where I enter the API
key, save it so it can be used, and add a way to research a file — sending its path and details —
that tells whether it can be deleted or suggests another way to remove it; for example, for the
Xcode VMs it suggested commands to delete them. Implement Claude, Gemini, OpenAI and Ollama, with
a selector where the user enters the key and chooses which service it belongs to."

This stage implements **User Story 2** of [001-disk-space-cleaner](../001-disk-space-cleaner/spec.md)
and extends it with actionable recommendations. Requirements FR-012 to FR-017 of 001 still apply.

## Clarifications

### Session 2026-10-14

- Q: Where should the API key be stored? → A: In the macOS Keychain (as required by the
  constitution); provider, model and URL go to SQLite.
- Q: Which providers? → A: Claude (Anthropic), OpenAI, Gemini and Ollama, chosen in a selector.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Configure the AI provider (Priority: P1)

On the Settings screen the user picks a provider (or none), pastes the API key, chooses a model
from the list available to their account, saves and tests the connection. The key is never shown
back.

**Acceptance Scenarios**:

1. **Given** no provider, **When** the user picks Claude, pastes the key and saves, **Then** the key
   goes to the Keychain and the screen shows it as "saved in the Keychain".
2. **Given** a saved provider, **When** the user clicks "Test connection", **Then** they see
   "Connection ok" with provider and model, or the provider's error message.
3. **Given** Ollama, **When** selected, **Then** no key is requested and the local URL can be
   adjusted.

### User Story 2 - Analyse an item and learn how to free the space (Priority: P1)

In "Largest items", the user clicks "Analyse" on an item and sees: risk level, what the item is,
why it has that risk, the recommendation (delete, clean through the app/a command, move to
cloud/external, keep, review), steps and suggested commands with a copy button.

**Acceptance Scenarios**:

1. **Given** AI configured, **When** the user analyses `~/Library/Developer/CoreSimulator`,
   **Then** they get a recommendation to use Xcode/`xcrun simctl`, with commands and descriptions.
2. **Given** an item already analysed and unchanged, **When** it is reopened, **Then** the analysis
   is reused (no new request) and the risk badge shows in the table.
3. **Given** the AI is unavailable or fails, **When** the user analyses, **Then** they see the local
   rules' analysis and a notice with the error.
4. **Given** a protected item, **When** analysed, **Then** it is "Dangerous" without calling the AI.

## Requirements *(mandatory)*

- **FR-032**: The system MUST let the user choose Claude, OpenAI, Gemini, Ollama or no provider,
  with per-provider defaults for model and URL.
- **FR-033**: The API key MUST be stored only in the Keychain and never returned to the frontend.
- **FR-034**: Each analysis MUST include summary, explanation, level, category, recommendation,
  steps and suggested commands (with descriptions).
- **FR-035**: The app MUST NOT run suggested commands; it only displays them and allows copying.
- **FR-036**: For folders, the system MUST send the names/sizes of the largest catalogued children
  (up to 15) to improve the analysis; never file contents.
- **FR-037**: Local rules known to identify essential data MUST prevent the AI from lowering an item
  below "Caution".
- **FR-038** *(2026-10-14)*: The model MUST be chosen from a list fetched from the provider's API
  with the user's key (text models only); saving requires a model from the list.
- **FR-039** *(2026-10-14)*: The server URL MUST default to the official one (custom URLs only under
  "Advanced"); URLs are validated (http/https, no credentials or query string) and providers that
  receive a key require HTTPS unless the server is local.

## Success Criteria *(mandatory)*

- **SC-201**: Configuring a provider and getting the first analysis takes under 2 minutes.
- **SC-202**: 100% of saved keys stay out of SQLite (verified by tests).
- **SC-203**: Reopening an already analysed item sends no new AI request.

## Assumptions

- Default models: `claude-opus-5-5`, `gpt-6-astra`, `gemini-3.8-flash`, `llama3.2` (users pick from
  their account's model list).
- On Claude, models that support it are called with the server-side refusal fallback
  (`fallbacks: "default"`).
- Batch analysis of selected items is delivered in [003](../003-batch-ai-analysis/spec.md); the
  labelled benchmark (SC-006 of 001) is still pending.
