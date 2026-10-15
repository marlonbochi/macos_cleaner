//! Per-system wiring: where the app keeps its data, which paths are protected, how secrets are
//! stored and how the app itself is uninstalled on macOS, Windows and Linux.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use cleaner_core::domain::footprint::{AppRemoval, FootprintKind, FootprintLocation};
use cleaner_core::domain::platform::Platform;
use cleaner_core::policy::protected_paths::ProtectedPathPolicy;
use cleaner_core::ports::ai::{SecretError, SecretStore};
use cleaner_core::services::ai_settings_service::AiSettingsService;

/// The app's own folders, as resolved by Tauri for the current system.
#[derive(Debug, Clone)]
pub struct AppDirs {
    pub data: PathBuf,
    pub local_data: PathBuf,
    pub config: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
}

/// Name of the Debian package built by the release workflow.
#[cfg(target_os = "linux")]
const DEB_PACKAGE: &str = "disk-cleaner";

/// Identifier of the app before it was renamed to Disk Cleaner (macOS only, v0.1.x).
#[cfg(target_os = "macos")]
pub const LEGACY_BUNDLE_ID: &str = "com.marlonbochi.macoscleaner";

/// Paths the app must never act on.
pub fn protected_paths(home: &Path, app_data_dir: &Path) -> ProtectedPathPolicy {
    ProtectedPathPolicy::for_platform(Platform::current(), home, app_data_dir, &system_drive())
}

/// Drive Windows runs from (`C:\`); unused elsewhere.
pub fn system_drive() -> PathBuf {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    PathBuf::from(format!("{drive}\\"))
}

/// The system's secret store. When it is unavailable (e.g. no keyring daemon on a minimal Linux
/// desktop) the app keeps working without AI keys and says why when the user tries to save one.
pub fn secret_store(service: &str) -> Arc<dyn SecretStore> {
    match cleaner_infra::secrets::system_secret_store(service) {
        Ok(store) => store,
        Err(e) => {
            tracing::warn!(error = %e, "secret store unavailable");
            Arc::new(UnavailableSecretStore(e.0))
        }
    }
}

#[derive(Debug)]
struct UnavailableSecretStore(String);

impl SecretStore for UnavailableSecretStore {
    fn get(&self, _key: &str) -> Result<Option<String>, SecretError> {
        Ok(None)
    }

    fn set(&self, _key: &str, _value: &str) -> Result<(), SecretError> {
        Err(SecretError(format!(
            "no secret store available: {}",
            self.0
        )))
    }

    fn delete(&self, _key: &str) -> Result<(), SecretError> {
        Ok(())
    }
}

/// Folders scanned by default and the quick-pick locations offered on the scan screen.
#[derive(Debug, Clone)]
pub struct ScanPresets {
    pub default_roots: Vec<PathBuf>,
    pub applications: Vec<PathBuf>,
    pub disks: Vec<PathBuf>,
}

pub fn scan_presets(home: &Path) -> ScanPresets {
    match Platform::current() {
        Platform::MacOs => ScanPresets {
            default_roots: vec![home.to_path_buf(), "/Applications".into()],
            applications: vec!["/Applications".into()],
            disks: vec!["/".into()],
        },
        Platform::Windows => {
            let drive = system_drive();
            ScanPresets {
                default_roots: vec![home.to_path_buf()],
                applications: vec![
                    drive.join("Program Files"),
                    drive.join("Program Files (x86)"),
                ]
                .into_iter()
                .filter(|p| p.is_dir())
                .collect(),
                disks: (b'A'..=b'Z')
                    .map(|letter| PathBuf::from(format!("{}:\\", char::from(letter))))
                    .filter(|p| p.is_dir())
                    .collect(),
            }
        }
        Platform::Linux => ScanPresets {
            default_roots: vec![home.to_path_buf()],
            applications: vec!["/usr".into(), "/opt".into()]
                .into_iter()
                .filter(|p: &PathBuf| p.is_dir())
                .collect(),
            disks: vec!["/".into()],
        },
    }
}

/// Everything the app may leave on disk and how the app itself is removed (feature 009).
/// Locations that fail validation are left out rather than risk removing something else.
pub fn uninstall_plan(
    dirs: &AppDirs,
    home: &Path,
    bundle_id: &str,
) -> (Vec<FootprintLocation>, AppRemoval) {
    let mut candidates: Vec<(FootprintKind, PathBuf, String)> = vec![
        (FootprintKind::AppData, dirs.data.clone(), bundle_id.into()),
        (
            FootprintKind::AppData,
            dirs.local_data.clone(),
            bundle_id.into(),
        ),
        (
            FootprintKind::AppData,
            dirs.config.clone(),
            bundle_id.into(),
        ),
        (FootprintKind::Cache, dirs.cache.clone(), bundle_id.into()),
        (FootprintKind::Logs, dirs.logs.clone(), bundle_id.into()),
    ];
    let removal = app_removal(home, bundle_id, &mut candidates);
    let mut seen = Vec::new();
    let locations = candidates
        .into_iter()
        .filter(|(_, path, _)| {
            // Several Tauri folders are the same on some systems (data = config on macOS).
            let new = !seen.contains(path);
            seen.push(path.clone());
            new
        })
        .filter_map(
            |(kind, path, id)| match FootprintLocation::new(kind, path, &id) {
                Ok(location) => Some(location),
                Err(e) => {
                    tracing::debug!(error = %e, "skipping uninstall location");
                    None
                }
            },
        )
        .collect();
    (locations, removal)
}

