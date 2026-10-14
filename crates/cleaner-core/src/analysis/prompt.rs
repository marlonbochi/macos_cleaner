//! Prompt, JSON schema and response parsing for AI analysis (contract: ai-classification.md).
//!
//! Only metadata is sent (FR-014): the path with the home directory replaced by `~`, kind,
//! sizes, dates and the names/sizes of the largest catalogued children. Never file contents.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::verdict::Verdict;
use crate::domain::file_entry::{FileKind, FileRecord};
use crate::domain::locale::Locale;
use crate::domain::risk::{Recommendation, RiskCategory, RiskLevel, SuggestedCommand};
use crate::ports::ai::AiError;

/// Bump when the prompt or schema changes: cached AI assessments are then recomputed.
pub const PROMPT_VERSION: u32 = 2;
pub const SCHEMA_NAME: &str = "file_analysis";
/// How many children of a folder are described to the model.
pub const MAX_CHILDREN: usize = 15;
pub const BATCH_SCHEMA_NAME: &str = "file_analyses";
/// Items per batch request: keeps the answer well inside the output budget.
pub const MAX_BATCH: usize = 20;
/// Fewer children per folder in batches, to keep the request small.
pub const BATCH_MAX_CHILDREN: usize = 5;

const SYSTEM_PROMPT: &str = "\
You are a macOS expert helping someone free disk space safely.
You receive the METADATA of an item (never its contents): path (with ~ for the home folder),
kind, sizes, dates and, for folders, the largest children.

Assess, objectively:
- what the item most likely is (which app/tool created it);
- the risk of deleting it: Safe (re-creatable or disposable), Caution (may have value, check
  first), Dangerous (irreplaceable personal data, app libraries, databases, system),
  Unknown (cannot tell);
- the best way to free that space.

Rules:
- Be conservative: when in doubt, use Caution or Unknown. Never classify as Safe anything that may
  contain unique personal data.
- When there is an official way to clean it (an app setting, a menu, or an official command such
  as `xcrun simctl delete unavailable`, `brew cleanup`, `docker system prune`,
  `tmutil deletelocalsnapshots`), recommend it (recommendation use-app-or-command) instead of
  deleting the folder by hand, and explain why.
- In `steps`, describe menu paths or short steps (e.g. \"Xcode › Settings › Platforms\").
- In `commands`, include only real, safe commands, one per entry, each with a description of what
  it does. Never suggest `sudo rm -rf` on system folders nor commands that delete more than the
  analysed item. Leave `commands` empty when no command is useful.
- If the best option is to keep the data but off this disk, use move-to-cloud-or-external.";

/// Metadata sent to the model.
#[derive(Debug, Serialize)]
struct ItemContext {
    path: String,
    kind: &'static str,
    size_on_disk: String,
    size_on_disk_bytes: u64,
    file_count: u64,
    created: Option<String>,
    modified: Option<String>,
    last_accessed: Option<String>,
    largest_children: Vec<ChildContext>,
}

#[derive(Debug, Serialize)]
struct ChildContext {
    name: String,
    kind: &'static str,
    size_on_disk: String,
}

/// System prompt with the answer language appended (field values stay in English enums).
pub fn system_prompt(locale: Locale) -> String {
    format!(
        "{SYSTEM_PROMPT}\n\nWrite `summary`, `explanation`, `steps` and command descriptions in {}.",
        locale.language_name()
    )
}

pub fn response_schema() -> Value {
    let levels: Vec<&str> = ["Safe", "Caution", "Dangerous", "Unknown"].to_vec();
    let categories: Vec<&str> = RiskCategory::ALL.iter().map(|c| c.as_str()).collect();
    let recommendations: Vec<&str> = Recommendation::ALL.iter().map(|r| r.as_str()).collect();
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["level", "category", "summary", "explanation", "recommendation", "steps", "commands"],
        "properties": {
            "level": { "type": "string", "enum": levels },
            "category": { "type": "string", "enum": categories },
            "summary": { "type": "string", "description": "What the item is, in one sentence." },
            "explanation": { "type": "string", "description": "Why this risk level and what happens if it is deleted." },
            "recommendation": { "type": "string", "enum": recommendations },
            "steps": { "type": "array", "items": { "type": "string" } },
            "commands": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["command", "description"],
                    "properties": {
                        "command": { "type": "string" },
                        "description": { "type": "string" }
                    }
                }
            }
        }
    })
}

