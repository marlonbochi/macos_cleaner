//! Use case: remove the app completely — its data, its Keychain entries and the app itself
//! (feature 009). Everything goes to the Trash, never deleted outright (Constitution I).
//!
//! The user must type the exact confirmation phrase; the check is repeated here so no caller
//! can skip it.

use std::path::PathBuf;
use std::sync::Arc;

use crate::domain::ai::AiProviderKind;
use crate::domain::footprint::{FootprintItem, FootprintKind, FootprintLocation};
use crate::error::{CoreError, DomainError};
use crate::ports::ai::SecretStore;
use crate::ports::system::{DiskUsage, Trash};

/// What the user must type to confirm the uninstall.
pub const CONFIRMATION_PHRASE: &str = "Disk Cleaner";

/// Everything a complete uninstall would remove right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppFootprint {
    /// Existing locations, app bundle last.
    pub items: Vec<FootprintItem>,
    /// Providers with an API key in the Keychain.
    pub stored_keys: Vec<AiProviderKind>,
    /// `false` when the app does not run from a `.app` bundle (development builds).
    pub bundle_found: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallFailure {
    pub target: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UninstallReport {
    pub trashed: Vec<PathBuf>,
    pub keys_removed: usize,
    pub failures: Vec<UninstallFailure>,
}

#[derive(Clone)]
pub struct UninstallService {
    locations: Vec<FootprintLocation>,
    usage: Arc<dyn DiskUsage>,
    trash: Arc<dyn Trash>,
    secrets: Arc<dyn SecretStore>,
}

impl std::fmt::Debug for UninstallService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UninstallService")
            .field("locations", &self.locations)
            .finish_non_exhaustive()
    }
}

impl UninstallService {
    /// `locations` are validated `FootprintLocation`s; the app bundle is always handled last.
    pub fn new(
        mut locations: Vec<FootprintLocation>,
        usage: Arc<dyn DiskUsage>,
        trash: Arc<dyn Trash>,
        secrets: Arc<dyn SecretStore>,
    ) -> Self {
        locations.sort_by_key(|l| l.kind() == FootprintKind::AppBundle);
        Self {
            locations,
            usage,
            trash,
            secrets,
        }
    }

    pub fn footprint(&self) -> Result<AppFootprint, CoreError> {
        let mut items = Vec::new();
        for location in &self.locations {
            let size = self
                .usage
                .size_on_disk(location.path())
                .map_err(|e| DomainError::InvalidInput(e.to_string()))?;
            if let Some(size) = size {
                items.push(FootprintItem {
                    kind: location.kind(),
                    path: location.path().to_path_buf(),
                    size,
                });
            }
        }
        let mut stored_keys = Vec::new();
        for kind in AiProviderKind::ALL {
            if self.secrets.get(&kind.secret_key())?.is_some() {
                stored_keys.push(kind);
            }
        }
        Ok(AppFootprint {
            items,
            stored_keys,
            bundle_found: self
                .locations
                .iter()
                .any(|l| l.kind() == FootprintKind::AppBundle),
        })
    }

    /// Removes the Keychain entries, then moves every existing location to the Trash.
    /// Keeps going after a failure and reports every item that could not be removed.
    pub fn uninstall(&self, confirmation: &str) -> Result<UninstallReport, CoreError> {
        if confirmation.trim() != CONFIRMATION_PHRASE {
            return Err(DomainError::InvalidInput(format!(
                "type \"{CONFIRMATION_PHRASE}\" to confirm the uninstall"
            ))
            .into());
        }
        let mut report = UninstallReport::default();
        for kind in AiProviderKind::ALL {
            let key = kind.secret_key();
            match self.secrets.get(&key).and_then(|found| {
                found
                    .map(|_| self.secrets.delete(&key).map(|()| true))
                    .unwrap_or(Ok(false))
            }) {
                Ok(true) => report.keys_removed += 1,
                Ok(false) => {}
                Err(e) => report.failures.push(UninstallFailure {
                    target: key,
                    reason: e.to_string(),
                }),
            }
        }
        for location in &self.locations {
            let path = location.path();
            match self.usage.size_on_disk(path) {
                Ok(None) => continue,
                Ok(Some(_)) => {}
                Err(e) => {
                    report.failures.push(failure(path, &e));
                    continue;
                }
            }
            match self.trash.move_to_trash(path) {
                Ok(()) => report.trashed.push(path.to_path_buf()),
                Err(e) => report.failures.push(failure(path, &e)),
            }
        }
        tracing::info!(
            trashed = report.trashed.len(),
            keys = report.keys_removed,
            failures = report.failures.len(),
            "uninstall finished"
        );
        Ok(report)
    }
}

