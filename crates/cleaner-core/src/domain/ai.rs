//! AI provider configuration (FR-017: cloud or local provider, or none).

use serde::{Deserialize, Serialize};

use crate::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiProviderKind {
    Anthropic,
    OpenAi,
    Gemini,
    Ollama,
}

impl AiProviderKind {
    pub const ALL: [Self; 4] = [Self::Anthropic, Self::OpenAi, Self::Gemini, Self::Ollama];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
            Self::Gemini => "gemini",
            Self::Ollama => "ollama",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.id() == value)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "Claude (Anthropic)",
            Self::OpenAi => "OpenAI",
            Self::Gemini => "Gemini (Google)",
            Self::Ollama => "Ollama (local)",
        }
    }

    /// Suggested model; the user can type any model id the provider accepts.
    pub const fn default_model(self) -> &'static str {
        match self {
            Self::Anthropic => "claude-opus-5-5",
            Self::OpenAi => "gpt-6-astra",
            Self::Gemini => "gemini-3.8-flash",
            Self::Ollama => "llama3.2",
        }
    }

    pub const fn default_base_url(self) -> &'static str {
        match self {
            Self::Anthropic => "https://api.anthropic.com",
            Self::OpenAi => "https://api.openai.com",
            Self::Gemini => "https://generativelanguage.googleapis.com",
            Self::Ollama => "http://localhost:11434",
        }
    }

    /// Local models need no credentials.
    pub const fn needs_api_key(self) -> bool {
        !matches!(self, Self::Ollama)
    }

    /// Key under which the API key is stored in the secret store (never in SQLite).
    pub fn secret_key(self) -> String {
        format!("ai.{}.api_key", self.id())
    }
}

/// A model offered by a provider, as returned by its "list models" API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiModel {
    pub id: String,
    /// Human-readable name (falls back to the id).
    pub label: String,
}

/// Model ids are interpolated into URLs (Gemini), so only a safe charset is accepted.
pub fn validate_model_id(model: &str) -> Result<(), DomainError> {
    let ok = !model.is_empty()
        && !model.contains("..")
        && model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/'));
    if ok {
        Ok(())
    } else {
        Err(DomainError::InvalidInput(format!(
            "invalid model id: {model}"
        )))
    }
}

/// Accepts only `http(s)://host[:port][/path]`. Providers that receive an API key must use
/// HTTPS unless the server is on this machine, so the key never travels in clear text.
pub fn validate_base_url(kind: AiProviderKind, url: &str) -> Result<(), DomainError> {
    let invalid = |why: &str| {
        Err(DomainError::InvalidInput(format!(
            "invalid server URL ({why}): {url}"
        )))
    };
    let (scheme, rest) = match url.split_once("://") {
        Some((s @ ("http" | "https"), rest)) => (s, rest),
        _ => return invalid("must start with http:// or https://"),
    };
    if url
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '?' | '#' | '@'))
    {
        return invalid("unexpected characters");
    }
    let host = rest.split(['/', ':']).next().unwrap_or_default();
    if host.is_empty() {
        return invalid("missing host");
    }
    let local = matches!(host, "localhost" | "127.0.0.1")
        || host.ends_with(".localhost")
        || rest.starts_with("[::1]");
    if scheme == "http" && kind.needs_api_key() && !local {
        return invalid("use https so the API key is not sent in clear text");
    }
    Ok(())
}

/// Persisted AI settings. `provider: None` means "local rules only".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSettings {
    pub provider: Option<AiProviderKind>,
    pub model: Option<String>,
    pub base_url: Option<String>,
}

impl AiSettings {
    /// Validates the user-provided model id and server URL (when set).
    pub fn validate(&self) -> Result<(), DomainError> {
        let Some(kind) = self.provider else {
            return Ok(());
        };
        if let Some(model) = self
            .model
            .as_deref()
            .map(str::trim)
            .filter(|m| !m.is_empty())
        {
            validate_model_id(model)?;
        }
        if let Some(url) = self.effective_base_url() {
            validate_base_url(kind, &url)?;
        }
        Ok(())
    }

    pub fn effective_model(&self) -> Option<String> {
        let provider = self.provider?;
        Some(
            self.model
                .as_deref()
                .map(str::trim)
                .filter(|m| !m.is_empty())
                .unwrap_or(provider.default_model())
                .to_owned(),
        )
    }

    pub fn effective_base_url(&self) -> Option<String> {
        let provider = self.provider?;
        Some(
            self.base_url
                .as_deref()
                .map(|u| u.trim().trim_end_matches('/'))
                .filter(|u| !u.is_empty())
                .unwrap_or(provider.default_base_url())
                .to_owned(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_ids_round_trip() {
        for kind in AiProviderKind::ALL {
            assert_eq!(AiProviderKind::parse(kind.id()), Some(kind));
        }
        assert_eq!(AiProviderKind::parse("nope"), None);
    }

    #[test]
    fn only_ollama_runs_without_a_key() {
        assert!(AiProviderKind::Anthropic.needs_api_key());
        assert!(!AiProviderKind::Ollama.needs_api_key());
        assert_eq!(AiProviderKind::Gemini.secret_key(), "ai.gemini.api_key");
    }

    #[test]
    fn model_ids_are_restricted_to_a_safe_charset() {
        for ok in [
            "claude-opus-5-5",
            "gemini-3.8-flash",
            "llama3.2:3b",
            "org/model_v1",
        ] {
            assert!(validate_model_id(ok).is_ok(), "{ok}");
        }
        for bad in ["", "a b", "../../x", "x?y", "m;rm"] {
            assert!(validate_model_id(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn base_urls_must_be_http_and_keys_need_https_unless_local() {
        use AiProviderKind::{Anthropic, Ollama};
        assert!(validate_base_url(Anthropic, "https://api.anthropic.com").is_ok());
        assert!(validate_base_url(Anthropic, "http://127.0.0.1:8080").is_ok());
        assert!(validate_base_url(Anthropic, "http://localhost:8080/v1").is_ok());
        assert!(validate_base_url(Anthropic, "http://proxy.example.com").is_err());
        assert!(validate_base_url(Ollama, "http://gpu-box:11434").is_ok());
        assert!(validate_base_url(Anthropic, "http://[::1]:8080").is_ok());
        assert!(validate_base_url(Anthropic, "http://[2001:db8::1]:8080").is_err());
        for bad in [
            "ftp://x",
            "api.anthropic.com",
            "https://",
            "https://a b",
            "https://x?k=1",
            "https://u@x",
        ] {
            assert!(validate_base_url(Anthropic, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn settings_validation_checks_model_and_url() {
        let mut s = AiSettings {
            provider: Some(AiProviderKind::OpenAi),
            ..AiSettings::default()
        };
        assert!(s.validate().is_ok());
        s.model = Some("bad model".into());
        assert!(s.validate().is_err());
        s.model = None;
        s.base_url = Some("http://evil.example.com".into());
        assert!(s.validate().is_err());
        assert!(AiSettings::default().validate().is_ok());
    }

    #[test]
    fn effective_values_fall_back_to_provider_defaults() {
        let settings = AiSettings {
            provider: Some(AiProviderKind::Ollama),
            model: Some("  ".into()),
            base_url: Some("http://box:11434/".into()),
        };
        assert_eq!(settings.effective_model().as_deref(), Some("llama3.2"));
        assert_eq!(
            settings.effective_base_url().as_deref(),
            Some("http://box:11434")
        );
        assert_eq!(AiSettings::default().effective_model(), None);
    }
}