fn item_context(
    record: &FileRecord,
    children: &[FileRecord],
    home: &Path,
    max_children: usize,
) -> ItemContext {
    ItemContext {
        path: display_path(&record.path, home),
        kind: kind_label(record.kind),
        size_on_disk: record.size_on_disk.to_string(),
        size_on_disk_bytes: record.size_on_disk.bytes(),
        file_count: record.file_count,
        created: record.created_at.map(epoch_to_date),
        modified: record.modified_at.map(epoch_to_date),
        last_accessed: record.accessed_at.map(epoch_to_date),
        largest_children: children
            .iter()
            .take(max_children)
            .map(|c| ChildContext {
                name: c.name(),
                kind: kind_label(c.kind),
                size_on_disk: c.size_on_disk.to_string(),
            })
            .collect(),
    }
}

/// Builds the user message describing `record` (and its largest children, for folders).
pub fn user_message(record: &FileRecord, children: &[FileRecord], home: &Path) -> String {
    let context = item_context(record, children, home, MAX_CHILDREN);
    // Serialising a plain struct of strings and numbers cannot fail; fall back defensively.
    let json = serde_json::to_string_pretty(&context).unwrap_or_default();
    format!("Analyse this item and answer in the requested format.\n\n{json}")
}

/// One item of a batch request.
#[derive(Debug, Clone, Copy)]
pub struct BatchInput<'a> {
    /// Opaque id echoed back by the model to match answers to items.
    pub id: &'a str,
    pub record: &'a FileRecord,
    pub children: &'a [FileRecord],
}

#[derive(Debug, Serialize)]
struct BatchItem<'a> {
    id: &'a str,
    #[serde(flatten)]
    item: ItemContext,
}

/// One message describing many items. Compact JSON: in a batch every token counts.
pub fn batch_user_message(items: &[BatchInput<'_>], home: &Path) -> String {
    let items: Vec<BatchItem<'_>> = items
        .iter()
        .map(|i| BatchItem {
            id: i.id,
            item: item_context(i.record, i.children, home, BATCH_MAX_CHILDREN),
        })
        .collect();
    let json = serde_json::to_string(&json!({ "items": items })).unwrap_or_default();
    format!(
        "Analyse EVERY item in the list. Answer with one object in `analyses` per item, \
         repeating exactly the same `id`. Do not skip items.\n\n{json}"
    )
}

/// `{ "analyses": [ { "id": ..., <single-item fields> } ] }`
pub fn batch_response_schema() -> Value {
    let mut item = response_schema();
    item["properties"]["id"] = json!({ "type": "string" });
    if let Some(required) = item["required"].as_array_mut() {
        required.insert(0, json!("id"));
    }
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["analyses"],
        "properties": { "analyses": { "type": "array", "items": item } }
    })
}

/// Parses a batch answer into one result per id. A malformed item only affects that item;
/// a malformed envelope fails the whole batch.
pub fn parse_batch_response(
    value: &Value,
) -> Result<std::collections::HashMap<String, Result<Verdict, AiError>>, AiError> {
    let analyses = value
        .get("analyses")
        .and_then(Value::as_array)
        .ok_or_else(|| AiError::InvalidResponse("missing analyses array".into()))?;
    Ok(analyses
        .iter()
        .filter_map(|a| {
            let id = a.get("id").and_then(Value::as_str)?;
            Some((id.to_owned(), parse_response(a)))
        })
        .collect())
}

