//! The operating systems the app supports. Rules that depend on the system (protected paths,
//! local risk rules, what to tell the AI) take a `Platform` instead of guessing.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    MacOs,
    Windows,
    Linux,
}

impl Platform {
    /// The system this binary was built for.
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            Self::MacOs => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }

    /// Name used in prompts sent to the AI.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::MacOs => "macOS",
            Self::Windows => "Windows",
            Self::Linux => "Linux (Debian-based)",
        }
    }

    /// Windows paths are case-insensitive and use `\`; rules compare them normalised.
    pub const fn case_insensitive_paths(self) -> bool {
        matches!(self, Self::Windows)
    }
}

/// Lower-case, `/`-separated form of a path for rule matching, with the home directory shown as
/// `~`. Works on any host, so Windows rules can be tested on macOS and Linux too.
pub fn normalized_path(path: &str, home: &str, platform: Platform) -> String {
    let fold = |s: &str| {
        let s = s.to_lowercase();
        if platform == Platform::Windows {
            s.replace('\\', "/")
        } else {
            s
        }
    };
    let path = fold(path);
    let home = fold(home).trim_end_matches('/').to_owned();
    if !home.is_empty() && path == home {
        return "~".into();
    }
    match path.strip_prefix(&format!("{home}/")) {
        Some(rest) if !home.is_empty() => format!("~/{rest}"),
        _ => path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_platform_matches_the_build_target() {
        let expected = if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(windows) {
            "windows"
        } else {
            "linux"
        };
        assert_eq!(Platform::current().id(), expected);
    }

    #[test]
    fn normalises_paths_for_matching() {
        assert_eq!(
            normalized_path(
                "/Users/Alice/Library/Caches",
                "/Users/Alice",
                Platform::MacOs
            ),
            "~/library/caches"
        );
        assert_eq!(
            normalized_path(
                r"C:\Users\Alice\AppData\Local\Temp",
                r"C:\Users\Alice",
                Platform::Windows
            ),
            "~/appdata/local/temp"
        );
        assert_eq!(
            normalized_path(r"C:\Windows\WinSxS", r"C:\Users\Alice", Platform::Windows),
            "c:/windows/winsxs"
        );
        assert_eq!(
            normalized_path("/home/bob", "/home/bob/", Platform::Linux),
            "~"
        );
        assert_eq!(
            normalized_path("/home/bobby/x", "/home/bob", Platform::Linux),
            "/home/bobby/x",
            "a sibling folder is not inside home"
        );
        assert_eq!(
            normalized_path(r"/tmp/a\b", "/home/bob", Platform::Linux),
            r"/tmp/a\b",
            "backslashes are only separators on Windows"
        );
    }
}
