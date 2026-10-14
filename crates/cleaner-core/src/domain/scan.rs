//! The `Scan` aggregate and its state machine.
//!
//! ```text
//! Running ──complete──▶ Completed
//!    │──cancel────────▶ Cancelled
//!    └──fail──────────▶ Failed
//! ```

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::size::ByteSize;
use crate::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScanId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScanStatus {
    Running,
    Completed,
    Cancelled,
    Failed,
}

impl ScanStatus {
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Running)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Completed => "Completed",
            Self::Cancelled => "Cancelled",
            Self::Failed => "Failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Running" => Some(Self::Running),
            "Completed" => Some(Self::Completed),
            "Cancelled" => Some(Self::Cancelled),
            "Failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Running totals of a scan. Every field only grows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanStats {
    pub files_seen: u64,
    pub bytes_seen: ByteSize,
    pub inaccessible_count: u64,
}

/// Parameters to start a new scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewScan {
    pub roots: Vec<PathBuf>,
    pub min_file_size: ByteSize,
    pub started_at: i64,
}

impl NewScan {
    pub fn new(
        roots: Vec<PathBuf>,
        min_file_size: ByteSize,
        started_at: i64,
    ) -> Result<Self, DomainError> {
        if roots.is_empty() {
            return Err(DomainError::InvalidInput(
                "at least one scan root is required".into(),
            ));
        }
        if let Some(relative) = roots.iter().find(|r| !r.is_absolute()) {
            return Err(DomainError::InvalidInput(format!(
                "scan root must be absolute: {}",
                relative.display()
            )));
        }
        Ok(Self {
            roots,
            min_file_size,
            started_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scan {
    pub id: ScanId,
    pub roots: Vec<PathBuf>,
    pub min_file_size: ByteSize,
    pub status: ScanStatus,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub stats: ScanStats,
    pub error: Option<String>,
}

impl Scan {
    pub fn start(id: ScanId, new: NewScan) -> Self {
        Self {
            id,
            roots: new.roots,
            min_file_size: new.min_file_size,
            status: ScanStatus::Running,
            started_at: new.started_at,
            finished_at: None,
            stats: ScanStats::default(),
            error: None,
        }
    }

    pub fn complete(&mut self, at: i64, stats: ScanStats) -> Result<(), DomainError> {
        self.finish(ScanStatus::Completed, at, stats, None)
    }

    pub fn cancel(&mut self, at: i64, stats: ScanStats) -> Result<(), DomainError> {
        self.finish(ScanStatus::Cancelled, at, stats, None)
    }

    pub fn fail(&mut self, at: i64, stats: ScanStats, error: String) -> Result<(), DomainError> {
        self.finish(ScanStatus::Failed, at, stats, Some(error))
    }

    /// Cancelled scans keep partial results, which the UI flags as incomplete.
    pub const fn is_incomplete(&self) -> bool {
        !matches!(self.status, ScanStatus::Completed)
    }

    fn finish(
        &mut self,
        status: ScanStatus,
        at: i64,
        stats: ScanStats,
        error: Option<String>,
    ) -> Result<(), DomainError> {
        if self.status.is_terminal() {
            return Err(DomainError::InvalidTransition {
                from: self.status.as_str(),
                to: status.as_str(),
            });
        }
        self.status = status;
        self.finished_at = Some(at);
        self.stats = stats;
        self.error = error;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running() -> Scan {
        let new = NewScan::new(vec![PathBuf::from("/tmp")], ByteSize::from_mb(100), 1).unwrap();
        Scan::start(ScanId(1), new)
    }

    #[test]
    fn new_scan_requires_at_least_one_absolute_root() {
        assert!(NewScan::new(vec![], ByteSize::ZERO, 0).is_err());
        assert!(NewScan::new(vec![PathBuf::from("relative")], ByteSize::ZERO, 0).is_err());
        assert!(NewScan::new(vec![PathBuf::from("/abs")], ByteSize::ZERO, 0).is_ok());
    }

    #[test]
    fn running_scan_can_complete() {
        let mut scan = running();
        let stats = ScanStats {
            files_seen: 3,
            ..ScanStats::default()
        };
        scan.complete(5, stats).unwrap();
        assert_eq!(scan.status, ScanStatus::Completed);
        assert_eq!(scan.finished_at, Some(5));
        assert_eq!(scan.stats.files_seen, 3);
        assert!(!scan.is_incomplete());
    }

    #[test]
    fn cancelled_scan_is_incomplete() {
        let mut scan = running();
        scan.cancel(5, ScanStats::default()).unwrap();
        assert_eq!(scan.status, ScanStatus::Cancelled);
        assert!(scan.is_incomplete());
    }

    #[test]
    fn failed_scan_keeps_error_message() {
        let mut scan = running();
        scan.fail(5, ScanStats::default(), "disk full".into())
            .unwrap();
        assert_eq!(scan.error.as_deref(), Some("disk full"));
    }

    #[test]
    fn terminal_states_cannot_transition_again() {
        let mut scan = running();
        scan.complete(5, ScanStats::default()).unwrap();
        let err = scan.cancel(6, ScanStats::default()).unwrap_err();
        assert!(matches!(
            err,
            DomainError::InvalidTransition {
                from: "Completed",
                to: "Cancelled"
            }
        ));
    }
}
