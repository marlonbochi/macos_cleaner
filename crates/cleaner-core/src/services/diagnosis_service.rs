//! Use case: AI diagnosis of the largest items of a scan (feature 008).
//!
//! Right after a scan, the largest items are sent to the configured AI in batches so the user
//! gets an overview of what can go and how. Only meaningful items are picked:
//!
//! - protected items are skipped (they are never sent to the AI anyway);
//! - "wrapper" folders are skipped: a folder whose largest child holds almost all of its size
//!   (e.g. `Containers` → `com.docker.docker` → `Data` → … → `Docker.raw`) would repeat the
//!   same bytes several times, so only the innermost meaningful item of such a chain is kept.

use std::sync::Arc;

use crate::domain::file_entry::{FileEntry, FileKind};
use crate::domain::risk::RiskAssessment;
use crate::domain::scan::ScanId;
use crate::error::CoreError;
use crate::ports::ai::AiError;
use crate::ports::repository::{FileEntryRepository, FileFilter, FileQuery, FileSort};
use crate::services::ai_settings_service::AiSettingsService;
use crate::services::analysis_service::AnalysisService;

/// How many items the diagnosis covers.
pub const DIAGNOSIS_SIZE: usize = 40;
/// A folder is a wrapper when its largest child holds at least this share of its size.
const WRAPPER_SHARE_PERCENT: u128 = 90;
/// Upper bound of catalogued items examined while picking candidates.
const MAX_EXAMINED: u64 = 2_000;
const PAGE: u32 = 200;

/// One diagnosed item and its latest analysis, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosisItem {
    pub entry: FileEntry,
    pub assessment: Option<RiskAssessment>,
    /// Set when the AI failed for this item and local rules answered instead.
    pub ai_error: Option<String>,
}

#[derive(Clone)]
pub struct DiagnosisService {
    files: Arc<dyn FileEntryRepository>,
    analysis: AnalysisService,
    settings: AiSettingsService,
    limit: usize,
}

impl std::fmt::Debug for DiagnosisService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiagnosisService")
            .field("limit", &self.limit)
            .finish_non_exhaustive()
    }
}

impl DiagnosisService {
    pub fn new(
        files: Arc<dyn FileEntryRepository>,
        analysis: AnalysisService,
        settings: AiSettingsService,
    ) -> Self {
        Self {
            files,
            analysis,
            settings,
            limit: DIAGNOSIS_SIZE,
        }
    }

    /// Changes how many items are diagnosed (tests use small catalogues).
    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// The items the diagnosis covers, largest first.
    pub fn candidates(&self, scan: ScanId) -> Result<Vec<FileEntry>, CoreError> {
        let mut picked = Vec::with_capacity(self.limit);
        let mut offset = 0;
        while picked.len() < self.limit && offset < MAX_EXAMINED {
            let query = FileQuery::new(FileFilter::default(), FileSort::SizeDesc, offset, PAGE);
            let page = self.files.page(scan, &query)?;
            if page.items.is_empty() {
                break;
            }
            offset += page.items.len() as u64;
            for entry in page.items {
                if entry.record.is_protected || self.is_wrapper(scan, &entry)? {
                    continue;
                }
                picked.push(entry);
                if picked.len() == self.limit {
                    break;
                }
            }
        }
        Ok(picked)
    }

    /// The candidates with the analyses stored for them (the tab before / without a new run).
    pub fn latest(&self, scan: ScanId) -> Result<Vec<DiagnosisItem>, CoreError> {
        let entries = self.candidates(scan)?;
        let ids: Vec<_> = entries.iter().map(|e| e.id).collect();
        let mut found = self.analysis.latest_for(&ids)?;
        Ok(entries
            .into_iter()
            .map(|entry| {
                let assessment = found
                    .iter()
                    .position(|(id, _)| *id == entry.id)
                    .map(|i| found.swap_remove(i).1);
                DiagnosisItem {
                    entry,
                    assessment,
                    ai_error: None,
                }
            })
            .collect())
    }

