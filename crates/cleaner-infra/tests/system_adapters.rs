//! Integration tests for the disk-usage and Trash adapters on real temporary folders.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use cleaner_core::ports::system::{DiskUsage, Trash};
use cleaner_infra::fs::StdDiskUsage;
use cleaner_infra::macos::MacTrash;

#[test]
fn measures_folders_recursively_without_following_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("com.example.cleaner");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.bin"), vec![1u8; 64 * 1024]).unwrap();
    fs::write(root.join("nested/b.bin"), vec![1u8; 128 * 1024]).unwrap();
    // A link to a big file outside must not be counted as that file.
    let outside = dir.path().join("big.bin");
    fs::write(&outside, vec![1u8; 1024 * 1024]).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();

    let size = StdDiskUsage.size_on_disk(&root).unwrap().unwrap().bytes();

    assert!(size >= 192 * 1024, "both files are counted: {size}");
    assert!(size < 1024 * 1024, "the symlink target is not: {size}");
    let file = StdDiskUsage
        .size_on_disk(&root.join("a.bin"))
        .unwrap()
        .unwrap();
    assert!(file.bytes() >= 64 * 1024);
}

#[test]
fn missing_paths_have_no_size() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        StdDiskUsage
            .size_on_disk(&dir.path().join("nope"))
            .unwrap()
            .is_none()
    );
}

#[test]
#[ignore = "moves a temporary file to the real Trash"]
fn moves_items_to_the_trash() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("macos-cleaner-trash-test.txt");
    fs::write(&file, "bye").unwrap();
    MacTrash.move_to_trash(&file).unwrap();
    assert!(!file.exists());
    assert!(MacTrash.move_to_trash(&file).is_err(), "missing items fail");
}

#[test]
fn reads_the_bundle_identifier_of_an_app() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("Cleaner.app");
    fs::create_dir_all(app.join("Contents")).unwrap();
    fs::write(
        app.join("Contents/Info.plist"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>com.example.cleaner</string>
</dict></plist>"#,
    )
    .unwrap();

    assert_eq!(
        cleaner_infra::macos::bundle_identifier(&app).as_deref(),
        Some("com.example.cleaner")
    );
    assert_eq!(cleaner_infra::macos::bundle_identifier(dir.path()), None);
}

#[test]
fn development_builds_do_not_run_from_a_bundle() {
    // Test binaries live in target/, never inside a .app with this id.
    assert_eq!(
        cleaner_infra::macos::running_app_bundle("com.marlonbochi.macoscleaner"),
        None
    );
}
