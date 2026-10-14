<!--
Sync Impact Report
- Version change: (template) → 1.0.0
- Modified principles: n/a (initial ratification)
- Added principles:
  I. Segurança dos Dados do Usuário (NON-NEGOTIABLE)
  II. Arquitetura Limpa e Orientada a Domínio
  III. Testes Primeiro (NON-NEGOTIABLE)
  IV. IA como Consultora, Nunca Decisora
  V. Desempenho e Responsividade
  VI. Simplicidade e Aprendizado
- Added sections: Stack e Restrições Técnicas; Fluxo de Desenvolvimento e Quality Gates; Governance
- Removed sections: none
- Templates requiring updates: none (plan/spec/tasks templates read the constitution at runtime)
- Deferred TODOs: none
-->

# macOS Cleaner Constitution

## Core Principles

### I. Segurança dos Dados do Usuário (NON-NEGOTIABLE)

- O escaneamento DEVE ser estritamente somente-leitura: nenhuma operação de varredura,
  catalogação ou análise por IA pode alterar, mover ou excluir arquivos.
- Toda exclusão DEVE ser iniciada explicitamente pelo usuário, item a item ou em lote
  revisado, com uma tela de confirmação que exibe tamanho total, quantidade e nível de risco.
- A exclusão padrão DEVE mover os itens para a Lixeira do macOS. Exclusão permanente só pode
  existir como opção explícita, desativada por padrão, com confirmação adicional.
- Caminhos protegidos (diretórios do sistema, `/System`, `/usr`, `/bin`, `/sbin`, `/private/var/db`,
  bundles assinados pela Apple, a própria aplicação e seu banco de dados) DEVEM ser bloqueados
  para exclusão por uma lista de bloqueio testada.
- Links simbólicos NÃO DEVEM ser seguidos durante a varredura para evitar contagem dupla e
  ações fora do escopo escolhido.
- Toda ação destrutiva DEVE ser registrada em um log de auditoria local (o quê, quando, tamanho,
  destino).

Rationale: o produto lida com dados insubstituíveis; um único falso positivo custa mais do que
qualquer ganho de espaço.

### II. Arquitetura Limpa e Orientada a Domínio

- O núcleo em Rust DEVE seguir arquitetura hexagonal (ports & adapters): o crate de domínio
  não depende de Tauri, SQLite, sistema de arquivos real nem de provedores de IA.
- Dependências externas DEVEM ser acessadas por traits (ports) com implementações (adapters)
  injetáveis, permitindo substituição por fakes em testes.
- Padrões de projeto DEVEM ser aplicados onde resolvem um problema real, e documentados no
  código quando não óbvios: Repository (persistência), Strategy (detectores e classificadores
  de risco), Observer/Event (progresso da varredura), Command (ações de exclusão/offload
  auditáveis), Factory/Adapter (provedores de IA e de nuvem).
- Orientação a objetos é aplicada via structs + traits em Rust e via classes/composables
  tipados em TypeScript; herança é evitada em favor de composição.
- O frontend Vue DEVE usar TypeScript estrito, Composition API, e manter lógica de negócio fora
  dos componentes (stores/composables/services).
- Funções DEVEM ter responsabilidade única; `unwrap()`/`expect()` são proibidos fora de testes
  e de invariantes documentadas; erros usam tipos próprios (`thiserror`).

Rationale: separação de camadas torna o código testável, substituível e didático para o
estudo de Rust.

### III. Testes Primeiro (NON-NEGOTIABLE)

- Fluxo TDD: testes escritos e falhando antes da implementação (Red-Green-Refactor).
- Testes unitários DEVEM cobrir domínio e casos de uso; cobertura mínima de linhas de 80% no
  núcleo Rust e de 70% no frontend, verificada no CI.
- Testes de integração DEVEM exercitar adapters reais: sistema de arquivos em diretórios
  temporários (`tempfile`), SQLite real em arquivo temporário/memória, e comandos Tauri.
- Provedores de IA e de nuvem DEVEM ter testes de contrato com fakes; nenhum teste do CI
  depende de rede ou de credenciais.
- Toda regra de segurança do Princípio I DEVE ter teste dedicado (lista de bloqueio, Lixeira,
  symlinks, confirmação).

Rationale: um app que exclui arquivos precisa de garantias executáveis, não de boas intenções.

### IV. IA como Consultora, Nunca Decisora

