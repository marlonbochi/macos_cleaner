import type { AiModelDto } from "@/types/generated/AiModelDto";
import type { AiProviderIdDto } from "@/types/generated/AiProviderIdDto";
import type { AiSettingsDto } from "@/types/generated/AiSettingsDto";
import type { AppFootprintDto } from "@/types/generated/AppFootprintDto";
import type { AnalysisDto } from "@/types/generated/AnalysisDto";
import type { ConnectionTestDto } from "@/types/generated/ConnectionTestDto";
import type { DiagnosisItemDto } from "@/types/generated/DiagnosisItemDto";
import type { FileEntryDto } from "@/types/generated/FileEntryDto";
import type { FileFilterDto } from "@/types/generated/FileFilterDto";
import type { FileSortDto } from "@/types/generated/FileSortDto";
import type { PageDto } from "@/types/generated/PageDto";
import type { PermissionsStatusDto } from "@/types/generated/PermissionsStatusDto";
import type { PreferencesDto } from "@/types/generated/PreferencesDto";
import type { ScanDefaultsDto } from "@/types/generated/ScanDefaultsDto";
import type { ScanEvent } from "@/types/generated/ScanEvent";
import type { SaveAiSettingsInput } from "@/types/generated/SaveAiSettingsInput";
import type { ScanSummaryDto } from "@/types/generated/ScanSummaryDto";
import type { UninstallReportDto } from "@/types/generated/UninstallReportDto";
import type { Transport } from "./transport";

/** Typed wrappers for the scan commands (contract: tauri-commands.md, US1). */
export class ScanApi {
  constructor(private readonly transport: Transport) {}

  defaults(): Promise<ScanDefaultsDto> {
    return this.transport.invoke("get_scan_defaults");
  }

  start(
    roots: string[],
    minFileSize: number | null,
    onEvent: (event: ScanEvent) => void,
  ): Promise<ScanSummaryDto> {
    return this.transport.invoke("start_scan", {
      roots,
      minFileSize,
      onProgress: this.transport.channel(onEvent),
    });
  }

  cancel(scanId: number): Promise<void> {
    return this.transport.invoke("cancel_scan", { scanId });
  }

  latest(): Promise<ScanSummaryDto | null> {
    return this.transport.invoke("get_latest_scan");
  }
}

export interface ListFilesParams {
  scanId: number;
  offset: number;
  limit: number;
  filter?: Partial<FileFilterDto>;
  sort?: FileSortDto;
}

export class CatalogApi {
  constructor(private readonly transport: Transport) {}

  listFiles({
    scanId,
    offset,
    limit,
    filter,
    sort,
  }: ListFilesParams): Promise<PageDto<FileEntryDto>> {
    return this.transport.invoke("list_files", {
      scanId,
      offset,
      limit,
      filter: filter ?? null,
      sort: sort ?? null,
    });
  }

  listFolderChildren(
    scanId: number,
    path: string,
    offset: number,
    limit: number,
  ): Promise<PageDto<FileEntryDto>> {
    return this.transport.invoke("list_folder_children", { scanId, path, offset, limit });
  }
}

export class SystemApi {
  constructor(private readonly transport: Transport) {}

  revealInFinder(path: string): Promise<void> {
    return this.transport.invoke("reveal_in_finder", { path });
  }

  permissions(): Promise<PermissionsStatusDto> {
    return this.transport.invoke("get_permissions_status");
  }

  openFullDiskAccessSettings(): Promise<void> {
    return this.transport.invoke("open_full_disk_access_settings");
  }
}

/** AI-assisted risk analysis of catalogued items (US2). */
export class AnalysisApi {
  constructor(private readonly transport: Transport) {}

  analyze(fileId: number, force = false): Promise<AnalysisDto> {
    return this.transport.invoke("analyze_file", { fileId, force });
  }

  /** Analyses many items; the backend groups them into as few AI requests as possible. */
  analyzeMany(fileIds: number[], force = false): Promise<AnalysisDto[]> {
    return this.transport.invoke("analyze_files", { fileIds, force });
  }

  /** Stored analyses still valid for these items (used for risk badges). */
  list(fileIds: number[]): Promise<AnalysisDto[]> {
    return this.transport.invoke("get_analyses", { fileIds });
  }
}

/** AI diagnosis of the largest items of a scan (feature 008). */
export class DiagnosisApi {
  constructor(private readonly transport: Transport) {}

  /** The diagnosed items with their stored analyses; never calls the AI. */
  get(scanId: number): Promise<DiagnosisItemDto[]> {
    return this.transport.invoke("get_ai_diagnosis", { scanId });
  }

  /** Sends the items to the AI; unchanged items come from the cache unless `force`. */
  run(scanId: number, force = false): Promise<DiagnosisItemDto[]> {
    return this.transport.invoke("run_ai_diagnosis", { scanId, force });
  }
}

export class SettingsApi {
  constructor(private readonly transport: Transport) {}

  ai(): Promise<AiSettingsDto> {
    return this.transport.invoke("get_ai_settings");
  }

  saveAi(input: SaveAiSettingsInput): Promise<AiSettingsDto> {
    return this.transport.invoke("save_ai_settings", { input });
  }

  /** Models the typed (or stored) key can use. The typed key is not saved. */
  models(
    provider: AiProviderIdDto,
    baseUrl: string | null,
    apiKey: string | null,
  ): Promise<AiModelDto[]> {
    return this.transport.invoke("list_ai_models", { provider, baseUrl, apiKey });
  }

  testAi(): Promise<ConnectionTestDto> {
    return this.transport.invoke("test_ai_connection");
  }
}

export class PreferencesApi {
  constructor(private readonly transport: Transport) {}

  get(): Promise<PreferencesDto> {
    return this.transport.invoke("get_preferences");
  }

  setLocale(locale: string): Promise<void> {
    return this.transport.invoke("set_locale", { locale });
  }
}

/** Complete uninstall of the app (feature 009). */
export class UninstallApi {
  constructor(private readonly transport: Transport) {}

  /** What would be removed, with sizes. Changes nothing. */
  footprint(): Promise<AppFootprintDto> {
    return this.transport.invoke("get_app_footprint");
  }

  /** Needs the exact confirmation phrase; the app quits afterwards. */
  uninstall(confirmation: string): Promise<UninstallReportDto> {
    return this.transport.invoke("uninstall_app", { confirmation });
  }
}

/** Facade grouping every backend API. */
export class Api {
  readonly scan: ScanApi;
  readonly catalog: CatalogApi;
  readonly system: SystemApi;
  readonly analysis: AnalysisApi;
  readonly diagnosis: DiagnosisApi;
  readonly settings: SettingsApi;
  readonly preferences: PreferencesApi;
  readonly uninstall: UninstallApi;

  constructor(transport: Transport) {
    this.scan = new ScanApi(transport);
    this.catalog = new CatalogApi(transport);
    this.system = new SystemApi(transport);
    this.analysis = new AnalysisApi(transport);
    this.diagnosis = new DiagnosisApi(transport);
    this.settings = new SettingsApi(transport);
    this.preferences = new PreferencesApi(transport);
    this.uninstall = new UninstallApi(transport);
  }
}
