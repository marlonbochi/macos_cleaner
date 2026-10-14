# Research: Analisador e Limpador de Espaço em Disco com IA

**Feature**: `001-disk-space-cleaner` | **Date**: 2026-10-14

Cada seção segue o formato Decision / Rationale / Alternatives considered.

## R1. Shell desktop e organização do código

- **Decision**: Tauri v2 com workspace Cargo de três crates: `cleaner-core` (domínio + casos de uso
  + ports, sem I/O), `cleaner-infra` (adapters: FS, SQLite, IA, macOS) e `src-tauri` (composição,
  comandos IPC). Frontend Vue 3 + TypeScript + Vite na raiz (`src/`), padrão do template Tauri.
- **Rationale**: atende ao Princípio II (hexagonal). O `cleaner-core` compila e testa sem Tauri,
  o que acelera TDD e o aprendizado de Rust isolado da UI.
- **Alternatives considered**: crate único dentro de `src-tauri` (mais simples, mas acopla domínio
  ao framework); Electron (mais pesado, não atende ao objetivo de estudar Rust).

## R2. Varredura do sistema de arquivos

- **Decision**: `jwalk` (walker paralelo baseado em rayon) com:
  - `follow_links(false)`; não cruzar dispositivos (comparar `st_dev` com o da raiz).
  - Tamanho em disco = `st_blocks * 512` (`MetadataExt::blocks`), tamanho aparente = `len()`.
  - Deduplicação de hard links por `(st_dev, st_ino)` quando `nlink > 1`.
  - Ao varrer `/`, pular `/System/Volumes/Data` (firmlinks duplicariam o conteúdo), `/Volumes`,
    `/dev`, `/private/var/vm` e `/System/Volumes/*` exceto quando escolhidos.
  - Pastas agregadas calculadas por soma bottom-up em memória durante a varredura e persistidas
    apenas para pastas acima do limite.
  - Arquivos abaixo do limite não são persistidos individualmente (somente contam no agregado
    da pasta), mantendo o banco pequeno.
- **Rationale**: `jwalk` é o walker paralelo mais rápido para árvores grandes em Rust e expõe
  ordenação/filtragem por diretório; `st_blocks` reflete o espaço realmente liberável (arquivos
  esparsos, compressão APFS).
- **Alternatives considered**: `walkdir` (sequencial, mais lento); `ignore::WalkParallel`
  (orientado a .gitignore); `getattrlistbulk` via FFI (mais rápido, mas complexo — candidato a
  otimização futura atrás do mesmo port `FileSystemScanner`).
- **Clones APFS**: não há API barata para detectar blocos compartilhados; a UI exibe o aviso
  descrito nos edge cases. Registrado como limitação conhecida.

## R3. Re-varredura incremental

- **Decision** (revisada na implementação): cada varredura grava apenas as entradas acima do
  limite (poucos milhares de linhas, custo baixo) e, ao concluir, substitui o catálogo anterior.
  A parte cara de reprocessar — a avaliação de risco — é incremental: avaliações são cacheadas
  por `(caminho, fingerprint, versão do avaliador)` e reaproveitadas enquanto o item não muda.
  Varreduras canceladas não substituem o último catálogo completo.
- **Rationale**: atende FR-007/SC-003 no ponto de maior custo sem manter dois catálogos em
  sincronia; o walk em si é dominado por I/O de metadados e não tem atalho confiável no APFS.
- **Alternatives considered**: comparar fingerprints e escrever só o delta (complexidade sem
  ganho mensurável); FSEvents (`notify` crate) com journal persistido — evolução futura.

## R4. Persistência

- **Decision**: `rusqlite` (feature `bundled`) + `rusqlite_migration` com migrações SQL
  versionadas embutidas. WAL mode, `synchronous=NORMAL`, inserções em lote de 5.000 linhas por
  transação. Uma conexão de escrita dedicada (thread do scanner) e uma conexão de leitura
  protegida por `Mutex` para os comandos da UI. Banco em
  `~/Library/Application Support/<bundle-id>/catalog.sqlite3`.
