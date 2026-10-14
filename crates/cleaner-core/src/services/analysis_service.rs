//! Use case: analyse one catalogued item and explain whether / how to delete it (US2).
//!
//! Resolution chain (Chain of Responsibility):
//! 1. protected path → always `Dangerous`, never sent anywhere;
//! 2. cached assessment for the same file state and evaluator → reused (FR-015);
//! 3. configured AI provider → structured answer, validated against the schema;
//! 4. local rules → always available; also used when the AI fails (FR-013).
//!
//! The rules also act as a safety floor: if they know an item is essential (`Dangerous`),
//! an AI answer can never make it look safer than `Caution`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::analysis::heuristics::{HEURISTIC_VERSION, HeuristicAnalyzer};
use crate::analysis::prompt::{
    PROMPT_VERSION, SCHEMA_NAME, parse_response, response_schema, system_prompt, user_message,
};
use crate::analysis::verdict::{Verdict, protected_verdict};
use crate::domain::file_entry::{FileEntry, FileId, FileKind};
use crate::domain::risk::{AssessmentKey, AssessmentSource, RiskAssessment, RiskLevel};
use crate::error::CoreError;
use crate::ports::Clock;
use crate::ports::ai::{AiProvider, StructuredRequest};
use crate::ports::repository::{
    AssessmentRepository, FileEntryRepository, FileFilter, FileQuery, FileSort,
};
use crate::services::ai_settings_service::AiSettingsService;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisOutcome {
    pub file_id: FileId,
    pub assessment: RiskAssessment,
    /// Set when the AI was configured but failed and local rules answered instead.
    pub ai_error: Option<String>,
}

#[derive(Clone)]
pub struct AnalysisService {
    files: Arc<dyn FileEntryRepository>,
    assessments: Arc<dyn AssessmentRepository>,
    settings: AiSettingsService,
    heuristics: HeuristicAnalyzer,
    clock: Arc<dyn Clock>,
    home: PathBuf,
}

impl std::fmt::Debug for AnalysisService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalysisService").finish_non_exhaustive()
    }
}

impl AnalysisService {
    pub fn new(
        files: Arc<dyn FileEntryRepository>,
        assessments: Arc<dyn AssessmentRepository>,
        settings: AiSettingsService,
        clock: Arc<dyn Clock>,
        home: &Path,
    ) -> Self {
        Self {
            files,
            assessments,
            settings,
            heuristics: HeuristicAnalyzer::new(home),
            clock,
            home: home.to_path_buf(),
        }
    }

