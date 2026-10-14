//! Filesystem ports. The real implementation lives in `cleaner-infra::fs`.

use std::path::{Path, PathBuf};

use crate::cancel::CancellationToken;
use crate::domain::file_entry::{FileFingerprint, ScannedEntry};
use crate::error::ScanError;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanOptions {
    /// Subtrees that are never entered (e.g. firmlinked system volumes when scanning `/`).
    pub excluded: Vec<PathBuf>,
}

/// Observer that receives the scanner output as it is produced.
pub trait ScanSink {
    /// Called for every file, directory and package found, in no particular order.
    fn on_entry(&mut self, entry: ScannedEntry);
    /// Called for every path that could not be read. Scanning continues.
    fn on_inaccessible(&mut self, path: &Path, reason: &str);
}

/// Walks directory trees read-only (Constitution I).
///
/// Contract:
/// - never follows symbolic links;
/// - never leaves the device of each root;
/// - never yields paths outside the given roots or inside `options.excluded`;
/// - returns promptly (`Ok`) once `cancel` is set.
pub trait FileSystemScanner: Send + Sync {
    fn scan(
        &self,
        roots: &[PathBuf],
        options: &ScanOptions,
        cancel: &CancellationToken,
        sink: &mut dyn ScanSink,
    ) -> Result<(), ScanError>;
}

/// Reads the current state of a single path, used to re-validate items right before an action.
pub trait FileInspector: Send + Sync {
    /// `Ok(None)` when the path no longer exists.
    fn fingerprint(&self, path: &Path) -> std::io::Result<Option<FileFingerprint>>;
}
