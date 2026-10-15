//! Use case: browse the catalog produced by the last scan (US1).

use std::path::PathBuf;
use std::sync::Arc;

use crate::domain::file_entry::FileEntry;
use crate::domain::scan::{Scan, ScanId};
use crate::error::CoreError;
use crate::ports::repository::{
    FileEntryRepository, FileFilter, FileQuery, FileSort, Page, ScanRepository,
};

#[derive(Clone)]
pub struct CatalogService {
    scans: Arc<dyn ScanRepository>,
    files: Arc<dyn FileEntryRepository>,
}

impl std::fmt::Debug for CatalogService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CatalogService").finish_non_exhaustive()
    }
}

impl CatalogService {
    pub fn new(scans: Arc<dyn ScanRepository>, files: Arc<dyn FileEntryRepository>) -> Self {
        Self { scans, files }
    }

    /// The catalog shown when the app opens (acceptance scenario US1-3).
    pub fn latest_scan(&self) -> Result<Option<Scan>, CoreError> {
        Ok(self.scans.latest_finished()?)
    }

    pub fn list_files(
        &self,
        scan: ScanId,
        query: &FileQuery,
    ) -> Result<Page<FileEntry>, CoreError> {
        self.ensure_exists(scan)?;
        Ok(self.files.page(scan, query)?)
    }

    /// Direct children of `folder`, largest first, for folder drill-down.
    pub fn list_children(
        &self,
        scan: ScanId,
        folder: PathBuf,
        offset: u64,
        limit: u32,
    ) -> Result<Page<FileEntry>, CoreError> {
        let filter = FileFilter {
            parent: Some(folder),
            ..FileFilter::default()
        };
        self.list_files(
            scan,
            &FileQuery::new(filter, FileSort::SizeDesc, offset, limit),
        )
    }

    fn ensure_exists(&self, scan: ScanId) -> Result<(), CoreError> {
        match self.scans.get(scan)? {
            Some(_) => Ok(()),
            None => Err(CoreError::NotFound(format!("scan {}", scan.0))),
        }
    }
}

// Fixtures use POSIX paths, which are not absolute on Windows. The logic under test does not
// depend on the system; Windows paths are covered by the platform-specific tests.
#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;
    use crate::domain::file_entry::{FileKind, FileRecord};
    use crate::domain::scan::NewScan;
    use crate::domain::size::ByteSize;
    use crate::testing::{InMemoryFileEntryRepository, InMemoryScanRepository};

    fn record(path: &str, size: u64) -> FileRecord {
        FileRecord {
            path: path.into(),
            kind: FileKind::File,
            size_on_disk: ByteSize::new(size),
            apparent_size: ByteSize::new(size),
            file_count: 1,
            created_at: None,
            modified_at: None,
            accessed_at: None,
            inode: 0,
            device: 0,
            is_hardlink_dup: false,
            is_protected: false,
        }
    }

    #[test]
    fn lists_children_of_a_folder_largest_first() {
        let scans = Arc::new(InMemoryScanRepository::default());
        let files = Arc::new(InMemoryFileEntryRepository::default());
        let scan = scans
            .create(&NewScan::new(vec!["/u".into()], ByteSize::ZERO, 0).unwrap())
            .unwrap();
        files
            .insert_batch(
                scan.id,
                &[record("/u/a", 1), record("/u/b", 3), record("/u/x/c", 9)],
            )
            .unwrap();
        let service = CatalogService::new(scans, files);

        let page = service.list_children(scan.id, "/u".into(), 0, 50).unwrap();

        let paths: Vec<_> = page.items.iter().map(|e| e.record.path.clone()).collect();
        assert_eq!(paths, [PathBuf::from("/u/b"), PathBuf::from("/u/a")]);
    }

    #[test]
    fn unknown_scan_is_not_found() {
        let service = CatalogService::new(
            Arc::new(InMemoryScanRepository::default()),
            Arc::new(InMemoryFileEntryRepository::default()),
        );
        let query = FileQuery::new(FileFilter::default(), FileSort::SizeDesc, 0, 10);
        assert!(matches!(
            service.list_files(ScanId(9), &query),
            Err(CoreError::NotFound(_))
        ));
    }
}
