# Data Model: Analisador e Limpador de Espaço em Disco com IA

**Feature**: `001-disk-space-cleaner` | **Date**: 2026-10-14

Entidades de domínio (structs em `cleaner-core::domain`) e seu mapeamento em SQLite
(`cleaner-infra::db`). Tamanhos em bytes (`u64` no Rust, `INTEGER` no SQLite); datas em Unix
epoch segundos UTC.

## Value Objects

| Tipo | Descrição | Regras |
|---|---|---|
| `ByteSize(u64)` | Tamanho em bytes | Formatação humana (KB/MB/GB, base 1000 como o Finder) |
| `RiskLevel` | `Safe`, `Caution`, `Dangerous`, `Unknown` | Ordenável; `Dangerous` nunca é pré-selecionável |
| `AssessmentSource` | `ProtectedRule`, `Heuristic`, `Ai { provider, model }` | — |
| `FileKind` | `File`, `Directory`, `AppBundle`, `Package` | Bundles/pacotes são tratados como unidade |
| `FileFingerprint` | `(size_on_disk, mtime, inode)` | Igualdade invalida/revalida avaliações e ações |
| `ScanRoot` | Caminho absoluto + `cross_volumes: bool` | Caminho deve existir e ser absoluto |

## Entities

### Scan

| Campo | Tipo | Regras |
|---|---|---|
| id | i64 PK | — |
| roots | JSON (`Vec<ScanRoot>`) | ≥ 1 raiz |
| min_file_size | u64 | padrão 100 MB |
| status | enum | ver transições |
| started_at / finished_at | i64 / i64? | `finished_at` só em estado terminal |
| files_seen / bytes_seen | u64 | monotônicos |
| inaccessible_count | u64 | — |
| error | text? | só em `Failed` |

**Transições**:

```text
Pending ──start──▶ Running ──complete──▶ Completed
                      │──cancel────────▶ Cancelled   (resultados parciais visíveis, flag incompleto)
                      └──fatal error───▶ Failed
```

Só pode existir uma `Scan` em `Running` por vez. A UI exibe a última `Completed` ou `Cancelled`.

### FileEntry

| Campo | Tipo | Regras |
|---|---|---|
| id | i64 PK | — |
| scan_id | FK → Scan | — |
| path | text UNIQUE por scan | absoluto, NFC normalizado para exibição, bytes originais preservados |
| parent_path | text | índice para navegação por pasta |
| name / extension | text | — |
| kind | `FileKind` | — |
| size_on_disk | u64 | `st_blocks * 512`; agregado para diretórios |
| apparent_size | u64 | `len()`; agregado para diretórios |
| file_count | u64 | 1 para arquivos; soma para diretórios |
| created_at / modified_at / accessed_at | i64? | — |
| inode / device | u64 | dedup de hard links |
| is_hardlink_dup | bool | `true` → não soma no total liberável |
| is_protected | bool | resultado da `ProtectedPathPolicy` |
| cloud_location_id | FK? → CloudLocation | se dentro de pasta de nuvem |

Índices: `(scan_id, size_on_disk DESC)`, `(scan_id, parent_path)`, `(scan_id, extension)`.

Persistência: arquivos ≥ `min_file_size` e diretórios cujo agregado ≥ `min_file_size`.

### RiskAssessment

| Campo | Tipo | Regras |
|---|---|---|
| id | i64 PK | — |
| path | text | chave lógica junto do fingerprint |
| fingerprint | `FileFingerprint` | se o item mudar, a avaliação é inválida |
| level | `RiskLevel` | `ProtectedRule` ⇒ sempre `Dangerous` |
| category | text | ex.: `cache`, `user-document`, `system`, `app-data`, `media`, `installer`, `vm-image` |
| explanation | text | ≤ 280 caracteres, pt-BR |
| source | `AssessmentSource` | — |
| evaluator_version | text | muda quando prompt/regras mudam ⇒ reavaliação |
| created_at | i64 | — |

