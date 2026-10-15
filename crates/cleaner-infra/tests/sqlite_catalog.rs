//! Integration tests of the SQLite scan and file-entry repositories against a real database file.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;

use cleaner_core::domain::file_entry::{FileKind, FileRecord};
use cleaner_core::domain::scan::{NewScan, ScanStats};
use cleaner_core::domain::size::ByteSize;
use cleaner_core::ports::repository::{
    FileEntryRepository, FileFilter, FileQuery, FileSort, ScanRepository,
};
use cleaner_infra::db::{Database, SqliteFileEntryRepository, SqliteScanRepository};

struct Fixture {
    _dir: tempfile::TempDir,
    db: Arc<Database>,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let db = Arc::new(Database::open(&dir.path().join("nested/catalog.sqlite3")).unwrap());
    Fixture { _dir: dir, db }
}

fn record(path: &str, kind: FileKind, size: u64, modified: i64) -> FileRecord {
    FileRecord {
        path: PathBuf::from(path),
        kind,
        size_on_disk: ByteSize::new(size),
        apparent_size: ByteSize::new(size),
        file_count: 1,
        created_at: None,
        modified_at: Some(modified),
        accessed_at: None,
        inode: u64::MAX - size, // exercises values above i64::MAX
        device: 1,
        is_hardlink_dup: false,
        is_protected: false,
    }
}

fn new_scan(started_at: i64) -> NewScan {
    NewScan::new(
        // An absolute root on the system running the tests.
        vec![std::env::temp_dir()],
        ByteSize::from_mb(100),
        started_at,
    )
    .unwrap()
}

#[test]
fn opening_twice_reapplies_no_migration_and_keeps_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.sqlite3");
    {
        let db = Arc::new(Database::open(&path).unwrap());
        SqliteScanRepository::new(db).create(&new_scan(1)).unwrap();
    }
    let db = Arc::new(Database::open(&path).unwrap());
    assert_eq!(
        SqliteScanRepository::new(db).find_running().unwrap().len(),
        1
    );
}

#[test]
fn scan_lifecycle_is_persisted() {
    let f = fixture();
    let repo = SqliteScanRepository::new(f.db.clone());

    let mut scan = repo.create(&new_scan(10)).unwrap();
    assert_eq!(repo.find_running().unwrap(), vec![scan.clone()]);
    assert_eq!(repo.latest_finished().unwrap(), None);

    let stats = ScanStats {
        files_seen: 42,
        bytes_seen: ByteSize::new(9_000),
        inaccessible_count: 2,
    };
    scan.complete(20, stats).unwrap();
    repo.save(&scan).unwrap();

    assert_eq!(repo.get(scan.id).unwrap(), Some(scan.clone()));
    assert_eq!(repo.latest_finished().unwrap(), Some(scan));
    assert!(repo.find_running().unwrap().is_empty());
}

#[test]
fn latest_finished_ignores_failed_scans() {
    let f = fixture();
    let repo = SqliteScanRepository::new(f.db.clone());
    let mut ok = repo.create(&new_scan(1)).unwrap();
    ok.cancel(2, ScanStats::default()).unwrap();
    repo.save(&ok).unwrap();
    let mut failed = repo.create(&new_scan(3)).unwrap();
    failed.fail(4, ScanStats::default(), "boom".into()).unwrap();
    repo.save(&failed).unwrap();

    assert_eq!(repo.latest_finished().unwrap().map(|s| s.id), Some(ok.id));
}

#[test]
fn delete_all_except_cascades_to_entries() {
    let f = fixture();
    let scans = SqliteScanRepository::new(f.db.clone());
    let files = SqliteFileEntryRepository::new(f.db.clone());
    let old = scans.create(&new_scan(1)).unwrap();
    let new = scans.create(&new_scan(2)).unwrap();
    files
        .insert_batch(old.id, &[record("/tmp/a", FileKind::File, 1, 0)])
        .unwrap();
    files
        .insert_batch(new.id, &[record("/tmp/b", FileKind::File, 1, 0)])
        .unwrap();

    scans.delete_all_except(new.id).unwrap();

    assert_eq!(scans.get(old.id).unwrap(), None);
    let all = FileQuery::new(FileFilter::default(), FileSort::SizeDesc, 0, 100);
    assert_eq!(files.page(old.id, &all).unwrap().total, 0);
    assert_eq!(files.page(new.id, &all).unwrap().total, 1);
}