#[cfg(target_os = "macos")]
fn app_removal(
    home: &Path,
    bundle_id: &str,
    candidates: &mut Vec<(FootprintKind, PathBuf, String)>,
) -> AppRemoval {
    use cleaner_infra::macos;

    let library = home.join("Library");
    for (id, legacy) in [(bundle_id, false), (LEGACY_BUNDLE_ID, true)] {
        let kind = |k| if legacy { FootprintKind::Legacy } else { k };
        candidates.extend([
            (
                kind(FootprintKind::AppData),
                library.join("Application Support").join(id),
                id.into(),
            ),
            (
                kind(FootprintKind::Cache),
                library.join("Caches").join(id),
                id.into(),
            ),
            (
                kind(FootprintKind::WebData),
                library.join("WebKit").join(id),
                id.into(),
            ),
            (
                kind(FootprintKind::HttpStorage),
                library.join("HTTPStorages").join(id),
                id.into(),
            ),
            (
                kind(FootprintKind::Logs),
                library.join("Logs").join(id),
                id.into(),
            ),
            (
                kind(FootprintKind::Preferences),
                library.join("Preferences").join(format!("{id}.plist")),
                id.into(),
            ),
            (
                kind(FootprintKind::SavedState),
                library
                    .join("Saved Application State")
                    .join(format!("{id}.savedState")),
                id.into(),
            ),
        ]);
    }
    if let Some(app) = macos::installed_app_bundle(LEGACY_BUNDLE_ID) {
        candidates.push((FootprintKind::Legacy, app, LEGACY_BUNDLE_ID.into()));
    }
    match macos::running_app_bundle(bundle_id) {
        Some(app) => {
            candidates.push((FootprintKind::AppBundle, app, bundle_id.into()));
            AppRemoval::Trash
        }
        None => AppRemoval::NotInstalled,
    }
}

#[cfg(windows)]
fn app_removal(
    _home: &Path,
    _bundle_id: &str,
    _candidates: &mut Vec<(FootprintKind, PathBuf, String)>,
) -> AppRemoval {
    // The NSIS installer puts `uninstall.exe` next to the app. `/P` runs it without questions
    // (the user already confirmed in the app) and `/DELETEAPPDATA` makes it delete the data
    // folders too (see `windows/hooks.nsh`).
    let uninstaller = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("uninstall.exe")))
        .filter(|path| path.is_file());
    match uninstaller {
        Some(program) => AppRemoval::Uninstaller {
            program,
            args: vec!["/P".into(), "/DELETEAPPDATA".into()],
        },
        None => AppRemoval::NotInstalled,
    }
}

#[cfg(target_os = "linux")]
fn app_removal(
    _home: &Path,
    _bundle_id: &str,
    _candidates: &mut Vec<(FootprintKind, PathBuf, String)>,
) -> AppRemoval {
    // Installed from the .deb package: only apt (as administrator) can remove it.
    let from_package = std::env::current_exe().is_ok_and(|exe| exe.starts_with("/usr/"));
    if from_package {
        AppRemoval::PackageManager {
            command: format!("sudo apt remove {DEB_PACKAGE}"),
        }
    } else {
        AppRemoval::NotInstalled
    }
}

/// Moves the data folder of the app's previous name into place (macOS only). Must run before
/// the catalogue database is opened, or a fresh one would be created instead.
#[cfg(target_os = "macos")]
pub fn adopt_legacy_data(app_data_dir: &Path) {
    let legacy = app_data_dir.with_file_name(LEGACY_BUNDLE_ID);
    match cleaner_infra::fs::adopt_legacy_dir(&legacy, app_data_dir) {
        Ok(true) => {
            tracing::info!(from = %legacy.display(), "adopted data from the previous app name");
        }
        Ok(false) => {}
        Err(e) => tracing::warn!(error = %e, "could not adopt data from the previous app name"),
    }
}

/// Moves API keys saved under the app's previous name (macOS only).
#[cfg(target_os = "macos")]
pub fn import_legacy_keys(ai_settings: &AiSettingsService) {
    let legacy_keys = cleaner_infra::macos::KeychainSecretStore::new(LEGACY_BUNDLE_ID);
    match ai_settings.import_api_keys(&legacy_keys) {
        Ok(0) => {}
        Ok(imported) => tracing::info!(imported, "imported API keys from the previous app name"),
        Err(e) => {
            tracing::warn!(error = %e, "could not import API keys from the previous app name");
        }
    }
}

/// Nothing to migrate: the app was never released under another name on this system.
#[cfg(not(target_os = "macos"))]
pub fn adopt_legacy_data(_app_data_dir: &Path) {}

/// Nothing to migrate: the app was never released under another name on this system.
#[cfg(not(target_os = "macos"))]
pub fn import_legacy_keys(_ai_settings: &AiSettingsService) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_folders_are_listed_once() {
        let base = std::env::temp_dir();
        let data = base.join("com.example.app");
        let dirs = AppDirs {
            data: data.clone(),
            local_data: data.clone(),
            config: data.clone(),
            cache: base.join("cache").join("com.example.app"),
            logs: base.join("logs").join("com.example.app"),
        };
        let (locations, _) = uninstall_plan(&dirs, &base, "com.example.app");
        let data_count = locations.iter().filter(|l| l.path() == data).count();
        assert_eq!(data_count, 1);
        assert!(
            locations
                .iter()
                .all(|l| l.path().to_string_lossy().contains("com.example.app")
                    || l.kind() == FootprintKind::AppBundle
                    || l.kind() == FootprintKind::Legacy)
        );
    }

    #[test]
    fn scan_presets_include_the_home_folder() {
        let home = std::env::temp_dir();
        let presets = scan_presets(&home);
        assert_eq!(presets.default_roots.first(), Some(&home));
        assert!(!presets.disks.is_empty());
    }
}