/// Replaces the home directory with `~` so the user name is not sent.
pub fn display_path(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".into(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

#[derive(Debug, Deserialize)]
struct RawVerdict {
    level: String,
    category: String,
    summary: String,
    explanation: String,
    recommendation: String,
    #[serde(default)]
    steps: Vec<String>,
    #[serde(default)]
    commands: Vec<RawCommand>,
}

#[derive(Debug, Deserialize)]
struct RawCommand {
    command: String,
    #[serde(default)]
    description: String,
}

/// Validates a provider response against the contract. Unknown categories degrade to `other`;
/// an unknown level or recommendation is a hard error (the caller falls back to local rules).
pub fn parse_response(value: &Value) -> Result<Verdict, AiError> {
    let raw: RawVerdict = serde_json::from_value(value.clone())
        .map_err(|e| AiError::InvalidResponse(e.to_string()))?;
    let level = RiskLevel::parse(&raw.level)
        .ok_or_else(|| AiError::InvalidResponse(format!("unknown level {}", raw.level)))?;
    let recommendation = Recommendation::parse(&raw.recommendation).ok_or_else(|| {
        AiError::InvalidResponse(format!("unknown recommendation {}", raw.recommendation))
    })?;
    if raw.summary.trim().is_empty() || raw.explanation.trim().is_empty() {
        return Err(AiError::InvalidResponse(
            "empty summary or explanation".into(),
        ));
    }
    Ok(Verdict {
        level,
        category: RiskCategory::parse(&raw.category).unwrap_or(RiskCategory::Other),
        summary: raw.summary,
        explanation: raw.explanation,
        recommendation,
        steps: raw.steps,
        commands: raw
            .commands
            .into_iter()
            .map(|c| SuggestedCommand {
                command: c.command,
                description: c.description,
            })
            .collect(),
    }
    .sanitized())
}

/// Parses JSON that may be wrapped in a Markdown code fence (some local models add one).
pub fn parse_json_text(text: &str) -> Result<Value, AiError> {
    let trimmed = text.trim();
    let body = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|rest| rest.strip_suffix("```"))
        .unwrap_or(trimmed);
    serde_json::from_str(body.trim()).map_err(|e| AiError::InvalidResponse(e.to_string()))
}

const fn kind_label(kind: FileKind) -> &'static str {
    match kind {
        FileKind::File => "file",
        FileKind::Directory => "folder",
        FileKind::Package => "package (macOS bundle)",
    }
}

