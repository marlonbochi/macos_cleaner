//! Repository ports (Repository pattern). SQLite implementations live in `cleaner-infra::db`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::file_entry::{FileEntry, FileId, FileKind, FileRecord};
use crate::domain::scan::{NewScan, Scan, ScanId};
use crate::domain::size::ByteSize;
use crate::error::RepoError;

/// Hard upper bound for a page, so the UI can never ask for the whole catalog at once.
pub const MAX_PAGE_SIZE: u32 = 500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub offset: u64,
    pub limit: u32,
}

impl<T> Page<T> {
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            total: self.total,
            offset: self.offset,
            limit: self.limit,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileSort {
    #[default]
    SizeDesc,
    SizeAsc,
    ModifiedAsc,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFilter {
    pub min_size: Option<ByteSize>,
    /// Lower-case extensions without the dot.
    pub extensions: Vec<String>,
    /// Only direct children of this directory.
    pub parent: Option<PathBuf>,
    /// Anything below this directory.
    pub path_prefix: Option<PathBuf>,
    pub kinds: Vec<FileKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileQuery {
    pub filter: FileFilter,
    pub sort: FileSort,
    pub offset: u64,
    pub limit: u32,
}

impl FileQuery {
    /// Clamps the limit to `1..=MAX_PAGE_SIZE`.
    pub fn new(filter: FileFilter, sort: FileSort, offset: u64, limit: u32) -> Self {
        Self {
            filter,
            sort,
            offset,
            limit: limit.clamp(1, MAX_PAGE_SIZE),
        }
    }
}

pub trait ScanRepository: Send + Sync {
    fn create(&self, scan: &NewScan) -> Result<Scan, RepoError>;
    /// Persists status, stats, finish time and error of an existing scan.
    fn save(&self, scan: &Scan) -> Result<(), RepoError>;
    fn get(&self, id: ScanId) -> Result<Option<Scan>, RepoError>;
    fn find_running(&self) -> Result<Vec<Scan>, RepoError>;
    /// Most recent scan in a terminal state other than `Failed`.
    fn latest_finished(&self) -> Result<Option<Scan>, RepoError>;
    /// Deletes every other scan and its entries, keeping the catalog bounded.
    fn delete_all_except(&self, keep: ScanId) -> Result<(), RepoError>;
}

pub trait FileEntryRepository: Send + Sync {
    fn insert_batch(&self, scan: ScanId, records: &[FileRecord]) -> Result<(), RepoError>;
    fn page(&self, scan: ScanId, query: &FileQuery) -> Result<Page<FileEntry>, RepoError>;
    fn get_many(&self, ids: &[FileId]) -> Result<Vec<FileEntry>, RepoError>;
    fn remove(&self, ids: &[FileId]) -> Result<(), RepoError>;
}

/// Cache of risk assessments, keyed by path + fingerprint + evaluator version (FR-015).
pub trait AssessmentRepository: Send + Sync {
    fn find(
        &self,
        key: &crate::domain::risk::AssessmentKey,
    ) -> Result<Option<crate::domain::risk::RiskAssessment>, RepoError>;
    fn save(&self, assessment: &crate::domain::risk::RiskAssessment) -> Result<(), RepoError>;
    /// Most recent assessment of each path, from any evaluator, whose fingerprint still matches.
    fn latest_matching(
        &self,
        items: &[(PathBuf, crate::domain::file_entry::FileFingerprint)],
    ) -> Result<Vec<crate::domain::risk::RiskAssessment>, RepoError>;
}

/// Small key/value store for user settings (JSON values).
pub trait SettingsRepository: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<serde_json::Value>, RepoError>;
    fn set(&self, key: &str, value: &serde_json::Value) -> Result<(), RepoError>;
}