    /// Analyses an item. With `force`, ignores the cache (the "analyse again" button).
    pub fn analyze(&self, id: FileId, force: bool) -> Result<AnalysisOutcome, CoreError> {
        let entry = self
            .files
            .get_many(&[id])?
            .pop()
            .ok_or_else(|| CoreError::NotFound(format!("file {}", id.0)))?;
        let record = &entry.record;
        let now = self.clock.now();

        if record.is_protected {
            let assessment = protected_verdict(&record.path).into_assessment(
                record,
                AssessmentSource::ProtectedRule,
                "protected".into(),
                now,
            );
            return Ok(AnalysisOutcome {
                file_id: id,
                assessment,
                ai_error: None,
            });
        }

        // A misconfigured provider (e.g. missing key) is reported, not fatal.
        let (provider, setup_error) = match self.settings.provider() {
            Ok(p) => (p, None),
            Err(e) => (None, Some(e.to_string())),
        };
        let version = provider.as_ref().map_or_else(
            || HEURISTIC_VERSION.to_owned(),
            |p| {
                let (name, model) = p.descriptor();
                format!("{name}:{model}:prompt-v{PROMPT_VERSION}")
            },
        );

        if !force {
            let key = AssessmentKey {
                path: record.path.clone(),
                fingerprint: record.fingerprint(),
                evaluator_version: version.clone(),
            };
            if let Some(assessment) = self.assessments.find(&key)? {
                return Ok(AnalysisOutcome {
                    file_id: id,
                    assessment,
                    ai_error: setup_error,
                });
            }
        }

        let rules = self.heuristics.analyze(record);
        let (assessment, ai_error) = match provider {
            Some(provider) => match self.ask_ai(provider.as_ref(), &entry) {
                Ok(verdict) => {
                    let (name, model) = provider.descriptor();
                    let verdict = apply_safety_floor(verdict, &rules);
                    let source = AssessmentSource::Ai {
                        provider: name,
                        model,
                    };
                    (verdict.into_assessment(record, source, version, now), None)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "AI analysis failed, using local rules");
                    let fallback = rules.into_assessment(
                        record,
                        AssessmentSource::Heuristic,
                        HEURISTIC_VERSION.into(),
                        now,
                    );
                    (fallback, Some(e.to_string()))
                }
            },
            None => {
                let assessment = rules.into_assessment(
                    record,
                    AssessmentSource::Heuristic,
                    HEURISTIC_VERSION.into(),
                    now,
                );
                (assessment, setup_error)
            }
        };
        self.assessments.save(&assessment)?;
        Ok(AnalysisOutcome {
            file_id: id,
            assessment,
            ai_error,
        })
    }

    /// Latest stored assessment for each item that still matches its current state (badges).
    pub fn latest_for(&self, ids: &[FileId]) -> Result<Vec<(FileId, RiskAssessment)>, CoreError> {
        let entries = self.files.get_many(ids)?;
        let keys: Vec<_> = entries
            .iter()
            .map(|e| (e.record.path.clone(), e.record.fingerprint()))
            .collect();
        let found = self.assessments.latest_matching(&keys)?;
        Ok(entries
            .iter()
            .filter_map(|e| {
                found
                    .iter()
                    .find(|a| a.path == e.record.path && a.fingerprint == e.record.fingerprint())
                    .map(|a| (e.id, a.clone()))
            })
            .collect())
    }

    fn ask_ai(&self, provider: &dyn AiProvider, entry: &FileEntry) -> Result<Verdict, CoreError> {
        let children = if entry.record.kind == FileKind::File {
            Vec::new()
        } else {
            let filter = FileFilter {
                parent: Some(entry.record.path.clone()),
                ..FileFilter::default()
            };
            let query = FileQuery::new(
                filter,
                FileSort::SizeDesc,
                0,
                u32::try_from(crate::analysis::prompt::MAX_CHILDREN).unwrap_or(15),
            );
            self.files
                .page(entry.scan_id, &query)?
                .items
                .into_iter()
                .map(|c| c.record)
                .collect()
        };
        let schema = response_schema();
        let user = user_message(&entry.record, &children, &self.home);
        let reply = provider.complete_json(&StructuredRequest {
            system: system_prompt(),
            user: &user,
            schema: &schema,
            schema_name: SCHEMA_NAME,
        })?;
        Ok(parse_response(&reply)?)
    }
}

