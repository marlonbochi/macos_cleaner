# Quickstart: validação da feature 001-disk-space-cleaner

## Pré-requisitos

- macOS 13+, Xcode Command Line Tools (`xcode-select --install`)
- Rust stable via rustup (o repositório fixa a toolchain em `rust-toolchain.toml`)
- Node.js 22 LTS + npm
- Opcional: Ollama (`ollama pull llama3.2`) ou chave de API do provedor de IA em nuvem

## Setup

```bash
npm ci
cargo build --workspace
```

## Rodar o app em desenvolvimento

```bash
npm run tauri dev
```

## Rodar os quality gates localmente (mesmos do CI)

```bash
npm run check          # eslint + prettier --check + vue-tsc
npm run test:coverage  # vitest com cobertura
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo llvm-cov --workspace --fail-under-lines 80
```

## Cenários de validação manual

Gere uma árvore de teste com `scripts/make-fixture-tree.sh /tmp/mc-fixture` (cria arquivos de
5 GB/2 GB esparsos, 50 MB, hard link, symlink, pasta sem permissão).

| # | História | Passos | Resultado esperado |
|---|---|---|---|
| 1 | US1 | Varrer `/tmp/mc-fixture` com limite 100 MB | Lista: 5 GB, 2 GB; sem o de 50 MB; pasta sem permissão listada como inacessível; hard link contado uma vez |
| 2 | US1 | Iniciar varredura de `~` e cancelar | Para em ≤ 2 s; resultados parciais marcados "incompleto" |
| 3 | US1 | Fechar e reabrir o app | Último catálogo exibido com data |
| 4 | US2 | IA = nenhuma; avaliar os 20 maiores | Todos com nível + explicação, origem "Regras locais" |
| 5 | US2 | IA = Ollama/nuvem; avaliar | Origem "IA"; reavaliar não gera nova chamada (cache) |
| 6 | US2 | Avaliar item em `/System` | Sempre "Perigoso", botão excluir desabilitado |
| 7 | US3 | Selecionar 3 arquivos da fixture → Excluir | Revisão com total; após confirmar, itens na Lixeira e no Histórico |
| 8 | US3 | Alterar um arquivo após a varredura e tentar excluí-lo | Item pulado com motivo "alterado desde a varredura" |
| 9 | US4 | Abrir "Aplicações e Jogos" | Apps ordenados por tamanho total com composição; apps do sistema não removíveis |
| 10 | US5 | Com iCloud Drive ativo, abrir "Nuvem" | Arquivos baixados listados por tamanho; "manter só na nuvem" libera espaço local |
| 11 | US5 | Arquivo com upload pendente | Ação indisponível, estado "não sincronizado" |

### Validação por provedor de nuvem (spike R12)

Para cada provedor instalado (iCloud, OneDrive, Google Drive, Dropbox), registrar em
`docs/cloud-provider-matrix.md`: detecção ok?, estado ok?, evict ok?, observações.

## Benchmark de risco (SC-006)

```bash
cargo test -p cleaner-core --test risk_benchmark
```

Falha se algum item rotulado essencial for classificado como `Safe` ou se a acurácia < 85%.
