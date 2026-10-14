//! "List models" for each provider, filtered to models usable for text generation.

use cleaner_core::domain::ai::{AiModel, AiProviderKind};
use cleaner_core::ports::ai::AiError;
use serde_json::Value;

use super::http::JsonHttp;

/// Safety bound on paginated listings.
const MAX_PAGES: usize = 10;

/// Substrings of model ids that are not text-generation models (audio, images, embeddings...).
const NON_TEXT_MARKERS: &[&str] = &[
    "embedding",
    "embed",
    "tts",
    "audio",
    "realtime",
    "transcribe",
    "whisper",
    "dall-e",
    "image",
    "moderation",
    "search",
    "babbage",
    "davinci",
    "aqa",
    "live",
];

pub(crate) fn list(
    http: &JsonHttp,
    kind: AiProviderKind,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<AiModel>, AiError> {
    let key = || api_key.ok_or(AiError::MissingApiKey(kind.label()));
    let mut models = match kind {
        AiProviderKind::Anthropic => anthropic(http, base_url, key()?)?,
        AiProviderKind::OpenAi => openai(http, base_url, key()?)?,
        AiProviderKind::Gemini => gemini(http, base_url, key()?)?,
        AiProviderKind::Ollama => ollama(http, base_url)?,
    };
    if kind != AiProviderKind::Ollama {
        // Local models are whatever the user pulled; cloud catalogues include non-text models.
        models.retain(|m| is_text_model(&m.id));
    }
    let mut seen = std::collections::HashSet::new();
    models.retain(|m| seen.insert(m.id.clone()));
    Ok(models)
}

fn is_text_model(id: &str) -> bool {
    let id = id.to_lowercase();
    !NON_TEXT_MARKERS.iter().any(|marker| id.contains(marker))
}

fn model(id: &str, label: Option<&str>) -> AiModel {
    AiModel {
        id: id.to_owned(),
        label: label
            .filter(|l| !l.trim().is_empty())
            .unwrap_or(id)
            .to_owned(),
    }
}

/// `GET /v1/models` — newest first, paginated with `after_id`.
fn anthropic(http: &JsonHttp, base: &str, key: &str) -> Result<Vec<AiModel>, AiError> {
    let headers = [("x-api-key", key), ("anthropic-version", "2023-06-01")];
    let mut out = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let url = match &after {
            Some(id) => format!("{base}/v1/models?limit=1000&after_id={id}"),
            None => format!("{base}/v1/models?limit=1000"),
        };
        let page = http.get(&url, &headers)?;
        for m in page
            .get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(id) = m.get("id").and_then(Value::as_str) {
                out.push(model(id, m.get("display_name").and_then(Value::as_str)));
            }
        }
        let more = page
            .get("has_more")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        after = page
            .get("last_id")
            .and_then(Value::as_str)
            .map(str::to_owned);
        if !more || after.is_none() {
            break;
        }
    }
    Ok(out)
}

/// `GET /v1/models` — unordered; sorted newest first by `created`.
fn openai(http: &JsonHttp, base: &str, key: &str) -> Result<Vec<AiModel>, AiError> {
    let base = base.trim_end_matches("/v1");
    let auth = format!("Bearer {key}");
    let page = http.get(
        &format!("{base}/v1/models"),
        &[("authorization", auth.as_str())],
    )?;
    let mut entries: Vec<(i64, AiModel)> = page
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let id = m.get("id")?.as_str()?;
            let created = m.get("created").and_then(Value::as_i64).unwrap_or(0);
            let label = match m.get("shutdown_date").and_then(Value::as_str) {
                Some(date) => format!("{id} (desativação em {date})"),
                None => id.to_owned(),
            };
            Some((created, model(id, Some(&label))))
        })
        .collect();
    entries.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
    Ok(entries.into_iter().map(|(_, m)| m).collect())
}

/// `GET /v1beta/models` — only models that support `generateContent`; paginated.
fn gemini(http: &JsonHttp, base: &str, key: &str) -> Result<Vec<AiModel>, AiError> {
    let headers = [("x-goog-api-key", key)];
    let mut out = Vec::new();
    let mut token: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let url = match &token {
            Some(t) => format!("{base}/v1beta/models?pageSize=1000&pageToken={}", encode(t)),
            None => format!("{base}/v1beta/models?pageSize=1000"),
        };
        let page = http.get(&url, &headers)?;
        for m in page
            .get("models")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let methods = m
                .get("supportedGenerationMethods")
                .or_else(|| m.get("supportedActions"))
                .and_then(Value::as_array);
            let generates = methods
                .is_none_or(|list| list.iter().any(|v| v.as_str() == Some("generateContent")));
            let name = m.get("name").and_then(Value::as_str).unwrap_or_default();
            let id = name.strip_prefix("models/").unwrap_or(name);
            if generates && !id.is_empty() {
                out.push(model(id, m.get("displayName").and_then(Value::as_str)));
            }
        }
        token = page
            .get("nextPageToken")
            .and_then(Value::as_str)
            .map(str::to_owned);
        if token.is_none() {
            break;
        }
    }
    Ok(out)
}

/// `GET /api/tags` — models pulled on the local Ollama server.
fn ollama(http: &JsonHttp, base: &str) -> Result<Vec<AiModel>, AiError> {
    let page = http
        .get(&format!("{base}/api/tags"), &[])
        .map_err(|e| match e {
            AiError::Network(msg) => {
                AiError::Network(format!("{msg} (o Ollama está rodando em {base}?)"))
            }
            other => other,
        })?;
    let mut out: Vec<AiModel> = page
        .get("models")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|m| {
            m.get("name")
                .and_then(Value::as_str)
                .map(|id| model(id, None))
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Minimal percent-encoding for opaque page tokens.
fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_non_text_models() {
        assert!(is_text_model("gpt-6-astra"));
        assert!(is_text_model("claude-opus-5-5"));
        assert!(!is_text_model("text-embedding-3-large"));
        assert!(!is_text_model("gpt-6-realtime"));
        assert!(!is_text_model("gemini-3.8-flash-tts"));
        assert!(!is_text_model("gemini-3.8-live"));
    }

    #[test]
    fn encodes_page_tokens() {
        assert_eq!(encode("a+b/c=="), "a%2Bb%2Fc%3D%3D");
    }
}
