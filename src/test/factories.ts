import type { AiProviderInfoDto } from "@/types/generated/AiProviderInfoDto";
import type { AiSettingsDto } from "@/types/generated/AiSettingsDto";
import type { AnalysisDto } from "@/types/generated/AnalysisDto";
import type { DiagnosisItemDto } from "@/types/generated/DiagnosisItemDto";
import type { FileEntryDto } from "@/types/generated/FileEntryDto";
import type { PageDto } from "@/types/generated/PageDto";
import type { ScanSummaryDto } from "@/types/generated/ScanSummaryDto";

export function scanSummary(overrides: Partial<ScanSummaryDto> = {}): ScanSummaryDto {
  return {
    id: 1,
    roots: ["/Users/alice"],
    status: "Completed",
    minFileSize: 100_000_000,
    startedAt: 1_700_000_000,
    finishedAt: 1_700_000_060,
    filesSeen: 1000,
    bytesSeen: 50_000_000_000,
    inaccessibleCount: 0,
    error: null,
    incomplete: false,
    ...overrides,
  };
}

export function fileEntry(overrides: Partial<FileEntryDto> = {}): FileEntryDto {
  const path = overrides.path ?? "/Users/alice/Movies/video.mov";
  return {
    id: 1,
    path,
    name: path.split("/").at(-1) ?? path,
    kind: "File",
    sizeOnDisk: 5_000_000_000,
    apparentSize: 5_000_000_000,
    fileCount: 1,
    modifiedAt: 1_700_000_000,
    accessedAt: null,
    isProtected: false,
    ...overrides,
  };
}

export function page<T>(items: T[], total = items.length, offset = 0): PageDto<T> {
  return { items, total, offset, limit: 100 };
}

export function analysis(overrides: Partial<AnalysisDto> = {}): AnalysisDto {
  return {
    fileId: 1,
    level: "Caution",
    category: "vm-image",
    summary: "Simuladores do Xcode",
    explanation: "Use as ferramentas do Xcode.",
    recommendation: "use-app-or-command",
    steps: ["Xcode › Settings › Platforms"],
    commands: [{ command: "xcrun simctl delete unavailable", description: "Remove simuladores" }],
    source: { kind: "ai", provider: "anthropic", model: "claude-opus-5-5" },
    createdAt: 1_760_000_000,
    aiError: null,
    ...overrides,
  };
}

export function aiSettings(overrides: Partial<AiSettingsDto> = {}): AiSettingsDto {
  const provider = (
    id: AiProviderInfoDto["id"],
    label: string,
    defaultModel: string,
    needsApiKey = true,
    hasApiKey = false,
  ): AiProviderInfoDto => ({
    id,
    label,
    defaultModel,
    defaultBaseUrl: `https://${id}.example`,
    needsApiKey,
    hasApiKey,
  });
  return {
    provider: null,
    model: null,
    baseUrl: null,
    providers: [
      provider("anthropic", "Claude (Anthropic)", "claude-opus-5-5"),
      provider("openai", "OpenAI", "gpt-6-astra"),
      provider("gemini", "Gemini (Google)", "gemini-3.8-flash"),
      provider("ollama", "Ollama (local)", "llama3.2", false),
    ],
    secretStoreAvailable: true,
    ...overrides,
  };
}

/** Settings with Claude selected and its key saved: AI features are enabled. */
export function aiSettingsWithClaude(): AiSettingsDto {
  const settings = aiSettings({ provider: "anthropic", model: "claude-opus-5-5" });
  return {
    ...settings,
    providers: settings.providers.map((p) =>
      p.id === "anthropic" ? { ...p, hasApiKey: true } : p,
    ),
  };
}

export function diagnosisItem(
  file: Partial<FileEntryDto>,
  result: Partial<AnalysisDto> | null = {},
): DiagnosisItemDto {
  const entry = fileEntry(file);
  return { file: entry, analysis: result ? analysis({ fileId: entry.id, ...result }) : null };
}
