# Contract: Ports do `cleaner-core` (Rust traits)

Assinaturas de referência; nomes finais podem variar levemente na implementação, mas a
responsabilidade de cada port é contratual. Todos os ports são `Send + Sync` e têm um fake
in-memory em `cleaner-core::testing` (feature `testing`).

```rust
// ── Filesystem (Strategy: JwalkScanner em infra) ───────────────────────────
pub trait FileSystemScanner: Send + Sync {
    fn scan(
        &self,
        roots: &[ScanRoot],
        options: &ScanOptions,
        cancel: &CancellationToken,
        sink: &mut dyn ScanSink,          // Observer: recebe entradas e progresso
    ) -> Result<ScanStats, ScanError>;
}

pub trait ScanSink {
    fn on_entry(&mut self, entry: ScannedEntry);
    fn on_inaccessible(&mut self, path: &Path, err: &io::Error);
    fn on_progress(&mut self, progress: ScanProgress);
}

pub trait FileInspector: Send + Sync {            // revalidação antes de ações
    fn fingerprint(&self, path: &Path) -> Result<Option<FileFingerprint>, io::Error>;
}

// ── Repositories (Repository pattern: Sqlite* em infra) ───────────────────
pub trait ScanRepository: Send + Sync {
    fn create(&self, scan: &NewScan) -> Result<ScanId, RepoError>;
    fn update_status(&self, id: ScanId, status: ScanStatus, stats: &ScanStats) -> Result<(), RepoError>;
    fn latest_finished(&self) -> Result<Option<Scan>, RepoError>;
}

pub trait FileEntryRepository: Send + Sync {
    fn insert_batch(&self, scan: ScanId, entries: &[FileEntry]) -> Result<(), RepoError>;
    fn page(&self, scan: ScanId, q: &FileQuery) -> Result<Page<FileEntry>, RepoError>;
    fn get_many(&self, ids: &[FileId]) -> Result<Vec<FileEntry>, RepoError>;
    fn remove(&self, ids: &[FileId]) -> Result<(), RepoError>;
    fn fingerprints_for(&self, scan: ScanId) -> Result<HashMap<PathBuf, FileFingerprint>, RepoError>;
}

pub trait AssessmentRepository: Send + Sync {
    fn find(&self, key: &AssessmentKey) -> Result<Option<RiskAssessment>, RepoError>;
    fn save(&self, a: &RiskAssessment) -> Result<(), RepoError>;
}

pub trait ActionLogRepository: Send + Sync {        // append-only
    fn append(&self, records: &[ActionRecord]) -> Result<(), RepoError>;
    fn page(&self, offset: u64, limit: u64) -> Result<Page<ActionRecord>, RepoError>;
}

pub trait AppRepository: Send + Sync { /* replace_all, list, components */ }
pub trait SettingsRepository: Send + Sync { /* get<T>, set<T> */ }

// ── Risk classification (Strategy + Chain of Responsibility) ─────────────
pub trait RiskClassifier: Send + Sync {
    /// Retorna avaliações apenas para os itens que conseguiu classificar;
    /// os restantes seguem para o próximo elo da cadeia.
    fn classify(&self, items: &[FileEntry]) -> Result<Vec<RiskAssessment>, ClassifyError>;
    fn name(&self) -> &'static str;
}
// Implementações: ProtectedPathClassifier, CachedClassifier, AiClassifier, HeuristicClassifier
// Composição:   ClassifierChain(Vec<Box<dyn RiskClassifier>>)

pub trait AiProvider: Send + Sync {               // Adapter por provedor; criado pela Factory
    fn complete_structured(&self, req: &AiRequest) -> Result<AiResponse, AiError>;
    fn descriptor(&self) -> AiDescriptor;          // provider + model, para auditoria
}

pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>, SecretError>;
    fn set(&self, key: &str, value: &str) -> Result<(), SecretError>;
    fn delete(&self, key: &str) -> Result<(), SecretError>;
}

// ── Actions (Command pattern) ─────────────────────────────────────────────
pub trait FileAction: Send + Sync {
    fn kind(&self) -> ActionKind;
    fn target(&self) -> &Path;
    fn execute(&self) -> Result<(), ActionError>;
}
pub trait TrashPort: Send + Sync { fn move_to_trash(&self, path: &Path) -> Result<(), ActionError>; }
pub trait CloudEvictPort: Send + Sync { fn evict(&self, path: &Path) -> Result<(), ActionError>; }
// ActionExecutor: valida (policy + fingerprint) → executa Command → registra ActionRecord.

// ── Apps & Cloud (Strategy) ───────────────────────────────────────────────
pub trait AppDetector: Send + Sync {
    fn detect(&self) -> Result<Vec<DetectedApp>, DetectError>;
}
pub trait CloudProviderDetector: Send + Sync {
    fn detect(&self) -> Result<Vec<CloudLocation>, DetectError>;
    fn file_state(&self, path: &Path) -> Result<CloudFileStatus, DetectError>;
}

// ── Misc ──────────────────────────────────────────────────────────────────
pub trait Clock: Send + Sync { fn now(&self) -> i64; }
```

## Invariantes testadas por contrato

1. `ProtectedPathClassifier` sempre vence: nenhum elo posterior pode rebaixar `Dangerous`.
2. `ActionExecutor` nunca chama `FileAction::execute` para item protegido ou com fingerprint
   divergente; ambos geram `ActionRecord { status: Skipped }`.
3. `FileSystemScanner` nunca reporta entradas fora das raízes nem via symlink.
4. `ActionLogRepository` não expõe update/delete.