Índice único: `(path, fingerprint, evaluator_version)` — funciona como cache (FR-015).

### InstalledApp

| Campo | Tipo | Regras |
|---|---|---|
| id | i64 PK | — |
| bundle_id | text? | ausente para jogos sem bundle |
| name | text | — |
| kind | `App`, `Game`, `System` | `System` ⇒ não removível (FR-027) |
| store | `None`, `AppStore`, `Steam`, `Epic`, `Gog`, `BattleNet` | — |
| last_used_at | i64? | Spotlight, quando disponível |
| total_size | u64 | soma dos componentes |
| detected_at | i64 | — |

### AppComponent

| Campo | Tipo | Regras |
|---|---|---|
| id | i64 PK | — |
| app_id | FK → InstalledApp | — |
| role | `Bundle`, `InstallDir`, `AppSupport`, `Cache`, `Container`, `GroupContainer`, `Preferences`, `Logs`, `SavedState`, `Other` | — |
| path | text | — |
| size_on_disk | u64 | — |
| match_confidence | `Exact`, `Heuristic` | `Heuristic` exibe aviso e não é pré-selecionado |

### CloudLocation

| Campo | Tipo | Regras |
|---|---|---|
| id | i64 PK | — |
| provider | `ICloudDrive`, `OneDrive`, `GoogleDrive`, `Dropbox`, `OtherFileProvider` | — |
| root_path | text UNIQUE | — |
| detected_at | i64 | — |

### CloudFileState (derivado, consultado sob demanda e cacheado por varredura)

| Campo | Tipo | Regras |
|---|---|---|
| file_entry_id | FK → FileEntry | — |
| status | `Downloaded`, `CloudOnly`, `UploadPending`, `Error` | só `Downloaded` + uploaded é elegível (FR-030) |
| is_uploaded | bool | — |
| error | text? | — |

### ActionRecord (log de auditoria — append-only)

| Campo | Tipo | Regras |
|---|---|---|
| id | i64 PK | — |
| batch_id | text (UUID) | agrupa uma confirmação do usuário |
| action | `MoveToTrash`, `EvictToCloud` (`DeletePermanently` reservado) | — |
| path | text | — |
| size_on_disk | u64 | tamanho no momento da ação |
| risk_level | `RiskLevel` | nível exibido na confirmação |
| status | `Succeeded`, `Skipped`, `Failed` | — |
| reason | text? | ex.: `changed-since-scan`, `protected`, `in-use`, `not-uploaded` |
| executed_at | i64 | — |

Nunca é atualizado nem removido pelo app (Princípio I).

### Settings (chave/valor)

| Chave | Tipo | Padrão |
|---|---|---|
| `scan.min_file_size` | u64 | 100_000_000 |
| `scan.default_roots` | JSON | `[$HOME, /Applications]` |
| `scan.excluded_paths` | JSON | `[]` |
| `ai.provider` | `none` / `anthropic` / `openai` / `ollama` | `none` |
| `ai.model` | text | depende do provedor |
| `ai.base_url` | text? | `http://localhost:11434` para Ollama |
| `ai.share_content` | bool | `false` (FR-014) |
| `ui.locale` | text | `pt-BR` |

Chaves de API **não** ficam aqui: ficam no Keychain (`SecretStore`), chave `ai.<provider>.api_key`.

## Relationships

```text
Scan 1──* FileEntry *──0..1 CloudLocation
FileEntry (path+fingerprint) 1──0..* RiskAssessment (versões do avaliador)
InstalledApp 1──* AppComponent
FileEntry 1──0..1 CloudFileState
ActionRecord (independente; referencia path, não FK — sobrevive à limpeza do catálogo)
```

## Migrations

`cleaner-infra/src/db/migrations/V001__initial.sql` cria todas as tabelas acima. Toda mudança de
schema é uma nova migração; migrações nunca são editadas após o merge.
