//! The app's own footprint on disk: everything a complete uninstall must remove (feature 009).
//!
//! Every location is validated when it is built, so the uninstaller can only ever touch the app
//! bundle itself or folders and files named after the app's bundle identifier.

use std::path::{Path, PathBuf};

use crate::domain::size::ByteSize;
use crate::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FootprintKind {
    /// Catalogue database (SQLite) and other app data.
    AppData,
    /// Web view storage (local storage, IndexedDB).
    WebData,
    Cache,
    Logs,
    Preferences,
    SavedState,
    HttpStorage,
    /// The `.app` bundle itself; always removed last.
    AppBundle,
    /// Data or app left by a previous identity of the app (it was called macOS Cleaner).
    Legacy,
}

/// A place where the app may leave data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FootprintLocation {
    kind: FootprintKind,
    path: PathBuf,
}

impl FootprintLocation {
    /// Builds a location that the uninstaller may remove.
    ///
    /// Rules: the path is absolute and has no `..`; the app bundle ends in `.app`; any other
    /// location is named after `bundle_id` (e.g. `~/Library/Caches/<bundle_id>` or
    /// `~/Library/Preferences/<bundle_id>.plist`).
    pub fn new(kind: FootprintKind, path: PathBuf, bundle_id: &str) -> Result<Self, DomainError> {
        let invalid = |why: &str| {
            DomainError::InvalidInput(format!("{} cannot be uninstalled: {why}", path.display()))
        };
        if !path.is_absolute() || path.components().any(|c| c.as_os_str() == "..") {
            return Err(invalid("not a normalised absolute path"));
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| invalid("no file name"))?;
        let is_app = Path::new(name).extension().is_some_and(|e| e == "app");
        let named_after_app = !bundle_id.trim().is_empty()
            && (name == bundle_id
                || name
                    .strip_prefix(bundle_id)
                    .is_some_and(|rest| rest.starts_with('.')));
        let allowed = match kind {
            FootprintKind::AppBundle => is_app,
            FootprintKind::Legacy => is_app || named_after_app,
            _ => named_after_app,
        };
        if !allowed {
            return Err(invalid("not named after the app"));
        }
        Ok(Self { kind, path })
    }

    pub fn kind(&self) -> FootprintKind {
        self.kind
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// How the app itself (not its data) is removed, which depends on how it was installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppRemoval {
    /// macOS: the `.app` bundle is one of the footprint locations and goes to the Trash.
    Trash,
    /// Windows: the installer's uninstaller removes the program and the data folders, which
    /// Windows keeps locked while the app runs.
    Uninstaller { program: PathBuf, args: Vec<String> },
    /// Linux packages: removing the app needs the package manager (and an administrator), so the
    /// user is given the command.
    PackageManager { command: String },
    /// Development build: there is no installed app to remove.
    NotInstalled,
}

/// A location that exists right now, with its size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FootprintItem {
    pub kind: FootprintKind,
    pub path: PathBuf,
    pub size: ByteSize,
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "com.example.cleaner";

    fn location(kind: FootprintKind, path: &str) -> Result<FootprintLocation, DomainError> {
        FootprintLocation::new(kind, PathBuf::from(path), ID)
    }

    #[cfg(unix)]
    #[test]
    fn accepts_locations_named_after_the_bundle_id() {
        for (kind, path) in [
            (
                FootprintKind::AppData,
                "/Users/a/Library/Application Support/com.example.cleaner",
            ),
            (
                FootprintKind::Cache,
                "/Users/a/Library/Caches/com.example.cleaner",
            ),
            (
                FootprintKind::Preferences,
                "/Users/a/Library/Preferences/com.example.cleaner.plist",
            ),
            (
                FootprintKind::SavedState,
                "/Users/a/Library/Saved Application State/com.example.cleaner.savedState",
            ),
            (FootprintKind::AppBundle, "/Applications/Cleaner.app"),
            (FootprintKind::Legacy, "/Applications/Old Cleaner.app"),
            (
                FootprintKind::Legacy,
                "/Users/a/Library/Caches/com.example.cleaner",
            ),
        ] {
            assert!(location(kind, path).is_ok(), "{path}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn accepts_windows_locations_named_after_the_bundle_id() {
        for (kind, path) in [
            (
                FootprintKind::AppData,
                r"C:\Users\a\AppData\Roaming\com.example.cleaner",
            ),
            (
                FootprintKind::Cache,
                r"C:\Users\a\AppData\Local\com.example.cleaner",
            ),
        ] {
            assert!(location(kind, path).is_ok(), "{path}");
        }
        assert!(location(FootprintKind::Cache, r"C:\Users\a\AppData\Local").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_anything_else() {
        for (kind, path) in [
            (FootprintKind::Cache, "/Users/a/Library/Caches"),
            (
                FootprintKind::Cache,
                "/Users/a/Library/Caches/com.example.cleaner2",
            ),
            (
                FootprintKind::AppData,
                "/Users/a/Library/com.example.cleaner/..",
            ),
            (FootprintKind::AppData, "relative/com.example.cleaner"),
            (FootprintKind::AppBundle, "/Applications"),
            (FootprintKind::AppBundle, "/Users/a/Documents/report.pdf"),
            (FootprintKind::AppData, "/"),
            (FootprintKind::Legacy, "/Users/a/Library/Caches"),
        ] {
            assert!(location(kind, path).is_err(), "{path}");
        }
        assert!(
            FootprintLocation::new(FootprintKind::Cache, PathBuf::from("/x/ "), " ").is_err(),
            "an empty bundle id matches nothing"
        );
    }
}
