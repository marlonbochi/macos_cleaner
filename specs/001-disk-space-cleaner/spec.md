# Feature Specification: Analisador e Limpador de Espaço em Disco com IA

**Feature Branch**: `001-disk-space-cleaner`

**Created**: 2026-10-14

**Status**: Draft

**Input**: User description: "Aplicativo desktop que varre o computador buscando e catalogando os
arquivos mais pesados para possíveis exclusões, usando IA para avaliar a importância de cada arquivo
para que o usuário esteja ciente dos riscos ao excluir. Lista os arquivos em ordem decrescente de
tamanho e o usuário decide o que excluir. Também identifica jogos e aplicações instaladas cujo
tamanho total (muitos arquivos pequenos) ocupa muito espaço. Verifica arquivos em pastas de nuvem
(iCloud, OneDrive e afins) que ainda estão ocupando espaço local e poderiam ficar apenas na nuvem,
deixando somente o apontador na máquina."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Varredura e ranking dos maiores arquivos (Priority: P1)

O usuário abre o app, escolhe o que varrer (o disco inteiro ou pastas específicas) e inicia a
varredura. Durante o processo ele vê o progresso (itens analisados, volume total encontrado) e pode
cancelar. Ao final, vê uma lista dos arquivos ordenada do maior para o menor, com nome, local,
tamanho, tipo e data do último acesso/modificação. Pode filtrar por tamanho mínimo, tipo e pasta,
e abrir o local do arquivo no Finder. O catálogo fica salvo: ao reabrir o app, o último resultado
continua disponível e uma nova varredura só reprocessa o que mudou.

**Why this priority**: é o núcleo do produto; sozinho já entrega valor ("onde está meu espaço?")
sem IA, sem exclusão e sem nuvem.

**Independent Test**: varrer uma pasta de teste com arquivos de tamanhos conhecidos e verificar que
a lista apresenta todos os arquivos acima do limite, em ordem decrescente, com tamanhos corretos.

**Acceptance Scenarios**:

1. **Given** uma pasta com arquivos de 5 GB, 2 GB e 50 MB e limite mínimo de 100 MB,
   **When** o usuário varre essa pasta, **Then** a lista mostra o arquivo de 5 GB e depois o de
   2 GB, e não mostra o de 50 MB.
2. **Given** uma varredura em andamento, **When** o usuário clica em cancelar,
   **Then** a varredura para em até 2 segundos e os resultados parciais permanecem visíveis
   e marcados como incompletos.
3. **Given** uma varredura concluída anteriormente, **When** o usuário reabre o app,
   **Then** o último catálogo é exibido com a data da varredura, sem precisar varrer de novo.
4. **Given** uma pasta sem permissão de leitura, **When** a varredura passa por ela,
   **Then** a pasta é ignorada, registrada como "inacessível" e a varredura continua.
5. **Given** o app sem a permissão de Acesso Total ao Disco, **When** o usuário inicia a
   varredura do disco inteiro, **Then** o app explica a permissão e como concedê-la, e permite
   continuar com acesso parcial.

---

### User Story 2 - Avaliação de risco assistida por IA (Priority: P2)

