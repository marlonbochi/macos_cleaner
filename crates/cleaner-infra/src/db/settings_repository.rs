use std::sync::Arc;

use cleaner_core::error::RepoError;
use cleaner_core::ports::repository::SettingsRepository;
use rusqlite::{OptionalExtension, params};

use super::{Database, storage};

/// Key/value settings stored as JSON text. Never used for secrets (those go to the Keychain).
#[derive(Debug, Clone)]
pub struct SqliteSettingsRepository {
    db: Arc<Database>,
}

impl SqliteSettingsRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

impl SettingsRepository for SqliteSettingsRepository {
    fn get(&self, key: &str) -> Result<Option<serde_json::Value>, RepoError> {
        let raw: Option<String> = self
            .db
            .reader()?
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .map_err(storage)?;
        raw.map(|text| serde_json::from_str(&text).map_err(|e| RepoError::Storage(e.to_string())))
            .transpose()
    }

    fn set(&self, key: &str, value: &serde_json::Value) -> Result<(), RepoError> {
        self.db
            .writer()?
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value.to_string()],
            )
            .map(|_| ())
            .map_err(storage)
    }
}