- **Rationale**: API síncrona e simples (bom para estudo), sem runtime extra; WAL permite ler
  enquanto a varredura escreve.
- **Alternatives considered**: `sqlx` (async, macros com checagem em compile-time, porém exige
  `DATABASE_URL`/offline mode e adiciona complexidade); `diesel` (ORM pesado para o caso).

## R5. Comunicação Rust ↔ Vue

- **Decision**: comandos Tauri (`#[tauri::command]`) para request/response e `tauri::ipc::Channel`
  para o stream de progresso da varredura (throttle de 100 ms). Tipos compartilhados gerados
  com `ts-rs` para `src/types/generated/`, verificados no CI (diff vazio).
- **Rationale**: `Channel` é o mecanismo recomendado no Tauri v2 para streams ordenados e de alto
  volume; `ts-rs` é estável e evita divergência de tipos entre camadas.
- **Alternatives considered**: eventos globais (`emit`) — sem garantia de ordem e mais acoplados;
  `tauri-specta` — gera também os wrappers de comandos, mas a linha v2 ainda é release candidate.

## R6. Concorrência e cancelamento

- **Decision**: a varredura roda em `tauri::async_runtime::spawn_blocking`; cancelamento via
  `Arc<AtomicBool>` (`CancellationToken` próprio no core) checado pelo walker por entrada.
  Comunicação scanner → writer por `crossbeam-channel` limitado (backpressure).
- **Rationale**: o walk é CPU/I/O bound e síncrono; não há ganho em async aqui.
- **Alternatives considered**: `tokio-util::CancellationToken` — traria tokio para o core, que
  deve ficar livre de runtime.

## R7. Exclusão para a Lixeira

- **Decision**: crate `trash` (usa `NSFileManager trashItemAtURL` no macOS), encapsulado no port
  `TrashPort`. Antes de cada item: revalidação `(exists, size, mtime, inode)` e checagem da
  `ProtectedPathPolicy`.
- **Rationale**: Lixeira nativa preserva o "Colocar de Volta" do Finder (SC-005).
- **Alternatives considered**: `osascript` com Finder (lento, exige automação); mover para
  `~/.Trash` manualmente (perde metadados de restauração).

## R8. Locais protegidos e permissão de Acesso Total ao Disco

- **Decision**: `ProtectedPathPolicy` no core com prefixos bloqueados (`/System`, `/usr`
  exceto `/usr/local`, `/bin`, `/sbin`, `/private/var/db`, `/Library/Apple`,
  `/System/Applications`, `/Applications/Safari.app`, diretório de dados do próprio app,
  `~/Library/Mobile Documents/com~apple~CloudDocs` só via fluxo de nuvem) + itens com flag
  `SF_RESTRICTED`/SIP. Detecção de Acesso Total ao Disco: tentativa de leitura de
  `~/Library/Containers/com.apple.stocks` ou `~/Library/Safari`; abertura do painel via
  `x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles`.
- **Rationale**: regras determinísticas e testáveis (Princípio I, SC-004).
- **Alternatives considered**: depender só da IA para bloquear (viola Princípio IV).

## R9. Provedores de IA (cloud + local)

- **Decision**: port `AiProvider` com adapters criados por `AiProviderFactory` a partir de
  `Settings`:
  - **Anthropic (Claude)**: Messages API, modelo padrão configurável
    (sugestão: `claude-haiku-5-5` pelo custo em lote), saída estruturada por JSON schema.
  - **OpenAI**: Responses API (`POST /v1/responses`) com `text.format` = JSON schema estrito;
    modelo padrão `gpt-6-astra` (recomendação atual da OpenAI; snapshots `gpt-5-*` em
    descontinuação). *Revisado em 2026-10-14.*
  - **Gemini**: `generateContent` (segue suportado) em modo JSON, schema nas instruções e
    validação no cliente; modelo padrão `gemini-3.8-flash` (o 2.5 só atende contas que já o
    usavam). A API Interactions é recomendada para projetos novos — candidata a migração futura.
    *Revisado em 2026-10-14.*
  - **Ollama (local)**: `POST http://localhost:11434/api/chat` com `format` = JSON schema.
  - HTTP via `reqwest` (rustls). Lotes de até 25 itens por chamada; timeout de 30 s; 2 retries
    com backoff exponencial em 429/5xx.
  - Resposta validada contra o schema (`serde` + validação de enum/IDs); itens inválidos ou
    ausentes caem para o `HeuristicClassifier`.
