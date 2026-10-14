//! macOS "packages": directories that Finder shows as a single file.

use std::path::Path;

/// Extensions of directory bundles that should be treated as one unit.
const PACKAGE_EXTENSIONS: &[&str] = &[
    "app",
    "appex",
    "bundle",
    "framework",
    "kext",
    "plugin",
    "photoslibrary",
    "musiclibrary",
    "tvlibrary",
    "imovielibrary",
    "fcpbundle",
    "logicx",
    "band",
    "xcarchive",
    "sparsebundle",
    "pkg",
    "mpkg",
    "rtfd",
    "playground",
    "docarchive",
];

/// Whether a directory at `path` is a package. Caller must already know it is a directory.
pub fn is_package_dir(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| {
            PACKAGE_EXTENSIONS
                .iter()
                .any(|p| p.eq_ignore_ascii_case(ext))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_common_bundles_case_insensitively() {
        assert!(is_package_dir(Path::new("/Applications/Xcode.app")));
        assert!(is_package_dir(Path::new(
            "/Users/u/Pictures/Photos Library.photoslibrary"
        )));
        assert!(is_package_dir(Path::new("/x/Backup.SparseBundle")));
    }

    #[test]
    fn plain_directories_are_not_packages() {
        assert!(!is_package_dir(Path::new("/Users/u/Documents")));
        assert!(!is_package_dir(Path::new("/Users/u/project.d")));
    }
}
