//! Use case: analyse catalogued items and explain whether / how to delete them (US2).
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
    BATCH_MAX_CHILDREN, BATCH_SCHEMA_NAME, BatchInput, MAX_BATCH, MAX_CHILDREN, PROMPT_VERSION,
    SCHEMA_NAME, batch_response_schema, batch_user_message, parse_batch_response, parse_response,
    response_schema, system_prompt, user_message,
};
use crate::analysis::verdict::{Verdict, protected_verdict};
use crate::domain::file_entry::{FileEntry, FileId, FileKind, FileRecord};
use crate::domain::locale::Locale;
use crate::domain::platform::Platform;
use crate::domain::risk::{AssessmentKey, AssessmentSource, RiskAssessment, RiskLevel};
use crate::error::CoreError;
use crate::ports::Clock;
use crate::ports::ai::{AiError, AiProvider, StructuredRequest};
use crate::ports::repository::{
    AssessmentRepository, FileEntryRepository, FileFilter, FileQuery, FileSort,
};
use crate::services::ai_settings_service::AiSettingsService;
use crate::services::preferences_service::PreferencesService;

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
    preferences: PreferencesService,
    heuristics: HeuristicAnalyzer,
    clock: Arc<dyn Clock>,
    home: PathBuf,
    platform: Platform,
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
        preferences: PreferencesService,
        clock: Arc<dyn Clock>,
        home: &Path,
    ) -> Self {
        Self {
            files,
            assessments,
            settings,
            preferences,
            heuristics: HeuristicAnalyzer::new(home),
            clock,
            home: home.to_path_buf(),
            platform: Platform::current(),
        }
    }

    /// Analyses items as found on `platform` (rules and AI instructions differ per system).
    #[must_use]
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.heuristics = HeuristicAnalyzer::for_platform(&self.home, platform);
        self.platform = platform;
        self
    }

    /// Analyses an item. With `force`, ignores the cache (the "analyse again" button).
    pub fn analyze(&self, id: FileId, force: bool) -> Result<AnalysisOutcome, CoreError> {
        self.analyze_many(&[id], force)?
            .pop()
            .ok_or_else(|| CoreError::NotFound(format!("file {}", id.0)))
    }

    /// Analyses several items, sending everything that still needs the AI in as few requests
    /// as possible (up to `MAX_BATCH` items each): the system prompt and schema are sent once
    /// per batch instead of once per item. Unknown ids are skipped. Results keep input order.
    pub fn analyze_many(
        &self,
        ids: &[FileId],
        force: bool,
    ) -> Result<Vec<AnalysisOutcome>, CoreError> {
        let mut entries = self.files.get_many(ids)?;
        entries.sort_by_key(|e| ids.iter().position(|id| *id == e.id));
        let now = self.clock.now();
        // Analyses are written in the user's language; it is part of the cache key.
        let locale = self.preferences.locale();

        // A misconfigured provider (e.g. missing key) is reported, not fatal.
        let (provider, setup_error) = match self.settings.provider() {
            Ok(p) => (p, None),
            Err(e) => (None, Some(e.to_string())),
        };
        let version = provider.as_ref().map_or_else(
            || rules_version(locale),
            |p| {
                let (name, model) = p.descriptor();
                format!("{name}:{model}:prompt-v{PROMPT_VERSION}:{}", locale.tag())
            },
        );

        let mut outcomes: Vec<AnalysisOutcome> = Vec::with_capacity(entries.len());
        let mut pending: Vec<&FileEntry> = Vec::new();
        for entry in &entries {
            let record = &entry.record;
            if record.is_protected {
                let assessment = protected_verdict(&record.path, locale).into_assessment(
                    record,
                    AssessmentSource::ProtectedRule,
                    "protected".into(),
                    now,
                );
                outcomes.push(AnalysisOutcome {
                    file_id: entry.id,
                    assessment,
                    ai_error: None,
                });
                continue;
            }
            if !force {
                let key = AssessmentKey {
                    path: record.path.clone(),
                    fingerprint: record.fingerprint(),
                    evaluator_version: version.clone(),
                };
                if let Some(assessment) = self.assessments.find(&key)? {
                    outcomes.push(AnalysisOutcome {
                        file_id: entry.id,
                        assessment,
                        ai_error: setup_error.clone(),
                    });
                    continue;
                }
            }
            pending.push(entry);
        }

        let fresh: Vec<AnalysisOutcome> = match provider {
            None => pending
                .iter()
                .map(|e| self.rules_outcome(e, locale, now, setup_error.clone()))
                .collect(),
            Some(provider) if pending.len() == 1 => {
                let entry = pending[0];
                vec![match self.ask_ai(provider.as_ref(), entry, locale) {
                    Ok(verdict) => {
                        self.ai_outcome(provider.as_ref(), entry, verdict, &version, locale, now)
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "AI analysis failed, using local rules");
                        self.rules_outcome(entry, locale, now, Some(e.to_string()))
                    }
                }]
            }
            Some(provider) => {
                let mut out = Vec::with_capacity(pending.len());
                for chunk in pending.chunks(MAX_BATCH) {
                    out.extend(self.analyze_batch(
                        provider.as_ref(),
                        chunk,
                        &version,
                        locale,
                        now,
                    )?);
                }
                out
            }
        };
        for outcome in &fresh {
            self.assessments.save(&outcome.assessment)?;
        }
        outcomes.extend(fresh);
        outcomes.sort_by_key(|o| ids.iter().position(|id| *id == o.file_id));
        Ok(outcomes)
    }

    /// One AI request for a chunk of items; items missing or invalid in the answer fall back
    /// to local rules individually.
    fn analyze_batch(
        &self,
        provider: &dyn AiProvider,
        chunk: &[&FileEntry],
        version: &str,
        locale: Locale,
        now: i64,
    ) -> Result<Vec<AnalysisOutcome>, CoreError> {
        let children: Vec<Vec<FileRecord>> = chunk
            .iter()
            .map(|e| self.children_of(e, BATCH_MAX_CHILDREN))
            .collect::<Result<_, _>>()?;
        let ids: Vec<String> = chunk.iter().map(|e| e.id.0.to_string()).collect();
        let inputs: Vec<BatchInput<'_>> = chunk
            .iter()
            .zip(&children)
            .zip(&ids)
            .map(|((e, c), id)| BatchInput {
                id,
                record: &e.record,
                children: c,
            })
            .collect();
        let schema = batch_response_schema();
        let user = batch_user_message(&inputs, &self.home);
        let answer = provider
            .complete_json(&StructuredRequest {
                system: &system_prompt(locale, self.platform),
                user: &user,
                schema: &schema,
                schema_name: BATCH_SCHEMA_NAME,
            })
            .and_then(|reply| parse_batch_response(&reply));

        Ok(match answer {
            Ok(mut by_id) => chunk
                .iter()
                .zip(&ids)
                .map(|(entry, id)| match by_id.remove(id) {
                    Some(Ok(verdict)) => {
                        self.ai_outcome(provider, entry, verdict, version, locale, now)
                    }
                    Some(Err(e)) => self.rules_outcome(entry, locale, now, Some(e.to_string())),
                    None => self.rules_outcome(
                        entry,
                        locale,
                        now,
                        Some("the AI did not return an analysis for this item".into()),
                    ),
                })
                .collect(),
            Err(e) => {
                tracing::warn!(error = %e, items = chunk.len(), "batch AI analysis failed");
                chunk
                    .iter()
                    .map(|entry| self.rules_outcome(entry, locale, now, Some(e.to_string())))
                    .collect()
            }
        })
    }

    fn ai_outcome(
        &self,
        provider: &dyn AiProvider,
        entry: &FileEntry,
        verdict: Verdict,
        version: &str,
        locale: Locale,
        now: i64,
    ) -> AnalysisOutcome {
        let (name, model) = provider.descriptor();
        let rules = self.heuristics.analyze(&entry.record, locale);
        let assessment = apply_safety_floor(verdict, &rules, locale).into_assessment(
            &entry.record,
            AssessmentSource::Ai {
                provider: name,
                model,
            },
            version.to_owned(),
            now,
        );
        AnalysisOutcome {
            file_id: entry.id,
            assessment,
            ai_error: None,
        }
    }

    fn rules_outcome(
        &self,
        entry: &FileEntry,
        locale: Locale,
        now: i64,
        ai_error: Option<String>,
    ) -> AnalysisOutcome {
        let assessment = self
            .heuristics
            .analyze(&entry.record, locale)
            .into_assessment(
                &entry.record,
                AssessmentSource::Heuristic,
                rules_version(locale),
                now,
            );
        AnalysisOutcome {
            file_id: entry.id,
            assessment,
            ai_error,
        }
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

    fn ask_ai(
        &self,
        provider: &dyn AiProvider,
        entry: &FileEntry,
        locale: Locale,
    ) -> Result<Verdict, AiError> {
        let children = self
            .children_of(entry, MAX_CHILDREN)
            .map_err(|e| AiError::InvalidConfig(e.to_string()))?;
        let schema = response_schema();
        let user = user_message(&entry.record, &children, &self.home);
        let reply = provider.complete_json(&StructuredRequest {
            system: &system_prompt(locale, self.platform),
            user: &user,
            schema: &schema,
            schema_name: SCHEMA_NAME,
        })?;
        parse_response(&reply)
    }

    /// Largest catalogued children of a folder (empty for files).
    fn children_of(&self, entry: &FileEntry, limit: usize) -> Result<Vec<FileRecord>, CoreError> {
        if entry.record.kind == FileKind::File {
            return Ok(Vec::new());
        }
        let filter = FileFilter {
            parent: Some(entry.record.path.clone()),
            ..FileFilter::default()
        };
        let query = FileQuery::new(
            filter,
            FileSort::SizeDesc,
            0,
            u32::try_from(limit).unwrap_or(u32::MAX),
        );
        Ok(self
            .files
            .page(entry.scan_id, &query)?
            .items
            .into_iter()
            .map(|c| c.record)
            .collect())
    }
}

