//! Paths the app must never act on (Constitution I, FR-016, SC-004).
//!
//! The policy is deliberately conservative: anything it cannot reason about
//! (relative paths, `..` components) is treated as protected.

use std::path::{Component, Path, PathBuf};

use crate::domain::platform::Platform;

#[derive(Debug, Clone, Default)]
pub struct ProtectedPathPolicy {
    /// The directory and everything below it is protected.
    trees: Vec<PathBuf>,
    /// Exceptions carved out of `trees` (e.g. `/usr/local` inside `/usr`).
    allowed_trees: Vec<PathBuf>,
    /// Only the directory itself is protected (deleting `~/Documents` as a whole is never what
    /// the user wants), its contents are not.
    exact: Vec<PathBuf>,
    /// Compare paths ignoring case (Windows).
    case_insensitive: bool,
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

    /// The default Windows policy. `system_drive` is the drive Windows runs from (`C:\`).
    pub fn windows_default(home: &Path, app_data_dir: &Path, system_drive: &Path) -> Self {
        let drive = |p: &str| system_drive.join(p);
        let mut builder = Self::builder()
            .case_insensitive()
            .tree(drive("Windows"))
            .allow_tree(drive(r"Windows\Temp"))
            .allow_tree(drive(r"Windows\SoftwareDistribution\Download"))
            .tree(drive("System Volume Information"))
            .tree(drive("Recovery"))
            .tree(drive("Boot"))
            .tree(drive("$WinREAgent"))
            .tree(app_data_dir)
            .exact(system_drive)
            .exact(drive("Users"))
            .exact(drive("Program Files"))
            .exact(drive("Program Files (x86)"))
            .exact(drive("ProgramData"))
            .exact(drive("pagefile.sys"))
            .exact(drive("swapfile.sys"))
            .exact(drive("hiberfil.sys"))
            .exact(drive("$Recycle.Bin"))
            .exact(home);
        for dir in [
            "Desktop",
            "Documents",
            "Downloads",
            "Music",
            "Pictures",
            "Videos",
            "OneDrive",
            "AppData",
            r"AppData\Local",
            r"AppData\LocalLow",
            r"AppData\Roaming",
        ] {
            builder = builder.exact(home.join(dir));
        }
        builder.build()
    }

    /// The default Linux policy (Debian and derivatives).
    pub fn linux_default(home: &Path, app_data_dir: &Path) -> Self {
        let mut builder = Self::builder()
            .tree("/bin")
            .tree("/sbin")
            .tree("/lib")
            .tree("/lib32")
            .tree("/lib64")
            .tree("/usr")
            .allow_tree("/usr/local")
            .tree("/etc")
            .tree("/boot")
            .tree("/proc")
            .tree("/sys")
            .tree("/dev")
            .tree("/run")
            .tree("/var/lib/dpkg")
            .tree("/var/lib/apt")
            .tree("/var/lib/systemd")
            .allow_tree("/var/lib/systemd/coredump")
            .tree(app_data_dir)
            .exact("/")
            .exact("/home")
            .exact("/root")
            .exact("/opt")
            .exact("/var")
            .exact("/var/lib")
            .exact("/media")
            .exact("/mnt")
            .exact("/snap")
            .exact("/swapfile")
            .exact(home);
        for dir in [
            "Desktop",
            "Documents",
            "Downloads",
            "Music",
            "Pictures",
            "Videos",
            "Public",
            ".config",
            ".local",
            ".local/share",
            ".ssh",
            ".gnupg",
        ] {
            builder = builder.exact(home.join(dir));
        }
        builder.build()
    }

    /// The default policy for `platform`. `system_drive` is only used on Windows.
    pub fn for_platform(
        platform: Platform,
        home: &Path,
        app_data_dir: &Path,
        system_drive: &Path,
    ) -> Self {
        match platform {
            Platform::MacOs => Self::macos_default(home, app_data_dir),
            Platform::Windows => Self::windows_default(home, app_data_dir, system_drive),
            Platform::Linux => Self::linux_default(home, app_data_dir),
        }
    }

    pub fn is_protected(&self, path: &Path) -> bool {
        if !path.is_absolute() || has_parent_components(path) {
            return true;
        }
        let path = self.fold(&strip_trailing_curdir(path));
        if self.exact.iter().any(|p| p == &path) {
            return true;
        }
        let in_tree = self.trees.iter().any(|t| path.starts_with(t));
        in_tree && !self.allowed_trees.iter().any(|t| path.starts_with(t))
    }