    /// Analyses the candidates with the AI (in batches; unchanged items come from the cache).
    /// Requires a configured provider: without one there is no AI diagnosis to show.
    pub fn run(&self, scan: ScanId, force: bool) -> Result<Vec<DiagnosisItem>, CoreError> {
        if self.settings.provider()?.is_none() {
            return Err(AiError::NotConfigured.into());
        }
        let entries = self.candidates(scan)?;
        let ids: Vec<_> = entries.iter().map(|e| e.id).collect();
        let mut outcomes = self.analysis.analyze_many(&ids, force)?;
        Ok(entries
            .into_iter()
            .map(|entry| {
                let outcome = outcomes
                    .iter()
                    .position(|o| o.file_id == entry.id)
                    .map(|i| outcomes.swap_remove(i));
                DiagnosisItem {
                    entry,
                    ai_error: outcome.as_ref().and_then(|o| o.ai_error.clone()),
                    assessment: outcome.map(|o| o.assessment),
                }
            })
            .collect())
    }

    /// A folder whose largest catalogued child holds almost all of its size.
    fn is_wrapper(&self, scan: ScanId, entry: &FileEntry) -> Result<bool, CoreError> {
        if entry.record.kind != FileKind::Directory {
            return Ok(false);
        }
        let filter = FileFilter {
            parent: Some(entry.record.path.clone()),
            ..FileFilter::default()
        };
        let largest = self
            .files
            .page(scan, &FileQuery::new(filter, FileSort::SizeDesc, 0, 1))?
            .items
            .into_iter()
            .next();
        Ok(largest.is_some_and(|child| {
            u128::from(child.record.size_on_disk.bytes()) * 100
                >= u128::from(entry.record.size_on_disk.bytes()) * WRAPPER_SHARE_PERCENT
        }))
    }
}

