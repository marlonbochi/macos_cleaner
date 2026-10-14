# Tasks: Análise em lote

- [X] B001 Prompt em lote: mensagem compacta com ids, schema `analyses[]`, parsing isolado por item (testes)
- [X] B002 `AnalysisService::analyze_many`: protegidos/cache fora do lote, lotes de 20, fallback por item (testes)
- [X] B003 Fake de IA programável para testes (`FakeAiFactory::responding`)
- [X] B004 Comando IPC `analyze_files` (máx. 200 ids)
- [X] B005 Seleção na lista (checkbox por linha e da página; protegidos desabilitados) (testes)
- [X] B006 Barra de seleção com "Analisar selecionados com IA", progresso e resumo do lote (testes)
- [ ] B007 Validação manual com chave real e medição de tokens (individual × lote)

## Ajustes de configuração da IA (2026-10-14)

- [X] B008 Modelos padrão atualizados (Gemini `gemini-3.8-flash`, OpenAI `gpt-6-astra`) e OpenAI via Responses API
- [X] B009 Listagem de modelos por provedor (Anthropic, OpenAI, Gemini, Ollama) com filtro de modelos de texto (testes de contrato)
- [X] B010 Validação de modelo e URL no domínio (HTTPS obrigatório para provedores com chave, salvo local)
- [X] B011 Tela: seletor de modelos da conta, "Carregar/Atualizar lista", URL oficial por padrão e personalizada em "Avançado" (testes)
