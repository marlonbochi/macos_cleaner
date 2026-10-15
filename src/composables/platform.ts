import { computed, type ComputedRef } from "vue";
import { useI18n } from "vue-i18n";
import { useScanStore } from "@/stores/scan";
import type { PlatformDto } from "@/types/generated/PlatformDto";

/** The system the app runs on, as reported by the backend (macOS until it is known). */
export function usePlatform(): ComputedRef<PlatformDto> {
  const scan = useScanStore();
  return computed(() => scan.defaults?.platform ?? "macos");
}

const TERMS = [
  "inFileManager",
  "fileManagerButton",
  "toTrash",
  "emptyTrash",
  "inKeychain",
  "fromKeychain",
  "keychainName",
  "inTerminal",
] as const;

export type SystemTerms = Record<(typeof TERMS)[number], string>;

/** Localised names of the system's file manager, Trash, keychain and terminal, ready to be
 * interpolated into messages (e.g. `$t("common.revealInFinder", terms)`). */
export function useSystemTerms(): ComputedRef<SystemTerms> {
  const platform = usePlatform();
  const { t } = useI18n();
  return computed(
    () =>
      Object.fromEntries(
        TERMS.map((term) => [term, t(`system.${platform.value}.${term}`)]),
      ) as SystemTerms,
  );
}