/// Items the local rules know to be essential never drop below `Caution` (SC-006).
fn apply_safety_floor(mut ai: Verdict, rules: &Verdict) -> Verdict {
    if rules.level == RiskLevel::Dangerous && ai.level < RiskLevel::Caution {
        ai.level = rules.level;
        ai.explanation = format!(
            "{} (Nível elevado pelas regras de segurança: {})",
            ai.explanation, rules.summary
        );
    }
    ai
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ai::{AiProviderKind, AiSettings};
    use crate::domain::file_entry::FileRecord;
    use crate::domain::risk::{Recommendation, RiskCategory};
    use crate::domain::scan::{NewScan, ScanId};
    use crate::domain::size::ByteSize;
    use crate::ports::ai::AiError;
    use crate::ports::repository::ScanRepository;
    use crate::services::ai_settings_service::ApiKeyChange;
    use crate::testing::{
        FakeAiFactory, FakeClock, InMemoryAssessmentRepository, InMemoryFileEntryRepository,
        InMemoryScanRepository, InMemorySecretStore, InMemorySettingsRepository,
    };
    use serde_json::json;

    const HOME: &str = "/Users/alice";

    struct Harness {
        service: AnalysisService,
        settings: AiSettingsService,
        factory: Arc<FakeAiFactory>,
        files: Arc<InMemoryFileEntryRepository>,
        scan: ScanId,
    }

    fn record(path: &str, kind: FileKind, protected: bool) -> FileRecord {
        FileRecord {
            path: PathBuf::from(path),
            kind,
            size_on_disk: ByteSize::new(5_000_000_000),
            apparent_size: ByteSize::new(5_000_000_000),
            file_count: 10,
            created_at: None,
            modified_at: Some(1),
            accessed_at: None,
            inode: 9,
            device: 1,
            is_hardlink_dup: false,
            is_protected: protected,
        }
    }

    fn harness(factory: FakeAiFactory) -> Harness {
        let scans = InMemoryScanRepository::default();
        let scan = scans
            .create(&NewScan::new(vec![HOME.into()], ByteSize::ZERO, 0).unwrap())
            .unwrap();
        let files = Arc::new(InMemoryFileEntryRepository::default());
        files
            .insert_batch(
                scan.id,
                &[
                    record(
                        "/Users/alice/Library/Developer/CoreSimulator",
                        FileKind::Directory,
                        false,
                    ),
                    record(
                        "/Users/alice/Library/Developer/CoreSimulator/Devices",
                        FileKind::Directory,
                        false,
                    ),
                    record("/System/Library", FileKind::Directory, true),
                    record(
                        "/Users/alice/Pictures/Photos Library.photoslibrary",
                        FileKind::Package,
                        false,
                    ),
                ],
            )
            .unwrap();
        let factory = Arc::new(factory);
        let settings = AiSettingsService::new(
            Arc::new(InMemorySettingsRepository::default()),
            Arc::new(InMemorySecretStore::default()),
            factory.clone(),
        );
        let service = AnalysisService::new(
            files.clone(),
            Arc::new(InMemoryAssessmentRepository::default()),
            settings.clone(),
            Arc::new(FakeClock::at(100)),
            Path::new(HOME),
        );
        Harness {
            service,
            settings,
            factory,
            files,
            scan: scan.id,
        }
    }

    fn enable_claude(h: &Harness) {
        let s = AiSettings {
            provider: Some(AiProviderKind::Anthropic),
            ..AiSettings::default()
        };
        h.settings.save(&s, ApiKeyChange::Set("sk".into())).unwrap();
    }

    fn ai_reply(level: &str) -> serde_json::Value {
        json!({
            "level": level, "category": "vm-image",
            "summary": "Simuladores do Xcode", "explanation": "Use o Xcode.",
            "recommendation": "use-app-or-command",
            "steps": ["Xcode › Settings › Platforms"],
            "commands": [{"command": "xcrun simctl delete unavailable", "description": "limpa"}]
        })
    }

    #[test]
    fn without_ai_uses_local_rules_and_caches_them() {
        let h = harness(FakeAiFactory::replying(json!({})));
        let out = h.service.analyze(FileId(1), false).unwrap();
        assert_eq!(out.assessment.source, AssessmentSource::Heuristic);
        assert_eq!(
            out.assessment.recommendation,
            Recommendation::UseAppOrCommand
        );
        assert!(out.ai_error.is_none());

        let badges = h.service.latest_for(&[FileId(1), FileId(2)]).unwrap();
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].0, FileId(1));
    }

    #[test]
    fn protected_items_are_never_sent_to_the_ai() {
        let h = harness(FakeAiFactory::replying(ai_reply("Safe")));
        enable_claude(&h);
        let out = h.service.analyze(FileId(3), false).unwrap();
        assert_eq!(out.assessment.level, RiskLevel::Dangerous);
        assert_eq!(out.assessment.source, AssessmentSource::ProtectedRule);
        assert!(h.factory.requests().is_empty());
    }

    #[test]
    fn ai_answer_is_used_with_folder_children_and_cached() {
        let h = harness(FakeAiFactory::replying(ai_reply("Caution")));
        enable_claude(&h);

        let out = h.service.analyze(FileId(1), false).unwrap();

        assert_eq!(
            out.assessment.source,
            AssessmentSource::Ai {
                provider: "fake".into(),
                model: "fake-model".into()
            }
        );
        assert_eq!(out.assessment.category, RiskCategory::VmImage);
        assert_eq!(
            out.assessment.commands[0].command,
            "xcrun simctl delete unavailable"
        );
        let sent = h.factory.requests();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].contains("~/Library/Developer/CoreSimulator"));
        assert!(
            sent[0].contains("\"name\": \"Devices\""),
            "children are described"
        );
        assert!(!sent[0].contains("alice"), "home directory is never sent");

        h.service.analyze(FileId(1), false).unwrap();
        assert_eq!(h.factory.requests().len(), 1, "second call hits the cache");
        h.service.analyze(FileId(1), true).unwrap();
        assert_eq!(h.factory.requests().len(), 2, "force bypasses the cache");
    }

    #[test]
    fn ai_failure_falls_back_to_rules_and_reports_the_error() {
        let h = harness(FakeAiFactory::failing(AiError::RateLimited));
        enable_claude(&h);
        let out = h.service.analyze(FileId(1), false).unwrap();
        assert_eq!(out.assessment.source, AssessmentSource::Heuristic);
        assert_eq!(
            out.ai_error.as_deref(),
            Some("rate limited by the provider")
        );
    }

    #[test]
    fn invalid_ai_json_falls_back_to_rules() {
        let h = harness(FakeAiFactory::replying(json!({"level": "Whatever"})));
        enable_claude(&h);
        let out = h.service.analyze(FileId(1), false).unwrap();
        assert_eq!(out.assessment.source, AssessmentSource::Heuristic);
        assert!(out.ai_error.unwrap().contains("invalid response"));
    }

    #[test]
    fn missing_key_is_reported_and_rules_answer() {
        let h = harness(FakeAiFactory::replying(ai_reply("Safe")));
        let s = AiSettings {
            provider: Some(AiProviderKind::OpenAi),
            ..AiSettings::default()
        };
        h.settings.save(&s, ApiKeyChange::Keep).unwrap();
        let out = h.service.analyze(FileId(1), false).unwrap();
        assert_eq!(out.assessment.source, AssessmentSource::Heuristic);
        assert!(out.ai_error.unwrap().contains("missing API key"));
    }

    #[test]
    fn rules_floor_prevents_ai_from_calling_essential_data_safe() {
        let h = harness(FakeAiFactory::replying(ai_reply("Safe")));
        enable_claude(&h);
        let out = h.service.analyze(FileId(4), false).unwrap();
        assert_eq!(out.assessment.level, RiskLevel::Dangerous);
        assert!(out.assessment.explanation.contains("regras de segurança"));
    }

    #[test]
    fn stale_assessments_are_not_shown_after_the_file_changes() {
        let h = harness(FakeAiFactory::replying(json!({})));
        h.service.analyze(FileId(2), false).unwrap();
        // Simulate a rescan where the folder changed size.
        let mut changed = h.files.get_many(&[FileId(2)]).unwrap().remove(0).record;
        changed.size_on_disk = ByteSize::new(1);
        h.files.remove(&[FileId(2)]).unwrap();
        h.files.insert_batch(h.scan, &[changed]).unwrap();
        let new_id = h
            .files
            .find_by_path(Path::new(
                "/Users/alice/Library/Developer/CoreSimulator/Devices",
            ))
            .unwrap()
            .id;
        assert!(h.service.latest_for(&[new_id]).unwrap().is_empty());
    }

    #[test]
    fn unknown_file_is_not_found() {
        let h = harness(FakeAiFactory::replying(json!({})));
        assert!(matches!(
            h.service.analyze(FileId(99), false),
            Err(CoreError::NotFound(_))
        ));
    }
}