    fn fold(&self, path: &Path) -> PathBuf {
        if self.case_insensitive {
            PathBuf::from(path.to_string_lossy().to_lowercase())
        } else {
            path.to_path_buf()
        }
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

    /// Paths are compared ignoring case (Windows file systems).
    #[must_use]
    pub fn case_insensitive(mut self) -> Self {
        self.policy.case_insensitive = true;
        self
    }

    pub fn build(self) -> ProtectedPathPolicy {
        let mut policy = self.policy;
        if policy.case_insensitive {
            for list in [
                &mut policy.trees,
                &mut policy.allowed_trees,
                &mut policy.exact,
            ] {
                for path in list.iter_mut() {
                    *path = PathBuf::from(path.to_string_lossy().to_lowercase());
                }
            }
        }
        policy
    }
}

/// Subtrees skipped by default when scanning: on macOS, firmlinks would double count the data
/// volume; on Linux, virtual file systems hold no files that take disk space.
pub fn default_scan_exclusions(platform: Platform) -> Vec<PathBuf> {
    let paths: &[&str] = match platform {
        Platform::MacOs => &[
            "/System/Volumes",
            "/Volumes",
            "/dev",
            "/private/var/vm",
            "/cores",
        ],
        Platform::Linux => &["/proc", "/sys", "/dev", "/run", "/snap", "/media", "/mnt"],
        Platform::Windows => &[],
    };
    paths.iter().map(PathBuf::from).collect()
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

    #[cfg(unix)]
    #[test]
    fn usr_local_is_an_exception() {
        assert!(!policy().is_protected(Path::new("/usr/local/Cellar/big-formula")));
    }

    #[cfg(unix)]
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

    #[cfg(unix)]
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
        assert!(
            default_scan_exclusions(Platform::MacOs).contains(&PathBuf::from("/System/Volumes"))
        );
        assert!(default_scan_exclusions(Platform::Linux).contains(&PathBuf::from("/proc")));
    }

    #[cfg(unix)]
    #[test]
    fn linux_policy_protects_the_system_but_not_user_files() {
        let p = ProtectedPathPolicy::linux_default(
            Path::new("/home/bob"),
            Path::new("/home/bob/.local/share/com.example.app"),
        );
        for path in [
            "/",
            "/usr/lib/x86_64-linux-gnu",
            "/etc/fstab",
            "/var/lib/dpkg/status",
            "/home",
            "/home/bob",
            "/home/bob/Documents",
            "/home/bob/.local/share",
            "/home/bob/.local/share/com.example.app/catalog.sqlite3",
        ] {
            assert!(p.is_protected(Path::new(path)), "{path}");
        }
        for path in [
            "/usr/local/games",
            "/var/cache/apt/archives",
            "/var/log/journal",
            "/var/lib/systemd/coredump/core.1",
            "/home/bob/.cache",
            "/home/bob/Downloads/debian.iso",
            "/opt/google",
        ] {
            assert!(!p.is_protected(Path::new(path)), "{path}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_policy_ignores_case_and_protects_the_system() {
        let p = ProtectedPathPolicy::windows_default(
            Path::new(r"C:\Users\Bob"),
            Path::new(r"C:\Users\Bob\AppData\Roaming\com.example.app"),
            Path::new(r"C:\"),
        );
        for path in [
            r"C:\",
            r"C:\Windows\System32",
            r"c:\windows\winsxs",
            r"C:\Program Files",
            r"C:\pagefile.sys",
            r"C:\Users\Bob",
            r"C:\Users\bob\documents",
            r"C:\Users\Bob\AppData\Local",
            r"C:\Users\Bob\AppData\Roaming\com.example.app\catalog.sqlite3",
        ] {
            assert!(p.is_protected(Path::new(path)), "{path}");
        }
        for path in [
            r"C:\Windows\Temp\x.log",
            r"C:\Windows\SoftwareDistribution\Download",
            r"C:\Program Files\Some Game",
            r"C:\Users\Bob\AppData\Local\Temp",
            r"C:\Users\Bob\Downloads\setup.exe",
            r"D:\Games",
        ] {
            assert!(!p.is_protected(Path::new(path)), "{path}");
        }
    }

    /// macOS policy properties (POSIX paths).
    #[cfg(unix)]
    mod posix_properties {
        use super::*;

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
}