Para cada item da lista, o usuário vê um nível de risco de exclusão (Seguro, Atenção, Perigoso,
Desconhecido) e uma explicação curta do que o arquivo provavelmente é e por que ele tem esse risco
(ex.: "Cache do Xcode, pode ser recriado — Seguro"; "Banco de dados da biblioteca do Fotos —
Perigoso"). A avaliação acontece sob demanda ou em lote para os maiores itens. Sem IA disponível,
o app ainda mostra uma classificação básica por regras locais, identificada como tal.

**Why this priority**: é o diferencial do produto — transforma uma lista de tamanhos em uma decisão
informada e reduz exclusões acidentais.

**Independent Test**: com um catálogo de exemplo contendo caches, documentos pessoais e arquivos de
sistema conhecidos, solicitar a avaliação e verificar que cada item recebe nível de risco e
justificativa, e que itens conhecidos de sistema nunca aparecem como "Seguro".

**Acceptance Scenarios**:

1. **Given** um item catalogado, **When** o usuário pede a avaliação, **Then** o item exibe nível
   de risco, explicação em linguagem simples e a origem da avaliação (IA ou regras locais).
2. **Given** um item já avaliado e não modificado, **When** o usuário volta a vê-lo,
   **Then** a avaliação anterior é reaproveitada sem nova consulta.
3. **Given** a IA indisponível ou não configurada, **When** o usuário pede avaliação,
   **Then** a classificação por regras locais é usada e o usuário é avisado.
4. **Given** um item dentro de uma pasta protegida do sistema, **When** qualquer avaliação ocorre,
   **Then** o risco é sempre "Perigoso" e o item não pode ser excluído pelo app.
5. **Given** a configuração padrão, **When** uma avaliação por IA é feita, **Then** apenas
   metadados do arquivo (nome, caminho, extensão, tamanho, datas, tipo) são compartilhados,
   nunca o conteúdo.

---

### User Story 3 - Exclusão segura e consciente (Priority: P3)

O usuário seleciona um ou mais itens e pede para excluí-los. O app mostra uma tela de revisão com
o total a ser liberado, a quantidade de itens e destaca os itens de risco "Atenção" ou "Perigoso",
exigindo confirmação adicional para eles. Os itens vão para a Lixeira, de onde podem ser
recuperados. Um histórico mostra tudo o que foi removido pelo app.

**Why this priority**: concretiza a liberação de espaço; depende das histórias 1 e 2 para ser
segura e informada.

**Independent Test**: selecionar arquivos de uma pasta de teste, confirmar a exclusão e verificar
que estão na Lixeira, que o catálogo foi atualizado e que o histórico registra a ação.

**Acceptance Scenarios**:

1. **Given** 3 itens selecionados somando 4 GB, **When** o usuário pede a exclusão,
   **Then** vê uma revisão com "3 itens — 4 GB" antes de confirmar.
2. **Given** a revisão confirmada, **When** a exclusão termina, **Then** os itens estão na
   Lixeira, saem da lista e o espaço liberado é exibido.
3. **Given** um item "Perigoso" na seleção, **When** o usuário revisa, **Then** o item fica
   destacado e só é incluído após uma confirmação específica para ele.
4. **Given** um item que mudou ou foi removido desde a varredura, **When** a exclusão é
   executada, **Then** esse item é pulado e o usuário é informado.
5. **Given** exclusões realizadas, **When** o usuário abre o histórico, **Then** vê data,
   caminho, tamanho e resultado de cada item.

---

### User Story 4 - Aplicações e jogos que mais ocupam espaço (Priority: P4)

O usuário abre a visão "Aplicações e Jogos" e vê cada aplicação ou jogo instalado com seu tamanho
total real — somando o pacote do app e os dados associados (suporte, caches, contêineres,
bibliotecas de jogos de lojas como Steam, Epic, etc.) — ordenado do maior para o menor, com data
do último uso quando disponível. Pode expandir para ver a composição do tamanho e recebe a
avaliação de risco/importância da IA (ex.: "não usado há 14 meses").

**Why this priority**: resolve o caso em que milhares de arquivos pequenos não aparecem no ranking
de arquivos individuais, mas somados ocupam dezenas de GB.

**Independent Test**: em um ambiente com aplicações de teste com tamanhos conhecidos de pacote e
dados, verificar que o total agregado e a composição batem com os valores esperados.

**Acceptance Scenarios**:

1. **Given** um app com pacote de 1 GB e 3 GB de dados associados, **When** a visão é aberta,
   **Then** o app aparece com 4 GB e a composição mostra 1 GB + 3 GB.
2. **Given** um jogo instalado por uma loja de jogos conhecida, **When** a visão é aberta,
   **Then** o jogo aparece identificado como jogo, com o tamanho total da sua pasta de instalação.
3. **Given** um aplicativo do sistema, **When** listado, **Then** é marcado como do sistema e
   não pode ser removido pelo app.
4. **Given** um app selecionado para remoção, **When** o usuário confirma, **Then** o pacote e
   os dados associados selecionados vão para a Lixeira, seguindo o fluxo da história 3.

---

### User Story 5 - Liberar espaço de arquivos já sincronizados com a nuvem (Priority: P5)

O app detecta as pastas de serviços de nuvem presentes (iCloud Drive, OneDrive, Google Drive,
Dropbox) e lista os arquivos que estão baixados localmente e já sincronizados, ordenados por
tamanho. O usuário pode pedir para "manter somente na nuvem", liberando o espaço local e deixando
o apontador (o arquivo continua visível e é baixado sob demanda).

**Why this priority**: libera espaço sem perder nada, mas depende dos serviços de nuvem do
usuário e de suas regras de sincronização — por isso vem depois do núcleo.

**Independent Test**: com uma pasta de nuvem contendo arquivos sincronizados e baixados, solicitar
"manter somente na nuvem" e verificar que o espaço local é liberado e o arquivo continua listado
como disponível na nuvem.

**Acceptance Scenarios**:

1. **Given** iCloud Drive com 10 GB de arquivos baixados e sincronizados, **When** o usuário abre
   a visão de nuvem, **Then** vê esses arquivos, o provedor e o total de 10 GB liberável.
2. **Given** um arquivo ainda não sincronizado (upload pendente ou com erro), **When** listado,
   **Then** ele aparece como "não sincronizado" e a ação "manter somente na nuvem" fica
   indisponível.
3. **Given** arquivos selecionados, **When** o usuário confirma "manter somente na nuvem",
   **Then** o espaço local é liberado e os arquivos continuam acessíveis pelo sistema.
4. **Given** nenhum serviço de nuvem instalado, **When** a visão é aberta, **Then** o app informa
   que nenhum provedor foi detectado.
5. **Given** arquivos grandes fora das pastas de nuvem, **When** a visão é aberta,
   **Then** [NEEDS CLARIFICATION: o app deve sugerir mover arquivos locais para dentro de uma
   pasta de nuvem para depois liberá-los, ou atuar somente sobre arquivos que já estão em pastas
   de nuvem?]

---

### Edge Cases

- Discos com milhões de arquivos: a interface continua navegável e a lista é paginada.
- Hard links e clones APFS: o mesmo conteúdo não pode ser contado duas vezes no total liberável;
  o app avisa que excluir um clone pode liberar menos espaço que o tamanho aparente.
- Links simbólicos e pontos de montagem: não são seguidos; volumes externos e de rede só são
  varridos se o usuário os escolher explicitamente.
- Arquivos em uso ou bloqueados no momento da exclusão: são pulados e reportados.
- Arquivos que somem ou mudam entre a varredura e a ação: são revalidados antes de qualquer ação.
- Snapshots locais do Time Machine e "espaço purgável": o app explica que o espaço livre pode não
  aumentar imediatamente.
- Disco quase cheio durante a varredura: o catálogo não pode corromper; a varredura falha com
  mensagem clara.
- IA retornando resposta inválida, lenta ou com erro: a avaliação cai para as regras locais.
- Arquivos do próprio app (catálogo, configurações): nunca aparecem como candidatos à exclusão.
- Nomes de arquivo com caracteres especiais, Unicode ou caminhos muito longos: exibidos e tratados
  corretamente.

## Requirements *(mandatory)*

### Functional Requirements

**Varredura e catálogo**

- **FR-001**: O sistema MUST permitir varrer o disco de inicialização inteiro ou pastas/volumes
  escolhidos pelo usuário.
- **FR-002**: O sistema MUST catalogar, para cada arquivo acima do limite mínimo, caminho, nome,
  tamanho em disco, tipo, datas de criação/modificação/último acesso e a varredura de origem.
- **FR-003**: O sistema MUST calcular o tamanho agregado de pastas para permitir navegação por
  pastas que mais ocupam espaço.
- **FR-004**: O sistema MUST exibir os itens em ordem decrescente de tamanho, com filtros por
  tamanho mínimo (padrão 100 MB, configurável), tipo de arquivo, pasta e nível de risco.
- **FR-005**: O sistema MUST mostrar progresso da varredura e permitir cancelá-la.
- **FR-006**: O sistema MUST persistir o catálogo localmente e exibi-lo ao reabrir o app.
- **FR-007**: O sistema MUST reprocessar em novas varreduras somente o que mudou desde a anterior.
- **FR-008**: O sistema MUST ignorar e registrar locais inacessíveis sem interromper a varredura.
- **FR-009**: O sistema MUST NOT seguir links simbólicos nem entrar em outros volumes não
  escolhidos explicitamente.
- **FR-010**: O sistema MUST orientar o usuário a conceder Acesso Total ao Disco quando
  necessário, permitindo uso parcial sem ela.
- **FR-011**: O usuário MUST poder revelar qualquer item no Finder.

**Avaliação de risco**

- **FR-012**: O sistema MUST atribuir a cada item avaliado um nível de risco (Seguro, Atenção,
  Perigoso, Desconhecido), uma explicação em linguagem simples e a origem (IA ou regras locais).
- **FR-013**: O sistema MUST possuir classificação por regras locais que funcione sem conexão e
  sem IA configurada.
- **FR-014**: Por padrão, o sistema MUST compartilhar com a IA apenas metadados dos arquivos;
  compartilhar conteúdo exige ativação explícita pelo usuário.
- **FR-015**: O sistema MUST armazenar avaliações e reaproveitá-las enquanto o item não mudar.
- **FR-016**: O sistema MUST classificar como "Perigoso" e bloquear para exclusão itens em locais
  protegidos do sistema, independentemente da avaliação da IA.
- **FR-017**: O usuário MUST poder configurar o provedor de IA e suas credenciais, que devem ser
  guardadas de forma segura pelo sistema operacional. Provedores suportados:
  [NEEDS CLARIFICATION: a IA deve ser um serviço na nuvem (ex.: Claude/OpenAI, via chave do
  usuário), um modelo local rodando na máquina (ex.: Ollama), ou ambos com escolha nas
  configurações?]

**Exclusão**

- **FR-018**: O sistema MUST excluir itens somente após uma tela de revisão com total, quantidade
  e destaque dos itens de risco, e confirmação do usuário.
- **FR-019**: Itens "Atenção"/"Perigoso" desbloqueados MUST exigir confirmação individual.
- **FR-020**: O sistema MUST mover itens excluídos para a Lixeira por padrão.
- **FR-021**: O sistema MUST revalidar cada item (existência, tamanho, data) imediatamente antes
  da ação e pular itens alterados, informando o usuário.
- **FR-022**: O sistema MUST manter um histórico local de todas as ações destrutivas e de
  liberação de espaço (data, caminho, tamanho, ação, resultado).
- **FR-023**: O sistema MUST NOT pré-selecionar itens de risco "Atenção" ou "Perigoso".

**Aplicações e jogos**

- **FR-024**: O sistema MUST listar aplicações instaladas com tamanho total = pacote + dados
  associados (suporte, caches, contêineres, preferências, logs).
- **FR-025**: O sistema MUST identificar jogos de lojas conhecidas (ex.: Steam, Epic Games,
  GOG, Battle.net, App Store) e calcular o tamanho total de cada jogo.
- **FR-026**: O sistema MUST mostrar a composição do tamanho de cada aplicação/jogo e a data do
  último uso quando disponível.
- **FR-027**: O sistema MUST marcar aplicações do sistema como não removíveis.

**Nuvem**

- **FR-028**: O sistema MUST detectar pastas dos provedores iCloud Drive, OneDrive, Google Drive
  e Dropbox presentes na máquina.
- **FR-029**: O sistema MUST listar arquivos baixados localmente nessas pastas com seu estado de
  sincronização (sincronizado, pendente, erro, somente na nuvem).
- **FR-030**: O sistema MUST permitir "manter somente na nuvem" apenas para arquivos totalmente
  sincronizados, usando o mecanismo oficial de cada provedor.
- **FR-031**: O sistema MUST exibir o total liberável por provedor.

### Key Entities

- **Scan (Varredura)**: uma execução de varredura; raízes escolhidas, início/fim, estado
  (em andamento, concluída, cancelada, falhou), totais e itens inacessíveis.
- **FileEntry (Item de arquivo)**: arquivo ou pasta catalogado; caminho, tamanho em disco,
  tamanho aparente, tipo, datas, identificador de conteúdo para detectar hard links, varredura.
- **RiskAssessment (Avaliação de risco)**: nível, explicação, origem (IA/regras), versão do
  avaliador, data; vinculada a um FileEntry e invalidada quando ele muda.
- **InstalledApp (Aplicação/Jogo)**: nome, identificador, tipo (app, jogo, sistema), loja de
  origem, último uso, componentes (pacote + dados associados) e tamanho total.
- **CloudLocation (Local de nuvem)**: provedor, pasta raiz, arquivos associados e seus estados
  de sincronização.
- **ActionRecord (Registro de ação)**: ação (lixeira, exclusão permanente, manter na nuvem),
  item, tamanho, data, resultado e motivo de falha.
- **Settings (Configurações)**: limite mínimo, raízes padrão, provedor de IA, opção de
  compartilhar conteúdo, exclusões de caminhos definidas pelo usuário.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Uma varredura completa de um disco com 1 milhão de arquivos termina em menos de
  5 minutos em um Mac com SSD, e a interface responde a interações em menos de 200 ms durante a
  varredura.
- **SC-002**: Após a varredura, o usuário identifica os 20 maiores itens do disco em menos de
  30 segundos.
- **SC-003**: Uma re-varredura sem mudanças significativas leva menos de 25% do tempo da
  varredura completa.
- **SC-004**: 0 itens de locais protegidos do sistema podem ser excluídos pelo app (verificado
  por testes automatizados).
- **SC-005**: 100% dos itens removidos pelo app são recuperáveis pela Lixeira no fluxo padrão.
- **SC-006**: Em um conjunto de referência de 200 arquivos rotulados, nenhum item rotulado como
  essencial é classificado como "Seguro", e pelo menos 85% dos itens recebem o nível esperado.
- **SC-007**: 90% dos usuários de teste conseguem liberar pelo menos 5 GB na primeira sessão sem
  consultar ajuda.
- **SC-008**: O total de espaço exibido como liberável difere em no máximo 5% do espaço
  efetivamente liberado (descontando snapshots locais).

## Assumptions

- Usuários são pessoas usando macOS 13 ou superior em seu próprio computador, com conta de
  administrador ou usuário padrão; não há multiusuário nem sincronização entre máquinas.
- O app é distribuído fora da Mac App Store (necessário para Acesso Total ao Disco e varredura
  ampla).
- Escopo padrão de varredura: pasta do usuário e `/Applications`; disco inteiro é opcional.
- Limite mínimo padrão para listar arquivos individuais: 100 MB (configurável).
- O uso de IA pode ter custo para o usuário (chave própria); o app não fornece créditos de IA.
- Exclusão permanente fica fora do escopo da v1 (somente Lixeira); pode ser adicionada depois
  conforme a constituição.
- Detecção de jogos cobre lojas populares no macOS; lojas desconhecidas aparecem como apps
  comuns.
- Interface em português do Brasil na v1, preparada para internacionalização.
- Não-objetivos da v1: Windows/Linux, limpeza automática agendada, detecção de arquivos
  duplicados por conteúdo.