#[test]
fn entries_are_paginated_in_size_order() {
    let f = fixture();
    let scan = SqliteScanRepository::new(f.db.clone())
        .create(&new_scan(1))
        .unwrap();
    let files = SqliteFileEntryRepository::new(f.db.clone());
    let records: Vec<_> = (1..=25)
        .map(|i| {
            record(
                &format!("/tmp/f{i}.bin"),
                FileKind::File,
                i * 1_000,
                i64::try_from(i).unwrap(),
            )
        })
        .collect();
    files.insert_batch(scan.id, &records).unwrap();

    let page = files
        .page(
            scan.id,
            &FileQuery::new(FileFilter::default(), FileSort::SizeDesc, 10, 10),
        )
        .unwrap();

    assert_eq!(page.total, 25);
    assert_eq!(page.items.len(), 10);
    let sizes: Vec<u64> = page
        .items
        .iter()
        .map(|e| e.record.size_on_disk.bytes())
        .collect();
    assert_eq!(sizes.first(), Some(&15_000));
    assert_eq!(sizes.last(), Some(&6_000));
    assert!(sizes.windows(2).all(|w| w[0] >= w[1]));
    assert_eq!(page.items[0].record, records[14]);
}

#[test]
fn filters_by_size_extension_kind_parent_and_prefix() {
    let f = fixture();
    let scan = SqliteScanRepository::new(f.db.clone())
        .create(&new_scan(1))
        .unwrap();
    let files = SqliteFileEntryRepository::new(f.db.clone());
    files
        .insert_batch(
            scan.id,
            &[
                record("/u/Movies/a.MOV", FileKind::File, 5_000, 1),
                record("/u/Movies/b.zip", FileKind::File, 3_000, 2),
                record("/u/Movies/sub/c.mov", FileKind::File, 1_000, 3),
                record("/u/Movies", FileKind::Directory, 9_000, 4),
                record("/u/Movies_old/d.mov", FileKind::File, 8_000, 5),
                record("/u/100%_done/e.mov", FileKind::File, 2_000, 6),
            ],
        )
        .unwrap();
    let query = |filter: FileFilter| {
        files
            .page(scan.id, &FileQuery::new(filter, FileSort::SizeDesc, 0, 100))
            .unwrap()
            .items
            .into_iter()
            .map(|e| e.record.path.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
    };

    assert_eq!(
        query(FileFilter {
            min_size: Some(ByteSize::new(5_000)),
            ..FileFilter::default()
        }),
        ["/u/Movies", "/u/Movies_old/d.mov", "/u/Movies/a.MOV"]
    );
    assert_eq!(
        query(FileFilter {
            extensions: vec!["MOV".into()],
            ..FileFilter::default()
        }),
        [
            "/u/Movies_old/d.mov",
            "/u/Movies/a.MOV",
            "/u/100%_done/e.mov",
            "/u/Movies/sub/c.mov"
        ]
    );
    assert_eq!(
        query(FileFilter {
            kinds: vec![FileKind::Directory],
            ..FileFilter::default()
        }),
        ["/u/Movies"]
    );
    assert_eq!(
        query(FileFilter {
            parent: Some("/u/Movies".into()),
            ..FileFilter::default()
        }),
        ["/u/Movies/a.MOV", "/u/Movies/b.zip"]
    );
    // Prefix is component based: /u/Movies_old is not below /u/Movies.
    assert_eq!(
        query(FileFilter {
            path_prefix: Some("/u/Movies".into()),
            ..FileFilter::default()
        }),
        ["/u/Movies/a.MOV", "/u/Movies/b.zip", "/u/Movies/sub/c.mov"]
    );
    // LIKE wildcards in the prefix are escaped.
    assert_eq!(
        query(FileFilter {
            path_prefix: Some("/u/100%_done".into()),
            ..FileFilter::default()
        }),
        ["/u/100%_done/e.mov"]
    );
}

#[test]
fn get_many_and_remove_work_across_chunks() {
    let f = fixture();
    let scan = SqliteScanRepository::new(f.db.clone())
        .create(&new_scan(1))
        .unwrap();
    let files = SqliteFileEntryRepository::new(f.db.clone());
    let records: Vec<_> = (0..1_200)
        .map(|i| record(&format!("/tmp/{i}"), FileKind::File, i + 1, 0))
        .collect();
    files.insert_batch(scan.id, &records).unwrap();
    let all = files
        .page(
            scan.id,
            &FileQuery::new(FileFilter::default(), FileSort::SizeAsc, 0, 500),
        )
        .unwrap();
    assert_eq!(all.total, 1_200);

    let ids: Vec<_> = (1..=1_200)
        .map(cleaner_core::domain::file_entry::FileId)
        .collect();
    assert_eq!(files.get_many(&ids).unwrap().len(), 1_200);

    files.remove(&ids[..700]).unwrap();
    assert_eq!(files.get_many(&ids).unwrap().len(), 500);
}

#[test]
fn duplicate_paths_in_a_scan_are_ignored() {
    let f = fixture();
    let scan = SqliteScanRepository::new(f.db.clone())
        .create(&new_scan(1))
        .unwrap();
    let files = SqliteFileEntryRepository::new(f.db.clone());
    let r = record("/tmp/a", FileKind::File, 1, 0);
    files.insert_batch(scan.id, &[r.clone(), r]).unwrap();
    let page = files
        .page(
            scan.id,
            &FileQuery::new(FileFilter::default(), FileSort::SizeDesc, 0, 10),
        )
        .unwrap();
    assert_eq!(page.total, 1);
}
