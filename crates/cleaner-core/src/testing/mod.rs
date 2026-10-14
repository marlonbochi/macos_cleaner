//! In-memory fakes of the ports, for unit tests here and in other crates
//! (enable the `testing` feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, Ordering};

use crate::cancel::CancellationToken;
use crate::domain::file_entry::{FileEntry, FileId, FileRecord, ScannedEntry};
use crate::domain::scan::{NewScan, Scan, ScanId, ScanStatus};
use crate::error::{RepoError, ScanError};
use crate::ports::Clock;
use crate::ports::filesystem::{FileSystemScanner, ScanOptions, ScanSink};
use crate::ports::repository::{FileEntryRepository, FileQuery, FileSort, Page, ScanRepository};

/// A clock that can be moved by hand.
#[derive(Debug, Default)]
pub struct FakeClock(AtomicI64);

impl FakeClock {
    pub fn at(now: i64) -> Self {
        Self(AtomicI64::new(now))
    }

    pub fn advance(&self, seconds: i64) {
        self.0.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
}

/// Emits a pre-programmed list of entries. Can cancel itself after N entries to simulate
/// a user pressing "cancel" mid-scan.
#[derive(Debug, Default)]
pub struct FakeScanner {
    pub entries: Vec<ScannedEntry>,
    pub inaccessible: Vec<PathBuf>,
    pub cancel_after: Option<usize>,
    pub fail_with: Option<String>,
}

impl FileSystemScanner for FakeScanner {
    fn scan(
        &self,
        roots: &[PathBuf],
        options: &ScanOptions,
        cancel: &CancellationToken,
        sink: &mut dyn ScanSink,
    ) -> Result<(), ScanError> {
        if let Some(message) = &self.fail_with {
            return Err(ScanError::Io(message.clone()));
        }
        for path in &self.inaccessible {
            sink.on_inaccessible(path, "permission denied");
        }
        let in_scope = |p: &Path| {
            roots.iter().any(|r| p.starts_with(r))
                && !options.excluded.iter().any(|e| p.starts_with(e))
        };
        for (i, entry) in self.entries.iter().enumerate() {
            if self.cancel_after == Some(i) {
                cancel.cancel();
            }
            if cancel.is_cancelled() {
                return Ok(());
            }
            if in_scope(&entry.path) {
                sink.on_entry(entry.clone());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct InMemoryScanRepository {
    scans: Mutex<Vec<Scan>>,
}

impl ScanRepository for InMemoryScanRepository {
    fn create(&self, new: &NewScan) -> Result<Scan, RepoError> {
        let mut scans = self.scans.lock().unwrap();
        let id = ScanId(i64::try_from(scans.len()).unwrap() + 1);
        let scan = Scan::start(id, new.clone());
        scans.push(scan.clone());
        Ok(scan)
    }

    fn save(&self, scan: &Scan) -> Result<(), RepoError> {
        let mut scans = self.scans.lock().unwrap();
        let slot = scans
            .iter_mut()
            .find(|s| s.id == scan.id)
            .ok_or_else(|| RepoError::NotFound(format!("scan {}", scan.id.0)))?;
        *slot = scan.clone();
        Ok(())
    }

    fn get(&self, id: ScanId) -> Result<Option<Scan>, RepoError> {
        Ok(self
            .scans
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.id == id)
            .cloned())
    }

    fn find_running(&self) -> Result<Vec<Scan>, RepoError> {
        Ok(self
            .scans
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.status == ScanStatus::Running)
            .cloned()
            .collect())
    }

    fn latest_finished(&self) -> Result<Option<Scan>, RepoError> {
        Ok(self
            .scans
            .lock()
            .unwrap()
            .iter()
            .filter(|s| matches!(s.status, ScanStatus::Completed | ScanStatus::Cancelled))
            .max_by_key(|s| (s.started_at, s.id))
            .cloned())
    }

    fn delete_all_except(&self, keep: ScanId) -> Result<(), RepoError> {
        self.scans.lock().unwrap().retain(|s| s.id == keep);
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct InMemoryFileEntryRepository {
    entries: Mutex<Vec<FileEntry>>,
}

impl InMemoryFileEntryRepository {
    pub fn all(&self) -> Vec<FileEntry> {
        self.entries.lock().unwrap().clone()
    }

    pub fn find_by_path(&self, path: &Path) -> Option<FileEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .find(|e| e.record.path == path)
            .cloned()
    }
}

impl FileEntryRepository for InMemoryFileEntryRepository {
    fn insert_batch(&self, scan: ScanId, records: &[FileRecord]) -> Result<(), RepoError> {
        let mut entries = self.entries.lock().unwrap();
        for record in records {
            let id = FileId(i64::try_from(entries.len()).unwrap() + 1);
            entries.push(FileEntry {
                id,
                scan_id: scan,
                record: record.clone(),
            });
        }
        Ok(())
    }

    fn page(&self, scan: ScanId, query: &FileQuery) -> Result<Page<FileEntry>, RepoError> {
        let f = &query.filter;
        let mut matching: Vec<FileEntry> = self
            .entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.scan_id == scan)
            .filter(|e| f.min_size.is_none_or(|m| e.record.size_on_disk >= m))
            .filter(|e| {
                f.extensions.is_empty()
                    || e.record
                        .extension()
                        .is_some_and(|x| f.extensions.contains(&x))
            })
            .filter(|e| {
                f.parent
                    .as_ref()
                    .is_none_or(|p| e.record.parent() == Some(p.as_path()))
            })
            .filter(|e| {
                f.path_prefix
                    .as_ref()
                    .is_none_or(|p| e.record.path.starts_with(p) && &e.record.path != p)
            })
            .filter(|e| f.kinds.is_empty() || f.kinds.contains(&e.record.kind))
            .cloned()
            .collect();
        match query.sort {
            FileSort::SizeDesc => matching.sort_by(|a, b| {
                b.record
                    .size_on_disk
                    .cmp(&a.record.size_on_disk)
                    .then(a.id.cmp(&b.id))
            }),
            FileSort::SizeAsc => matching.sort_by(|a, b| {
                a.record
                    .size_on_disk
                    .cmp(&b.record.size_on_disk)
                    .then(a.id.cmp(&b.id))
            }),
            FileSort::ModifiedAsc => matching.sort_by_key(|e| (e.record.modified_at, e.id)),
        }
        let total = matching.len() as u64;
        let items = matching
            .into_iter()
            .skip(usize::try_from(query.offset).unwrap())
            .take(query.limit as usize)
            .collect();
        Ok(Page {
            items,
            total,
            offset: query.offset,
            limit: query.limit,
        })
    }

    fn get_many(&self, ids: &[FileId]) -> Result<Vec<FileEntry>, RepoError> {
        Ok(self
            .entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| ids.contains(&e.id))
            .cloned()
            .collect())
    }

    fn remove(&self, ids: &[FileId]) -> Result<(), RepoError> {
        self.entries
            .lock()
            .unwrap()
            .retain(|e| !ids.contains(&e.id));
        Ok(())
    }
}
