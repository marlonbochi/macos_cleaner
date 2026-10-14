# Tasks: Análise de risco assistida por IA

**Input**: [spec.md](./spec.md); design em [001 plan](../001-disk-space-cleaner/plan.md),
[ai-classification contract](../001-disk-space-cleaner/contracts/ai-classification.md).

## Core (`cleaner-core`)

- [X] A001 Domínio: `Recommendation`, `SuggestedCommand`, `AiProviderKind`, `AiSettings`
- [X] A002 Ports: `AiProvider`, `AiProviderFactory`, `SecretStore`, `AssessmentRepository`, `SettingsRepository`
- [X] A003 Prompt versionado + JSON schema + validação/limites da resposta (testes)
- [X] A004 Regras locais macOS (Xcode, Docker, Fotos, caches, backups iOS, nuvem...) (testes)
- [X] A005 `AnalysisService` (protegido > cache > IA > regras; piso de segurança) (testes)
- [X] A006 `AiSettingsService` (chave só no SecretStore; teste de conexão) (testes)

## Infra (`cleaner-infra`)

- [X] A007 Adapters HTTP: Claude, OpenAI, Gemini, Ollama + retries/backoff (testes de contrato com mock server)
- [X] A008 `KeychainSecretStore` (teste real sob demanda)
- [X] A009 Migração V002 + repositórios SQLite de avaliações e configurações (testes)

## App (`src-tauri`) e UI

- [X] A010 Comandos `analyze_file`, `get_analyses`, `get/save_ai_settings`, `test_ai_connection` + DTOs/TS
- [X] A011 Tela Configurações (seletor de provedor, chave, modelo, URL, testar, remover chave)
- [X] A012 Botão "Analisar", selo de risco na tabela e painel de análise com comandos copiáveis
- [X] A013 Testes de stores, componentes e views

## Próximos passos

- [ ] A014 Validação manual com uma chave real de cada provedor
- [ ] A015 Análise em lote dos maiores itens + benchmark rotulado (SC-006)
