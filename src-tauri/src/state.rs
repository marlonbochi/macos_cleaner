//! Application state shared by all commands, built once by the composition root.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use cleaner_core::cancel::CancellationToken;
use cleaner_core::domain::scan::ScanId;
use cleaner_core::policy::protected_paths::ProtectedPathPolicy;
use cleaner_core::services::catalog_service::CatalogService;
use cleaner_core::services::scan_service::{ScanConfig, ScanService};
use cleaner_infra::SystemClock;
use cleaner_infra::db::{Database, SqliteFileEntryRepository, SqliteScanRepository};
use cleaner_infra::fs::JwalkScanner;

use crate::error::{AppError, ErrorCode};

const CATALOG_FILE: &str = "catalog.sqlite3";

#[derive(Debug)]
pub struct AppState {
    pub scan: ScanService,
    pub catalog: CatalogService,
    pub home: PathBuf,
    pub running: RunningScans,
}

/// Tracks the scan currently running so it can be cancelled. Cheap to clone into worker threads.
#[derive(Debug, Clone, Default)]
pub struct RunningScans(Arc<Mutex<Option<RunningScan>>>);

#[derive(Debug, Clone)]
struct RunningScan {
    id: ScanId,
    cancel: CancellationToken,
}

impl AppState {
    /// Composition root: wires adapters into the core services (dependency injection by hand).
    pub fn build(app_data_dir: &Path, home: PathBuf) -> Result<Self, AppError> {
        let db = Arc::new(
            Database::open(&app_data_dir.join(CATALOG_FILE))
                .map_err(|e| AppError::new(ErrorCode::Storage, e.to_string()))?,
        );
        let scans = Arc::new(SqliteScanRepository::new(db.clone()));
        let files = Arc::new(SqliteFileEntryRepository::new(db));
        let policy = Arc::new(ProtectedPathPolicy::macos_default(&home, app_data_dir));

        let scan = ScanService::new(
            Arc::new(JwalkScanner::default()),
            scans.clone(),
            files.clone(),
            policy,
            Arc::new(SystemClock),
            ScanConfig::default(),
        );
        let recovered = scan.recover_interrupted()?;
        if recovered > 0 {
            tracing::info!(recovered, "marked interrupted scans as failed");
        }
        Ok(Self {
            scan,
            catalog: CatalogService::new(scans, files),
            home,
            running: RunningScans::default(),
        })
    }
}

impl RunningScans {
    pub fn set(&self, id: ScanId, cancel: CancellationToken) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(RunningScan { id, cancel });
        }
    }

    pub fn clear(&self, id: ScanId) {
        if let Ok(mut slot) = self.0.lock()
            && slot.as_ref().is_some_and(|r| r.id == id)
        {
            *slot = None;
        }
    }

    /// Returns `false` when `id` is not the scan currently running.
    pub fn cancel(&self, id: ScanId) -> bool {
        let Ok(slot) = self.0.lock() else {
            return false;
        };
        match slot.as_ref() {
            Some(running) if running.id == id => {
                running.cancel.cancel();
                true
            }
            _ => false,
        }
    }
}
