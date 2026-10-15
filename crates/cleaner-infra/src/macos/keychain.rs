//! `SecretStore` backed by the macOS Keychain (generic passwords).

use cleaner_core::ports::ai::{SecretError, SecretStore};
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

/// `errSecItemNotFound`: the item does not exist.
const ITEM_NOT_FOUND: i32 = -25_300;

/// Stores each secret as a generic password under `service`, with the key as the account.
#[derive(Debug, Clone)]
pub struct KeychainSecretStore {
    service: String,
}

impl KeychainSecretStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }
}

impl SecretStore for KeychainSecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, SecretError> {
        match get_generic_password(&self.service, key) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| SecretError("stored secret is not UTF-8".into())),
            Err(e) if e.code() == ITEM_NOT_FOUND => Ok(None),
            Err(e) => Err(SecretError(e.to_string())),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<(), SecretError> {
        set_generic_password(&self.service, key, value.as_bytes())
            .map_err(|e| SecretError(e.to_string()))
    }

    fn delete(&self, key: &str) -> Result<(), SecretError> {
        match delete_generic_password(&self.service, key) {
            Ok(()) => Ok(()),
            Err(e) if e.code() == ITEM_NOT_FOUND => Ok(()),
            Err(e) => Err(SecretError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Touches the real login Keychain, so it only runs on demand:
    /// `cargo test -p cleaner-infra keychain -- --ignored`
    #[test]
    #[ignore = "uses the real macOS Keychain"]
    fn round_trips_a_secret() {
        let store = KeychainSecretStore::new("com.marlonbochi.diskcleaner.tests");
        store.delete("test.key").unwrap();
        assert_eq!(store.get("test.key").unwrap(), None);
        store.set("test.key", "s3cret").unwrap();
        assert_eq!(store.get("test.key").unwrap().as_deref(), Some("s3cret"));
        store.delete("test.key").unwrap();
        assert_eq!(store.get("test.key").unwrap(), None);
    }
}
