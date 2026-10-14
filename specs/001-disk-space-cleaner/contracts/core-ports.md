# Contract: `cleaner-core` ports (Rust traits)

The ports the core depends on. Implemented ports are shown with their actual signatures; planned
ones (later user stories) are marked as such and may still change. Every port is `Send + Sync`
and has an in-memory fake in `cleaner-core::testing` (`testing` feature).

```rust
// ── Filesystem (Strategy: JwalkScanner in infra) ───────────────────────────
pub trait FileSystemScanner: Send + Sync {
    /// Read-only walk. Never follows symlinks, never leaves the device of each root, never
    /// yields paths outside the roots or inside `options.excluded`, returns promptly on cancel.
    fn scan(
        &self,
        roots: &[PathBuf],
        options: &ScanOptions,          // { excluded: Vec<PathBuf> }
        cancel: &CancellationToken,
        sink: &mut dyn ScanSink,        // Observer: receives entries as they are produced
    ) -> Result<(), ScanError>;
}

pub trait ScanSink {
    fn on_entry(&mut self, entry: ScannedEntry);
    fn on_inaccessible(&mut self, path: &Path, reason: &str);
}

/// Re-validation right before an action (planned use: US3).
pub trait FileInspector: Send + Sync {
    fn fingerprint(&self, path: &Path) -> std::io::Result<Option<FileFingerprint>>;
}

// ── Repositories (Repository pattern: Sqlite* in infra) ────────────────────
pub trait ScanRepository: Send + Sync {
    fn create(&self, scan: &NewScan) -> Result<Scan, RepoError>;
    fn save(&self, scan: &Scan) -> Result<(), RepoError>;
    fn get(&self, id: ScanId) -> Result<Option<Scan>, RepoError>;
    fn find_running(&self) -> Result<Vec<Scan>, RepoError>;
    fn latest_finished(&self) -> Result<Option<Scan>, RepoError>;
    fn delete_all_except(&self, keep: ScanId) -> Result<(), RepoError>;
}

pub trait FileEntryRepository: Send + Sync {
    fn insert_batch(&self, scan: ScanId, records: &[FileRecord]) -> Result<(), RepoError>;
    fn page(&self, scan: ScanId, query: &FileQuery) -> Result<Page<FileEntry>, RepoError>;
    fn get_many(&self, ids: &[FileId]) -> Result<Vec<FileEntry>, RepoError>;
    fn remove(&self, ids: &[FileId]) -> Result<(), RepoError>;
}

/// Assessment cache keyed by path + fingerprint + evaluator version (FR-015).
pub trait AssessmentRepository: Send + Sync {
    fn find(&self, key: &AssessmentKey) -> Result<Option<RiskAssessment>, RepoError>;
    fn save(&self, assessment: &RiskAssessment) -> Result<(), RepoError>;
    fn latest_matching(
        &self,
        items: &[(PathBuf, FileFingerprint)],
    ) -> Result<Vec<RiskAssessment>, RepoError>;
}

pub trait SettingsRepository: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<serde_json::Value>, RepoError>;
    fn set(&self, key: &str, value: &serde_json::Value) -> Result<(), RepoError>;
}

// ── AI (Adapter per provider + Factory) ─────────────────────────────────────
pub trait AiProvider: Send + Sync {
    /// The provider must answer with JSON matching `request.schema`.
    fn complete_json(&self, request: &StructuredRequest<'_>) -> Result<serde_json::Value, AiError>;
    /// (provider id, model) recorded with every assessment.
    fn descriptor(&self) -> (String, String);
}

pub trait AiProviderFactory: Send + Sync {
    fn create(&self, settings: &AiSettings, api_key: Option<String>)
        -> Result<Box<dyn AiProvider>, AiError>;
    /// Text-generation models available to these credentials.
    fn list_models(&self, kind: AiProviderKind, base_url: &str, api_key: Option<String>)
        -> Result<Vec<AiModel>, AiError>;
}

/// OS-backed secret storage (Keychain). Secrets never go to SQLite.
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>, SecretError>;
    fn set(&self, key: &str, value: &str) -> Result<(), SecretError>;
    fn delete(&self, key: &str) -> Result<(), SecretError>;
}

pub trait Clock: Send + Sync {
    fn now(&self) -> i64; // Unix seconds, UTC
}

// ── Planned (US3–US5) ───────────────────────────────────────────────────────
pub trait ActionLogRepository: Send + Sync {            // append-only
    fn append(&self, records: &[ActionRecord]) -> Result<(), RepoError>;
    fn page(&self, offset: u64, limit: u32) -> Result<Page<ActionRecord>, RepoError>;
}
pub trait FileAction: Send + Sync {                     // Command pattern
    fn kind(&self) -> ActionKind;
    fn target(&self) -> &Path;
    fn execute(&self) -> Result<(), ActionError>;
}
pub trait TrashPort: Send + Sync { fn move_to_trash(&self, path: &Path) -> Result<(), ActionError>; }
pub trait CloudEvictPort: Send + Sync { fn evict(&self, path: &Path) -> Result<(), ActionError>; }
// ActionExecutor: validate (policy + fingerprint) → run the Command → record an ActionRecord.

pub trait AppDetector: Send + Sync {                    // Strategy per store
    fn detect(&self) -> Result<Vec<DetectedApp>, DetectError>;
}
pub trait CloudProviderDetector: Send + Sync {
    fn detect(&self) -> Result<Vec<CloudLocation>, DetectError>;
    fn file_state(&self, path: &Path) -> Result<CloudFileStatus, DetectError>;
}
```

## Classification chain

Implemented inside `AnalysisService` (Chain of Responsibility over the ports above):

1. protected path (`ProtectedPathPolicy`) → `Dangerous`, never sent to any provider;
2. cached assessment for the same fingerprint and evaluator version → reused;
3. configured `AiProvider` → validated structured answer (single item or batches of 20);
4. `HeuristicAnalyzer` (local rules) → always available; also the fallback when the AI fails and a
   safety floor for items the rules know are essential.

## Invariants tested by contract

1. Protected items always resolve to `Dangerous` and are never sent to an AI provider.
2. An AI answer never makes an item the rules know is essential look safer than `Caution`.
3. `FileSystemScanner` never reports entries outside the roots or through a symlink.
4. API keys are only written to the `SecretStore`, never to the settings repository.
5. (Planned) `ActionExecutor` never runs `FileAction::execute` for a protected item or one whose
   fingerprint changed; both produce `ActionRecord { status: Skipped }`.
6. (Planned) `ActionLogRepository` exposes no update/delete (also enforced by SQLite triggers).