- **Pipeline de classificação** (Chain of Responsibility + Strategy):
  `ProtectedPathRule → CachedAssessment → AiClassifier (se configurado) → HeuristicClassifier`.
- **Rationale**: atende à clarificação Q1 (ambos), ao Princípio IV (fallback, schema, cache) e
  permite trocar de provedor sem tocar no domínio.
- **Alternatives considered**: somente nuvem ou somente local (rejeitados pela clarificação);
  SDKs oficiais por provedor — não há SDK Rust oficial estável para todos; HTTP direto é simples
  e testável com `wiremock`.

## R10. Segredos

- **Decision**: crate `keyring` (feature `apple-native`) atrás do port `SecretStore`.
- **Rationale**: Keychain do macOS, conforme a constituição.
- **Alternatives considered**: arquivo criptografado próprio (reinventa o Keychain).

## R11. Aplicações e jogos instalados

- **Decision**: Strategy `AppDetector` com implementações:
  - `BundleAppDetector`: `/Applications`, `~/Applications`, `/System/Applications` (marcado
    sistema); lê `Info.plist` (crate `plist`): `CFBundleIdentifier`, nome,
    `LSApplicationCategoryType` (categoria `*-games` → jogo).
  - Dados associados por bundle id em `~/Library/{Application Support, Caches, Containers,
    Group Containers, Preferences, Logs, Saved Application State, HTTPStorages, WebKit}`
    (match por bundle id exato e, com confiança menor, por nome do app — marcado na UI).
  - `SteamDetector`: `~/Library/Application Support/Steam/steamapps/appmanifest_*.acf`
    (formato VDF) → `installdir` em `steamapps/common`.
  - `EpicDetector`: `~/Library/Application Support/Epic/EpicGamesLauncher/Data/Manifests/*.item`
    (JSON, `InstallLocation`).
  - `GogDetector`, `BattleNetDetector`: pastas padrão de instalação; detecção por diretório.
  - Último uso: `kMDItemLastUsedDate` via `mdls -raw` (Spotlight), opcional.
- **Rationale**: cada loja tem seu formato; Strategy permite adicionar lojas sem mudar o serviço.
- **Alternatives considered**: `system_profiler SPApplicationsDataType` (lento, não traz dados
  associados nem jogos de lojas).

## R12. Nuvem: detecção, estado de sincronização e "manter somente na nuvem"

- **Decision**:
  - Detecção: iCloud Drive em `~/Library/Mobile Documents/com~apple~CloudDocs`; provedores File
    Provider (OneDrive, Google Drive, Dropbox) em `~/Library/CloudStorage/<Provider>-*`.
  - Estado: chaves de recurso `NSURLUbiquitousItemDownloadingStatusKey`,
    `NSURLUbiquitousItemIsUploadedKey`, `NSURLUbiquitousItemIsUploadingKey`,
    `NSURLUbiquitousItemUploadingErrorKey` via `objc2-foundation`; arquivo já "somente na nuvem"
    detectado também pela flag `SF_DATALESS` em `st_flags`.
  - Evict: `NSFileManager evictUbiquitousItemAtURL:error:` (iCloud e File Provider em macOS 13+).
    Fallback: `brctl evict` (iCloud) e, se falhar para um provedor, a UI orienta "Remover
    Download" no Finder.
  - Somente arquivos com estado `Current` + `IsUploaded=true` são elegíveis (FR-030).
- **Rationale**: mecanismos oficiais do sistema (constituição); escopo restrito a pastas de nuvem
  existentes (clarificação Q2).
