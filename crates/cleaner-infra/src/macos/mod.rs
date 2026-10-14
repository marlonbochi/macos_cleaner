//! Thin wrappers around macOS system facilities.

use std::io;
use std::path::Path;
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
    fn full_disk_access_probe_is_false_for_a_fake_home() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!has_full_disk_access(dir.path()));
    }
}
