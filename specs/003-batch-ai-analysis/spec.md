# Feature Specification: Análise em lote dos itens selecionados

**Feature Branch**: `003-batch-ai-analysis` (a partir de `002-ai-risk-analysis`)

**Created**: 2026-10-14

**Status**: Implemented (aguardando validação do usuário)

**Input**: User description: "Colocar um seletor/checkbox na tela que lista os arquivos e, a partir
desse seletor, mandar as infos dos arquivos selecionados para a IA, fazendo uma única requisição
ao invés de uma por arquivo, garantindo uma busca menor e uso menor de token."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Selecionar e analisar vários itens de uma vez (Priority: P1)

Na lista de maiores itens, o usuário marca checkboxes (ou o da página inteira) e clica em
"Analisar selecionados com IA". Os itens são analisados juntos e cada linha ganha seu selo de
risco; o painel de cada item mostra a análise completa sem nova chamada.

**Acceptance Scenarios**:

1. **Given** 2 itens selecionados, **When** clica em analisar, **Then** é feita 1 requisição à IA
   e os 2 itens recebem análise.
2. **Given** 45 itens selecionados, **When** analisa, **Then** são feitas 3 requisições (20+20+5).
3. **Given** itens já analisados e inalterados na seleção, **When** analisa, **Then** eles não são
   reenviados.
4. **Given** a IA omite ou responde inválido para um item, **When** o lote termina, **Then** só esse
   item usa as regras locais e o usuário é avisado.
5. **Given** itens protegidos, **When** lista, **Then** o checkbox fica desabilitado.

## Requirements *(mandatory)*

- **FR-301**: A lista MUST permitir selecionar itens individualmente e todos da página; a seleção
  persiste entre páginas e filtros da mesma varredura e é limpa ao trocar de varredura.
- **FR-302**: O sistema MUST agrupar os itens pendentes em requisições de até 20 itens, com prompt
  de sistema e schema enviados uma vez por requisição e JSON compacto.
- **FR-303**: Pastas em lote MUST enviar no máximo 5 subitens (15 na análise individual).
- **FR-304**: A resposta MUST ser validada item a item, casando pelo `id`; falhas são isoladas.
- **FR-305**: No máximo 200 itens por ação de análise.

## Success Criteria *(mandatory)*

- **SC-301**: Analisar N itens selecionados faz ⌈N/20⌉ requisições em vez de N (testado).
- **SC-302**: Para 20 itens, o prompt de sistema e o schema são enviados 1 vez em vez de 20.
