//! One adapter per provider. Each translates a `StructuredRequest` into the provider's wire
//! format and extracts the JSON answer.

use cleaner_core::analysis::prompt::parse_json_text;
use cleaner_core::ports::ai::{AiError, AiProvider, StructuredRequest};
use serde_json::{Value, json};

use super::http::{JsonHttp, require_str};

/// Generous output budget for a single non-streaming analysis.
const MAX_OUTPUT_TOKENS: u32 = 16_000;

/// Models that accept Anthropic's server-side refusal fallback (`fallbacks: "default"`).
const ANTHROPIC_FALLBACK_MODELS: &[&str] = &[
    "claude-opus-5-5",
    "claude-opus-5",
    "claude-fable-5-1",
    "claude-sonnet-5-5",
];

#[derive(Debug)]
pub(crate) struct Anthropic {
    pub(crate) http: JsonHttp,
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) model: String,
}

impl AiProvider for Anthropic {
    fn complete_json(&self, request: &StructuredRequest<'_>) -> Result<Value, AiError> {
        let mut body = json!({
            "model": self.model,
            "max_tokens": MAX_OUTPUT_TOKENS,
            "system": request.system,
            "messages": [{ "role": "user", "content": request.user }],
            "output_config": { "format": { "type": "json_schema", "schema": request.schema } },
        });
        let mut headers = vec![
            ("x-api-key", self.api_key.as_str()),
            ("anthropic-version", "2023-06-01"),
        ];
        if ANTHROPIC_FALLBACK_MODELS.contains(&self.model.as_str()) {
            // On a safety decline, Anthropic re-runs the request on a recommended model.
            body["fallbacks"] = json!("default");
            headers.push(("anthropic-beta", "server-side-fallback-2026-07-01"));
        }
        let reply = self
            .http
            .post(&format!("{}/v1/messages", self.base_url), &headers, &body)?;
        match reply.get("stop_reason").and_then(Value::as_str) {
            Some("refusal") => return Err(AiError::Refused),
            Some("max_tokens") => {
                return Err(AiError::InvalidResponse("response truncated".into()));
            }
            _ => {}
        }
        let text = reply
            .get("content")
            .and_then(Value::as_array)
            .and_then(|blocks| {
                blocks
                    .iter()
                    .find(|b| b.get("type").and_then(Value::as_str) == Some("text"))
            })
            .and_then(|b| b.get("text"))
            .and_then(Value::as_str)
            .ok_or_else(|| AiError::InvalidResponse("no text block".into()))?;
        parse_json_text(text)
    }

    fn descriptor(&self) -> (String, String) {
        ("anthropic".into(), self.model.clone())
    }
}

#[derive(Debug)]
pub(crate) struct OpenAi {
    pub(crate) http: JsonHttp,
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) model: String,
}

/// Uses the Responses API (`/v1/responses`), where OpenAI documents its current models and
/// Structured Outputs (`text.format` with a strict JSON schema).
impl AiProvider for OpenAi {
    fn complete_json(&self, request: &StructuredRequest<'_>) -> Result<Value, AiError> {
        let body = json!({
            "model": self.model,
            "input": [
                { "role": "system", "content": request.system },
                { "role": "user", "content": request.user },
            ],
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": request.schema_name,
                    "schema": request.schema,
                    "strict": true,
                },
            },
        });
        // Accept both "https://api.openai.com" and ".../v1" base URLs.
        let base = self.base_url.trim_end_matches("/v1");
        let auth = format!("Bearer {}", self.api_key);
        let reply = self.http.post(
            &format!("{base}/v1/responses"),
            &[("authorization", auth.as_str())],
            &body,
        )?;
        if reply.get("status").and_then(Value::as_str) == Some("incomplete") {
            return Err(AiError::InvalidResponse("response truncated".into()));
        }
        // output[] → items of type "message" → content[] of "output_text" or "refusal".
        let mut text = String::new();
        for item in reply
            .get("output")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            for part in item
                .get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                match part.get("type").and_then(Value::as_str) {
                    Some("refusal") => return Err(AiError::Refused),
                    Some("output_text") => {
                        text.push_str(part.get("text").and_then(Value::as_str).unwrap_or_default());
                    }
                    _ => {}
                }
            }
        }
        if text.is_empty() {
            return Err(AiError::InvalidResponse(
                "no output_text in response".into(),
            ));
        }
        parse_json_text(&text)
    }

    fn descriptor(&self) -> (String, String) {
        ("openai".into(), self.model.clone())
    }
}

#[derive(Debug)]
pub(crate) struct Gemini {
    pub(crate) http: JsonHttp,
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) model: String,
}

impl AiProvider for Gemini {
    fn complete_json(&self, request: &StructuredRequest<'_>) -> Result<Value, AiError> {
        // Gemini's schema dialect differs from JSON Schema, so the schema goes in the
        // instructions and the answer is validated client-side like every other provider.
        let system = format!(
            "{}\n\nResponda SOMENTE com JSON válido que siga este JSON Schema:\n{}",
            request.system, request.schema
        );
        let body = json!({
            "systemInstruction": { "parts": [{ "text": system }] },
            "contents": [{ "role": "user", "parts": [{ "text": request.user }] }],
            "generationConfig": { "responseMimeType": "application/json" },
        });
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, self.model
        );
        let reply = self
            .http
            .post(&url, &[("x-goog-api-key", self.api_key.as_str())], &body)?;
        if reply.pointer("/promptFeedback/blockReason").is_some() {
            return Err(AiError::Refused);
        }
        match reply
            .pointer("/candidates/0/finishReason")
            .and_then(Value::as_str)
        {
            Some("SAFETY" | "PROHIBITED_CONTENT" | "BLOCKLIST" | "SPII" | "RECITATION") => {
                return Err(AiError::Refused);
            }
            Some("MAX_TOKENS") => {
                return Err(AiError::InvalidResponse("response truncated".into()));
            }
            _ => {}
        }
        let text: String = reply
            .pointer("/candidates/0/content/parts")
            .and_then(Value::as_array)
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|p| p.get("text")?.as_str())
                    .collect()
            })
            .unwrap_or_default();
        parse_json_text(&text)
    }

    fn descriptor(&self) -> (String, String) {
        ("gemini".into(), self.model.clone())
    }
}

#[derive(Debug)]
pub(crate) struct Ollama {
    pub(crate) http: JsonHttp,
    pub(crate) base_url: String,
    pub(crate) model: String,
}

impl AiProvider for Ollama {
    fn complete_json(&self, request: &StructuredRequest<'_>) -> Result<Value, AiError> {
        let body = json!({
            "model": self.model,
            "stream": false,
            "format": request.schema,
            "options": { "temperature": 0 },
            "messages": [
                { "role": "system", "content": request.system },
                { "role": "user", "content": request.user },
            ],
        });
        let reply = self
            .http
            .post(&format!("{}/api/chat", self.base_url), &[], &body)
            .map_err(|e| match e {
                AiError::Network(msg) => {
                    AiError::Network(format!("{msg} (is Ollama running at {}?)", self.base_url))
                }
                other => other,
            })?;
        parse_json_text(require_str(&reply, "/message/content")?)
    }

    fn descriptor(&self) -> (String, String) {
        ("ollama".into(), self.model.clone())
    }
}
