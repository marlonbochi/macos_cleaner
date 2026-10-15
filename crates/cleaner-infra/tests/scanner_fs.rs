//! Integration tests of `JwalkScanner` against real directory trees.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cleaner_core::cancel::CancellationToken;
use cleaner_core::domain::file_entry::{FileKind, ScannedEntry};
use cleaner_core::domain::size::ByteSize;
use cleaner_core::policy::protected_paths::ProtectedPathPolicy;
use cleaner_core::ports::filesystem::{FileInspector, FileSystemScanner, ScanOptions, ScanSink};
use cleaner_core::services::scan_service::{ScanConfig, ScanObserver, ScanProgress, ScanService};
use cleaner_core::testing::FakeClock;
use cleaner_infra::db::{Database, SqliteFileEntryRepository, SqliteScanRepository};
use cleaner_infra::fs::{JwalkScanner, StdFileInspector};

#[derive(Default)]
struct Collect {
    entries: HashMap<PathBuf, ScannedEntry>,
    inaccessible: Vec<PathBuf>,
}

impl ScanSink for Collect {
    fn on_entry(&mut self, entry: ScannedEntry) {
        self.entries.insert(entry.path.clone(), entry);
    }
    fn on_inaccessible(&mut self, path: &Path, _reason: &str) {
        self.inaccessible.push(path.to_path_buf());
    }
}

fn write_file(path: &Path, bytes: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, vec![7u8; bytes]).unwrap();
}

fn scan(root: &Path, options: &ScanOptions) -> Collect {
    let mut sink = Collect::default();
    JwalkScanner::new(2)
        .scan(
            &[root.to_path_buf()],
            options,
            &CancellationToken::new(),
            &mut sink,
        )
        .unwrap();
    sink
}

/// Canonical temp dir: on macOS `/var` is a symlink to `/private/var`, and roots must not be links.
fn tempdir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    // On Windows `canonicalize` returns `\\?\` paths, which do not accept `/` separators.
    let root = if cfg!(windows) {
        dir.path().to_path_buf()
    } else {
        dir.path().canonicalize().unwrap()
    };
    (dir, root)
}

#[test]
fn reports_files_directories_and_packages_with_real_sizes() {
    let (_dir, root) = tempdir();
    write_file(&root.join("docs/report.pdf"), 100_000);
    write_file(&root.join("Tool.app/Contents/MacOS/tool"), 10_000);

    let out = scan(&root, &ScanOptions::default());

    let report = &out.entries[&root.join("docs/report.pdf")];
    assert_eq!(report.kind, FileKind::File);
    assert_eq!(report.apparent_size, ByteSize::new(100_000));
    assert!(
        report.size_on_disk >= ByteSize::new(100_000),
        "allocated blocks cover the data"
    );
    assert_eq!(report.size_on_disk.bytes() % 512, 0);
    assert!(report.modified_at.is_some());
    assert_eq!(out.entries[&root].kind, FileKind::Directory);
    assert_eq!(out.entries[&root.join("docs")].kind, FileKind::Directory);
    let bundle_kind = if cfg!(target_os = "macos") {
        FileKind::Package
    } else {
        FileKind::Directory
    };
    assert_eq!(out.entries[&root.join("Tool.app")].kind, bundle_kind);
    assert!(
        out.entries
            .contains_key(&root.join("Tool.app/Contents/MacOS/tool"))
    );
    assert!(out.inaccessible.is_empty());
}

#[cfg(unix)]
#[test]
fn symlinks_are_neither_reported_nor_followed() {
    let (_dir, root) = tempdir();
    let (_outside_dir, outside) = tempdir();
    write_file(&outside.join("elsewhere.bin"), 1_000);
    write_file(&root.join("real.bin"), 1_000);
    std::os::unix::fs::symlink(&outside, root.join("link-to-dir")).unwrap();
    std::os::unix::fs::symlink(root.join("real.bin"), root.join("link-to-file")).unwrap();

    let out = scan(&root, &ScanOptions::default());

    assert!(!out.entries.contains_key(&root.join("link-to-dir")));
    assert!(!out.entries.contains_key(&root.join("link-to-file")));
    assert!(!out.entries.keys().any(|p| p.ends_with("elsewhere.bin")));
    assert!(out.entries.keys().all(|p| p.starts_with(&root)));
}

#[cfg(unix)]
#[test]
fn hard_links_share_inode_and_report_link_count() {
    let (_dir, root) = tempdir();
    write_file(&root.join("a.bin"), 4_096);
    fs::hard_link(root.join("a.bin"), root.join("b.bin")).unwrap();

    let out = scan(&root, &ScanOptions::default());

    let (a, b) = (
        &out.entries[&root.join("a.bin")],
        &out.entries[&root.join("b.bin")],
    );
    assert_eq!(a.inode, b.inode);
    assert_eq!(a.hard_links, 2);
}

