//! Integration tests of the assessment cache and settings repositories on a real SQLite file.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;

use cleaner_core::domain::file_entry::FileFingerprint;
use cleaner_core::domain::risk::{
    AssessmentSource, Recommendation, RiskAssessment, RiskCategory, RiskLevel, SuggestedCommand,
};
use cleaner_core::domain::size::ByteSize;
use cleaner_core::ports::repository::{AssessmentRepository, SettingsRepository};
use cleaner_infra::db::{Database, SqliteAssessmentRepository, SqliteSettingsRepository};
use serde_json::json;

fn db() -> (tempfile::TempDir, Arc<Database>) {
    let dir = tempfile::tempdir().unwrap();
    let db = Arc::new(Database::open(&dir.path().join("c.sqlite3")).unwrap());
    (dir, db)
}

fn fp(size: u64, modified: Option<i64>) -> FileFingerprint {
    FileFingerprint {
        size_on_disk: ByteSize::new(size),
        modified_at: modified,
        inode: u64::MAX,
    }
}

fn assessment(
    version: &str,
    level: RiskLevel,
    fingerprint: FileFingerprint,
    at: i64,
) -> RiskAssessment {
    RiskAssessment {
        path: PathBuf::from("/Users/a/Library/Developer/CoreSimulator"),
        fingerprint,
        level,
        category: RiskCategory::VmImage,
        summary: "Simuladores".into(),
        explanation: "Use o Xcode".into(),
        recommendation: Recommendation::UseAppOrCommand,
        steps: vec!["Xcode › Settings › Platforms".into()],
        commands: vec![SuggestedCommand {
            command: "xcrun simctl delete unavailable".into(),
            description: "limpa".into(),
        }],
        source: AssessmentSource::Ai {
            provider: "anthropic".into(),
            model: "claude-opus-5-5".into(),
        },
        evaluator_version: version.into(),
        created_at: at,
    }
}

#[test]
fn round_trips_every_field_and_upserts_by_key() {
    let (_d, db) = db();
    let repo = SqliteAssessmentRepository::new(db);
    let a = assessment("v1", RiskLevel::Caution, fp(10, None), 1);
    repo.save(&a).unwrap();
    assert_eq!(repo.find(&a.key()).unwrap(), Some(a.clone()));

    let updated = RiskAssessment {
        level: RiskLevel::Safe,
        created_at: 2,
        ..a.clone()
    };
    repo.save(&updated).unwrap();
    assert_eq!(repo.find(&a.key()).unwrap().unwrap().level, RiskLevel::Safe);
}

#[test]
fn cache_misses_when_file_or_evaluator_changes() {
    let (_d, db) = db();
    let repo = SqliteAssessmentRepository::new(db);
    let a = assessment("v1", RiskLevel::Caution, fp(10, Some(5)), 1);
    repo.save(&a).unwrap();

    let mut other_version = a.key();
    other_version.evaluator_version = "v2".into();
    assert_eq!(repo.find(&other_version).unwrap(), None);

    let mut changed = a.key();
    changed.fingerprint = fp(11, Some(5));
    assert_eq!(repo.find(&changed).unwrap(), None);
}

#[test]
fn latest_matching_returns_newest_for_current_fingerprint_only() {
    let (_d, db) = db();
    let repo = SqliteAssessmentRepository::new(db);
    repo.save(&assessment(
        "heuristic-v1",
        RiskLevel::Caution,
        fp(10, None),
        1,
    ))
    .unwrap();
    repo.save(&assessment("anthropic:x", RiskLevel::Safe, fp(10, None), 2))
        .unwrap();
    repo.save(&assessment("old", RiskLevel::Dangerous, fp(99, None), 3))
        .unwrap();

    let path = PathBuf::from("/Users/a/Library/Developer/CoreSimulator");
    let found = repo
        .latest_matching(&[(path.clone(), fp(10, None))])
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].evaluator_version, "anthropic:x");
    assert!(
        repo.latest_matching(&[(path, fp(1, None))])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn settings_store_json_values() {
    let (_d, db) = db();
    let repo = SqliteSettingsRepository::new(db);
    assert_eq!(repo.get("ai").unwrap(), None);
    repo.set("ai", &json!({"provider": "anthropic"})).unwrap();
    repo.set("ai", &json!({"provider": "ollama", "model": "llama3.2"}))
        .unwrap();
    assert_eq!(
        repo.get("ai").unwrap(),
        Some(json!({"provider": "ollama", "model": "llama3.2"}))
    );
}
