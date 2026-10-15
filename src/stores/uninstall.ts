import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { IpcError, useApi } from "@/services/ipc";
import type { AppFootprintDto } from "@/types/generated/AppFootprintDto";
import type { UninstallReportDto } from "@/types/generated/UninstallReportDto";

/** Complete uninstall of the app (feature 009). */
export const useUninstallStore = defineStore("uninstall", () => {
  const footprint = ref<AppFootprintDto | null>(null);
  const loading = ref(false);
  const running = ref(false);
  const report = ref<UninstallReportDto | null>(null);
  const error = ref<string | null>(null);

  const totalSize = computed(
    () => footprint.value?.items.reduce((sum, item) => sum + item.size, 0) ?? 0,
  );

  /** Whether the typed text unlocks the uninstall (same rule as the backend). */
  function confirms(typed: string): boolean {
    return !!footprint.value && typed.trim() === footprint.value.confirmationPhrase;
  }

  async function load(): Promise<void> {
    loading.value = true;
    error.value = null;
    report.value = null;
    try {
      footprint.value = await useApi().uninstall.footprint();
    } catch (e) {
      error.value = IpcError.from(e).message;
    } finally {
      loading.value = false;
    }
  }

  async function uninstall(typed: string): Promise<void> {
    if (!confirms(typed) || running.value) return;
    running.value = true;
    error.value = null;
    try {
      report.value = await useApi().uninstall.uninstall(typed.trim());
    } catch (e) {
      error.value = IpcError.from(e).message;
    } finally {
      running.value = false;
    }
  }

  function reset(): void {
    footprint.value = null;
    report.value = null;
    error.value = null;
  }

  return {
    footprint,
    loading,
    running,
    report,
    error,
    totalSize,
    confirms,
    load,
    uninstall,
    reset,
  };
});
