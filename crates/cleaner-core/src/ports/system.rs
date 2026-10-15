//! System ports used by actions that change the disk. Implemented in `cleaner-infra`.

use std::io;
use std::path::Path;

use crate::domain::size::ByteSize;

/// Moves items to the user's Trash, so every removal can be undone (Constitution I).
pub trait Trash: Send + Sync {
    fn move_to_trash(&self, path: &Path) -> io::Result<()>;
}

/// Measures how much disk space a path uses.
pub trait DiskUsage: Send + Sync {
    /// Allocated size of a file or of everything below a folder (symlinks are not followed);
    /// `Ok(None)` when the path does not exist.
    fn size_on_disk(&self, path: &Path) -> io::Result<Option<ByteSize>>;
}

/// Starts another program without waiting for it (e.g. the Windows uninstaller).
pub trait Launcher: Send + Sync {
    fn spawn(&self, program: &Path, args: &[String]) -> io::Result<()>;
}
