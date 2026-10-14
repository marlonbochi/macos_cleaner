# Feature Specification: Análise de risco assistida por IA

**Feature Branch**: `002-ai-risk-analysis`

**Created**: 2026-10-14

**Status**: Implemented (aguardando validação do usuário)

**Input**: User description: "Mostrar na estrutura do app onde coloco a KEY, salvar para ser usada,
e criar algo de pesquisar sobre o arquivo, mandando o caminho e infos dele para retornar se pode
ser apagado ou se sugere outra forma de apagar — por exemplo, para as VMs do Xcode foram sugeridos
comandos para apagar. Implementar Claude, Gemini, OpenAI e Ollama, com um seletor onde a pessoa
coloca a key e escolhe de qual serviço ela é."

Esta etapa implementa a **User Story 2** de [001-disk-space-cleaner](../001-disk-space-cleaner/spec.md)
e a amplia com recomendações acionáveis. Os requisitos FR-012 a FR-017 da 001 continuam valendo.

## Clarifications

### Session 2026-10-14

- Q: Onde guardar a API key? → A: Keychain do macOS (conforme a constituição); provedor, modelo e
  URL ficam no SQLite.
- Q: Quais provedores? → A: Claude (Anthropic), OpenAI, Gemini e Ollama, escolhidos num seletor.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Configurar o provedor de IA (Priority: P1)

Na tela Configurações, o usuário escolhe o provedor (ou "nenhum"), cola a API key, opcionalmente
troca o modelo/URL, salva e testa a conexão. A chave nunca é exibida de volta.

**Acceptance Scenarios**:

1. **Given** nenhum provedor, **When** escolhe Claude, cola a chave e salva, **Then** a chave vai
   para o Keychain e a tela indica "salva no Keychain".
2. **Given** um provedor salvo, **When** clica em "Testar conexão", **Then** vê "Conexão ok" com
   provedor e modelo, ou a mensagem de erro do provedor.
3. **Given** Ollama, **When** seleciona, **Then** não é pedida chave e pode ajustar a URL local.

### User Story 2 - Analisar um item e saber como liberar o espaço (Priority: P1)

Em "Maiores itens", o usuário clica em "Analisar" num item e vê: nível de risco, o que o item é,
por que tem esse risco, a recomendação (apagar, limpar pelo app/comando, mover para nuvem/externo,
manter, revisar), passos e comandos sugeridos com botão de copiar.

**Acceptance Scenarios**:

1. **Given** IA configurada, **When** analisa `~/Library/Developer/CoreSimulator`, **Then** recebe
   recomendação de usar o Xcode/`xcrun simctl` com os comandos e descrições.
2. **Given** item já analisado e inalterado, **When** reabre, **Then** a análise é reaproveitada
   (sem nova chamada) e o selo de risco aparece na tabela.
3. **Given** IA indisponível ou com erro, **When** analisa, **Then** vê a análise das regras locais
   e o aviso do erro.
4. **Given** item protegido, **When** analisa, **Then** é "Perigoso" sem consultar a IA.

## Requirements *(mandatory)*

- **FR-032**: O sistema MUST permitir escolher entre Claude, OpenAI, Gemini, Ollama ou nenhum
  provedor, com modelo e URL opcionais (padrões por provedor).
- **FR-033**: A API key MUST ser armazenada apenas no Keychain e nunca devolvida ao frontend.
- **FR-034**: Cada análise MUST trazer resumo, explicação, nível, categoria, recomendação, passos e
  comandos sugeridos (com descrição).
- **FR-035**: O app MUST NOT executar comandos sugeridos; apenas exibi-los e permitir copiar.
- **FR-036**: Para pastas, o sistema MUST enviar os nomes/tamanhos dos maiores subitens
  catalogados (até 15) para melhorar a análise; nunca o conteúdo dos arquivos.
- **FR-037**: Regras locais conhecidas como essenciais MUST impedir que a IA rebaixe o item
  abaixo de "Atenção".

## Success Criteria *(mandatory)*

- **SC-201**: Configurar um provedor e obter a primeira análise leva menos de 2 minutos.
- **SC-202**: 100% das chaves salvas ficam fora do SQLite (verificado por teste).
- **SC-203**: Reabrir um item já analisado não gera nova chamada à IA.

## Assumptions

- Modelos padrão: `claude-opus-5-5`, `gpt-6-astra`, `gemini-3.8-flash`, `llama3.2` (editáveis).
- No Claude, modelos que suportam são chamados com fallback de recusa do servidor
  (`fallbacks: "default"`).
- Análise em lote dos N maiores e o benchmark rotulado (SC-006 da 001) ficam para uma etapa futura.
