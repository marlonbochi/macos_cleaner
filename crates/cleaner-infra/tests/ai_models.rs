//! Contract tests of the "list models" calls against a local mock HTTP server.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use cleaner_core::domain::ai::{AiModel, AiProviderKind};
use cleaner_core::ports::ai::{AiError, AiProviderFactory};
use cleaner_infra::ai::HttpAiProviderFactory;
use mockito::{Matcher, Server};
use serde_json::json;

fn factory() -> HttpAiProviderFactory {
    HttpAiProviderFactory::new(Duration::from_secs(5), 0, Duration::ZERO)
}

fn ids(models: &[AiModel]) -> Vec<&str> {
    models.iter().map(|m| m.id.as_str()).collect()
}

#[test]
fn anthropic_lists_models_across_pages_with_display_names() {
    let mut server = Server::new();
    let first = server
        .mock("GET", "/v1/models")
        .match_query(Matcher::UrlEncoded("limit".into(), "1000".into()))
        .match_header("x-api-key", "sk-ant")
        .match_header("anthropic-version", "2023-06-01")
        .with_body(
            json!({
                "data": [{"id": "claude-opus-5-5", "display_name": "Claude Opus 5.5"}],
                "has_more": true, "last_id": "claude-opus-5-5"
            })
            .to_string(),
        )
        .create();
    let second = server
        .mock("GET", "/v1/models")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("limit".into(), "1000".into()),
            Matcher::UrlEncoded("after_id".into(), "claude-opus-5-5".into()),
        ]))
        .with_body(
            json!({"data": [{"id": "claude-haiku-5-5", "display_name": "Claude Haiku 5.5"}], "has_more": false})
                .to_string(),
        )
        .create();

    let models = factory()
        .list_models(
            AiProviderKind::Anthropic,
            &server.url(),
            Some("sk-ant".into()),
        )
        .unwrap();

    assert_eq!(ids(&models), ["claude-opus-5-5", "claude-haiku-5-5"]);
    assert_eq!(models[0].label, "Claude Opus 5.5");
    first.assert();
    second.assert();
}

#[test]
fn openai_sorts_newest_first_and_drops_non_text_models() {
    let mut server = Server::new();
    server
        .mock("GET", "/v1/models")
        .match_header("authorization", "Bearer sk-oai")
        .with_body(
            json!({"object": "list", "data": [
                {"id": "gpt-6-luna", "created": 200},
                {"id": "text-embedding-3-large", "created": 300},
                {"id": "gpt-6-astra", "created": 400},
                {"id": "gpt-5-2025-08-07", "created": 100, "shutdown_date": "2026-12-11"},
                {"id": "gpt-6-realtime", "created": 500}
            ]})
            .to_string(),
        )
        .create();

    let models = factory()
        .list_models(AiProviderKind::OpenAi, &server.url(), Some("sk-oai".into()))
        .unwrap();

    assert_eq!(
        ids(&models),
        ["gpt-6-astra", "gpt-6-luna", "gpt-5-2025-08-07"]
    );
    assert_eq!(
        models[2].label,
        "gpt-5-2025-08-07 (desativação em 2026-12-11)"
    );
}

#[test]
fn gemini_keeps_generate_content_models_and_follows_page_tokens() {
    let mut server = Server::new();
    server
        .mock("GET", "/v1beta/models")
        .match_query(Matcher::Exact("pageSize=1000".into()))
        .match_header("x-goog-api-key", "g-key")
        .with_body(
            json!({
                "models": [
                    {"name": "models/gemini-3.8-flash", "displayName": "Gemini 3.8 Flash", "supportedGenerationMethods": ["generateContent", "countTokens"]},
                    {"name": "models/gemini-embedding-2", "supportedGenerationMethods": ["embedContent"]},
                    {"name": "models/gemini-3.8-live", "supportedGenerationMethods": ["bidiGenerateContent"]}
                ],
                "nextPageToken": "tok+1"
            })
            .to_string(),
        )
        .create();
    server
        .mock("GET", "/v1beta/models")
        .match_query(Matcher::Exact("pageSize=1000&pageToken=tok%2B1".into()))
        .with_body(
            json!({"models": [{"name": "models/gemini-3.5-flash-lite", "displayName": "Gemini 3.5 Flash-Lite", "supportedGenerationMethods": ["generateContent"]}]})
                .to_string(),
        )
        .create();

    let models = factory()
        .list_models(AiProviderKind::Gemini, &server.url(), Some("g-key".into()))
        .unwrap();

    assert_eq!(ids(&models), ["gemini-3.8-flash", "gemini-3.5-flash-lite"]);
    assert_eq!(models[0].label, "Gemini 3.8 Flash");
}

#[test]
fn ollama_lists_pulled_models_without_a_key() {
    let mut server = Server::new();
    server
        .mock("GET", "/api/tags")
        .match_header("authorization", Matcher::Missing)
        .with_body(
            json!({"models": [{"name": "qwen3:8b"}, {"name": "llama3.2:latest"}]}).to_string(),
        )
        .create();

    let models = factory()
        .list_models(AiProviderKind::Ollama, &server.url(), None)
        .unwrap();

    assert_eq!(ids(&models), ["llama3.2:latest", "qwen3:8b"]);
}

#[test]
fn listing_errors_are_typed() {
    let mut server = Server::new();
    server
        .mock("GET", "/v1/models")
        .with_status(401)
        .with_body("{}")
        .create();
    let f = factory();
    assert_eq!(
        f.list_models(AiProviderKind::OpenAi, &server.url(), Some("bad".into())),
        Err(AiError::Unauthorized)
    );
    assert!(matches!(
        f.list_models(AiProviderKind::Anthropic, &server.url(), None),
        Err(AiError::MissingApiKey(_))
    ));
    assert!(matches!(
        f.list_models(
            AiProviderKind::OpenAi,
            "http://evil.example.com",
            Some("k".into())
        ),
        Err(AiError::InvalidConfig(_))
    ));
    match f.list_models(AiProviderKind::Ollama, "http://127.0.0.1:9", None) {
        Err(AiError::Network(msg)) => assert!(msg.contains("Ollama")),
        other => panic!("unexpected {other:?}"),
    }
}
