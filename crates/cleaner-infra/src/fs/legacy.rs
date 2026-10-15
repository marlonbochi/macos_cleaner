use std::io;
use std::path::Path;

/// Moves a data folder left by a previous identity of the app (before it was renamed) to the
/// current location, so the catalogue and settings survive the rename.
///
/// Never overwrites: returns `Ok(false)` when there is nothing to move or the current folder
/// already exists.
pub fn adopt_legacy_dir(legacy: &Path, current: &Path) -> io::Result<bool> {
    if current.exists() || !legacy.is_dir() {
        return Ok(false);
    }
    if let Some(parent) = current.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(legacy, current)?;
    Ok(true)
}