/// Unix seconds → `YYYY-MM-DD` (UTC), without pulling a date library into the core.
fn epoch_to_date(secs: i64) -> String {
    // Civil-from-days algorithm (Howard Hinnant).
    let days = secs.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::size::ByteSize;
    use std::path::PathBuf;

    fn record(path: &str, kind: FileKind, size: u64) -> FileRecord {
        FileRecord {
            path: PathBuf::from(path),
            kind,
            size_on_disk: ByteSize::new(size),
            apparent_size: ByteSize::new(size),
            file_count: 3,
            created_at: Some(0),
            modified_at: Some(1_760_400_000),
            accessed_at: None,
            inode: 1,
            device: 1,
            is_hardlink_dup: false,
            is_protected: false,
        }
    }

    #[test]
    fn user_message_hides_the_home_directory_and_lists_children() {
        let home = Path::new("/Users/alice");
        let parent = record(
            "/Users/alice/Library/Developer/CoreSimulator",
            FileKind::Directory,
            9,
        );
        let children = vec![record(
            "/Users/alice/Library/Developer/CoreSimulator/Devices",
            FileKind::Directory,
            8_000_000_000,
        )];

        let message = user_message(&parent, &children, home);

        assert!(message.contains("\"path\": \"~/Library/Developer/CoreSimulator\""));
        assert!(message.contains("\"name\": \"Devices\""));
        assert!(message.contains("8.0 GB"));
        assert!(message.contains("\"modified\": \"2025-10-14\""));
        assert!(!message.contains("alice"));
    }

    #[test]
    fn paths_outside_home_are_sent_as_is() {
        let home = Path::new("/Users/alice");
        assert_eq!(
            display_path(Path::new("/Applications/Xcode.app"), home),
            "/Applications/Xcode.app"
        );
        assert_eq!(display_path(home, home), "~");
    }

    #[test]
    fn schema_lists_every_enum_value() {
        let schema = response_schema();
        assert_eq!(
            schema["properties"]["category"]["enum"]
                .as_array()
                .unwrap()
                .len(),
            14
        );
        assert_eq!(
            schema["properties"]["recommendation"]["enum"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        assert_eq!(schema["additionalProperties"], json!(false));
    }

    #[test]
    fn parses_a_valid_response() {
        let value = json!({
            "level": "Caution",
            "category": "cache",
            "summary": "Simuladores do Xcode",
            "explanation": "Podem ser recriados, mas perdem dados de apps de teste.",
            "recommendation": "use-app-or-command",
            "steps": ["Xcode › Settings › Platforms"],
            "commands": [{"command": "xcrun simctl delete unavailable", "description": "Remove simuladores indisponíveis"}]
        });
        let v = parse_response(&value).unwrap();
        assert_eq!(v.level, RiskLevel::Caution);
        assert_eq!(v.recommendation, Recommendation::UseAppOrCommand);
        assert_eq!(v.commands[0].command, "xcrun simctl delete unavailable");
    }

    #[test]
    fn unknown_category_degrades_but_unknown_level_fails() {
        let mut value = json!({
            "level": "Safe", "category": "weird", "summary": "s", "explanation": "e",
            "recommendation": "keep", "steps": [], "commands": []
        });
        assert_eq!(
            parse_response(&value).unwrap().category,
            RiskCategory::Other
        );
        value["level"] = json!("Totally fine");
        assert!(matches!(
            parse_response(&value),
            Err(AiError::InvalidResponse(_))
        ));
        assert!(parse_response(&json!({"level": "Safe"})).is_err());
    }

    #[test]
    fn empty_explanation_is_rejected() {
        let value = json!({
            "level": "Safe", "category": "cache", "summary": "s", "explanation": "  ",
            "recommendation": "keep"
        });
        assert!(parse_response(&value).is_err());
    }

    #[test]
    fn json_text_may_be_fenced() {
        assert_eq!(
            parse_json_text("```json\n{\"a\":1}\n```").unwrap(),
            json!({"a": 1})
        );
        assert_eq!(parse_json_text(" {\"a\":1} ").unwrap(), json!({"a": 1}));
        assert!(parse_json_text("nope").is_err());
    }

    #[test]
    fn system_prompt_asks_for_the_chosen_language() {
        assert!(system_prompt(Locale::PtBr).ends_with("in Brazilian Portuguese."));
        assert!(system_prompt(Locale::EnUs).ends_with("in English."));
    }

    #[test]
    fn batch_message_is_compact_and_carries_ids() {
        let home = Path::new("/Users/alice");
        let a = record("/Users/alice/Movies/a.mov", FileKind::File, 1);
        let dir = record("/Users/alice/Library/Caches", FileKind::Directory, 2);
        let children: Vec<_> = (0..10)
            .map(|i| {
                record(
                    &format!("/Users/alice/Library/Caches/c{i}"),
                    FileKind::Directory,
                    1,
                )
            })
            .collect();

        let message = batch_user_message(
            &[
                BatchInput {
                    id: "1",
                    record: &a,
                    children: &[],
                },
                BatchInput {
                    id: "7",
                    record: &dir,
                    children: &children,
                },
            ],
            home,
        );

        assert!(message.contains(r#""id":"1""#));
        assert!(message.contains(r#""path":"~/Movies/a.mov""#));
        assert!(message.contains(r#""id":"7""#));
        assert_eq!(message.matches(r#""name":"c"#).count(), BATCH_MAX_CHILDREN);
        assert!(!message.contains("alice"));
        assert!(!message.contains("\n  "), "no pretty-printing in batches");
    }

    #[test]
    fn batch_schema_requires_ids() {
        let schema = batch_response_schema();
        let item = &schema["properties"]["analyses"]["items"];
        assert_eq!(item["required"][0], json!("id"));
        assert_eq!(item["properties"]["id"], json!({"type": "string"}));
        assert_eq!(item["additionalProperties"], json!(false));
    }

    #[test]
    fn batch_parsing_isolates_bad_items() {
        let good = json!({
            "id": "1", "level": "Safe", "category": "cache", "summary": "s", "explanation": "e",
            "recommendation": "delete-safely", "steps": [], "commands": []
        });
        let bad = json!({ "id": "2", "level": "Nope" });
        let no_id = json!({ "level": "Safe" });
        let parsed = parse_batch_response(&json!({ "analyses": [good, bad, no_id] })).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed["1"].as_ref().unwrap().level, RiskLevel::Safe);
        assert!(parsed["2"].is_err());
        assert!(parse_batch_response(&json!({ "nope": [] })).is_err());
    }

    #[test]
    fn epoch_to_date_handles_known_dates() {
        assert_eq!(epoch_to_date(0), "1970-01-01");
        assert_eq!(epoch_to_date(951_782_400), "2000-02-29");
        assert_eq!(epoch_to_date(-86_400), "1969-12-31");
    }
}
