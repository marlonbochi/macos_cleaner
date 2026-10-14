# macOS Cleaner

Aplicativo desktop para macOS que encontra o que está ocupando espaço em disco e ajuda a decidir,
com segurança, o que pode ser removido.

- Varre o disco (somente leitura) e lista arquivos e pastas do maior para o menor.
- Classifica o risco de excluir cada item com IA (nuvem ou local) ou regras locais. *(em andamento)*
- Exclui somente após revisão, sempre para a Lixeira, com histórico. *(em andamento)*
- Mostra aplicativos e jogos pelo tamanho total real. *(planejado)*
- Libera espaço de arquivos já sincronizados com iCloud/OneDrive/Google Drive/Dropbox. *(planejado)*

O projeto também é um estudo de Rust: o núcleo é escrito em Rust com arquitetura hexagonal e a
interface em Vue 3 + TypeScript sobre Tauri v2.

## Documentação

| Documento | Conteúdo |
|---|---|
| [Constituição](.specify/memory/constitution.md) | Princípios inegociáveis (segurança, arquitetura, testes, IA) |
| [Especificação](specs/001-disk-space-cleaner/spec.md) | Histórias de usuário, requisitos e critérios de sucesso |
| [Plano](specs/001-disk-space-cleaner/plan.md) | Stack, estrutura e decisões ([research](specs/001-disk-space-cleaner/research.md)) |
| [Modelo de dados](specs/001-disk-space-cleaner/data-model.md) | Entidades e schema SQLite |
| [Contratos](specs/001-disk-space-cleaner/contracts/) | Comandos IPC, ports do core, contrato da IA |
| [Tarefas](specs/001-disk-space-cleaner/tasks.md) | Backlog ordenado por história |

O desenvolvimento segue [Spec Kit](https://github.com/github/spec-kit): `/speckit-specify` →
`/speckit-plan` → `/speckit-tasks` → `/speckit-implement`.

## Arquitetura

```text
┌──────────────── Vue 3 (src/) ────────────────┐
│ views → stores (Pinia) → services/ipc (Api)  │
└──────────────────────┬───────────────────────┘
                       │ Tauri IPC (comandos + Channel)
┌──────────── src-tauri (composition root) ────┐
│ commands → DTOs (ts-rs) → AppState           │
└──────────────────────┬───────────────────────┘
┌──────────── cleaner-core (sem I/O) ──────────┐
│ domain · services (casos de uso) · policy    │
│ ports (traits) ◀── implementados por ──┐     │
└────────────────────────────────────────│─────┘
┌──────────── cleaner-infra ─────────────┴─────┐
│ fs (jwalk) · db (SQLite) · macos             │
└──────────────────────────────────────────────┘
```

Padrões usados: Repository (persistência), Strategy (scanner/classificadores), Observer
(progresso da varredura), Command (ações auditáveis), Factory/Adapter (provedores de IA e
transporte IPC no frontend), Facade (serviços e `Api`).

## Desenvolvimento

Pré-requisitos: macOS 13+, Xcode Command Line Tools, [rustup](https://rustup.rs) (a toolchain é
fixada em `rust-toolchain.toml`) e Node.js 22.

```bash
npm ci
```

```bash
npm run tauri dev
```

### Quality gates (os mesmos do CI)

```bash
npm run check
```

```bash
npm run test:coverage
```

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo llvm-cov --workspace --ignore-filename-regex 'src-tauri/' --fail-under-lines 80
```

Após alterar DTOs em `src-tauri/src/dto.rs`, regenere os tipos TypeScript:

```bash
npm run gen:types
```

### Fixture para testes manuais

```bash
scripts/make-fixture-tree.sh /tmp/mc-fixture
```

## CI/CD

- `ci.yml`: em todo push/PR — fmt, clippy, testes com cobertura (Rust ≥ 80%, frontend ≥ 70%),
  ESLint, Prettier, vue-tsc, checagem de tipos gerados e build do `.dmg`.
- `release.yml`: tags `v*` geram um release (rascunho) com `.dmg` universal.
- `audit.yml`: auditoria semanal de dependências.

## Licença

MIT