- **Risco**: comportamento do evict em File Provider de terceiros varia por provedor → **spike
  técnico** como primeira tarefa da US5 com teste manual por provedor documentado no quickstart.
- **Alternatives considered**: APIs REST de cada provedor (exige OAuth, fora do escopo);
  `fileproviderctl` (ferramenta de desenvolvedor, não garantida em máquinas de usuário).

## R13. Frontend

- **Decision**: Vue 3 (Composition API, `<script setup>`), TypeScript `strict`, Pinia, Vue Router, paginação no banco
  (100 itens por página) em vez de virtualização, i18n com `vue-i18n` (pt-BR inicial), sem kit de UI
  pesado (CSS próprio com variáveis + modo escuro). Camada `services/ipc` encapsula `invoke` e
  `Channel` (Adapter), stores consomem os services, componentes só apresentam.
- **Rationale**: lógica fora dos componentes (Princípio II) e mínimo de dependências (VI).
- **Alternatives considered**: Naive UI / Vuetify (mais rápido para montar telas, mas adiciona
  peso e estilos difíceis de testar); Nuxt (desnecessário em desktop).

## R14. Estratégia de testes

- **Decision**:
  - Rust unit: `cargo test` no `cleaner-core` com fakes in-memory dos ports; `proptest` para
    `ProtectedPathPolicy` e agregação de tamanhos; `insta` para snapshots de prompts de IA.
  - Rust integração: `cleaner-infra/tests/` com `tempfile` (árvores reais, hard links, symlinks,
    permissões negadas), SQLite real em arquivo temporário, `wiremock` para provedores de IA,
    lixeira testada em diretório temporário (testes marcados `#[ignore]` quando exigirem
    interação com o Finder, rodados localmente).
  - Comandos Tauri: testes com `tauri::test::mock_builder()` + `get_ipc_response`.
  - Frontend: Vitest + Vue Test Utils + `@tauri-apps/api/mocks` (`mockIPC`) para stores e
    componentes.
  - Conjunto de referência de 200 caminhos rotulados (`fixtures/risk-benchmark.json`) para SC-006,
    executado contra o heurístico no CI e contra IA real apenas manualmente.
  - Cobertura: `cargo llvm-cov --fail-under-lines 80` (core + infra), Vitest v8 com limite 70%.
- **E2E**: WebDriver do Tauri (`tauri-driver`) não suporta macOS; E2E automatizado fica fora da
  v1. Os cenários de aceitação ficam cobertos por testes de integração + roteiro manual do
  quickstart.
- **Alternatives considered**: Playwright contra o dev server com IPC mockado — possível
  evolução para smoke tests de UI.

## R15. CI/CD

- **Decision**: GitHub Actions:
  - `ci.yml` (push e PR): job `rust` (macos-latest: fmt, clippy `-D warnings`, test, llvm-cov),
    job `frontend` (ubuntu-latest: `npm ci`, eslint, prettier --check, vue-tsc, vitest
    --coverage), job `types` (gera ts-rs e falha se houver diff), job `build` (macos-latest,
    `tauri build --bundles dmg` sem assinatura, artefato anexado ao run). Cache com
    `Swatinem/rust-cache` e cache do npm.
  - `release.yml` (tags `v*`): `tauri-apps/tauri-action` gera release com `.dmg` universal
    (aarch64 + x86_64).
  - Dependabot para cargo, npm e actions; `cargo audit`/`cargo deny` em job semanal.
- **Rationale**: atende à seção Quality Gates da constituição.
- **Alternatives considered**: assinatura e notarização Apple na v1 — requer conta paga de
  desenvolvedor; preparada via secrets opcionais e documentada como passo futuro.

## R16. Gerenciador de pacotes e toolchain

- **Decision**: npm (já instalado; Node 22 LTS), Rust stable via `rustup` com
  `rust-toolchain.toml` fixando o canal `stable` e componentes `rustfmt`, `clippy`,
  `llvm-tools-preview`.
- **Rationale**: menos ferramentas para instalar; toolchain reproduzível no CI.
- **Alternatives considered**: pnpm/bun (sem ganho relevante para o projeto).
