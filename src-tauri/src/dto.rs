//! IPC data transfer objects. TypeScript types are generated from these with `ts-rs`
//! (`npm run gen:types`), so the frontend can never drift from the backend contract.
//!
//! Sizes and counts are `u64` in Rust but well below 2^53, so they are exposed as `number`.

use std::path::PathBuf;

use cleaner_core::domain::file_entry::{FileEntry, FileKind};
use cleaner_core::domain::scan::{Scan, ScanStatus};
use cleaner_core::domain::size::ByteSize;
use cleaner_core::ports::repository::{FileFilter, FileSort, Page};
use cleaner_core::services::scan_service::ScanProgress;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PageDto<T> {
    pub items: Vec<T>,
    #[ts(type = "number")]
    pub total: u64,
    #[ts(type = "number")]
    pub offset: u64,
    pub limit: u32,
}

impl<T> PageDto<T> {
    pub fn from_page<S>(page: Page<S>, f: impl FnMut(S) -> T) -> Self {
        let page = page.map(f);
        Self {
            items: page.items,
            total: page.total,
            offset: page.offset,
            limit: page.limit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ScanStatusDto {
    Running,
    Completed,
    Cancelled,
    Failed,
}

impl From<ScanStatus> for ScanStatusDto {
    fn from(s: ScanStatus) -> Self {
        match s {
            ScanStatus::Running => Self::Running,
            ScanStatus::Completed => Self::Completed,
            ScanStatus::Cancelled => Self::Cancelled,
            ScanStatus::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScanSummaryDto {
    #[ts(type = "number")]
    pub id: i64,
    pub roots: Vec<String>,
    pub status: ScanStatusDto,
    #[ts(type = "number")]
    pub min_file_size: u64,
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number | null")]
    pub finished_at: Option<i64>,
    #[ts(type = "number")]
    pub files_seen: u64,
    #[ts(type = "number")]
    pub bytes_seen: u64,
    #[ts(type = "number")]
    pub inaccessible_count: u64,
    pub error: Option<String>,
    /// True when results are partial (cancelled or failed).
    pub incomplete: bool,
}

impl From<&Scan> for ScanSummaryDto {
    fn from(s: &Scan) -> Self {
        Self {
            id: s.id.0,
            roots: s
                .roots
                .iter()
                .map(|r| r.to_string_lossy().into_owned())
                .collect(),
            status: s.status.into(),
            min_file_size: s.min_file_size.bytes(),
            started_at: s.started_at,
            finished_at: s.finished_at,
            files_seen: s.stats.files_seen,
            bytes_seen: s.stats.bytes_seen.bytes(),
            inaccessible_count: s.stats.inaccessible_count,
            error: s.error.clone(),
            incomplete: s.status != ScanStatus::Running && s.is_incomplete(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FileKindDto {
    File,
    Directory,
    Package,
}

impl From<FileKind> for FileKindDto {
    fn from(k: FileKind) -> Self {
        match k {
            FileKind::File => Self::File,
            FileKind::Directory => Self::Directory,
            FileKind::Package => Self::Package,
        }
    }
}

impl From<FileKindDto> for FileKind {
    fn from(k: FileKindDto) -> Self {
        match k {
            FileKindDto::File => Self::File,
            FileKindDto::Directory => Self::Directory,
            FileKindDto::Package => Self::Package,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FileEntryDto {
    #[ts(type = "number")]
    pub id: i64,
    pub path: String,
    pub name: String,
    pub kind: FileKindDto,
    #[ts(type = "number")]
    pub size_on_disk: u64,
    #[ts(type = "number")]
    pub apparent_size: u64,
    #[ts(type = "number")]
    pub file_count: u64,
    #[ts(type = "number | null")]
    pub modified_at: Option<i64>,
    #[ts(type = "number | null")]
    pub accessed_at: Option<i64>,
    pub is_protected: bool,
}

impl From<FileEntry> for FileEntryDto {
    fn from(e: FileEntry) -> Self {
        let r = e.record;
        Self {
            id: e.id.0,
            name: r.name(),
            path: r.path.to_string_lossy().into_owned(),
            kind: r.kind.into(),
            size_on_disk: r.size_on_disk.bytes(),
            apparent_size: r.apparent_size.bytes(),
            file_count: r.file_count,
            modified_at: r.modified_at,
            accessed_at: r.accessed_at,
            is_protected: r.is_protected,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct FileFilterDto {
    #[ts(type = "number | null")]
    pub min_size: Option<u64>,
    pub extensions: Vec<String>,
    pub path_prefix: Option<String>,
    pub kinds: Vec<FileKindDto>,
}

impl From<FileFilterDto> for FileFilter {
    fn from(f: FileFilterDto) -> Self {
        Self {
            min_size: f.min_size.map(ByteSize::new),
            extensions: f
                .extensions
                .into_iter()
                .map(|e| e.trim_start_matches('.').to_lowercase())
                .filter(|e| !e.is_empty())
                .collect(),
            parent: None,
            path_prefix: f.path_prefix.map(PathBuf::from),
            kinds: f.kinds.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FileSortDto {
    #[default]
    SizeDesc,
    SizeAsc,
    ModifiedAsc,
}

impl From<FileSortDto> for FileSort {
    fn from(s: FileSortDto) -> Self {
        match s {
            FileSortDto::SizeDesc => Self::SizeDesc,
            FileSortDto::SizeAsc => Self::SizeAsc,
            FileSortDto::ModifiedAsc => Self::ModifiedAsc,
        }
    }
}

/// Streamed through a `tauri::ipc::Channel` while a scan runs.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum ScanEvent {
    Started {
        summary: ScanSummaryDto,
    },
    Progress {
        #[ts(type = "number")]
        files_seen: u64,
        #[ts(type = "number")]
        bytes_seen: u64,
        #[ts(type = "number")]
        inaccessible_count: u64,
        current_path: String,
    },
    Inaccessible {
        path: String,
    },
    Finished {
        summary: ScanSummaryDto,
    },
    Error {
        message: String,
    },
}

impl From<&ScanProgress> for ScanEvent {
    fn from(p: &ScanProgress) -> Self {
        Self::Progress {
            files_seen: p.stats.files_seen,
            bytes_seen: p.stats.bytes_seen.bytes(),
            inaccessible_count: p.stats.inaccessible_count,
            current_path: p.current_path.to_string_lossy().into_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PermissionsStatusDto {
    pub full_disk_access: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScanDefaultsDto {
    pub roots: Vec<String>,
    pub home: String,
    #[ts(type = "number")]
    pub min_file_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cleaner_core::domain::scan::{NewScan, ScanId, ScanStats};

    #[test]
    fn filter_normalises_extensions() {
        let filter: FileFilter = FileFilterDto {
            extensions: vec![".MOV".into(), "zip".into(), ".".into()],
            ..FileFilterDto::default()
        }
        .into();
        assert_eq!(filter.extensions, ["mov", "zip"]);
    }

    #[test]
    fn scan_summary_flags_cancelled_as_incomplete() {
        let new = NewScan::new(vec!["/a".into()], ByteSize::ZERO, 1).unwrap();
        let mut scan = Scan::start(ScanId(3), new);
        assert!(
            !ScanSummaryDto::from(&scan).incomplete,
            "running is not 'incomplete'"
        );
        scan.cancel(2, ScanStats::default()).unwrap();
        let dto = ScanSummaryDto::from(&scan);
        assert!(dto.incomplete);
        assert_eq!(dto.status, ScanStatusDto::Cancelled);
    }

    #[test]
    fn scan_event_is_tagged_with_camel_case_fields() {
        let json = serde_json::to_value(ScanEvent::Inaccessible { path: "/x".into() }).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "inaccessible", "path": "/x" })
        );
    }
}
