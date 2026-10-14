//! AI ports. Adapters (Anthropic, OpenAI, Gemini, Ollama) live in `cleaner-infra::ai`.

use serde_json::Value;
use thiserror::Error;

use crate::domain::ai::AiSettings;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AiError {
    #[error("AI provider is not configured")]
    NotConfigured,
    #[error("invalid AI configuration: {0}")]
    InvalidConfig(String),
    #[error("missing API key for {0}")]
    MissingApiKey(&'static str),
    #[error("the provider rejected the API key")]
    Unauthorized,
    #[error("rate limited by the provider")]
    RateLimited,
    #[error("the provider declined the request")]
    Refused,
    #[error("could not reach the provider: {0}")]
    Network(String),
    #[error("provider error ({status}): {message}")]
    Provider { status: u16, message: String },
    #[error("invalid response from the provider: {0}")]
    InvalidResponse(String),
}

/// A single structured-output request: the provider must answer with JSON matching `schema`.
#[derive(Debug, Clone, PartialEq)]
pub struct StructuredRequest<'a> {
    pub system: &'a str,
    pub user: &'a str,
    /// JSON Schema (objects with `additionalProperties: false`).
    pub schema: &'a Value,
    /// Short name for the schema; some providers require one.
    pub schema_name: &'a str,
}

/// A configured model that can answer structured requests (Adapter pattern: one per provider).
pub trait AiProvider: Send + Sync {
    fn complete_json(&self, request: &StructuredRequest<'_>) -> Result<Value, AiError>;
    /// Provider id and model, recorded with every assessment.
    fn descriptor(&self) -> (String, String);
}

/// Builds the provider selected in the settings (Factory pattern).
pub trait AiProviderFactory: Send + Sync {
    fn create(
        &self,
        settings: &AiSettings,
        api_key: Option<String>,
    ) -> Result<Box<dyn AiProvider>, AiError>;
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("secret store error: {0}")]
pub struct SecretError(pub String);

/// OS-backed secret storage (Keychain on macOS). Secrets never go to SQLite.
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>, SecretError>;
    fn set(&self, key: &str, value: &str) -> Result<(), SecretError>;
    fn delete(&self, key: &str) -> Result<(), SecretError>;
}
