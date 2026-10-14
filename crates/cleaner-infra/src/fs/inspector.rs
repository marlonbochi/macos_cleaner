use std::io::ErrorKind;
use std::path::Path;

use cleaner_core::domain::file_entry::FileFingerprint;
use cleaner_core::ports::filesystem::FileInspector;

use super::metadata::fingerprint;

/// Reads the current fingerprint of a path without following symlinks.
#[derive(Debug, Default, Clone, Copy)]
pub struct StdFileInspector;

impl FileInspector for StdFileInspector {
    fn fingerprint(&self, path: &Path) -> std::io::Result<Option<FileFingerprint>> {
        match std::fs::symlink_metadata(path) {
            Ok(meta) => Ok(Some(fingerprint(&meta))),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
}
