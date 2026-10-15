//! Parallel, read-only directory walker built on `jwalk` (rayon under the hood).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use cleaner_core::cancel::CancellationToken;
use cleaner_core::error::ScanError;
use cleaner_core::ports::filesystem::{FileSystemScanner, ScanOptions, ScanSink};
use jwalk::{Parallelism, WalkDirGeneric};

use super::metadata::{device, kind_of, to_scanned_entry};

/// Walks each root in parallel. Directory reads happen on rayon threads; entries are delivered
/// to the sink on the calling thread, so the sink needs no synchronisation.
#[derive(Debug, Clone, Copy)]
pub struct JwalkScanner {
    threads: usize,
}

impl JwalkScanner {
    pub fn new(threads: usize) -> Self {
        Self {
            threads: threads.max(1),
        }
    }
}

impl Default for JwalkScanner {
    fn default() -> Self {
        let threads = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
        Self::new(threads)
    }
}

impl FileSystemScanner for JwalkScanner {
    fn scan(
        &self,
        roots: &[PathBuf],
        options: &ScanOptions,
        cancel: &CancellationToken,
        sink: &mut dyn ScanSink,
    ) -> Result<(), ScanError> {
        for root in roots {
            if cancel.is_cancelled() {
                break;
            }
            self.scan_root(root, options, cancel, sink);
        }
        Ok(())
    }
}

impl JwalkScanner {
    fn scan_root(
        &self,
        root: &Path,
        options: &ScanOptions,
        cancel: &CancellationToken,
        sink: &mut dyn ScanSink,
    ) {
        let root_device = match std::fs::symlink_metadata(root) {
            Ok(meta) if meta.file_type().is_symlink() => {
                sink.on_inaccessible(root, "scan root is a symbolic link");
                return;
            }
            Ok(meta) => device(&meta),
            Err(e) => {
                sink.on_inaccessible(root, &e.to_string());
                return;
            }
        };
        let excluded: Arc<[PathBuf]> = options.excluded.clone().into();
        let walk_cancel = cancel.clone();

        let walker = WalkDirGeneric::<((), ())>::new(root)
            .follow_links(false)
            .skip_hidden(false)
            .sort(false)
            .parallelism(Parallelism::RayonNewPool(self.threads))
            // Runs on worker threads before descending: prune what we must not enter.
            .process_read_dir(move |_depth, _dir, _state, children| {
                if walk_cancel.is_cancelled() {
                    children.clear();
                    return;
                }
                children.retain(|child| match child {
                    Ok(entry) => !excluded.iter().any(|e| entry.path().starts_with(e)),
                    Err(_) => true,
                });
                for entry in children.iter_mut().flatten() {
                    if !entry.file_type.is_dir() {
                        continue;
                    }
                    // Never cross into another volume (mount points, external disks).
                    let same_device = entry
                        .metadata()
                        .map(|m| device(&m) == root_device)
                        .unwrap_or(false);
                    if !same_device {
                        entry.read_children = None;
                    }
                }
            });

        for item in walker {
            if cancel.is_cancelled() {
                // Dropping the iterator stops the remaining parallel reads.
                break;
            }
            match item {
                Ok(entry) => {
                    let path = entry.path();
                    match entry.metadata() {
                        Ok(meta) => {
                            if let Some(kind) = kind_of(&path, &meta) {
                                sink.on_entry(to_scanned_entry(path, kind, &meta));
                            }
                        }
                        Err(e) => sink.on_inaccessible(&path, &e.to_string()),
                    }
                    // A directory whose contents could not be listed (e.g. permission denied).
                    if let Some(err) = entry.read_children.as_ref().and_then(|rc| rc.error()) {
                        sink.on_inaccessible(&entry.path(), &err.to_string());
                    }
                }
                Err(e) => {
                    let path = e
                        .path()
                        .map_or_else(|| root.to_path_buf(), Path::to_path_buf);
                    sink.on_inaccessible(&path, &e.to_string());
                }
            }
        }
    }
}
