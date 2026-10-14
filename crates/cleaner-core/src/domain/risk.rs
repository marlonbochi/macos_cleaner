//! Risk assessment value objects (Constitution IV: AI advises, never decides).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::file_entry::FileFingerprint;

/// How risky it is to delete an item. Declaration order is severity order,
/// so `Ord` can be used to pick the most severe of several opinions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    Safe,
    Unknown,
    Caution,
    Dangerous,
}

impl RiskLevel {
    /// Items at these levels need an individual confirmation (FR-019)
    /// and must never be pre-selected (FR-023).
    pub const fn requires_confirmation(self) -> bool {
        matches!(self, Self::Caution | Self::Dangerous)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Unknown => "Unknown",
            Self::Caution => "Caution",
            Self::Dangerous => "Dangerous",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Safe" => Some(Self::Safe),
            "Unknown" => Some(Self::Unknown),
            "Caution" => Some(Self::Caution),
            "Dangerous" => Some(Self::Dangerous),
            _ => None,
        }
    }
}

/// What kind of thing an item is, as understood by a classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskCategory {
    Cache,
    BuildArtifact,
    Installer,
    VmImage,
    Media,
    Archive,
    UserDocument,
    AppData,
    AppBundle,
    Game,
    System,
    Backup,
    Log,
    Other,
}

impl RiskCategory {
    pub const ALL: [Self; 14] = [
        Self::Cache,
        Self::BuildArtifact,
        Self::Installer,
        Self::VmImage,
        Self::Media,
        Self::Archive,
        Self::UserDocument,
        Self::AppData,
        Self::AppBundle,
        Self::Game,
        Self::System,
        Self::Backup,
        Self::Log,
        Self::Other,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cache => "cache",
            Self::BuildArtifact => "build-artifact",
            Self::Installer => "installer",
            Self::VmImage => "vm-image",
            Self::Media => "media",
            Self::Archive => "archive",
            Self::UserDocument => "user-document",
            Self::AppData => "app-data",
            Self::AppBundle => "app-bundle",
            Self::Game => "game",
            Self::System => "system",
            Self::Backup => "backup",
            Self::Log => "log",
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == value)
    }
}

/// Who produced an assessment. Shown to the user next to every risk badge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum AssessmentSource {
    ProtectedRule,
    Heuristic,
    Ai { provider: String, model: String },
}

/// Cache key: an assessment is valid only for the exact file state and evaluator version.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssessmentKey {
    pub path: PathBuf,
    pub fingerprint: FileFingerprint,
    pub evaluator_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskAssessment {
    pub path: PathBuf,
    pub fingerprint: FileFingerprint,
    pub level: RiskLevel,
    pub category: RiskCategory,
    pub explanation: String,
    pub source: AssessmentSource,
    pub evaluator_version: String,
    pub created_at: i64,
}

impl RiskAssessment {
    pub fn key(&self) -> AssessmentKey {
        AssessmentKey {
            path: self.path.clone(),
            fingerprint: self.fingerprint,
            evaluator_version: self.evaluator_version.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_ordered_by_severity() {
        assert!(RiskLevel::Safe < RiskLevel::Unknown);
        assert!(RiskLevel::Unknown < RiskLevel::Caution);
        assert!(RiskLevel::Caution < RiskLevel::Dangerous);
        assert_eq!(
            [RiskLevel::Caution, RiskLevel::Dangerous, RiskLevel::Safe]
                .iter()
                .max(),
            Some(&RiskLevel::Dangerous)
        );
    }

    #[test]
    fn only_caution_and_dangerous_require_confirmation() {
        assert!(!RiskLevel::Safe.requires_confirmation());
        assert!(!RiskLevel::Unknown.requires_confirmation());
        assert!(RiskLevel::Caution.requires_confirmation());
        assert!(RiskLevel::Dangerous.requires_confirmation());
    }

    #[test]
    fn level_and_category_round_trip_through_strings() {
        for level in [
            RiskLevel::Safe,
            RiskLevel::Unknown,
            RiskLevel::Caution,
            RiskLevel::Dangerous,
        ] {
            assert_eq!(RiskLevel::parse(level.as_str()), Some(level));
        }
        for category in RiskCategory::ALL {
            assert_eq!(RiskCategory::parse(category.as_str()), Some(category));
        }
        assert_eq!(RiskLevel::parse("nope"), None);
        assert_eq!(RiskCategory::parse("nope"), None);
    }

    #[test]
    fn category_serializes_as_kebab_case() {
        let json = serde_json::to_string(&RiskCategory::BuildArtifact).unwrap();
        assert_eq!(json, "\"build-artifact\"");
    }
}
