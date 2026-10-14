//! Use case: configure the AI provider (FR-017). Provider/model/base URL are stored in the
//! settings repository; the API key only ever goes to the `SecretStore` (Keychain).

use std::sync::Arc;

use serde_json::json;

use crate::domain::ai::{AiProviderKind, AiSettings};
use crate::error::CoreError;
use crate::ports::ai::{AiError, AiProvider, AiProviderFactory, SecretStore, StructuredRequest};
use crate::ports::repository::SettingsRepository;

const SETTINGS_KEY: &str = "ai";

/// What to do with the stored API key when saving settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeyChange {
    Keep,
    Set(String),
    Clear,
}

#[derive(Clone)]
pub struct AiSettingsService {
    settings: Arc<dyn SettingsRepository>,
    secrets: Arc<dyn SecretStore>,
    factory: Arc<dyn AiProviderFactory>,
}

impl std::fmt::Debug for AiSettingsService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiSettingsService").finish_non_exhaustive()
    }
}

impl AiSettingsService {
    pub fn new(
        settings: Arc<dyn SettingsRepository>,
        secrets: Arc<dyn SecretStore>,
        factory: Arc<dyn AiProviderFactory>,
    ) -> Self {
        Self {
            settings,
            secrets,
            factory,
        }
    }

    pub fn load(&self) -> Result<AiSettings, CoreError> {
        Ok(self
            .settings
            .get(SETTINGS_KEY)?
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default())
    }

    pub fn save(&self, settings: &AiSettings, key: ApiKeyChange) -> Result<(), CoreError> {
        if let Some(provider) = settings.provider {
            match key {
                ApiKeyChange::Keep => {}
                ApiKeyChange::Set(value) if !value.trim().is_empty() => {
                    self.secrets.set(&provider.secret_key(), value.trim())?;
                }
                ApiKeyChange::Set(_) | ApiKeyChange::Clear => {
                    self.secrets.delete(&provider.secret_key())?;
                }
            }
        }
        let value = serde_json::to_value(settings)
            .map_err(|e| crate::error::DomainError::InvalidInput(e.to_string()))?;
        self.settings.set(SETTINGS_KEY, &value)?;
        Ok(())
    }

    pub fn has_api_key(&self, provider: AiProviderKind) -> Result<bool, CoreError> {
        Ok(self.secrets.get(&provider.secret_key())?.is_some())
    }

    /// The configured provider, or `None` when the user chose local rules only.
    pub fn provider(&self) -> Result<Option<Box<dyn AiProvider>>, CoreError> {
        let settings = self.load()?;
        let Some(kind) = settings.provider else {
            return Ok(None);
        };
        let key = self.secrets.get(&kind.secret_key())?;
        if kind.needs_api_key() && key.is_none() {
            return Err(AiError::MissingApiKey(kind.label()).into());
        }
        Ok(Some(self.factory.create(&settings, key)?))
    }

    /// Sends a tiny structured request to check credentials, model and connectivity.
    pub fn test_connection(&self) -> Result<(String, String), CoreError> {
        let provider = self.provider()?.ok_or(AiError::NotConfigured)?;
        let schema = json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["ok"],
            "properties": { "ok": { "type": "boolean" } }
        });
        let reply = provider.complete_json(&StructuredRequest {
            system: "Responda apenas com o JSON pedido.",
            user: "Responda {\"ok\": true}.",
            schema: &schema,
            schema_name: "connection_test",
        })?;
        if reply.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
            return Err(AiError::InvalidResponse(reply.to_string()).into());
        }
        Ok(provider.descriptor())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeAiFactory, InMemorySecretStore, InMemorySettingsRepository};

    fn service(factory: FakeAiFactory) -> (AiSettingsService, Arc<InMemorySecretStore>) {
        let secrets = Arc::new(InMemorySecretStore::default());
        let service = AiSettingsService::new(
            Arc::new(InMemorySettingsRepository::default()),
            secrets.clone(),
            Arc::new(factory),
        );
        (service, secrets)
    }

    fn claude() -> AiSettings {
        AiSettings {
            provider: Some(AiProviderKind::Anthropic),
            model: None,
            base_url: None,
        }
    }

    #[test]
    fn defaults_to_local_rules_only() {
        let (s, _) = service(FakeAiFactory::replying(json!({})));
        assert_eq!(s.load().unwrap(), AiSettings::default());
        assert!(s.provider().unwrap().is_none());
    }

    #[test]
    fn key_goes_to_the_secret_store_not_the_settings() {
        let (s, secrets) = service(FakeAiFactory::replying(json!({})));
        s.save(&claude(), ApiKeyChange::Set("  sk-test  ".into()))
            .unwrap();

        assert_eq!(
            secrets.get("ai.anthropic.api_key").unwrap().as_deref(),
            Some("sk-test")
        );
        assert!(
            !s.settings
                .get("ai")
                .unwrap()
                .unwrap()
                .to_string()
                .contains("sk-test")
        );
        assert!(s.has_api_key(AiProviderKind::Anthropic).unwrap());

        s.save(&claude(), ApiKeyChange::Keep).unwrap();
        assert!(s.has_api_key(AiProviderKind::Anthropic).unwrap());
        s.save(&claude(), ApiKeyChange::Clear).unwrap();
        assert!(!s.has_api_key(AiProviderKind::Anthropic).unwrap());
    }

    #[test]
    fn cloud_provider_without_key_is_an_error_but_ollama_is_not() {
        let (s, _) = service(FakeAiFactory::replying(json!({"ok": true})));
        s.save(&claude(), ApiKeyChange::Keep).unwrap();
        assert!(matches!(
            s.provider(),
            Err(CoreError::Ai(AiError::MissingApiKey(_)))
        ));

        let ollama = AiSettings {
            provider: Some(AiProviderKind::Ollama),
            ..AiSettings::default()
        };
        s.save(&ollama, ApiKeyChange::Keep).unwrap();
        assert!(s.provider().unwrap().is_some());
    }

    #[test]
    fn connection_test_requires_the_expected_reply() {
        let (s, _) = service(FakeAiFactory::replying(json!({"ok": true})));
        s.save(&claude(), ApiKeyChange::Set("k".into())).unwrap();
        assert_eq!(
            s.test_connection().unwrap(),
            ("fake".into(), "fake-model".into())
        );

        let (s, _) = service(FakeAiFactory::replying(json!({"ok": false})));
        s.save(&claude(), ApiKeyChange::Set("k".into())).unwrap();
        assert!(s.test_connection().is_err());

        let (s, _) = service(FakeAiFactory::failing(AiError::Unauthorized));
        s.save(&claude(), ApiKeyChange::Set("k".into())).unwrap();
        assert!(matches!(
            s.test_connection(),
            Err(CoreError::Ai(AiError::Unauthorized))
        ));
    }
}
