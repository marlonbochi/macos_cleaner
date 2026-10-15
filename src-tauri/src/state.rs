//! Application state shared by all commands, built once by the composition root.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use cleaner_core::cancel::CancellationToken;
use cleaner_core::domain::footprint::{FootprintKind, FootprintLocation};
use cleaner_core::domain::scan::ScanId;
use cleaner_core::policy::protected_paths::ProtectedPathPolicy;
use cleaner_core::services::ai_settings_service::AiSettingsService;
use cleaner_core::services::analysis_service::AnalysisService;
use cleaner_core::services::catalog_service::CatalogService;
use cleaner_core::services::diagnosis_service::DiagnosisService;
use cleaner_core::services::preferences_service::PreferencesService;
use cleaner_core::services::scan_service::{ScanConfig, ScanService};
use cleaner_core::services::uninstall_service::UninstallService;
use cleaner_infra::SystemClock;
use cleaner_infra::ai::HttpAiProviderFactory;
use cleaner_infra::db::{
    Database, SqliteAssessmentRepository, SqliteFileEntryRepository, SqliteScanRepository,
    SqliteSettingsRepository,
};
use cleaner_infra::fs::{JwalkScanner, StdDiskUsage};
use cleaner_infra::macos::{self, KeychainSecretStore, MacTrash};

use crate::error::{AppError, ErrorCode};

const CATALOG_FILE: &str = "catalog.sqlite3";
/// Identifier of the app before it was renamed to Disk Cleaner (v0.1.x, "macOS Cleaner").
const LEGACY_BUNDLE_ID: &str = "com.marlonbochi.macoscleaner";

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
    pub fn build(app_data_dir: &Path, home: PathBuf, bundle_id: &str) -> Result<Self, AppError> {
        adopt_legacy_data(app_data_dir);
        let db = Arc::new(
            Database::open(&app_data_dir.join(CATALOG_FILE))
                .map_err(|e| AppError::new(ErrorCode::Storage, e.to_string()))?,
        );
        let scans = Arc::new(SqliteScanRepository::new(db.clone()));
        let files = Arc::new(SqliteFileEntryRepository::new(db.clone()));
        let policy = Arc::new(ProtectedPathPolicy::macos_default(&home, app_data_dir));

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
        let secrets = Arc::new(KeychainSecretStore::new(bundle_id));
        let ai_settings = AiSettingsService::new(
            settings_repo,
            secrets.clone(),
            Arc::new(HttpAiProviderFactory::default()),
        );
        match ai_settings.import_api_keys(&KeychainSecretStore::new(LEGACY_BUNDLE_ID)) {
            Ok(0) => {}
            Ok(imported) => {
                tracing::info!(imported, "imported API keys from the previous app name");
            }
            Err(e) => {
                tracing::warn!(error = %e, "could not import API keys from the previous app name");
            }
        }
        let uninstall = UninstallService::new(
            footprint_locations(app_data_dir, &home, bundle_id),
            Arc::new(StdDiskUsage),
            Arc::new(MacTrash),
            secrets,
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

/// Keeps the catalogue and settings of users upgrading from the previous app name.
fn adopt_legacy_data(app_data_dir: &Path) {
    let legacy = app_data_dir.with_file_name(LEGACY_BUNDLE_ID);
    match cleaner_infra::fs::adopt_legacy_dir(&legacy, app_data_dir) {
        Ok(true) => {
            tracing::info!(from = %legacy.display(), "adopted data from the previous app name");
        }
        Ok(false) => {}
        Err(e) => tracing::warn!(error = %e, "could not adopt data from the previous app name"),
    }
}

/// Every place the app may leave data, for a complete uninstall (feature 009), including what
/// the app left under its previous name. Locations that fail validation are left out rather
/// than risk removing something else.
fn footprint_locations(
    app_data_dir: &Path,
    home: &Path,
    bundle_id: &str,
) -> Vec<FootprintLocation> {
    let library = home.join("Library");
    let mut candidates: Vec<(FootprintKind, PathBuf, &str)> = vec![
        (
            FootprintKind::AppData,
            app_data_dir.to_path_buf(),
            bundle_id,
        ),
        (
            FootprintKind::Legacy,
            app_data_dir.with_file_name(LEGACY_BUNDLE_ID),
            LEGACY_BUNDLE_ID,
        ),
    ];
    for (id, legacy) in [(bundle_id, false), (LEGACY_BUNDLE_ID, true)] {
        let kind = |k| if legacy { FootprintKind::Legacy } else { k };
        candidates.extend([
            (
                kind(FootprintKind::Cache),
                library.join("Caches").join(id),
                id,
            ),
            (
                kind(FootprintKind::WebData),
                library.join("WebKit").join(id),
                id,
            ),
            (
                kind(FootprintKind::HttpStorage),
                library.join("HTTPStorages").join(id),
                id,
            ),
            (kind(FootprintKind::Logs), library.join("Logs").join(id), id),
            (
                kind(FootprintKind::Preferences),
                library.join("Preferences").join(format!("{id}.plist")),
                id,
            ),
            (
                kind(FootprintKind::SavedState),
                library
                    .join("Saved Application State")
                    .join(format!("{id}.savedState")),
                id,
            ),
        ]);
    }
    if let Some(app) = macos::installed_app_bundle(LEGACY_BUNDLE_ID) {
        candidates.push((FootprintKind::Legacy, app, LEGACY_BUNDLE_ID));
    }
    if let Some(app) = macos::running_app_bundle(bundle_id) {
        candidates.push((FootprintKind::AppBundle, app, bundle_id));
    }
    candidates
        .into_iter()
        .filter_map(
            |(kind, path, id)| match FootprintLocation::new(kind, path, id) {
                Ok(location) => Some(location),
                Err(e) => {
                    tracing::warn!(error = %e, "skipping uninstall location");
                    None
                }
            },
        )
        .collect()
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
