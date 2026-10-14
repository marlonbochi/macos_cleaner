# Contract: Classificação de risco por IA

Contrato entre `AiClassifier` e qualquer `AiProvider` (Anthropic, OpenAI-compatível, Ollama).

## Entrada enviada ao modelo (somente metadados — FR-014)

```json
{
  "locale": "pt-BR",
  "home": "~",
  "items": [
    {
      "id": "42",
      "path": "~/Library/Developer/Xcode/DerivedData",
      "kind": "Directory",
      "extension": null,
      "size_on_disk": 18234567890,
      "file_count": 120394,
      "modified_at": "2026-09-30",
      "accessed_at": "2026-10-01",
      "bundle_id": null
    }
  ]
}
```

Regras:
- O diretório home do usuário é substituído por `~` (não enviar o nome de usuário).
- No máximo 25 itens por requisição.
- Conteúdo de arquivo só é incluído (campo `excerpt`, ≤ 2 KB de texto) se `ai.share_content = true`.

## Saída esperada (JSON schema)

```json
{
  "type": "object",
  "required": ["assessments"],
  "additionalProperties": false,
  "properties": {
    "assessments": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["id", "level", "category", "explanation"],
        "additionalProperties": false,
        "properties": {
          "id": { "type": "string" },
          "level": { "enum": ["Safe", "Caution", "Dangerous", "Unknown"] },
          "category": {
            "enum": ["cache", "build-artifact", "installer", "vm-image", "media", "archive",
                     "user-document", "app-data", "app-bundle", "game", "system",
                     "backup", "log", "other"]
          },
          "explanation": { "type": "string", "maxLength": 280 }
        }
      }
    }
  }
}
```

## Regras de validação no cliente

1. IDs desconhecidos são ignorados; IDs ausentes caem para o `HeuristicClassifier`.
2. JSON inválido ⇒ todo o lote cai para o heurístico, com log de aviso.
3. Itens já marcados `is_protected` nunca são enviados (já resolvidos pelo elo anterior).
4. `evaluator_version = "<provider>:<model>:prompt-v<N>"`; mudar o prompt incrementa `N`.

## Prompt de sistema (resumo, versionado em `cleaner-infra/src/ai/prompt.rs`)

Papel: especialista em macOS que avalia o risco de excluir itens para liberar espaço. Deve ser
conservador: na dúvida, `Caution` ou `Unknown`; dados pessoais insubstituíveis (fotos, documentos,
bibliotecas de apps, bancos de dados) são `Dangerous`; caches e artefatos recriáveis são `Safe`.
Explicações curtas em pt-BR, dizendo o que o item é e a consequência de excluí-lo.
