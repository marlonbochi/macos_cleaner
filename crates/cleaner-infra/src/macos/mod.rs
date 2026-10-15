//! Thin wrappers around macOS system facilities.

mod keychain;
mod trash;

pub use keychain::KeychainSecretStore;
pub use trash::MacTrash;

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Settings pane for "Full Disk Access" (Privacy & Security).
const FULL_DISK_ACCESS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles";

/// Whether this process has Full Disk Access.
///
/// The TCC database is readable only with that permission, which makes it a reliable probe
/// that does not touch any user data.
pub fn has_full_disk_access(home: &Path) -> bool {
    let probe = home.join("Library/Application Support/com.apple.TCC/TCC.db");
    std::fs::File::open(probe).is_ok()
}

/// The app macOS attributes this process to, i.e. the one that must be enabled under
/// Privacy & Security → Full Disk Access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionTarget {
    pub name: String,
    /// The `.app` bundle, when it could be located (lets the UI reveal it in Finder).
    pub app_path: Option<PathBuf>,
    /// True when running unbundled (`tauri dev`): TCC then credits the terminal or IDE that
    /// launched the process, not this app.
    pub is_dev_host: bool,
}

/// Works out which app needs Full Disk Access for the current process.
pub fn permission_target() -> Option<PermissionTarget> {
    let exe = std::env::current_exe().ok()?;
    if let Some(app) = enclosing_app_bundle(&exe) {
        return Some(PermissionTarget {
            name: app_name(&app),
            app_path: Some(app),
            is_dev_host: false,
        });
    }
    // Unbundled binary: macOS sets `__CFBundleIdentifier` to the app that launched the
    // process tree (Terminal, iTerm, VS Code...), which is the one TCC checks.
    let bundle_id = std::env::var("__CFBundleIdentifier").ok()?;
    let app_path = app_path_for_bundle_id(&bundle_id);
    Some(PermissionTarget {
        name: app_path
            .as_deref()
            .map_or_else(|| bundle_id.clone(), app_name),
        app_path,
        is_dev_host: true,
    })
}

/// Outermost `.app` bundle containing `path` (outermost, so nested helper apps resolve to the
/// app the user sees).
pub fn enclosing_app_bundle(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app")))
        .last()
        .map(Path::to_path_buf)
}

/// `CFBundleIdentifier` of an app bundle, read from its `Info.plist`.
pub fn bundle_identifier(app: &Path) -> Option<String> {
    let output = Command::new("/usr/bin/defaults")
        .arg("read")
        .arg(app.join("Contents/Info"))
        .arg("CFBundleIdentifier")
        .output()
        .ok()?;
    let id = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !id.is_empty()).then_some(id)
}

/// The `.app` bundle this process runs from, only if it carries `bundle_id` (so a development
/// build, or an app embedded somewhere unexpected, never resolves to someone else's bundle).
pub fn running_app_bundle(bundle_id: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let app = enclosing_app_bundle(&exe)?;
    (bundle_identifier(&app).as_deref() == Some(bundle_id)).then_some(app)
}

/// An installed app with this bundle id (found through Spotlight and double-checked against its
/// `Info.plist`), e.g. the copy of the app installed under its previous name.
pub fn installed_app_bundle(bundle_id: &str) -> Option<PathBuf> {
    let app = app_path_for_bundle_id(bundle_id)?;
    (bundle_identifier(&app).as_deref() == Some(bundle_id)).then_some(app)
}

fn app_name(app: &Path) -> String {
    app.file_stem().map_or_else(
        || app.display().to_string(),
        |s| s.to_string_lossy().into_owned(),
    )
}

/// Locates an installed app by bundle id through Spotlight.
fn app_path_for_bundle_id(bundle_id: &str) -> Option<PathBuf> {
    // The id comes from the environment; only allow the characters bundle ids can contain so it
    // cannot alter the Spotlight query.
    if bundle_id.is_empty()
        || !bundle_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return None;
    }
    let output = Command::new("/usr/bin/mdfind")
        .arg(format!("kMDItemCFBundleIdentifier == '{bundle_id}'"))
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(PathBuf::from)
        .find(|p| p.extension().is_some_and(|e| e == "app"))
}

pub fn open_full_disk_access_settings() -> io::Result<()> {
    run_open(&[FULL_DISK_ACCESS_URL])
}

/// Selects the item in a Finder window (`open -R`).
pub fn reveal_in_finder(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path must be absolute",
        ));
    }
    let path = path
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "non UTF-8 path"))?;
    run_open(&["-R", path])
}

fn run_open(args: &[&str]) -> io::Result<()> {
    // Arguments are passed directly to the process, never through a shell.
    let status = Command::new("/usr/bin/open").args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("open exited with {status}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_rejects_relative_paths() {
        let err = reveal_in_finder(Path::new("relative")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn finds_the_outermost_app_bundle() {
        assert_eq!(
            enclosing_app_bundle(Path::new(
                "/Applications/Disk Cleaner.app/Contents/MacOS/disk-cleaner"
            )),
            Some(PathBuf::from("/Applications/Disk Cleaner.app"))
        );
        assert_eq!(
            enclosing_app_bundle(Path::new(
                "/Applications/Host.app/Contents/Frameworks/Helper.app/Contents/MacOS/h"
            )),
            Some(PathBuf::from("/Applications/Host.app"))
        );
        assert_eq!(
            enclosing_app_bundle(Path::new("/repo/target/debug/disk-cleaner")),
            None
        );
    }

    #[test]
    fn app_name_is_the_bundle_stem() {
        assert_eq!(app_name(Path::new("/Applications/iTerm.app")), "iTerm");
    }

    #[test]
    fn rejects_suspicious_bundle_ids() {
        assert_eq!(app_path_for_bundle_id("x' || kMDItemFSName == '*"), None);
        assert_eq!(app_path_for_bundle_id(""), None);
    }

    #[test]
    fn full_disk_access_probe_is_false_for_a_fake_home() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!has_full_disk_access(dir.path()));
    }
}
