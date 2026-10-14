//! Prompt, JSON schema and response parsing for AI analysis (contract: ai-classification.md).
//!
//! Only metadata is sent (FR-014): the path with the home directory replaced by `~`, kind,
//! sizes, dates and the names/sizes of the largest catalogued children. Never file contents.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::verdict::Verdict;
use crate::domain::file_entry::{FileKind, FileRecord};
use crate::domain::risk::{Recommendation, RiskCategory, RiskLevel, SuggestedCommand};
use crate::ports::ai::AiError;

/// Bump when the prompt or schema changes: cached AI assessments are then recomputed.
pub const PROMPT_VERSION: u32 = 1;
pub const SCHEMA_NAME: &str = "file_analysis";
/// How many children of a folder are described to the model.
pub const MAX_CHILDREN: usize = 15;

const SYSTEM_PROMPT: &str = "\
Você é um especialista em macOS que ajuda uma pessoa a liberar espaço em disco com segurança.
Você recebe os METADADOS de um item (nunca o conteúdo): caminho (com ~ para a pasta pessoal),
tipo, tamanhos, datas e, para pastas, os maiores subitens.

Responda em português do Brasil, de forma objetiva, avaliando:
- o que o item provavelmente é (qual app/ferramenta o criou);
- o risco de apagá-lo: Safe (recriável ou descartável), Caution (pode ter valor, conferir antes),
  Dangerous (dados pessoais insubstituíveis, bibliotecas de apps, bancos de dados, sistema),
  Unknown (não dá para saber);
- a melhor forma de liberar esse espaço.

Regras:
- Seja conservador: na dúvida, use Caution ou Unknown. Nunca classifique como Safe algo que
  possa conter dados pessoais únicos.
- Quando existir uma forma oficial de limpar (configuração do app, menu, ou comando oficial
  como `xcrun simctl delete unavailable`, `brew cleanup`, `docker system prune`,
  `tmutil deletelocalsnapshots`), prefira recomendá-la (recommendation use-app-or-command) em vez
  de apagar a pasta manualmente, e explique o porquê.
- Em `steps`, descreva caminhos de menu ou passos curtos (ex.: \"Xcode › Settings › Platforms\").
- Em `commands`, inclua apenas comandos reais e seguros, um por linha, com uma descrição do
  que cada um faz. Nunca sugira `sudo rm -rf` em pastas do sistema nem comandos que apaguem
  mais do que o item analisado. Deixe `commands` vazio quando não houver um comando útil.
- Se o melhor for manter os dados mas fora deste disco, use move-to-cloud-or-external.";

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

pub fn system_prompt() -> &'static str {
    SYSTEM_PROMPT
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
            "summary": { "type": "string", "description": "O que é o item, em uma frase." },
            "explanation": { "type": "string", "description": "Por que esse nível de risco e o que acontece se apagar." },
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

/// Builds the user message describing `record` (and its largest children, for folders).
pub fn user_message(record: &FileRecord, children: &[FileRecord], home: &Path) -> String {
    let context = ItemContext {
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
            .take(MAX_CHILDREN)
            .map(|c| ChildContext {
                name: c.name(),
                kind: kind_label(c.kind),
                size_on_disk: c.size_on_disk.to_string(),
            })
            .collect(),
    };
    // Serialising a plain struct of strings and numbers cannot fail; fall back defensively.
    let json = serde_json::to_string_pretty(&context).unwrap_or_default();
    format!("Analise este item e responda no formato pedido.\n\n{json}")
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
        FileKind::File => "arquivo",
        FileKind::Directory => "pasta",
        FileKind::Package => "pacote (bundle macOS)",
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
    fn epoch_to_date_handles_known_dates() {
        assert_eq!(epoch_to_date(0), "1970-01-01");
        assert_eq!(epoch_to_date(951_782_400), "2000-02-29");
        assert_eq!(epoch_to_date(-86_400), "1969-12-31");
    }
}
