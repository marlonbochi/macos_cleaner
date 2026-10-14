//! AI provider configuration (FR-017: cloud or local provider, or none).

use serde::{Deserialize, Serialize};

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

/// Persisted AI settings. `provider: None` means "local rules only".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSettings {
    pub provider: Option<AiProviderKind>,
    pub model: Option<String>,
    pub base_url: Option<String>,
}

impl AiSettings {
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
