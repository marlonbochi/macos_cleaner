//! Error type returned by every IPC command (contract: `AppError` in tauri-commands.md).

use cleaner_core::error::{CoreError, RepoError};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum ErrorCode {
    NotFound,
    InvalidInput,
    PermissionDenied,
    ScanInProgress,
    ProtectedPath,
    AiUnavailable,
    CloudUnsupported,
    Storage,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, thiserror::Error)]
#[error("{code:?}: {message}")]
#[ts(export)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }
}

impl From<CoreError> for AppError {
    fn from(err: CoreError) -> Self {
        let code = match &err {
            CoreError::Domain(_) => ErrorCode::InvalidInput,
            CoreError::Repo(RepoError::NotFound(_)) | CoreError::NotFound(_) => ErrorCode::NotFound,
            CoreError::Repo(RepoError::Storage(_)) => ErrorCode::Storage,
            CoreError::Scan(_) => ErrorCode::Internal,
            CoreError::ScanInProgress => ErrorCode::ScanInProgress,
            CoreError::Ai(_) => ErrorCode::AiUnavailable,
            CoreError::Secret(_) => ErrorCode::Internal,
        };
        tracing::warn!(error = %err, "command failed");
        Self::new(code, err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        let code = match err.kind() {
            std::io::ErrorKind::PermissionDenied => ErrorCode::PermissionDenied,
            std::io::ErrorKind::NotFound => ErrorCode::NotFound,
            std::io::ErrorKind::InvalidInput => ErrorCode::InvalidInput,
            _ => ErrorCode::Internal,
        };
        Self::new(code, err.to_string())
    }
}

pub type CommandResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_errors_map_to_stable_codes() {
        assert_eq!(
            AppError::from(CoreError::ScanInProgress).code,
            ErrorCode::ScanInProgress
        );
        assert_eq!(
            AppError::from(CoreError::NotFound("x".into())).code,
            ErrorCode::NotFound
        );
        assert_eq!(
            AppError::from(CoreError::Repo(RepoError::Storage("disk".into()))).code,
            ErrorCode::Storage
        );
    }

    #[test]
    fn serializes_with_screaming_snake_code() {
        let json = serde_json::to_value(AppError::new(ErrorCode::ScanInProgress, "busy")).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "SCAN_IN_PROGRESS", "message": "busy" })
        );
    }
}
