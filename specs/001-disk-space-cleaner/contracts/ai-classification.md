# Contract: AI risk analysis

The contract between `AnalysisService` and any `AiProvider` (Claude, OpenAI, Gemini, Ollama). The
prompt, schema and parser live in `crates/cleaner-core/src/analysis/prompt.rs`.

## Input sent to the model (metadata only — FR-014)

Single item (one request per item, used by "Analyse"):

```json
{
  "path": "~/Library/Developer/CoreSimulator",
  "kind": "pasta",
  "size_on_disk": "48.2 GB",
  "size_on_disk_bytes": 48200000000,
  "file_count": 210334,
  "created": "2024-03-02",
  "modified": "2026-09-30",
  "last_accessed": null,
  "largest_children": [{ "name": "Devices", "kind": "pasta", "size_on_disk": "41.0 GB" }]
}
```

Batch (used by "Analyse selected with AI"), compact JSON:

```json
{ "items": [ { "id": "12", "path": "~/Movies/a.mov", "...": "..." }, { "id": "37", "...": "..." } ] }
```

Rules:

- The user's home directory is replaced by `~` (the user name is never sent).
- Protected items are never sent (they are resolved before the AI is called).
- Folders include their largest catalogued children: up to 15 in single requests, 5 in batches.
- At most 20 items per batch request; larger selections are split into several requests.
- File contents are never sent (a future opt-in may add a short text excerpt).

## Expected output (JSON schema)

Single item:

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["level", "category", "summary", "explanation", "recommendation", "steps", "commands"],
  "properties": {
    "level": { "enum": ["Safe", "Caution", "Dangerous", "Unknown"] },
    "category": {
      "enum": ["cache", "build-artifact", "installer", "vm-image", "media", "archive",
               "user-document", "app-data", "app-bundle", "game", "system", "backup", "log", "other"]
    },
    "summary": { "type": "string" },
    "explanation": { "type": "string" },
    "recommendation": {
      "enum": ["delete-safely", "use-app-or-command", "move-to-cloud-or-external", "keep", "review-manually"]
    },
    "steps": { "type": "array", "items": { "type": "string" } },
    "commands": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["command", "description"],
        "properties": { "command": { "type": "string" }, "description": { "type": "string" } }
      }
    }
  }
}
```

Batch: `{ "analyses": [ { "id": "<echoed id>", ...single-item fields } ] }` with `id` required.

## Client-side validation rules

1. Unknown `level` or `recommendation` values reject the item; unknown categories degrade to
   `other`; empty summary/explanation reject the item.
2. In batches, answers are matched by `id`; unknown ids are ignored, and items missing or invalid
   in the answer fall back to local rules individually (with the reason shown to the user).
3. An invalid envelope (or a failed request) makes the whole batch fall back to local rules.
4. Every field is bounded: summary ≤ 300 chars, explanation ≤ 900, ≤ 8 steps, ≤ 6 commands
   (single-line, ≤ 400 chars).
5. Safety floor: if local rules classify an item as `Dangerous`, an AI answer below `Caution` is
   raised to `Dangerous`, with a note in the explanation.
6. `evaluator_version = "<provider>:<model>:prompt-v<N>"`; changing the prompt increments `N`, which
   invalidates cached AI assessments.

## Provider wire formats

| Provider | Endpoint | Structured output |
|---|---|---|
| Claude | `POST /v1/messages` | `output_config.format` = `json_schema`; `fallbacks: "default"` on supported models |
| OpenAI | `POST /v1/responses` | `text.format` = strict `json_schema` |
| Gemini | `POST /v1beta/models/{model}:generateContent` | `responseMimeType: application/json`, schema in the instructions |
| Ollama | `POST /api/chat` | `format` = JSON schema |

## System prompt (summary; versioned in `prompt.rs`)

Role: a macOS expert helping someone free disk space safely. It must be conservative: when in
doubt, `Caution` or `Unknown`; irreplaceable personal data (photos, documents, app libraries,
databases) is `Dangerous`; caches and re-creatable artifacts are `Safe`. When an official cleaning
route exists (an app setting or an official command such as `xcrun simctl delete unavailable`,
`brew cleanup`, `docker system prune`), it recommends that route instead of deleting the folder by
hand. Suggested commands must be real, safe and scoped to the analysed item. The model answers in
Brazilian Portuguese, the app's UI language (to be revisited with the EN-US locale).
