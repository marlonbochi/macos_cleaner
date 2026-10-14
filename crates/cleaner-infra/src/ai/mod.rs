//! AI provider adapters (Claude, OpenAI, Gemini, Ollama) and the factory that builds them.

mod http;
mod providers;

use std::time::Duration;

use cleaner_core::domain::ai::{AiProviderKind, AiSettings};
use cleaner_core::ports::ai::{AiError, AiProvider, AiProviderFactory};

use self::http::JsonHttp;
use self::providers::{Anthropic, Gemini, Ollama, OpenAi};

/// Builds HTTP-backed providers from the user's settings (Factory pattern).
#[derive(Debug, Clone)]
pub struct HttpAiProviderFactory {
    timeout: Duration,
    retries: u32,
    backoff: Duration,
}

impl Default for HttpAiProviderFactory {
    fn default() -> Self {
        // Large models can take a while to answer a single analysis.
        Self::new(Duration::from_secs(180), 2, Duration::from_secs(1))
    }
}

impl HttpAiProviderFactory {
    pub fn new(timeout: Duration, retries: u32, backoff: Duration) -> Self {
        Self {
            timeout,
            retries,
            backoff,
        }
    }
}

impl AiProviderFactory for HttpAiProviderFactory {
    fn create(
        &self,
        settings: &AiSettings,
        api_key: Option<String>,
    ) -> Result<Box<dyn AiProvider>, AiError> {
        let kind = settings.provider.ok_or(AiError::NotConfigured)?;
        let model = settings.effective_model().ok_or(AiError::NotConfigured)?;
        let base_url = settings
            .effective_base_url()
            .ok_or(AiError::NotConfigured)?;
        validate(&model, &base_url)?;
        let http = JsonHttp::new(self.timeout, self.retries, self.backoff)?;
        let key = || {
            api_key
                .clone()
                .filter(|k| !k.trim().is_empty())
                .ok_or(AiError::MissingApiKey(kind.label()))
        };
        Ok(match kind {
            AiProviderKind::Anthropic => Box::new(Anthropic {
                http,
                base_url,
                api_key: key()?,
                model,
            }),
            AiProviderKind::OpenAi => Box::new(OpenAi {
                http,
                base_url,
                api_key: key()?,
                model,
            }),
            AiProviderKind::Gemini => Box::new(Gemini {
                http,
                base_url,
                api_key: key()?,
                model,
            }),
            AiProviderKind::Ollama => Box::new(Ollama {
                http,
                base_url,
                model,
            }),
        })
    }
}

/// The model id is interpolated into URLs (Gemini) and the base URL is user-provided.
fn validate(model: &str, base_url: &str) -> Result<(), AiError> {
    let model_ok = model
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/'));
    if !model_ok || model.contains("..") {
        return Err(AiError::InvalidConfig(format!("invalid model id: {model}")));
    }
    if !(base_url.starts_with("https://") || base_url.starts_with("http://")) {
        return Err(AiError::InvalidConfig(
            "base URL must start with http:// or https://".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(kind: AiProviderKind, model: &str, base: &str) -> AiSettings {
        AiSettings {
            provider: Some(kind),
            model: Some(model.into()),
            base_url: Some(base.into()),
        }
    }

    #[test]
    fn cloud_providers_need_a_key() {
        let f = HttpAiProviderFactory::default();
        let s = settings(AiProviderKind::Gemini, "gemini-3.8-flash", "https://x");
        assert!(matches!(f.create(&s, None), Err(AiError::MissingApiKey(_))));
        assert!(matches!(
            f.create(&s, Some("  ".into())),
            Err(AiError::MissingApiKey(_))
        ));
        assert_eq!(
            f.create(&s, Some("k".into())).unwrap().descriptor().0,
            "gemini"
        );
    }

    #[test]
    fn ollama_needs_no_key() {
        let f = HttpAiProviderFactory::default();
        let s = settings(
            AiProviderKind::Ollama,
            "llama3.2:3b",
            "http://localhost:11434",
        );
        assert_eq!(
            f.create(&s, None).unwrap().descriptor(),
            ("ollama".into(), "llama3.2:3b".into())
        );
    }

    #[test]
    fn rejects_unsafe_models_and_urls() {
        let f = HttpAiProviderFactory::default();
        for (model, base) in [
            ("../../etc", "https://x"),
            ("a b", "https://x"),
            ("ok", "file:///tmp"),
        ] {
            let s = settings(AiProviderKind::Anthropic, model, base);
            assert!(matches!(
                f.create(&s, Some("k".into())),
                Err(AiError::InvalidConfig(_))
            ));
        }
        assert!(matches!(
            f.create(&AiSettings::default(), None),
            Err(AiError::NotConfigured)
        ));
    }
}
