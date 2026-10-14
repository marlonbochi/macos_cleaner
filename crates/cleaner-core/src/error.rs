//! Error types of the core crate. Adapters convert their own errors into these.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("invalid state transition from {from} to {to}")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },
}

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("storage error: {0}")]
    Storage(String),
}

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("scan root is not accessible: {0}")]
    RootInaccessible(String),
    #[error("scan I/O error: {0}")]
    Io(String),
}

/// Error returned by application services (use cases).
#[derive(Debug, Error)]
pub enum CoreError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Scan(#[from] ScanError),
    #[error("a scan is already running")]
    ScanInProgress,
    #[error("not found: {0}")]
    NotFound(String),
}
