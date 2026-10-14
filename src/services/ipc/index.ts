import { Api } from "./api";
import { TauriTransport, type Transport } from "./transport";

export {
  AnalysisApi,
  Api,
  CatalogApi,
  PreferencesApi,
  ScanApi,
  SettingsApi,
  SystemApi,
} from "./api";
export type { ListFilesParams } from "./api";
export { IpcError, TauriTransport } from "./transport";
export type { Transport } from "./transport";

let current: Api | null = null;

/** Returns the API used by stores. Lazily backed by Tauri IPC. */
export function useApi(): Api {
  current ??= new Api(new TauriTransport());
  return current;
}

/** Replaces the API (tests inject a fake transport through this). */
export function provideApi(transportOrApi: Transport | Api): Api {
  current = transportOrApi instanceof Api ? transportOrApi : new Api(transportOrApi);
  return current;
}
