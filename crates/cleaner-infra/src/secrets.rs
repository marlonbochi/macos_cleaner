//! Where API keys are kept on each system: the macOS Keychain, the Windows Credential Manager or
//! the freedesktop Secret Service (GNOME Keyring, KWallet) on Linux.

use std::sync::Arc;

use cleaner_core::ports::ai::{SecretError, SecretStore};
use keyring_core::{CredentialStore, Entry, Error};

/// `SecretStore` over any `keyring-core` credential store; each key is a credential of `service`.
pub struct KeyringSecretStore {
    store: Arc<CredentialStore>,
    service: String,
}

impl std::fmt::Debug for KeyringSecretStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyringSecretStore")
            .field("service", &self.service)
            .finish_non_exhaustive()
    }
}

impl KeyringSecretStore {
    pub fn new(store: Arc<CredentialStore>, service: impl Into<String>) -> Self {
        Self {
            store,
            service: service.into(),
        }
    }

    fn entry(&self, key: &str) -> Result<Entry, SecretError> {
        self.store
            .build(&self.service, key, None)
            .map_err(|e| SecretError(e.to_string()))
    }
}

impl SecretStore for KeyringSecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, SecretError> {
        match self.entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(SecretError(e.to_string())),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<(), SecretError> {
        self.entry(key)?
            .set_password(value)
            .map_err(|e| SecretError(e.to_string()))
    }

    fn delete(&self, key: &str) -> Result<(), SecretError> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(e) => Err(SecretError(e.to_string())),
        }
    }
}

/// The system's secret store, with secrets filed under `service` (the app's identifier).
#[cfg(target_os = "macos")]
pub fn system_secret_store(service: &str) -> Result<Arc<dyn SecretStore>, SecretError> {
    Ok(Arc::new(crate::macos::KeychainSecretStore::new(service)))
}

/// The system's secret store, with secrets filed under `service` (the app's identifier).
#[cfg(windows)]
pub fn system_secret_store(service: &str) -> Result<Arc<dyn SecretStore>, SecretError> {
    let store =
        windows_native_keyring_store::Store::new().map_err(|e| SecretError(e.to_string()))?;
    Ok(Arc::new(KeyringSecretStore::new(store, service)))
}

/// The system's secret store, with secrets filed under `service` (the app's identifier).
#[cfg(target_os = "linux")]
pub fn system_secret_store(service: &str) -> Result<Arc<dyn SecretStore>, SecretError> {
    let store =
        zbus_secret_service_keyring_store::Store::new().map_err(|e| SecretError(e.to_string()))?;
    Ok(Arc::new(KeyringSecretStore::new(store, service)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_reads_and_deletes_secrets() {
        let store = KeyringSecretStore::new(
            keyring_core::mock::Store::new().unwrap(),
            "com.example.cleaner",
        );
        assert_eq!(store.get("ai.openai.api_key").unwrap(), None);

        store.set("ai.openai.api_key", "sk-1").unwrap();
        assert_eq!(
            store.get("ai.openai.api_key").unwrap().as_deref(),
            Some("sk-1")
        );
        store.set("ai.openai.api_key", "sk-2").unwrap();
        assert_eq!(
            store.get("ai.openai.api_key").unwrap().as_deref(),
            Some("sk-2")
        );
        assert_eq!(
            store.get("ai.gemini.api_key").unwrap(),
            None,
            "keys are separate"
        );

        store.delete("ai.openai.api_key").unwrap();
        assert_eq!(store.get("ai.openai.api_key").unwrap(), None);
        store.delete("ai.openai.api_key").unwrap();
    }
}
