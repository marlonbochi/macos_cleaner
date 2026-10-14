use std::io::{self, ErrorKind};
use std::path::Path;

use cleaner_core::domain::size::ByteSize;
use cleaner_core::ports::system::DiskUsage;

use super::metadata::size_on_disk;

/// Adds up allocated sizes below a path without following symlinks (a link counts as itself).
/// Unreadable entries below the path are skipped: the result is then a lower bound.
#[derive(Debug, Default, Clone, Copy)]
pub struct StdDiskUsage;

impl DiskUsage for StdDiskUsage {
    fn size_on_disk(&self, path: &Path) -> io::Result<Option<ByteSize>> {
        match std::fs::symlink_metadata(path) {
            Ok(meta) => Ok(Some(total(path, &meta))),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
}

fn total(path: &Path, meta: &std::fs::Metadata) -> ByteSize {
    let own = size_on_disk(meta);
    if !meta.is_dir() {
        return own;
    }
    let Ok(children) = std::fs::read_dir(path) else {
        return own;
    };
    children
        .flatten()
        .filter_map(|child| {
            let meta = std::fs::symlink_metadata(child.path()).ok()?;
            Some(total(&child.path(), &meta))
        })
        .fold(own, |sum, size| sum + size)
}
