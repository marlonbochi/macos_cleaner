//! Use case: run a scan and build the catalog (US1).
//!
//! The scanner streams raw entries into a [`CatalogBuilder`] (an Observer of the scan), which
//! - de-duplicates hard links,
//! - aggregates file sizes bottom-up into every ancestor directory,
//! - persists files above the size threshold in batches while the walk is still running,
//! - persists directories above the threshold once their totals are final.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::cancel::CancellationToken;
use crate::domain::file_entry::{FileKind, FileRecord, ScannedEntry};
use crate::domain::platform::Platform;
use crate::domain::scan::{NewScan, Scan, ScanId, ScanStats};
use crate::domain::size::ByteSize;
use crate::error::{CoreError, RepoError};
use crate::policy::protected_paths::{ProtectedPathPolicy, default_scan_exclusions};
use crate::ports::Clock;
use crate::ports::filesystem::{FileSystemScanner, ScanOptions, ScanSink};
use crate::ports::repository::{FileEntryRepository, ScanRepository};

/// Snapshot sent to the UI while scanning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanProgress {
    pub scan_id: ScanId,
    pub stats: ScanStats,
    pub current_path: PathBuf,
}

/// Receives throttled progress notifications (Observer pattern).
pub trait ScanObserver {
    fn on_progress(&mut self, progress: &ScanProgress);
    fn on_inaccessible(&mut self, path: &Path);
}

#[derive(Debug, Clone, Copy)]
pub struct ScanConfig {
    /// Rows per insert transaction.
    pub batch_size: usize,
    /// Minimum time between two progress notifications.
    pub progress_interval: Duration,
    /// System being scanned (default exclusions depend on it).
    pub platform: Platform,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            batch_size: 5_000,
            progress_interval: Duration::from_millis(100),
            platform: Platform::current(),
        }
    }
}

/// Dependencies of the scan use case, injected by the composition root.
#[derive(Clone)]
pub struct ScanService {
    scanner: Arc<dyn FileSystemScanner>,
    scans: Arc<dyn ScanRepository>,
    files: Arc<dyn FileEntryRepository>,
    policy: Arc<ProtectedPathPolicy>,
    clock: Arc<dyn Clock>,
    config: ScanConfig,
}

impl std::fmt::Debug for ScanService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScanService")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl ScanService {
    pub fn new(
        scanner: Arc<dyn FileSystemScanner>,
        scans: Arc<dyn ScanRepository>,
        files: Arc<dyn FileEntryRepository>,
        policy: Arc<ProtectedPathPolicy>,
        clock: Arc<dyn Clock>,
        config: ScanConfig,
    ) -> Self {
        Self {
            scanner,
            scans,
            files,
            policy,
            clock,
            config,
        }
    }

    /// Marks scans left `Running` by a previous crash as failed. Call once at startup.
    pub fn recover_interrupted(&self) -> Result<usize, CoreError> {
        let running = self.scans.find_running()?;
        let count = running.len();
        for mut scan in running {
            let stats = scan.stats;
            scan.fail(self.clock.now(), stats, "interrupted".into())?;
            self.scans.save(&scan)?;
        }
        Ok(count)
    }

    /// Validates the request and registers a new `Running` scan.
    pub fn begin(&self, roots: Vec<PathBuf>, min_file_size: ByteSize) -> Result<Scan, CoreError> {
        if !self.scans.find_running()?.is_empty() {
            return Err(CoreError::ScanInProgress);
        }
        let new = NewScan::new(normalize_roots(roots), min_file_size, self.clock.now())?;
        Ok(self.scans.create(&new)?)
    }

    /// Runs a scan created by [`Self::begin`] to completion, cancellation or failure.
    ///
    /// A scanner failure is not an `Err`: it is recorded in the returned scan (`Failed`).
    /// `Err` means the catalog itself could not be written.
    pub fn execute(
        &self,
        mut scan: Scan,
        cancel: &CancellationToken,
        observer: &mut dyn ScanObserver,
    ) -> Result<Scan, CoreError> {
        let options = ScanOptions {
            excluded: exclusions_for(&scan.roots, self.config.platform),
        };
        // The builder cancels this internal token if persisting fails, so the walk stops early.
        let walk_cancel = CancellationToken::new();
        let mut builder = CatalogBuilder::new(&scan, self, &walk_cancel, cancel, observer);
        let walk = self
            .scanner
            .scan(&scan.roots, &options, &walk_cancel, &mut builder);
        let stats = builder.finish()?;

        let now = self.clock.now();
        match walk {
            Err(e) => scan.fail(now, stats, e.to_string())?,
            Ok(()) if cancel.is_cancelled() => scan.cancel(now, stats)?,
            Ok(()) => scan.complete(now, stats)?,
        }
        self.scans.save(&scan)?;
        if scan.status == crate::domain::scan::ScanStatus::Completed {
            // Only the newest complete catalog is kept; cancelled scans never replace it.
            self.scans.delete_all_except(scan.id)?;
        }
        Ok(scan)
    }
}

