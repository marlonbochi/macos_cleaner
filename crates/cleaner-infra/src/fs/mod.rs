//! Filesystem adapters.

mod inspector;
mod jwalk_scanner;
mod metadata;

pub use inspector::StdFileInspector;
pub use jwalk_scanner::JwalkScanner;
