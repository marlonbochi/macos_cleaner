//! Catalog entries: what the scanner saw and what the catalog stores.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::scan::ScanId;
use super::size::ByteSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FileId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileKind {
    File,
    Directory,
    /// A macOS bundle (`.app`, `.photoslibrary`, ...) treated as a single unit.
    Package,
}

impl FileKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Directory => "Directory",
            Self::Package => "Package",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "File" => Some(Self::File),
            "Directory" => Some(Self::Directory),
            "Package" => Some(Self::Package),
            _ => None,
        }
    }
}

/// Identity of a file's state. If any field changes, cached assessments and pending actions
/// for that path are no longer trustworthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileFingerprint {
    pub size_on_disk: ByteSize,
    pub modified_at: Option<i64>,
    pub inode: u64,
}

/// Raw metadata produced by a `FileSystemScanner` for a single filesystem object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedEntry {
    pub path: PathBuf,
    pub kind: FileKind,
    /// Allocated blocks (`st_blocks * 512`): the space actually freed on deletion.
    pub size_on_disk: ByteSize,
    /// Logical length (`st_size`).
    pub apparent_size: ByteSize,
    pub created_at: Option<i64>,
    pub modified_at: Option<i64>,
    pub accessed_at: Option<i64>,
    pub inode: u64,
    pub device: u64,
    pub hard_links: u64,
}

/// Catalog data for an entry, independent of persistence identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: PathBuf,
    pub kind: FileKind,
    pub size_on_disk: ByteSize,
    pub apparent_size: ByteSize,
    /// 1 for files; number of files below it for directories.
    pub file_count: u64,
    pub created_at: Option<i64>,
    pub modified_at: Option<i64>,
    pub accessed_at: Option<i64>,
    pub inode: u64,
    pub device: u64,
    /// A second (or later) hard link to content already counted elsewhere.
    pub is_hardlink_dup: bool,
    pub is_protected: bool,
}

impl FileRecord {
    pub fn fingerprint(&self) -> FileFingerprint {
        FileFingerprint {
            size_on_disk: self.size_on_disk,
            modified_at: self.modified_at,
            inode: self.inode,
        }
    }

    pub fn name(&self) -> String {
        file_name(&self.path)
    }

    pub fn extension(&self) -> Option<String> {
        if self.kind == FileKind::Directory {
            return None;
        }
        self.path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
    }

    pub fn parent(&self) -> Option<&Path> {
        self.path.parent()
    }
}

/// A persisted catalog entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub id: FileId,
    pub scan_id: ScanId,
    pub record: FileRecord,
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.to_string_lossy().into_owned(),
        |n| n.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(path: &str, kind: FileKind) -> FileRecord {
        FileRecord {
            path: PathBuf::from(path),
            kind,
            size_on_disk: ByteSize::new(4096),
            apparent_size: ByteSize::new(4000),
            file_count: 1,
            created_at: None,
            modified_at: Some(10),
            accessed_at: None,
            inode: 7,
            device: 1,
            is_hardlink_dup: false,
            is_protected: false,
        }
    }

    #[test]
    fn extension_is_lowercased_and_absent_for_directories() {
        assert_eq!(
            record("/a/Movie.MKV", FileKind::File)
                .extension()
                .as_deref(),
            Some("mkv")
        );
        assert_eq!(record("/a/dir.d", FileKind::Directory).extension(), None);
        assert_eq!(
            record("/a/Xcode.app", FileKind::Package)
                .extension()
                .as_deref(),
            Some("app")
        );
    }

    #[test]
    fn name_falls_back_to_full_path_for_root() {
        assert_eq!(record("/a/b.txt", FileKind::File).name(), "b.txt");
        assert_eq!(record("/", FileKind::Directory).name(), "/");
    }

    #[test]
    fn fingerprint_captures_size_mtime_and_inode() {
        let fp = record("/a", FileKind::File).fingerprint();
        assert_eq!(fp.size_on_disk, ByteSize::new(4096));
        assert_eq!(fp.modified_at, Some(10));
        assert_eq!(fp.inode, 7);
    }

    #[test]
    fn kind_round_trips_through_strings() {
        for kind in [FileKind::File, FileKind::Directory, FileKind::Package] {
            assert_eq!(FileKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(FileKind::parse("?"), None);
    }
}