/// Evaluator version of the local rules for a language.
fn rules_version(locale: Locale) -> String {
    format!("{HEURISTIC_VERSION}:{}", locale.tag())
}

/// Items the local rules know to be essential never drop below `Caution` (SC-006).
fn apply_safety_floor(mut ai: Verdict, rules: &Verdict, locale: Locale) -> Verdict {
    if rules.level == RiskLevel::Dangerous && ai.level < RiskLevel::Caution {
        ai.level = rules.level;
        let note = locale.pick(
            "Level raised by the safety rules",
            "Nível elevado pelas regras de segurança",
        );
        ai.explanation = format!("{} ({note}: {})", ai.explanation, rules.summary);
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
        prefs: PreferencesService,
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
        let prefs = PreferencesService::new(Arc::new(InMemorySettingsRepository::default()));
        let service = AnalysisService::new(
            files.clone(),
            Arc::new(InMemoryAssessmentRepository::default()),
            settings.clone(),
            prefs.clone(),
            Arc::new(FakeClock::at(100)),
            Path::new(HOME),
        );
        Harness {
            service,
            prefs,
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
    fn analyses_follow_the_saved_language_and_are_cached_per_language() {
        let h = harness(FakeAiFactory::replying(json!({})));
        let en = h.service.analyze(FileId(1), false).unwrap();
        assert_eq!(
            en.assessment.summary,
            "Xcode simulators and runtimes (CoreSimulator)."
        );
        assert_eq!(en.assessment.evaluator_version, "heuristic-v3:en-US");

        h.prefs.set_locale(Locale::PtBr).unwrap();
        let pt = h.service.analyze(FileId(1), false).unwrap();
        assert_eq!(
            pt.assessment.summary,
            "Simuladores e runtimes do Xcode (CoreSimulator)."
        );
        assert_eq!(pt.assessment.evaluator_version, "heuristic-v3:pt-BR");
    }

    #[test]
    fn ai_is_asked_to_answer_in_the_saved_language() {
        let h = harness(FakeAiFactory::responding(|req| {
            assert!(req.system.ends_with("in Brazilian Portuguese."));
            Ok(ai_reply("Caution"))
        }));
        enable_claude(&h);
        h.prefs.set_locale(Locale::PtBr).unwrap();
        let out = h.service.analyze(FileId(1), false).unwrap();
        assert_eq!(
            out.assessment.evaluator_version,
            "fake:fake-model:prompt-v3:pt-BR"
        );
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
        assert!(
            out.assessment
                .explanation
                .contains("Level raised by the safety rules")
        );

        h.prefs.set_locale(Locale::PtBr).unwrap();
        let out = h.service.analyze(FileId(4), false).unwrap();
        assert!(
            out.assessment
                .explanation
                .contains("Nível elevado pelas regras de segurança"),
            "the note follows the user's language"
        );
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

    /// Answers a batch request with one analysis per id found in the user message.
    fn batch_responder(level: &'static str) -> FakeAiFactory {
        FakeAiFactory::responding(move |req| {
            if req.schema_name != "file_analyses" {
                return Ok(ai_reply(level));
            }
            let start = req.user.find('{').unwrap();
            let items: serde_json::Value = serde_json::from_str(&req.user[start..]).unwrap();
            let analyses: Vec<_> = items["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| {
                    let mut a = ai_reply(level);
                    a["id"] = item["id"].clone();
                    a
                })
                .collect();
            Ok(json!({ "analyses": analyses }))
        })
    }

    #[test]
    fn batch_sends_one_request_for_many_items_and_keeps_order() {
        let h = harness(batch_responder("Safe"));
        enable_claude(&h);

        let out = h
            .service
            .analyze_many(
                &[FileId(4), FileId(3), FileId(1), FileId(2), FileId(99)],
                false,
            )
            .unwrap();

        assert_eq!(
            out.iter().map(|o| o.file_id).collect::<Vec<_>>(),
            [FileId(4), FileId(3), FileId(1), FileId(2)],
            "input order, unknown ids skipped"
        );
        assert_eq!(
            h.factory.schema_names(),
            ["file_analyses"],
            "exactly one batch request"
        );
        let sent = &h.factory.requests()[0];
        assert!(!sent.contains("/System"), "protected items are never sent");
        assert!(
            sent.contains(r#""id":"1""#)
                && sent.contains(r#""id":"2""#)
                && sent.contains(r#""id":"4""#)
        );
        assert_eq!(out[1].assessment.source, AssessmentSource::ProtectedRule);
        assert!(matches!(
            out[2].assessment.source,
            AssessmentSource::Ai { .. }
        ));
        // Safety floor still applies inside batches: the AI said Safe for the Photos library.
        assert_eq!(out[0].assessment.level, RiskLevel::Dangerous);
    }

    #[test]
    fn cached_items_are_not_resent_in_a_batch() {
        let h = harness(batch_responder("Caution"));
        enable_claude(&h);
        h.service.analyze(FileId(1), false).unwrap();

        h.service
            .analyze_many(&[FileId(1), FileId(2), FileId(4)], false)
            .unwrap();

        assert_eq!(h.factory.schema_names(), ["file_analysis", "file_analyses"]);
        assert!(!h.factory.requests()[1].contains(r#""id":"1""#));
    }

    #[test]
    fn items_missing_from_the_answer_fall_back_individually() {
        let h = harness(FakeAiFactory::responding(|_| {
            let mut a = ai_reply("Caution");
            a["id"] = json!("1");
            Ok(json!({ "analyses": [a, { "id": "2", "level": "???" }] }))
        }));
        enable_claude(&h);

        let out = h
            .service
            .analyze_many(&[FileId(1), FileId(2), FileId(4)], false)
            .unwrap();

        assert!(matches!(
            out[0].assessment.source,
            AssessmentSource::Ai { .. }
        ));
        assert_eq!(out[1].assessment.source, AssessmentSource::Heuristic);
        assert!(
            out[1]
                .ai_error
                .as_deref()
                .unwrap()
                .contains("invalid response")
        );
        assert_eq!(out[2].assessment.source, AssessmentSource::Heuristic);
        assert_eq!(
            out[2].ai_error.as_deref(),
            Some("the AI did not return an analysis for this item")
        );
    }

    #[test]
    fn a_failed_batch_falls_back_to_rules_for_all_items() {
        let h = harness(FakeAiFactory::failing(AiError::RateLimited));
        enable_claude(&h);
        let out = h
            .service
            .analyze_many(&[FileId(1), FileId(2)], false)
            .unwrap();
        assert!(
            out.iter()
                .all(|o| o.assessment.source == AssessmentSource::Heuristic)
        );
        assert!(
            out.iter()
                .all(|o| o.ai_error.as_deref() == Some("rate limited by the provider"))
        );
    }

    #[test]
    fn large_selections_are_split_into_batches() {
        let h = harness(batch_responder("Safe"));
        enable_claude(&h);
        let records: Vec<_> = (0..45)
            .map(|i| {
                record(
                    &format!("/Users/alice/Downloads/file{i}.zip"),
                    FileKind::File,
                    false,
                )
            })
            .collect();
        h.files.insert_batch(h.scan, &records).unwrap();
        let ids: Vec<FileId> = (5..50).map(FileId).collect();

        let out = h.service.analyze_many(&ids, false).unwrap();

        assert_eq!(out.len(), 45);
        assert_eq!(h.factory.schema_names().len(), 3, "20 + 20 + 5");
    }
}
