//! Application state shared by all commands, built once by the composition root.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cleaner_core::cancel::CancellationToken;
use cleaner_core::domain::scan::ScanId;
use cleaner_core::services::ai_settings_service::AiSettingsService;
use cleaner_core::services::analysis_service::AnalysisService;
use cleaner_core::services::catalog_service::CatalogService;
use cleaner_core::services::diagnosis_service::DiagnosisService;
use cleaner_core::services::preferences_service::PreferencesService;
use cleaner_core::services::scan_service::{ScanConfig, ScanService};
use cleaner_core::services::uninstall_service::{UninstallPorts, UninstallService};
use cleaner_infra::SystemClock;
use cleaner_infra::ai::HttpAiProviderFactory;
use cleaner_infra::db::{
    Database, SqliteAssessmentRepository, SqliteFileEntryRepository, SqliteScanRepository,
    SqliteSettingsRepository,
};
use cleaner_infra::fs::{JwalkScanner, StdDiskUsage};
use cleaner_infra::system::{ProcessLauncher, SystemTrash};

use crate::error::{AppError, ErrorCode};
use crate::platform::{self, AppDirs};

const CATALOG_FILE: &str = "catalog.sqlite3";

#[derive(Debug)]
pub struct AppState {
    pub scan: ScanService,
    pub catalog: CatalogService,
    pub analysis: AnalysisService,
    pub diagnosis: DiagnosisService,
    pub ai_settings: AiSettingsService,
    pub preferences: PreferencesService,
    pub uninstall: UninstallService,
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
    pub fn build(dirs: &AppDirs, home: PathBuf, bundle_id: &str) -> Result<Self, AppError> {
        let app_data_dir = dirs.data.as_path();
        platform::adopt_legacy_data(app_data_dir);
        let db = Arc::new(
            Database::open(&app_data_dir.join(CATALOG_FILE))
                .map_err(|e| AppError::new(ErrorCode::Storage, e.to_string()))?,
        );
        let scans = Arc::new(SqliteScanRepository::new(db.clone()));
        let files = Arc::new(SqliteFileEntryRepository::new(db.clone()));
        let policy = Arc::new(platform::protected_paths(&home, app_data_dir));

        let scan = ScanService::new(
            Arc::new(JwalkScanner::default()),
            scans.clone(),
            files.clone(),
            policy,
            Arc::new(SystemClock),
            ScanConfig::default(),
        );
        let settings_repo = Arc::new(SqliteSettingsRepository::new(db.clone()));
        let preferences = PreferencesService::new(settings_repo.clone());
        let secrets = platform::secret_store(bundle_id);
        let ai_settings = AiSettingsService::new(
            settings_repo,
            secrets.clone(),
            Arc::new(HttpAiProviderFactory::default()),
        );
        let (locations, app_removal) = platform::uninstall_plan(dirs, &home, bundle_id);
        let uninstall = UninstallService::new(
            locations,
            app_removal,
            UninstallPorts {
                usage: Arc::new(StdDiskUsage),
                trash: Arc::new(SystemTrash),
                secrets,
                launcher: Arc::new(ProcessLauncher),
            },
        );
        let analysis = AnalysisService::new(
            files.clone(),
            Arc::new(SqliteAssessmentRepository::new(db)),
            ai_settings.clone(),
            preferences.clone(),
            Arc::new(SystemClock),
            &home,
        );
        let diagnosis = DiagnosisService::new(files.clone(), analysis.clone(), ai_settings.clone());
        platform::import_legacy_keys(&ai_settings);
        let recovered = scan.recover_interrupted()?;
        if recovered > 0 {
            tracing::info!(recovered, "marked interrupted scans as failed");
        }
        Ok(Self {
            scan,
            catalog: CatalogService::new(scans, files),
            analysis,
            diagnosis,
            ai_settings,
            preferences,
            uninstall,
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
