//! Audit records of destructive or space-freeing actions (Constitution I: everything is logged).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::risk::RiskLevel;
use super::size::ByteSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionKind {
    MoveToTrash,
    EvictToCloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionStatus {
    Succeeded,
    Skipped,
    Failed,
}

/// Why an action was not attempted. Each variant maps to a safety rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkipReason {
    Protected,
    ChangedSinceScan,
    Missing,
    RiskNotConfirmed,
    NotUploaded,
}

impl SkipReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Protected => "protected",
            Self::ChangedSinceScan => "changed-since-scan",
            Self::Missing => "missing",
            Self::RiskNotConfirmed => "risk-not-confirmed",
            Self::NotUploaded => "not-uploaded",
        }
    }
}

/// Append-only audit log entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub batch_id: String,
    pub action: ActionKind,
    pub path: PathBuf,
    pub size_on_disk: ByteSize,
    pub risk_level: RiskLevel,
    pub status: ActionStatus,
    pub reason: Option<String>,
    pub executed_at: i64,
}
