//! SQLite persistence: connection management, migrations and repository adapters.

mod assessment_repository;
mod convert;
mod file_entry_repository;
mod scan_repository;
mod settings_repository;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use cleaner_core::error::RepoError;
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

pub use assessment_repository::SqliteAssessmentRepository;
pub use file_entry_repository::SqliteFileEntryRepository;
pub use scan_repository::SqliteScanRepository;
pub use settings_repository::SqliteSettingsRepository;

/// Ordered list of schema migrations. Append only.
fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(include_str!("migrations/V001__initial.sql")),
        M::up(include_str!("migrations/V002__assessment_details.sql")),
    ])
}

/// Owns the SQLite connections.
///
/// WAL mode lets the UI read (through `reader`) while a scan writes (through `writer`),
/// so long batched inserts never block the interface.
#[derive(Debug)]
pub struct Database {
    writer: Mutex<Connection>,
    reader: Mutex<Connection>,
}

impl Database {
    /// Opens (creating if needed) the database file and applies pending migrations.
    pub fn open(path: &Path) -> Result<Self, RepoError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| RepoError::Storage(e.to_string()))?;
        }
        let mut writer = Connection::open(path).map_err(storage)?;
        configure(&writer)?;
        migrations()
            .to_latest(&mut writer)
            .map_err(|e| RepoError::Storage(e.to_string()))?;
        let reader = Connection::open(path).map_err(storage)?;
        configure(&reader)?;
        Ok(Self {
            writer: Mutex::new(writer),
            reader: Mutex::new(reader),
        })
    }

    pub(crate) fn writer(&self) -> Result<MutexGuard<'_, Connection>, RepoError> {
        self.writer
            .lock()
            .map_err(|_| RepoError::Storage("writer connection poisoned".into()))
    }

    pub(crate) fn reader(&self) -> Result<MutexGuard<'_, Connection>, RepoError> {
        self.reader
            .lock()
            .map_err(|_| RepoError::Storage("reader connection poisoned".into()))
    }
}

fn configure(conn: &Connection) -> Result<(), RepoError> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;",
    )
    .map_err(storage)
}

// Takes the error by value so it can be passed directly to `map_err`.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn storage(e: rusqlite::Error) -> RepoError {
    RepoError::Storage(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        assert!(migrations().validate().is_ok());
    }
}