/// Removes duplicates and roots nested inside other roots, so nothing is counted twice.
fn normalize_roots(mut roots: Vec<PathBuf>) -> Vec<PathBuf> {
    roots.sort();
    roots.dedup();
    let mut out: Vec<PathBuf> = Vec::with_capacity(roots.len());
    for root in roots {
        if !out.iter().any(|kept| root.starts_with(kept)) {
            out.push(root);
        }
    }
    out
}

/// Default exclusions, except those the user explicitly chose to scan (e.g. `/Volumes/USB`).
fn exclusions_for(roots: &[PathBuf], platform: Platform) -> Vec<PathBuf> {
    default_scan_exclusions(platform)
        .into_iter()
        .filter(|excluded| !roots.iter().any(|root| root.starts_with(excluded)))
        .collect()
}

#[derive(Debug, Default, Clone, Copy)]
struct DirTotals {
    size_on_disk: ByteSize,
    apparent_size: ByteSize,
    file_count: u64,
}

struct CatalogBuilder<'a> {
    scan_id: ScanId,
    roots: &'a [PathBuf],
    min_file_size: ByteSize,
    service: &'a ScanService,
    walk_cancel: &'a CancellationToken,
    user_cancel: &'a CancellationToken,
    observer: &'a mut dyn ScanObserver,

    stats: ScanStats,
    seen_inodes: HashSet<(u64, u64)>,
    dir_totals: HashMap<PathBuf, DirTotals>,
    dir_meta: HashMap<PathBuf, ScannedEntry>,
    pending: Vec<FileRecord>,
    last_progress: Instant,
    error: Option<RepoError>,
}

impl<'a> CatalogBuilder<'a> {
    fn new(
        scan: &'a Scan,
        service: &'a ScanService,
        walk_cancel: &'a CancellationToken,
        user_cancel: &'a CancellationToken,
        observer: &'a mut dyn ScanObserver,
    ) -> Self {
        Self {
            scan_id: scan.id,
            roots: &scan.roots,
            min_file_size: scan.min_file_size,
            service,
            walk_cancel,
            user_cancel,
            observer,
            stats: ScanStats::default(),
            seen_inodes: HashSet::new(),
            dir_totals: HashMap::new(),
            dir_meta: HashMap::new(),
            pending: Vec::new(),
            last_progress: Instant::now(),
            error: None,
        }
    }

    fn on_file(&mut self, entry: &ScannedEntry) {
        // A file with several hard links occupies its blocks once; count the first link only.
        if entry.hard_links > 1 && !self.seen_inodes.insert((entry.device, entry.inode)) {
            return;
        }
        self.stats.files_seen += 1;
        self.stats.bytes_seen += entry.size_on_disk;
        self.add_to_ancestors(entry);
        if entry.size_on_disk >= self.min_file_size {
            let record = self.record_from(entry, 1, entry.size_on_disk, entry.apparent_size);
            self.pending.push(record);
            if self.pending.len() >= self.service.config.batch_size {
                self.flush();
            }
        }
    }

    fn add_to_ancestors(&mut self, entry: &ScannedEntry) {
        let Some(root) = self.roots.iter().find(|r| entry.path.starts_with(r)) else {
            return;
        };
        for ancestor in entry.path.ancestors().skip(1) {
            if !ancestor.starts_with(root) {
                break;
            }
            // `get_mut` first avoids allocating a PathBuf for every ancestor of every file.
            let totals = match self.dir_totals.get_mut(ancestor) {
                Some(t) => t,
                None => self.dir_totals.entry(ancestor.to_path_buf()).or_default(),
            };
            totals.size_on_disk += entry.size_on_disk;
            totals.apparent_size += entry.apparent_size;
            totals.file_count += 1;
        }
    }

    fn record_from(
        &self,
        entry: &ScannedEntry,
        file_count: u64,
        size_on_disk: ByteSize,
        apparent_size: ByteSize,
    ) -> FileRecord {
        FileRecord {
            path: entry.path.clone(),
            kind: entry.kind,
            size_on_disk,
            apparent_size,
            file_count,
            created_at: entry.created_at,
            modified_at: entry.modified_at,
            accessed_at: entry.accessed_at,
            inode: entry.inode,
            device: entry.device,
            is_hardlink_dup: false,
            is_protected: self.service.policy.is_protected(&entry.path),
        }
    }