// Fixtures use POSIX paths, which are not absolute on Windows. The logic under test does not
// depend on the system; Windows paths are covered by the platform-specific tests.
#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    use crate::domain::ai::{AiProviderKind, AiSettings};
    use crate::domain::file_entry::{FileId, FileRecord};
    use crate::domain::risk::AssessmentSource;
    use crate::domain::scan::NewScan;
    use crate::domain::size::ByteSize;
    use crate::ports::repository::ScanRepository;
    use crate::services::ai_settings_service::ApiKeyChange;
    use crate::services::preferences_service::PreferencesService;
    use crate::testing::{
        FakeAiFactory, FakeClock, InMemoryAssessmentRepository, InMemoryFileEntryRepository,
        InMemoryScanRepository, InMemorySecretStore, InMemorySettingsRepository,
    };
    use serde_json::json;

    const GB: u64 = 1_000_000_000;

    struct Harness {
        service: DiagnosisService,
        settings: AiSettingsService,
        factory: Arc<FakeAiFactory>,
        scan: ScanId,
    }

    fn record(path: &str, kind: FileKind, gb: u64, protected: bool) -> FileRecord {
        FileRecord {
            path: PathBuf::from(path),
            kind,
            size_on_disk: ByteSize::new(gb * GB),
            apparent_size: ByteSize::new(gb * GB),
            file_count: 1,
            created_at: None,
            modified_at: Some(1),
            accessed_at: None,
            inode: 1,
            device: 1,
            is_hardlink_dup: false,
            is_protected: protected,
        }
    }

    /// `Containers` → `docker` → `Docker.raw` is a wrapper chain; `Caches` is a real bucket.
    fn catalogue() -> Vec<FileRecord> {
        use FileKind::{Directory, File, Package};
        vec![
            record("/Users/alice", Directory, 300, true),
            record("/Users/alice/Library/Containers", Directory, 55, false),
            record(
                "/Users/alice/Library/Containers/docker",
                Directory,
                50,
                false,
            ),
            record(
                "/Users/alice/Library/Containers/docker/Docker.raw",
                File,
                50,
                false,
            ),
            record("/Users/alice/Library/Caches", Directory, 30, false),
            record(
                "/Users/alice/Library/Caches/com.spotify",
                Directory,
                10,
                false,
            ),
            record("/Users/alice/Library/Caches/com.npm", Directory, 9, false),
            record("/Applications/Xcode.app", Package, 20, false),
            record("/Users/alice/Movies/trip.mov", File, 8, false),
        ]
    }

    /// Answers a batch request with one `Safe` analysis per id found in the user message.
    fn batch_responder() -> FakeAiFactory {
        FakeAiFactory::responding(|req| {
            let start = req.user.find('{').unwrap();
            let items: serde_json::Value = serde_json::from_str(&req.user[start..]).unwrap();
            let analyses: Vec<_> = items["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| {
                    json!({
                        "id": item["id"], "level": "Safe", "category": "cache",
                        "summary": "Cache", "explanation": "Recreated on demand.",
                        "recommendation": "delete-safely", "steps": [], "commands": []
                    })
                })
                .collect();
            Ok(json!({ "analyses": analyses }))
        })
    }

    fn harness(factory: FakeAiFactory, limit: usize) -> Harness {
        let scans = InMemoryScanRepository::default();
        let scan = scans
            .create(&NewScan::new(vec!["/Users/alice".into()], ByteSize::ZERO, 0).unwrap())
            .unwrap();
        let files = Arc::new(InMemoryFileEntryRepository::default());
        files.insert_batch(scan.id, &catalogue()).unwrap();
        let factory = Arc::new(factory);
        let settings = AiSettingsService::new(
            Arc::new(InMemorySettingsRepository::default()),
            Arc::new(InMemorySecretStore::default()),
            factory.clone(),
        );
        let analysis = AnalysisService::new(
            files.clone(),
            Arc::new(InMemoryAssessmentRepository::default()),
            settings.clone(),
            PreferencesService::new(Arc::new(InMemorySettingsRepository::default())),
            Arc::new(FakeClock::at(100)),
            Path::new("/Users/alice"),
        );
        Harness {
            service: DiagnosisService::new(files, analysis, settings.clone()).with_limit(limit),
            settings,
            factory,
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

    fn names(entries: &[FileEntry]) -> Vec<String> {
        entries.iter().map(|e| e.record.name()).collect()
    }

    #[test]
    fn picks_the_largest_meaningful_items() {
        let h = harness(FakeAiFactory::replying(json!({})), 40);
        let picked = h.service.candidates(h.scan).unwrap();
        assert_eq!(
            names(&picked),
            [
                "Docker.raw",
                "Caches",
                "Xcode.app",
                "com.spotify",
                "com.npm",
                "trip.mov"
            ],
            "protected items and wrapper folders are skipped; packages are kept whole"
        );
    }

    #[test]
    fn stops_at_the_limit() {
        let h = harness(FakeAiFactory::replying(json!({})), 2);
        assert_eq!(
            names(&h.service.candidates(h.scan).unwrap()),
            ["Docker.raw", "Caches"]
        );
    }

    #[test]
    fn running_requires_a_configured_ai() {
        let h = harness(batch_responder(), 40);
        assert!(matches!(
            h.service.run(h.scan, false),
            Err(CoreError::Ai(AiError::NotConfigured))
        ));
        assert!(h.factory.requests().is_empty(), "nothing is sent");
    }

    #[test]
    fn runs_in_one_batch_and_reuses_the_cache() {
        let h = harness(batch_responder(), 40);
        enable_claude(&h);

        let items = h.service.run(h.scan, false).unwrap();

        assert_eq!(items.len(), 6);
        assert!(items.iter().all(|i| matches!(
            i.assessment.as_ref().map(|a| &a.source),
            Some(AssessmentSource::Ai { .. })
        )));
        assert_eq!(h.factory.schema_names(), ["file_analyses"]);

        h.service.run(h.scan, false).unwrap();
        assert_eq!(
            h.factory.requests().len(),
            1,
            "unchanged items are not resent"
        );
        h.service.run(h.scan, true).unwrap();
        assert_eq!(h.factory.requests().len(), 2, "force asks again");
    }

    #[test]
    fn latest_shows_stored_analyses_without_calling_the_ai() {
        let h = harness(batch_responder(), 40);
        let before = h.service.latest(h.scan).unwrap();
        assert!(before.iter().all(|i| i.assessment.is_none()));

        enable_claude(&h);
        h.service.run(h.scan, false).unwrap();
        let after = h.service.latest(h.scan).unwrap();

        assert_eq!(after.len(), 6);
        assert!(after.iter().all(|i| i.assessment.is_some()));
        assert_eq!(after[0].entry.id, FileId(4));
        assert_eq!(h.factory.requests().len(), 1);
    }
}
