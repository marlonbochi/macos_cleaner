//! Translation of `std::fs::Metadata` into domain values.

use std::fs::Metadata;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cleaner_core::domain::file_entry::{FileFingerprint, FileKind, ScannedEntry};
use cleaner_core::domain::package::is_package_dir;
use cleaner_core::domain::size::ByteSize;

/// Returns `None` for symlinks and special files (sockets, FIFOs, devices): they are never
/// catalogued, which also guarantees links are not followed. On Windows, junctions count as
/// links too.
pub(crate) fn kind_of(path: &std::path::Path, meta: &Metadata) -> Option<FileKind> {
    let ft = meta.file_type();
    if ft.is_symlink() {
        None
    } else if ft.is_dir() {
        // Packages (folders shown as one file) only exist on macOS.
        Some(if cfg!(target_os = "macos") && is_package_dir(path) {
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
    let native = native(meta);
    ScannedEntry {
        path,
        kind,
        size_on_disk: native.size_on_disk,
        apparent_size: ByteSize::new(meta.len()),
        created_at: meta.created().ok().and_then(epoch_seconds),
        modified_at: meta.modified().ok().and_then(epoch_seconds),
        accessed_at: meta.accessed().ok().and_then(epoch_seconds),
        inode: native.inode,
        device: native.device,
        hard_links: native.hard_links,
    }
}

pub(crate) fn fingerprint(meta: &Metadata) -> FileFingerprint {
    let native = native(meta);
    FileFingerprint {
        size_on_disk: native.size_on_disk,
        modified_at: meta.modified().ok().and_then(epoch_seconds),
        inode: native.inode,
    }
}

/// What deleting the item actually frees.
pub(crate) fn size_on_disk(meta: &Metadata) -> ByteSize {
    native(meta).size_on_disk
}

/// Volume the item lives on (used to avoid crossing into other disks).
pub(crate) fn device(meta: &Metadata) -> u64 {
    native(meta).device
}

/// System-specific facts about a file.
struct Native {
    size_on_disk: ByteSize,
    inode: u64,
    device: u64,
    hard_links: u64,
}

#[cfg(unix)]
fn native(meta: &Metadata) -> Native {
    use std::os::unix::fs::MetadataExt;
    /// POSIX `st_blocks` is always counted in 512-byte units, whatever the block size.
    const BLOCK_UNIT: u64 = 512;
    Native {
        // Allocated size: smaller than `len()` for sparse or compressed files.
        size_on_disk: ByteSize::new(meta.blocks().saturating_mul(BLOCK_UNIT)),
        inode: meta.ino(),
        device: meta.dev(),
        hard_links: meta.nlink(),
    }
}

#[cfg(windows)]
fn native(meta: &Metadata) -> Native {
    use std::os::windows::fs::MetadataExt;
    /// Cloud placeholders (OneDrive "online-only" files) take no local space.
    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
    const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
    const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
    /// NTFS default cluster size: small files still use a whole cluster.
    const CLUSTER: u64 = 4096;
    let placeholder = meta.file_attributes()
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0;
    let size = if placeholder || meta.is_dir() {
        0
    } else {
        meta.len().div_ceil(CLUSTER).saturating_mul(CLUSTER)
    };
    // Volume serial and file index need unstable APIs; without them hard links are not
    // de-duplicated (`hard_links = 1`) and every item counts as being on the same volume
    // (junctions and mount points are links, which are never followed anyway).
    Native {
        size_on_disk: ByteSize::new(size),
        inode: 0,
        device: 0,
        hard_links: 1,
    }
}

fn epoch_seconds(time: SystemTime) -> Option<i64> {
    let secs = time.duration_since(UNIX_EPOCH).ok()?.as_secs();
    i64::try_from(secs).ok()
}