    fn maybe_report_progress(&mut self, current: &Path) {
        if self.user_cancel.is_cancelled() {
            self.walk_cancel.cancel();
        }
        if self.last_progress.elapsed() >= self.service.config.progress_interval {
            self.last_progress = Instant::now();
            self.observer.on_progress(&ScanProgress {
                scan_id: self.scan_id,
                stats: self.stats,
                current_path: current.to_path_buf(),
            });
        }
    }

    fn flush(&mut self) {
        if self.pending.is_empty() || self.error.is_some() {
            return;
        }
        let batch = std::mem::take(&mut self.pending);
        if let Err(e) = self.service.files.insert_batch(self.scan_id, &batch) {
            self.error = Some(e);
            self.walk_cancel.cancel();
        }
    }

    /// Persists remaining files and every directory above the threshold.
    fn finish(mut self) -> Result<ScanStats, RepoError> {
        self.flush();
        if let Some(e) = self.error.take() {
            return Err(e);
        }
        let dir_totals = std::mem::take(&mut self.dir_totals);
        let mut dirs: Vec<FileRecord> = dir_totals
            .into_iter()
            .filter(|(_, t)| t.size_on_disk >= self.min_file_size)
            .map(|(path, t)| match self.dir_meta.get(&path) {
                Some(meta) => self.record_from(meta, t.file_count, t.size_on_disk, t.apparent_size),
                None => self.record_from(
                    &bare_dir(path),
                    t.file_count,
                    t.size_on_disk,
                    t.apparent_size,
                ),
            })
            .collect();
        dirs.sort_by(|a, b| a.path.cmp(&b.path));
        for chunk in dirs.chunks(self.service.config.batch_size.max(1)) {
            self.service.files.insert_batch(self.scan_id, chunk)?;
        }
        let stats = self.stats;
        let last_path = self.roots.first().cloned().unwrap_or_default();
        self.observer.on_progress(&ScanProgress {
            scan_id: self.scan_id,
            stats,
            current_path: last_path,
        });
        Ok(stats)
    }
}

impl ScanSink for CatalogBuilder<'_> {
    fn on_entry(&mut self, entry: ScannedEntry) {
        let path = entry.path.clone();
        match entry.kind {
            FileKind::File => self.on_file(&entry),
            FileKind::Directory | FileKind::Package => {
                self.dir_meta.insert(entry.path.clone(), entry);
            }
        }
        self.maybe_report_progress(&path);
    }

    fn on_inaccessible(&mut self, path: &Path, _reason: &str) {
        self.stats.inaccessible_count += 1;
        self.observer.on_inaccessible(path);
    }
}

