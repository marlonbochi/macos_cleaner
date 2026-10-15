//! Paths the app must never act on (Constitution I, FR-016, SC-004).
//!
//! The policy is deliberately conservative: anything it cannot reason about
//! (relative paths, `..` components) is treated as protected.

use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct ProtectedPathPolicy {
    /// The directory and everything below it is protected.
    trees: Vec<PathBuf>,
    /// Exceptions carved out of `trees` (e.g. `/usr/local` inside `/usr`).
    allowed_trees: Vec<PathBuf>,
    /// Only the directory itself is protected (deleting `~/Documents` as a whole is never what
    /// the user wants), its contents are not.
    exact: Vec<PathBuf>,
}

impl ProtectedPathPolicy {
    pub fn builder() -> ProtectedPathPolicyBuilder {
        ProtectedPathPolicyBuilder::default()
    }

    /// The default macOS policy.
    ///
    /// `home` is the user's home directory and `app_data_dir` the directory holding this app's
    /// own catalog and settings.
    pub fn macos_default(home: &Path, app_data_dir: &Path) -> Self {
        let mut builder = Self::builder()
            .tree("/System")
            .tree("/usr")
            .allow_tree("/usr/local")
            .tree("/bin")
            .tree("/sbin")
            .tree("/etc")
            .tree("/var")
            .tree("/private/etc")
            .tree("/private/var/db")
            .tree("/private/var/vm")
            .tree("/private/var/root")
            .tree("/Library/Apple")
            .tree("/Applications/Safari.app")
            .tree(app_data_dir)
            .exact("/")
            .exact("/Applications")
            .exact("/Library")
            .exact("/Users")
            .exact("/Volumes")
            .exact("/private")
            .exact(home);
        for dir in [
            "Library",
            "Desktop",
            "Documents",
            "Downloads",
            "Movies",
            "Music",
            "Pictures",
            "Public",
            "Applications",
            "Library/Mobile Documents",
            "Library/Mobile Documents/com~apple~CloudDocs",
            "Library/CloudStorage",
        ] {
            builder = builder.exact(home.join(dir));
        }
        builder.build()
    }

    pub fn is_protected(&self, path: &Path) -> bool {
        if !path.is_absolute() || has_parent_components(path) {
            return true;
        }
        let path = strip_trailing_curdir(path);
        if self.exact.iter().any(|p| p == &path) {
            return true;
        }
        let in_tree = self.trees.iter().any(|t| path.starts_with(t));
        in_tree && !self.allowed_trees.iter().any(|t| path.starts_with(t))
    }
}

#[derive(Debug, Default)]
pub struct ProtectedPathPolicyBuilder {
    policy: ProtectedPathPolicy,
}

impl ProtectedPathPolicyBuilder {
    #[must_use]
    pub fn tree(mut self, path: impl Into<PathBuf>) -> Self {
        self.policy.trees.push(path.into());
        self
    }

    #[must_use]
    pub fn allow_tree(mut self, path: impl Into<PathBuf>) -> Self {
        self.policy.allowed_trees.push(path.into());
        self
    }

    #[must_use]
    pub fn exact(mut self, path: impl Into<PathBuf>) -> Self {
        self.policy.exact.push(path.into());
        self
    }

    pub fn build(self) -> ProtectedPathPolicy {
        self.policy
    }
}

/// Subtrees skipped by default when scanning `/` (firmlinks would double count the data volume).
pub fn default_scan_exclusions() -> Vec<PathBuf> {
    [
        "/System/Volumes",
        "/Volumes",
        "/dev",
        "/private/var/vm",
        "/cores",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

fn has_parent_components(path: &Path) -> bool {
    path.components().any(|c| matches!(c, Component::ParentDir))
}

/// `Path` equality ignores trailing slashes but not `/.`; normalise that away.
fn strip_trailing_curdir(path: &Path) -> PathBuf {
    path.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn policy() -> ProtectedPathPolicy {
        ProtectedPathPolicy::macos_default(
            Path::new("/Users/alice"),
            Path::new("/Users/alice/Library/Application Support/com.marlonbochi.diskcleaner"),
        )
    }

    #[test]
    fn system_trees_are_protected() {
        let p = policy();
        for path in [
            "/System",
            "/System/Library/CoreServices/Finder.app",
            "/usr/bin/ls",
            "/bin/zsh",
            "/private/var/db/receipts",
            "/Applications/Safari.app/Contents/MacOS/Safari",
        ] {
            assert!(
                p.is_protected(Path::new(path)),
                "{path} should be protected"
            );
        }
    }

    #[test]
    fn usr_local_is_an_exception() {
        assert!(!policy().is_protected(Path::new("/usr/local/Cellar/big-formula")));
    }

    #[test]
    fn top_level_user_folders_are_protected_but_not_their_contents() {
        let p = policy();
        assert!(p.is_protected(Path::new("/Users/alice")));
        assert!(p.is_protected(Path::new("/Users/alice/Documents")));
        assert!(p.is_protected(Path::new("/Users/alice/Documents/")));
        assert!(p.is_protected(Path::new("/Users/alice/Library")));
        assert!(!p.is_protected(Path::new("/Users/alice/Documents/old-backup.zip")));
        assert!(!p.is_protected(Path::new("/Users/alice/Library/Caches/com.example")));
        assert!(!p.is_protected(Path::new("/Applications/Xcode.app")));
    }

    #[test]
    fn app_own_data_is_protected() {
        assert!(policy().is_protected(Path::new(
            "/Users/alice/Library/Application Support/com.marlonbochi.diskcleaner/catalog.sqlite3"
        )));
    }

    #[test]
    fn prefix_match_is_component_based() {
        let p = policy();
        assert!(!p.is_protected(Path::new("/Systemic/file")));
        assert!(!p.is_protected(Path::new("/usrdata/file")));
    }

    #[test]
    fn ambiguous_paths_fail_safe() {
        let p = policy();
        assert!(p.is_protected(Path::new("relative/path")));
        assert!(p.is_protected(Path::new("/Users/alice/Downloads/../../../System")));
        assert!(p.is_protected(Path::new("/Users/alice/Downloads/..")));
    }

    #[test]
    fn default_exclusions_cover_firmlinked_volumes() {
        assert!(default_scan_exclusions().contains(&PathBuf::from("/System/Volumes")));
    }

    proptest! {
        #[test]
        fn anything_below_system_is_protected(segments in prop::collection::vec("[a-zA-Z0-9 ._-]{1,12}", 0..6)) {
            let mut path = PathBuf::from("/System");
            for s in &segments { path.push(s); }
            prop_assert!(policy().is_protected(&path));
        }

        #[test]
        fn any_path_with_parent_component_is_protected(prefix in "[a-z]{1,8}", suffix in "[a-z]{1,8}") {
            let path = PathBuf::from(format!("/Users/alice/{prefix}/../{suffix}"));
            prop_assert!(policy().is_protected(&path));
        }

        #[test]
        fn files_inside_downloads_are_not_protected(name in "[a-zA-Z0-9_-]{1,20}\\.(zip|dmg|mov)") {
            let path = PathBuf::from("/Users/alice/Downloads").join(name);
            prop_assert!(!policy().is_protected(&path));
        }
    }
}