- A IA DEVE apenas classificar e explicar (nível de risco + justificativa legível); nunca
  dispara exclusões nem pré-seleciona itens de alto risco.
- Por padrão, somente metadados (caminho, nome, extensão, tamanho, datas, tipo, bundle id)
  podem ser enviados a um provedor de IA. Envio de conteúdo de arquivo exige opt-in explícito.
- O app DEVE funcionar sem IA (offline ou sem chave): um classificador heurístico local
  fornece a classificação básica de risco.
- Respostas da IA DEVEM ser validadas contra um schema; respostas inválidas são descartadas
  e caem para a heurística.
- Classificações DEVEM ser cacheadas no SQLite para evitar custo e reenvio de dados.

Rationale: privacidade e previsibilidade; a decisão final é sempre do usuário, bem informado.

### V. Desempenho e Responsividade

- A varredura DEVE rodar fora da thread de UI, ser cancelável, e emitir eventos de progresso
  em intervalos limitados (throttle) para a interface.
- A varredura DEVE ser paralelizada e persistir em lotes (transações) no SQLite.
- Re-varreduras DEVEM ser incrementais quando possível (comparar mtime/tamanho).
- A UI DEVE permanecer responsiva com catálogos de milhões de entradas (paginação e consultas
  ordenadas no banco, nunca carregar tudo em memória no frontend).

Rationale: discos modernos têm milhões de arquivos; uma UI travada destrói a confiança.

### VI. Simplicidade e Aprendizado

- Preferir o mínimo de dependências: SQLite local como única persistência, sem servidores.
- YAGNI: nenhuma funcionalidade especulativa; complexidade adicional exige justificativa na
  seção "Complexity Tracking" do plano.
- Código Rust DEVE ser idiomático e comentado onde ensina algo (ownership, lifetimes,
  concorrência), pois o projeto também é um veículo de aprendizado.

Rationale: simplicidade reduz bugs e mantém o projeto compreensível.

## Stack e Restrições Técnicas

- Desktop: Tauri v2; núcleo em Rust stable (edição 2021+), organizado em workspace Cargo.
- Frontend: Vue 3 + TypeScript (strict) + Vite; estado com Pinia; testes com Vitest +
  Vue Test Utils.
- Persistência: SQLite (via `rusqlite` ou `sqlx`) com migrações versionadas.
- Plataforma alvo: macOS 13+ (Apple Silicon e Intel). O app solicita e explica a permissão
  "Acesso Total ao Disco" quando necessário. Suporte a outras plataformas é não-objetivo.
- Integrações de nuvem (iCloud Drive, OneDrive, Google Drive, Dropbox) DEVEM usar mecanismos
  oficiais do sistema/cliente (ex.: File Provider / `brctl evict` para iCloud), nunca APIs
  não documentadas que manipulem dados diretamente.
- Segredos (chaves de API de IA) DEVEM ser armazenados no Keychain do macOS, nunca no SQLite
  nem em arquivos de configuração em texto puro.

## Fluxo de Desenvolvimento e Quality Gates

- Spec-Driven Development com Spec Kit: cada funcionalidade passa por specify → clarify (quando
  necessário) → plan → tasks → implement, em branch própria.
- Commits seguem Conventional Commits. `main` é protegida; mudanças entram por Pull Request.
- CI (GitHub Actions) DEVE rodar em todo push e PR, e todos os gates DEVEM passar:
  - Rust: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, cobertura
    (`cargo llvm-cov`) com limite mínimo.
  - Frontend: lint (ESLint), checagem de tipos (`vue-tsc`), formatação (Prettier),
    `vitest` com cobertura.
  - Build: `tauri build` em runner macOS.
- CD: tags `v*` geram release no GitHub com o `.dmg` como artefato.
- Code review DEVE verificar conformidade com esta constituição.

## Governance

- Esta constituição prevalece sobre quaisquer outras práticas do projeto.
- Emendas exigem PR dedicado com justificativa, atualização do Sync Impact Report e da versão.
- Versionamento semântico: MAJOR para remoção/redefinição de princípios, MINOR para novos
  princípios ou seções, PATCH para clarificações.
- Violações justificadas DEVEM ser registradas em "Complexity Tracking" no plano da feature.
- Orientação de desenvolvimento em tempo de execução fica em `CLAUDE.md` / `README.md`.

**Version**: 1.0.0 | **Ratified**: 2026-10-14 | **Last Amended**: 2026-10-14
