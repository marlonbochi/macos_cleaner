# Contract: Tauri IPC Commands

Interface entre o frontend Vue e o núcleo Rust. Todos os tipos são gerados por `ts-rs` em
`src/types/generated/`. Erros retornam `AppError` serializado:

```ts
type AppError = { code: ErrorCode; message: string };
type ErrorCode =
  | "NOT_FOUND" | "INVALID_INPUT" | "PERMISSION_DENIED" | "SCAN_IN_PROGRESS"
  | "PROTECTED_PATH" | "AI_UNAVAILABLE" | "CLOUD_UNSUPPORTED" | "STORAGE" | "INTERNAL";
```

Paginação comum:

```ts
type Page<T> = { items: T[]; total: number; offset: number; limit: number };
```

## Varredura (US1)

| Command | Args | Returns | Notes |
|---|---|---|---|
| `start_scan` | `{ roots: string[]; minFileSize?: number; onProgress: Channel<ScanEvent> }` | `ScanSummary` (status `Running`) | `SCAN_IN_PROGRESS` se já houver uma |
| `cancel_scan` | `{ scanId: number }` | `void` | idempotente; parada ≤ 2 s |
| `get_latest_scan` | — | `ScanSummary \| null` | última `Completed`/`Cancelled` |
| `list_files` | `{ scanId; offset; limit (≤500); filter: FileFilter; sort: "sizeDesc" \| "sizeAsc" \| "modifiedAsc" }` | `Page<FileEntryDto>` | ordenação feita no banco |
| `list_folder_children` | `{ scanId; path; offset; limit }` | `Page<FileEntryDto>` | navegação por pasta |
| `reveal_in_finder` | `{ path }` | `void` | FR-011 |
| `get_permissions_status` | — | `{ fullDiskAccess: boolean }` | FR-010 |
| `open_full_disk_access_settings` | — | `void` | abre Ajustes do Sistema |

```ts
type FileFilter = {
  minSize?: number; extensions?: string[]; pathPrefix?: string;
  riskLevels?: RiskLevel[]; kinds?: FileKind[];
};

type ScanEvent =
  | { type: "started"; scanId: number }
  | { type: "progress"; filesSeen: number; bytesSeen: number; currentPath: string }  // throttle 100 ms
  | { type: "inaccessible"; path: string }                                            // agregado na UI
  | { type: "finished"; summary: ScanSummary };

type ScanSummary = {
  id: number; roots: string[]; status: "Running" | "Completed" | "Cancelled" | "Failed";
  startedAt: number; finishedAt: number | null; filesSeen: number; bytesSeen: number;
  inaccessibleCount: number; error: string | null;
};

type FileEntryDto = {
  id: number; path: string; name: string; kind: FileKind; sizeOnDisk: number;
  apparentSize: number; fileCount: number; modifiedAt: number | null; accessedAt: number | null;
  isProtected: boolean; isHardlinkDup: boolean; assessment: RiskAssessmentDto | null;
};
```

## Avaliação de risco (US2)

| Command | Args | Returns | Notes |
|---|---|---|---|
| `assess_files` | `{ fileIds: number[] (≤200) }` | `RiskAssessmentDto[]` | usa cache; fallback heurístico |
| `assess_top` | `{ scanId; count (≤500); onProgress: Channel<AssessEvent> }` | `void` | avaliação em lote dos maiores |
| `get_ai_settings` | — | `AiSettingsDto` | nunca retorna a chave |
| `save_ai_settings` | `AiSettingsInput` | `void` | chave vai para o Keychain |
| `test_ai_connection` | — | `{ ok: boolean; message: string }` | — |

```ts
type RiskLevel = "Safe" | "Caution" | "Dangerous" | "Unknown";
type RiskAssessmentDto = {
  fileId: number; level: RiskLevel; category: string; explanation: string;
  source: { kind: "ProtectedRule" | "Heuristic" } | { kind: "Ai"; provider: string; model: string };
  createdAt: number;
};
type AiSettingsDto = {
  provider: "none" | "anthropic" | "openai" | "ollama"; model: string | null;
  baseUrl: string | null; shareContent: boolean; hasApiKey: boolean;
};
type AiSettingsInput = Omit<AiSettingsDto, "hasApiKey"> & { apiKey?: string | null };
type AssessEvent = { type: "progress"; done: number; total: number } | { type: "finished" };
```

## Exclusão (US3)

| Command | Args | Returns | Notes |
|---|---|---|---|
| `preview_cleanup` | `{ fileIds: number[] }` | `CleanupPreview` | não altera nada |
| `execute_cleanup` | `{ previewToken: string; confirmedRiskyIds: number[] }` | `CleanupResult` | itens arriscados não confirmados são pulados |
| `list_action_history` | `{ offset; limit }` | `Page<ActionRecordDto>` | FR-022 |

```ts
type CleanupPreview = {
  token: string; // amarra a execução exatamente a esta revisão (expira em 10 min)
  items: { fileId: number; path: string; sizeOnDisk: number; level: RiskLevel; blocked: boolean }[];
  totalBytes: number; count: number; riskyIds: number[]; blockedIds: number[];
};
type CleanupResult = {
  batchId: string; freedBytes: number;
  succeeded: number; skipped: { path: string; reason: string }[]; failed: { path: string; reason: string }[];
};
```

## Aplicações e jogos (US4)

| Command | Args | Returns |
|---|---|---|
| `list_installed_apps` | `{ refresh: boolean; kinds?: ("App" \| "Game" \| "System")[] }` | `InstalledAppDto[]` ordenado por `totalSize` desc |
| `get_app_components` | `{ appId }` | `AppComponentDto[]` |
| `preview_app_removal` | `{ appId; componentIds: number[] }` | `CleanupPreview` (reusa fluxo US3) |

## Nuvem (US5)

| Command | Args | Returns |
|---|---|---|
| `list_cloud_locations` | — | `CloudLocationDto[]` com `freeableBytes` |
| `list_cloud_files` | `{ locationId; offset; limit }` | `Page<CloudFileDto>` (ordenado por tamanho) |
| `preview_evict` | `{ fileIds }` | `CleanupPreview` (somente elegíveis; demais em `blockedIds`) |
| `execute_evict` | `{ previewToken }` | `CleanupResult` |
