//! Translation of `std::fs::Metadata` into domain values.

use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cleaner_core::domain::file_entry::{FileFingerprint, FileKind, ScannedEntry};
use cleaner_core::domain::package::is_package_dir;
use cleaner_core::domain::size::ByteSize;

/// POSIX `st_blocks` is always counted in 512-byte units, regardless of the filesystem block size.
const BLOCK_UNIT: u64 = 512;

/// Returns `None` for symlinks and special files (sockets, FIFOs, devices): they are never
/// catalogued, which also guarantees links are not followed.
pub(crate) fn kind_of(path: &std::path::Path, meta: &Metadata) -> Option<FileKind> {
    let ft = meta.file_type();
    if ft.is_symlink() {
        None
    } else if ft.is_dir() {
        Some(if is_package_dir(path) {
            FileKind::Package
        } else {
            FileKind::Directory
        })
    } else if ft.is_file() {
        Some(FileKind::File)
    } else {
        None
    }
}

pub(crate) fn to_scanned_entry(path: PathBuf, kind: FileKind, meta: &Metadata) -> ScannedEntry {
    ScannedEntry {
        path,
        kind,
        size_on_disk: size_on_disk(meta),
        apparent_size: ByteSize::new(meta.len()),
        created_at: meta.created().ok().and_then(epoch_seconds),
        modified_at: Some(meta.mtime()),
        accessed_at: Some(meta.atime()),
        inode: meta.ino(),
        device: meta.dev(),
        hard_links: meta.nlink(),
    }
}

pub(crate) fn fingerprint(meta: &Metadata) -> FileFingerprint {
    FileFingerprint {
        size_on_disk: size_on_disk(meta),
        modified_at: Some(meta.mtime()),
        inode: meta.ino(),
    }
}

/// Allocated size: what deleting the file actually frees (smaller than `len()` for sparse or
/// APFS-compressed files).
fn size_on_disk(meta: &Metadata) -> ByteSize {
    ByteSize::new(meta.blocks().saturating_mul(BLOCK_UNIT))
}

fn epoch_seconds(time: SystemTime) -> Option<i64> {
    let secs = time.duration_since(UNIX_EPOCH).ok()?.as_secs();
    i64::try_from(secs).ok()
}
