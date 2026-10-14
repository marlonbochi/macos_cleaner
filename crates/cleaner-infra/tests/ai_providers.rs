//! Contract tests of the AI adapters against a local mock HTTP server (no network, no keys).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use cleaner_core::domain::ai::{AiProviderKind, AiSettings};
use cleaner_core::ports::ai::{AiError, AiProvider, AiProviderFactory, StructuredRequest};
use cleaner_infra::ai::HttpAiProviderFactory;
use mockito::{Matcher, Server};
use serde_json::{Value, json};

const ANSWER: &str = r#"{"ok":true}"#;

fn schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["ok"],"properties":{"ok":{"type":"boolean"}}})
}

fn provider(
    kind: AiProviderKind,
    model: &str,
    base: &str,
    key: Option<&str>,
) -> Box<dyn AiProvider> {
    let settings = AiSettings {
        provider: Some(kind),
        model: Some(model.into()),
        base_url: Some(base.into()),
    };
    HttpAiProviderFactory::new(Duration::from_secs(5), 2, Duration::ZERO)
        .create(&settings, key.map(str::to_owned))
        .unwrap()
}

fn ask(p: &dyn AiProvider) -> Result<Value, AiError> {
    let schema = schema();
    p.complete_json(&StructuredRequest {
        system: "sys",
        user: "user message",
        schema: &schema,
        schema_name: "test",
    })
}

#[test]
fn anthropic_sends_structured_output_request_and_reads_text_block() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/v1/messages")
        .match_header("x-api-key", "sk-ant")
        .match_header("anthropic-version", "2023-06-01")
        .match_header("anthropic-beta", "server-side-fallback-2026-07-01")
        .match_body(Matcher::PartialJson(json!({
            "model": "claude-opus-5-5",
            "system": "sys",
            "messages": [{"role": "user", "content": "user message"}],
            "output_config": {"format": {"type": "json_schema", "schema": schema()}},
            "fallbacks": "default"
        })))
        .with_body(
            json!({
                "stop_reason": "end_turn",
                "content": [{"type": "thinking", "thinking": ""}, {"type": "text", "text": ANSWER}]
            })
            .to_string(),
        )
        .create();

    let p = provider(
        AiProviderKind::Anthropic,
        "claude-opus-5-5",
        &server.url(),
        Some("sk-ant"),
    );
    assert_eq!(ask(p.as_ref()).unwrap(), json!({"ok": true}));
    mock.assert();
}

#[test]
fn anthropic_models_without_fallback_support_do_not_send_it() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/v1/messages")
        .match_header("anthropic-beta", Matcher::Missing)
        .match_request(|req| !req.utf8_lossy_body().unwrap().contains("fallbacks"))
        .with_body(json!({"content": [{"type": "text", "text": ANSWER}]}).to_string())
        .create();
    let p = provider(
        AiProviderKind::Anthropic,
        "claude-haiku-5-5",
        &server.url(),
        Some("k"),
    );
    ask(p.as_ref()).unwrap();
    mock.assert();
}

#[test]
fn anthropic_refusal_and_truncation_are_errors() {
    let mut server = Server::new();
    server
        .mock("POST", "/v1/messages")
        .with_body(json!({"stop_reason": "refusal", "content": []}).to_string())
        .create();
    let p = provider(
        AiProviderKind::Anthropic,
        "claude-opus-5-5",
        &server.url(),
        Some("k"),
    );
    assert_eq!(ask(p.as_ref()), Err(AiError::Refused));

    let mut server = Server::new();
    server
        .mock("POST", "/v1/messages")
        .with_body(
            json!({"stop_reason": "max_tokens", "content": [{"type": "text", "text": "{"}]})
                .to_string(),
        )
        .create();
    let p = provider(
        AiProviderKind::Anthropic,
        "claude-opus-5-5",
        &server.url(),
        Some("k"),
    );
    assert!(matches!(ask(p.as_ref()), Err(AiError::InvalidResponse(_))));
}

#[test]
fn openai_uses_json_schema_response_format_and_accepts_v1_base_urls() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/v1/chat/completions")
        .match_header("authorization", "Bearer sk-oai")
        .match_body(Matcher::PartialJson(json!({
            "model": "gpt-5",
            "messages": [{"role": "system", "content": "sys"}, {"role": "user", "content": "user message"}],
            "response_format": {"type": "json_schema", "json_schema": {"name": "test", "strict": true}}
        })))
        .with_body(
            json!({"choices": [{"finish_reason": "stop", "message": {"content": ANSWER, "refusal": null}}]})
                .to_string(),
        )
        .expect(2)
        .create();

    let p = provider(
        AiProviderKind::OpenAi,
        "gpt-5",
        &server.url(),
        Some("sk-oai"),
    );
    assert_eq!(ask(p.as_ref()).unwrap(), json!({"ok": true}));
    let p = provider(
        AiProviderKind::OpenAi,
        "gpt-5",
        &format!("{}/v1", server.url()),
        Some("sk-oai"),
    );
    assert_eq!(ask(p.as_ref()).unwrap(), json!({"ok": true}));
    mock.assert();
}

