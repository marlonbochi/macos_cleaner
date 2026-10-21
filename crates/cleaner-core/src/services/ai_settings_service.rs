//! Use case: configure the AI provider (FR-017). Provider/model/base URL are stored in the
//! settings repository; the API key only ever goes to the `SecretStore` (Keychain).

use std::sync::Arc;

use serde_json::json;

use crate::domain::ai::{AiModel, AiProviderKind, AiSettings};
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
        settings.validate()?;
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

    /// Moves API keys kept under a previous identity of the app (before it was renamed) into the
    /// current store. A key already present in the current store wins. Returns how many keys
    /// were imported; the legacy entries are deleted either way.
    pub fn import_api_keys(&self, legacy: &dyn SecretStore) -> Result<usize, CoreError> {
        let mut imported = 0;
        for kind in AiProviderKind::ALL {
            let key = kind.secret_key();
            let Some(value) = legacy.get(&key)? else {
                continue;
            };
            if self.secrets.get(&key)?.is_none() {
                self.secrets.set(&key, &value)?;
                imported += 1;
            }
            legacy.delete(&key)?;
        }
        Ok(imported)
    }

    /// Whether the system has a secret store, i.e. whether API keys can be saved at all.
    pub fn can_store_api_keys(&self) -> bool {
        self.secrets.is_available()
    }

    pub fn has_api_key(&self, provider: AiProviderKind) -> Result<bool, CoreError> {
        Ok(self.secrets.get(&provider.secret_key())?.is_some())
    }

    /// Lists the models available to the user's credentials, so the UI offers only valid ids.
    ///
    /// `api_key` lets the user try a key before saving it; otherwise the stored key is used.
    pub fn list_models(
        &self,
        kind: AiProviderKind,
        base_url: Option<String>,
        api_key: Option<String>,
    ) -> Result<Vec<AiModel>, CoreError> {
        let probe = AiSettings {
            provider: Some(kind),
            model: None,
            base_url,
        };
        probe.validate()?;
        let base = probe.effective_base_url().ok_or(AiError::NotConfigured)?;
        let key = match api_key
            .map(|k| k.trim().to_owned())
            .filter(|k| !k.is_empty())
        {
            Some(typed) => Some(typed),
            None => self.secrets.get(&kind.secret_key())?,
        };
        if kind.needs_api_key() && key.is_none() {
            return Err(AiError::MissingApiKey(kind.label()).into());
        }
        Ok(self.factory.list_models(kind, &base, key)?)
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
            system: "Reply only with the requested JSON.",
            user: "Reply with {\"ok\": true}.",
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
    use crate::ports::ai::SecretError;
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
    fn says_whether_api_keys_can_be_stored() {
        #[derive(Debug)]
        struct NoStore;
        impl SecretStore for NoStore {
            fn get(&self, _: &str) -> Result<Option<String>, SecretError> {
                Ok(None)
            }
            fn set(&self, _: &str, _: &str) -> Result<(), SecretError> {
                Err(SecretError("no keyring".into()))
            }
            fn delete(&self, _: &str) -> Result<(), SecretError> {
                Ok(())
            }
            fn is_available(&self) -> bool {
                false
            }
        }

        let (s, _) = service(FakeAiFactory::replying(json!({})));
        assert!(s.can_store_api_keys());

        let without = AiSettingsService::new(
            Arc::new(InMemorySettingsRepository::default()),
            Arc::new(NoStore),
            Arc::new(FakeAiFactory::replying(json!({}))),
        );
        assert!(!without.can_store_api_keys());
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
    fn imports_keys_from_the_previous_identity_once() {
        let (s, secrets) = service(FakeAiFactory::replying(json!({})));
        let legacy = InMemorySecretStore::default();
        legacy
            .set(&AiProviderKind::Anthropic.secret_key(), "sk-old")
            .unwrap();
        legacy
            .set(&AiProviderKind::OpenAi.secret_key(), "sk-old-openai")
            .unwrap();
        secrets
            .set(&AiProviderKind::OpenAi.secret_key(), "sk-new-openai")
            .unwrap();

        assert_eq!(s.import_api_keys(&legacy).unwrap(), 1);
        assert_eq!(
            secrets
                .get(&AiProviderKind::Anthropic.secret_key())
                .unwrap()
                .as_deref(),
            Some("sk-old")
        );
        assert_eq!(
            secrets
                .get(&AiProviderKind::OpenAi.secret_key())
                .unwrap()
                .as_deref(),
            Some("sk-new-openai"),
            "the current key wins"
        );
        assert!(
            legacy
                .get(&AiProviderKind::Anthropic.secret_key())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            s.import_api_keys(&legacy).unwrap(),
            0,
            "nothing left to import"
        );
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
    fn invalid_model_or_url_is_rejected_on_save() {
        let (s, _) = service(FakeAiFactory::replying(json!({})));
        let bad_url = AiSettings {
            base_url: Some("http://evil.example.com".into()),
            ..claude()
        };
        assert!(matches!(
            s.save(&bad_url, ApiKeyChange::Keep),
            Err(CoreError::Domain(_))
        ));
        let bad_model = AiSettings {
            model: Some("x; rm".into()),
            ..claude()
        };
        assert!(matches!(
            s.save(&bad_model, ApiKeyChange::Keep),
            Err(CoreError::Domain(_))
        ));
    }

    #[test]
    fn lists_models_with_typed_or_stored_key() {
        let factory = FakeAiFactory::replying(json!({}));
        let (s, secrets) = service(factory.clone());

        assert!(matches!(
            s.list_models(AiProviderKind::Gemini, None, None),
            Err(CoreError::Ai(AiError::MissingApiKey(_)))
        ));

        let models = s
            .list_models(AiProviderKind::Gemini, None, Some(" typed ".into()))
            .unwrap();
        assert_eq!(models[0].id, "fake-model");
        secrets.set("ai.gemini.api_key", "stored").unwrap();
        s.list_models(AiProviderKind::Gemini, None, None).unwrap();
        s.list_models(
            AiProviderKind::Ollama,
            Some("http://box:11434".into()),
            None,
        )
        .unwrap();

        assert_eq!(
            factory.listings(),
            [
                (
                    AiProviderKind::Gemini,
                    "https://generativelanguage.googleapis.com".to_owned(),
                    Some("typed".to_owned())
                ),
                (
                    AiProviderKind::Gemini,
                    "https://generativelanguage.googleapis.com".to_owned(),
                    Some("stored".to_owned())
                ),
                (AiProviderKind::Ollama, "http://box:11434".to_owned(), None),
            ]
        );
        assert!(
            s.list_models(
                AiProviderKind::OpenAi,
                Some("ftp://x".into()),
                Some("k".into())
            )
            .is_err()
        );
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