#[cfg(unix)]
#[test]
fn unreadable_directories_are_reported_and_scan_continues() {
    use std::os::unix::fs::PermissionsExt;
    let (_dir, root) = tempdir();
    write_file(&root.join("locked/secret.bin"), 1_000);
    write_file(&root.join("open/visible.bin"), 1_000);
    let locked = root.join("locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    let out = scan(&root, &ScanOptions::default());
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    // Running as root (e.g. some CI containers) bypasses permissions; nothing to assert then.
    if fs::read_dir(&locked).is_ok() && out.inaccessible.is_empty() && is_root() {
        return;
    }
    assert!(
        out.inaccessible.contains(&locked),
        "inaccessible: {:?}",
        out.inaccessible
    );
    assert!(out.entries.contains_key(&root.join("open/visible.bin")));
    assert!(!out.entries.contains_key(&root.join("locked/secret.bin")));
}

#[test]
fn excluded_subtrees_are_not_entered() {
    let (_dir, root) = tempdir();
    write_file(&root.join("keep/a.bin"), 10);
    write_file(&root.join("skip/b.bin"), 10);

    let out = scan(
        &root,
        &ScanOptions {
            excluded: vec![root.join("skip")],
        },
    );

    assert!(out.entries.contains_key(&root.join("keep/a.bin")));
    assert!(!out.entries.keys().any(|p| p.starts_with(root.join("skip"))));
}

#[test]
fn missing_root_is_reported_as_inaccessible() {
    let (_dir, root) = tempdir();
    let missing = root.join("does-not-exist");
    let out = scan(&missing, &ScanOptions::default());
    assert!(out.entries.is_empty());
    assert_eq!(out.inaccessible, [missing]);
}

#[test]
fn cancelled_scan_returns_promptly_without_entries() {
    let (_dir, root) = tempdir();
    write_file(&root.join("a.bin"), 10);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let mut sink = Collect::default();
    JwalkScanner::new(2)
        .scan(&[root], &ScanOptions::default(), &cancel, &mut sink)
        .unwrap();
    assert!(sink.entries.is_empty());
}

#[test]
fn inspector_detects_changes_and_deletions() {
    let (_dir, root) = tempdir();
    let path = root.join("a.bin");
    write_file(&path, 1_000);
    let before = StdFileInspector.fingerprint(&path).unwrap().unwrap();

    fs::write(&path, vec![1u8; 200_000]).unwrap();
    let after = StdFileInspector.fingerprint(&path).unwrap().unwrap();
    assert_ne!(before, after);

    fs::remove_file(&path).unwrap();
    assert_eq!(StdFileInspector.fingerprint(&path).unwrap(), None);
}

struct NoopObserver;
impl ScanObserver for NoopObserver {
    fn on_progress(&mut self, _: &ScanProgress) {}
    fn on_inaccessible(&mut self, _: &Path) {}
}

#[test]
fn end_to_end_scan_into_sqlite() {
    let (_dir, root) = tempdir();
    write_file(&root.join("big/one.bin"), 300_000);
    write_file(&root.join("big/two.bin"), 200_000);
    write_file(&root.join("small.txt"), 10);
    let db = Arc::new(Database::open(&root.join(".app/catalog.sqlite3")).unwrap());
    let scans = Arc::new(SqliteScanRepository::new(db.clone()));
    let files = Arc::new(SqliteFileEntryRepository::new(db));
    let service = ScanService::new(
        Arc::new(JwalkScanner::new(2)),
        scans,
        files.clone(),
        Arc::new(ProtectedPathPolicy::default()),
        Arc::new(FakeClock::at(1)),
        ScanConfig::default(),
    );

    let started = service
        .begin(vec![root.clone()], ByteSize::new(150_000))
        .unwrap();
    let scan = service
        .execute(started, &CancellationToken::new(), &mut NoopObserver)
        .unwrap();

    use cleaner_core::ports::repository::{FileEntryRepository, FileFilter, FileQuery, FileSort};
    let page = files
        .page(
            scan.id,
            &FileQuery::new(FileFilter::default(), FileSort::SizeDesc, 0, 50),
        )
        .unwrap();
    let paths: Vec<_> = page.items.iter().map(|e| e.record.path.clone()).collect();
    // The catalog database itself lives under the root in this test and is small.
    assert_eq!(paths[..2], [root.clone(), root.join("big")]);
    assert!(paths.contains(&root.join("big/one.bin")));
    assert!(paths.contains(&root.join("big/two.bin")));
    assert!(!paths.contains(&root.join("small.txt")));
    assert!(scan.stats.files_seen >= 3);
}

#[cfg(unix)]
fn is_root() -> bool {
    std::env::var("USER").is_ok_and(|u| u == "root")
}