#[test]
fn openai_refusal_is_an_error() {
    let mut server = Server::new();
    server
        .mock("POST", "/v1/chat/completions")
        .with_body(
            json!({"choices": [{"message": {"content": null, "refusal": "no"}}]}).to_string(),
        )
        .create();
    let p = provider(AiProviderKind::OpenAi, "gpt-5", &server.url(), Some("k"));
    assert_eq!(ask(p.as_ref()), Err(AiError::Refused));
}

#[test]
fn gemini_puts_the_schema_in_instructions_and_joins_parts() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/v1beta/models/gemini-2.5-flash:generateContent")
        .match_header("x-goog-api-key", "g-key")
        .match_body(Matcher::PartialJson(json!({
            "contents": [{"role": "user", "parts": [{"text": "user message"}]}],
            "generationConfig": {"responseMimeType": "application/json"}
        })))
        .match_body(Matcher::Regex("JSON Schema".into()))
        .with_body(
            json!({"candidates": [{"finishReason": "STOP", "content": {"parts": [{"text": "{\"ok\":"}, {"text": "true}"}]}}]})
                .to_string(),
        )
        .create();
    let p = provider(
        AiProviderKind::Gemini,
        "gemini-2.5-flash",
        &server.url(),
        Some("g-key"),
    );
    assert_eq!(ask(p.as_ref()).unwrap(), json!({"ok": true}));
    mock.assert();
}

#[test]
fn gemini_safety_block_is_a_refusal() {
    let mut server = Server::new();
    server
        .mock("POST", Matcher::Any)
        .with_body(json!({"candidates": [{"finishReason": "SAFETY"}]}).to_string())
        .create();
    let p = provider(
        AiProviderKind::Gemini,
        "gemini-2.5-flash",
        &server.url(),
        Some("k"),
    );
    assert_eq!(ask(p.as_ref()), Err(AiError::Refused));
}

#[test]
fn ollama_sends_schema_as_format_and_tolerates_code_fences() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/api/chat")
        .match_header("authorization", Matcher::Missing)
        .match_body(Matcher::PartialJson(
            json!({"model": "llama3.2", "stream": false, "format": schema()}),
        ))
        .with_body(json!({"message": {"content": "```json\n{\"ok\": true}\n```"}}).to_string())
        .create();
    let p = provider(AiProviderKind::Ollama, "llama3.2", &server.url(), None);
    assert_eq!(ask(p.as_ref()).unwrap(), json!({"ok": true}));
    mock.assert();
}

#[test]
fn ollama_unreachable_mentions_the_server() {
    // Port 9 (discard) is closed on a normal machine: connection refused.
    let p = provider(
        AiProviderKind::Ollama,
        "llama3.2",
        "http://127.0.0.1:9",
        None,
    );
    match ask(p.as_ref()) {
        Err(AiError::Network(msg)) => assert!(msg.contains("Ollama")),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn http_errors_map_to_typed_errors() {
    let mut server = Server::new();
    server
        .mock("POST", "/v1/messages")
        .with_status(401)
        .with_body("{}")
        .create();
    let p = provider(
        AiProviderKind::Anthropic,
        "claude-opus-5-5",
        &server.url(),
        Some("bad"),
    );
    assert_eq!(ask(p.as_ref()), Err(AiError::Unauthorized));

    let mut server = Server::new();
    server
        .mock("POST", "/v1/messages")
        .with_status(400)
        .with_body(r#"{"type":"error","error":{"type":"invalid_request_error","message":"model: not found"}}"#)
        .create();
    let p = provider(
        AiProviderKind::Anthropic,
        "claude-nope",
        &server.url(),
        Some("k"),
    );
    assert_eq!(
        ask(p.as_ref()),
        Err(AiError::Provider {
            status: 400,
            message: "model: not found".into()
        })
    );
}

#[test]
fn transient_failures_are_retried_then_reported() {
    // 1 attempt + 2 retries, then the typed error surfaces.
    let mut server = Server::new();
    let limited = server
        .mock("POST", "/v1/messages")
        .with_status(429)
        .with_body("{}")
        .expect(3)
        .create();
    let p = provider(
        AiProviderKind::Anthropic,
        "claude-opus-5-5",
        &server.url(),
        Some("k"),
    );
    assert_eq!(ask(p.as_ref()), Err(AiError::RateLimited));
    limited.assert();

    let mut server = Server::new();
    let overloaded = server
        .mock("POST", "/v1/messages")
        .with_status(529)
        .with_body("{}")
        .expect(3)
        .create();
    let p = provider(
        AiProviderKind::Anthropic,
        "claude-opus-5-5",
        &server.url(),
        Some("k"),
    );
    assert!(matches!(
        ask(p.as_ref()),
        Err(AiError::Provider { status: 529, .. })
    ));
    overloaded.assert();
}

#[test]
fn client_errors_are_not_retried() {
    let mut server = Server::new();
    let bad = server
        .mock("POST", "/v1/messages")
        .with_status(400)
        .with_body("{}")
        .expect(1)
        .create();
    let p = provider(
        AiProviderKind::Anthropic,
        "claude-opus-5-5",
        &server.url(),
        Some("k"),
    );
    assert!(ask(p.as_ref()).is_err());
    bad.assert();
}