fn failure(path: &std::path::Path, e: &std::io::Error) -> UninstallFailure {
    UninstallFailure {
        target: path.display().to_string(),
        reason: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io;
    use std::path::Path;
    use std::sync::Mutex;

    use crate::domain::size::ByteSize;
    use crate::testing::InMemorySecretStore;

    const ID: &str = "com.example.cleaner";

    /// A fake disk: existing paths with sizes; trashing removes them.
    #[derive(Default)]
    struct FakeDisk {
        sizes: Mutex<HashMap<PathBuf, u64>>,
        trashed: Mutex<Vec<PathBuf>>,
        broken: Option<PathBuf>,
    }

    impl DiskUsage for FakeDisk {
        fn size_on_disk(&self, path: &Path) -> io::Result<Option<ByteSize>> {
            Ok(self
                .sizes
                .lock()
                .unwrap()
                .get(path)
                .copied()
                .map(ByteSize::new))
        }
    }

    impl Trash for FakeDisk {
        fn move_to_trash(&self, path: &Path) -> io::Result<()> {
            if self.broken.as_deref() == Some(path) {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
            }
            self.sizes.lock().unwrap().remove(path);
            self.trashed.lock().unwrap().push(path.to_path_buf());
            Ok(())
        }
    }

    fn loc(kind: FootprintKind, path: &str) -> FootprintLocation {
        FootprintLocation::new(kind, PathBuf::from(path), ID).unwrap()
    }

    const BUNDLE: &str = "/Applications/Cleaner.app";
    const DATA: &str = "/Users/a/Library/Application Support/com.example.cleaner";
    const CACHE: &str = "/Users/a/Library/Caches/com.example.cleaner";
    const PREFS: &str = "/Users/a/Library/Preferences/com.example.cleaner.plist";

    fn setup(broken: Option<&str>) -> (UninstallService, Arc<FakeDisk>, Arc<InMemorySecretStore>) {
        let disk = Arc::new(FakeDisk {
            broken: broken.map(PathBuf::from),
            ..FakeDisk::default()
        });
        disk.sizes.lock().unwrap().extend([
            (PathBuf::from(BUNDLE), 10_000_000),
            (PathBuf::from(DATA), 7_000_000),
            (PathBuf::from(CACHE), 8_000),
        ]);
        let secrets = Arc::new(InMemorySecretStore::default());
        secrets
            .set(&AiProviderKind::Anthropic.secret_key(), "sk")
            .unwrap();
        let service = UninstallService::new(
            vec![
                loc(FootprintKind::AppBundle, BUNDLE),
                loc(FootprintKind::AppData, DATA),
                loc(FootprintKind::Cache, CACHE),
                loc(FootprintKind::Preferences, PREFS),
            ],
            disk.clone(),
            disk.clone(),
            secrets.clone(),
        );
        (service, disk, secrets)
    }

    #[test]
    fn footprint_lists_existing_items_bundle_last_and_stored_keys() {
        let (service, _, _) = setup(None);
        let footprint = service.footprint().unwrap();
        let paths: Vec<_> = footprint.items.iter().map(|i| i.path.clone()).collect();
        assert_eq!(paths, [DATA, CACHE, BUNDLE].map(PathBuf::from));
        assert_eq!(footprint.items[0].size, ByteSize::new(7_000_000));
        assert_eq!(footprint.stored_keys, [AiProviderKind::Anthropic]);
        assert!(footprint.bundle_found);
    }

    #[test]
    fn nothing_happens_without_the_exact_confirmation() {
        let (service, disk, secrets) = setup(None);
        for typed in ["", "disk cleaner", "yes", "Disk Cleaner please"] {
            assert!(matches!(
                service.uninstall(typed),
                Err(CoreError::Domain(DomainError::InvalidInput(_)))
            ));
        }
        assert!(disk.trashed.lock().unwrap().is_empty());
        assert!(
            secrets
                .get(&AiProviderKind::Anthropic.secret_key())
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn removes_keys_and_trashes_everything_bundle_last() {
        let (service, disk, secrets) = setup(None);
        let report = service.uninstall(" Disk Cleaner ").unwrap();

        assert_eq!(report.keys_removed, 1);
        assert!(report.failures.is_empty());
        assert_eq!(
            *disk.trashed.lock().unwrap(),
            [DATA, CACHE, BUNDLE].map(PathBuf::from)
        );
        assert_eq!(report.trashed.len(), 3);
        assert!(
            secrets
                .get(&AiProviderKind::Anthropic.secret_key())
                .unwrap()
                .is_none()
        );
        assert!(service.footprint().unwrap().items.is_empty());
    }

    #[test]
    fn keeps_going_and_reports_failures() {
        let (service, disk, _) = setup(Some(CACHE));
        let report = service.uninstall(CONFIRMATION_PHRASE).unwrap();
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].target, CACHE);
        assert_eq!(disk.trashed.lock().unwrap().len(), 2, "data and bundle");
    }

    #[test]
    fn development_builds_have_no_bundle_to_remove() {
        let disk = Arc::new(FakeDisk::default());
        let service = UninstallService::new(
            vec![loc(FootprintKind::AppData, DATA)],
            disk.clone(),
            disk,
            Arc::new(InMemorySecretStore::default()),
        );
        assert!(!service.footprint().unwrap().bundle_found);
    }
}