/// Metadata for a directory the scanner never reported itself (only its children).
fn bare_dir(path: PathBuf) -> ScannedEntry {
    ScannedEntry {
        path,
        kind: FileKind::Directory,
        size_on_disk: ByteSize::ZERO,
        apparent_size: ByteSize::ZERO,
        created_at: None,
        modified_at: None,
        accessed_at: None,
        inode: 0,
        device: 0,
        hard_links: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::scan::ScanStatus;
    use crate::ports::repository::{FileFilter, FileQuery, FileSort};
    use crate::testing::{
        FakeClock, FakeScanner, InMemoryFileEntryRepository, InMemoryScanRepository,
    };

    const MB: u64 = 1_000_000;

    fn file(path: &str, size: u64) -> ScannedEntry {
        ScannedEntry {
            path: PathBuf::from(path),
            kind: FileKind::File,
            size_on_disk: ByteSize::new(size),
            apparent_size: ByteSize::new(size),
            created_at: None,
            modified_at: Some(1),
            accessed_at: None,
            inode: size,
            device: 1,
            hard_links: 1,
        }
    }

    fn dir(path: &str) -> ScannedEntry {
        ScannedEntry {
            kind: FileKind::Directory,
            ..file(path, 0)
        }
    }

    #[derive(Default)]
    struct RecordingObserver {
        progress: Vec<ScanProgress>,
        inaccessible: Vec<PathBuf>,
    }

    impl ScanObserver for RecordingObserver {
        fn on_progress(&mut self, progress: &ScanProgress) {
            self.progress.push(progress.clone());
        }
        fn on_inaccessible(&mut self, path: &Path) {
            self.inaccessible.push(path.to_path_buf());
        }
    }

    struct Harness {
        service: ScanService,
        scans: Arc<InMemoryScanRepository>,
        files: Arc<InMemoryFileEntryRepository>,
    }

    fn harness(scanner: FakeScanner) -> Harness {
        let scans = Arc::new(InMemoryScanRepository::default());
        let files = Arc::new(InMemoryFileEntryRepository::default());
        let policy = ProtectedPathPolicy::builder().tree("/protected").build();
        let service = ScanService::new(
            Arc::new(scanner),
            scans.clone(),
            files.clone(),
            Arc::new(policy),
            Arc::new(FakeClock::at(1_000)),
            ScanConfig {
                batch_size: 2,
                progress_interval: Duration::ZERO,
                platform: Platform::MacOs,
            },
        );
        Harness {
            service,
            scans,
            files,
        }
    }

    fn run(h: &Harness, roots: &[&str], min_mb: u64) -> (Scan, RecordingObserver) {
        let scan = h
            .service
            .begin(
                roots.iter().map(PathBuf::from).collect(),
                ByteSize::from_mb(min_mb),
            )
            .unwrap();
        let mut observer = RecordingObserver::default();
        let scan = h
            .service
            .execute(scan, &CancellationToken::new(), &mut observer)
            .unwrap();
        (scan, observer)
    }

    fn listed(h: &Harness, scan: &Scan) -> Vec<(String, u64)> {
        let q = FileQuery::new(FileFilter::default(), FileSort::SizeDesc, 0, 500);
        h.files
            .page(scan.id, &q)
            .unwrap()
            .items
            .into_iter()
            .map(|e| {
                (
                    e.record.path.to_string_lossy().into_owned(),
                    e.record.size_on_disk.bytes(),
                )
            })
            .collect()
    }

    #[test]
    fn lists_files_and_folders_above_threshold_in_descending_order() {
        // Acceptance scenario US1-1: 5 GB, 2 GB and 50 MB with a 100 MB threshold.
        let h = harness(FakeScanner {
            entries: vec![
                dir("/data"),
                file("/data/big.mov", 5_000 * MB),
                file("/data/medium.iso", 2_000 * MB),
                file("/data/small.txt", 50 * MB),
            ],
            ..FakeScanner::default()
        });

        let (scan, _) = run(&h, &["/data"], 100);

        assert_eq!(scan.status, ScanStatus::Completed);
        assert_eq!(
            listed(&h, &scan),
            [
                ("/data".to_owned(), 7_050 * MB),
                ("/data/big.mov".to_owned(), 5_000 * MB),
                ("/data/medium.iso".to_owned(), 2_000 * MB),
            ]
        );
        assert_eq!(scan.stats.files_seen, 3);
        assert_eq!(scan.stats.bytes_seen, ByteSize::new(7_050 * MB));
    }

    #[test]
    fn folders_aggregate_many_small_files() {
        let mut entries = vec![dir("/root"), dir("/root/game")];
        entries.extend((0..300).map(|i| file(&format!("/root/game/asset{i}.pak"), MB)));
        let h = harness(FakeScanner {
            entries,
            ..FakeScanner::default()
        });

        let (scan, _) = run(&h, &["/root"], 100);

        let game = h.files.find_by_path(Path::new("/root/game")).unwrap();
        assert_eq!(game.record.size_on_disk, ByteSize::new(300 * MB));
        assert_eq!(game.record.file_count, 300);
        assert_eq!(
            listed(&h, &scan).len(),
            2,
            "only the two folders cross 100 MB"
        );
    }

    #[test]
    fn hard_links_are_counted_once() {
        let mut a = file("/r/a.bin", 500 * MB);
        a.hard_links = 2;
        let b = ScannedEntry {
            path: PathBuf::from("/r/b.bin"),
            ..a.clone()
        };
        let h = harness(FakeScanner {
            entries: vec![a, b],
            ..FakeScanner::default()
        });

        let (scan, _) = run(&h, &["/r"], 100);

        assert_eq!(scan.stats.bytes_seen, ByteSize::new(500 * MB));
        assert_eq!(
            h.files
                .find_by_path(Path::new("/r"))
                .unwrap()
                .record
                .size_on_disk
                .bytes(),
            500 * MB
        );
        assert!(h.files.find_by_path(Path::new("/r/b.bin")).is_none());
    }

    #[test]
    fn inaccessible_paths_are_counted_and_reported() {
        // Acceptance scenario US1-4.
        let h = harness(FakeScanner {
            entries: vec![file("/r/ok.bin", 200 * MB)],
            inaccessible: vec![PathBuf::from("/r/secret")],
            ..FakeScanner::default()
        });

        let (scan, observer) = run(&h, &["/r"], 100);

        assert_eq!(scan.status, ScanStatus::Completed);
        assert_eq!(scan.stats.inaccessible_count, 1);
        assert_eq!(observer.inaccessible, [PathBuf::from("/r/secret")]);
    }

    #[test]
    fn cancelling_keeps_partial_results_and_marks_scan_cancelled() {
        // Acceptance scenario US1-2: the user cancels while the walk is running.
        let h = harness(FakeScanner {
            entries: vec![
                file("/r/a", 300 * MB),
                file("/r/b", 400 * MB),
                file("/r/c", 500 * MB),
            ],
            ..FakeScanner::default()
        });
        let scan = h
            .service
            .begin(vec!["/r".into()], ByteSize::from_mb(100))
            .unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();

        let scan = h
            .service
            .execute(scan, &cancel, &mut RecordingObserver::default())
            .unwrap();

        assert_eq!(scan.status, ScanStatus::Cancelled);
        assert!(scan.is_incomplete());
        assert_eq!(
            scan.stats.files_seen, 1,
            "the walk stops right after observing the cancel"
        );
        assert!(h.files.find_by_path(Path::new("/r/a")).is_some());
        assert_eq!(
            h.scans.latest_finished().unwrap().map(|s| s.id),
            Some(scan.id)
        );
    }

    #[test]
    fn scanner_failure_marks_scan_failed() {
        let h = harness(FakeScanner {
            fail_with: Some("disk vanished".into()),
            ..FakeScanner::default()
        });
        let (scan, _) = run(&h, &["/r"], 100);
        assert_eq!(scan.status, ScanStatus::Failed);
        assert_eq!(scan.error.as_deref(), Some("scan I/O error: disk vanished"));
    }

    #[test]
    fn only_one_scan_can_run_at_a_time() {
        let h = harness(FakeScanner::default());
        h.service.begin(vec!["/a".into()], ByteSize::ZERO).unwrap();
        let err = h
            .service
            .begin(vec!["/b".into()], ByteSize::ZERO)
            .unwrap_err();
        assert!(matches!(err, CoreError::ScanInProgress));
    }

    #[test]
    fn interrupted_scans_are_failed_on_recovery() {
        let h = harness(FakeScanner::default());
        let scan = h.service.begin(vec!["/a".into()], ByteSize::ZERO).unwrap();
        assert_eq!(h.service.recover_interrupted().unwrap(), 1);
        let stored = h.scans.get(scan.id).unwrap().unwrap();
        assert_eq!(stored.status, ScanStatus::Failed);
        assert!(h.service.begin(vec!["/a".into()], ByteSize::ZERO).is_ok());
    }

    #[test]
    fn completed_scan_replaces_previous_catalog() {
        let h = harness(FakeScanner::default());
        let (first, _) = run(&h, &["/r"], 1);
        let (second, _) = run(&h, &["/r"], 1);
        assert_eq!(h.scans.get(first.id).unwrap(), None);
        assert!(h.scans.get(second.id).unwrap().is_some());
    }

    #[test]
    fn protected_items_are_flagged() {
        let h = harness(FakeScanner {
            entries: vec![file("/protected/kernel", 900 * MB)],
            ..FakeScanner::default()
        });
        run(&h, &["/protected"], 100);
        assert!(
            h.files
                .find_by_path(Path::new("/protected/kernel"))
                .unwrap()
                .record
                .is_protected
        );
    }

    #[test]
    fn progress_is_reported_and_ends_with_final_totals() {
        let h = harness(FakeScanner {
            entries: vec![file("/r/a", 10), file("/r/b", 20)],
            ..FakeScanner::default()
        });
        let (_, observer) = run(&h, &["/r"], 100);
        let last = observer.progress.last().unwrap();
        assert_eq!(last.stats.files_seen, 2);
        assert_eq!(last.stats.bytes_seen, ByteSize::new(30));
    }

    #[test]
    fn nested_and_duplicate_roots_are_collapsed() {
        assert_eq!(
            normalize_roots(vec!["/a/b".into(), "/a".into(), "/c".into(), "/a".into()]),
            [PathBuf::from("/a"), PathBuf::from("/c")]
        );
    }

    #[test]
    fn explicit_external_volume_is_not_excluded() {
        let excluded = exclusions_for(&[PathBuf::from("/Volumes/USB")], Platform::MacOs);
        assert!(!excluded.contains(&PathBuf::from("/Volumes")));
        assert!(
            exclusions_for(&[PathBuf::from("/")], Platform::MacOs)
                .contains(&PathBuf::from("/Volumes"))
        );
    }
}
