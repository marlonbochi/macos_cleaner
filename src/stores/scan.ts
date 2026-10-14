import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { IpcError, useApi } from "@/services/ipc";
import type { PermissionsStatusDto } from "@/types/generated/PermissionsStatusDto";
import type { ScanDefaultsDto } from "@/types/generated/ScanDefaultsDto";
import type { ScanEvent } from "@/types/generated/ScanEvent";
import type { ScanSummaryDto } from "@/types/generated/ScanSummaryDto";

export interface ScanProgressState {
  filesSeen: number;
  bytesSeen: number;
  inaccessibleCount: number;
  currentPath: string;
}

/** Kept in the store so the UI stays responsive even if a disk has thousands of unreadable dirs. */
const MAX_INACCESSIBLE_SHOWN = 200;

export const useScanStore = defineStore("scan", () => {
  const defaults = ref<ScanDefaultsDto | null>(null);
  const roots = ref<string[]>([]);
  const minFileSize = ref(100_000_000);
  /** The running scan, or the last finished one. */
  const current = ref<ScanSummaryDto | null>(null);
  const progress = ref<ScanProgressState | null>(null);
  const inaccessible = ref<string[]>([]);
  const permissions = ref<PermissionsStatusDto | null>(null);
  const error = ref<string | null>(null);

  const isRunning = computed(() => current.value?.status === "Running");
  const hasCatalog = computed(
    () => !!current.value && ["Completed", "Cancelled"].includes(current.value.status),
  );

  async function init(): Promise<void> {
    const api = useApi();
    error.value = null;
    try {
      const [defs, latest, perms] = await Promise.all([
        api.scan.defaults(),
        api.scan.latest(),
        api.system.permissions(),
      ]);
      defaults.value = defs;
      permissions.value = perms;
      if (roots.value.length === 0) roots.value = [...defs.roots];
      minFileSize.value = latest?.minFileSize ?? defs.minFileSize;
      if (!isRunning.value) current.value = latest;
    } catch (e) {
      error.value = IpcError.from(e).message;
    }
  }

  function handleEvent(event: ScanEvent): void {
    switch (event.type) {
      case "started":
        current.value = event.summary;
        break;
      case "progress":
        progress.value = {
          filesSeen: event.filesSeen,
          bytesSeen: event.bytesSeen,
          inaccessibleCount: event.inaccessibleCount,
          currentPath: event.currentPath,
        };
        break;
      case "inaccessible":
        if (inaccessible.value.length < MAX_INACCESSIBLE_SHOWN) {
          inaccessible.value.push(event.path);
        }
        break;
      case "finished":
        current.value = event.summary;
        progress.value = {
          filesSeen: event.summary.filesSeen,
          bytesSeen: event.summary.bytesSeen,
          inaccessibleCount: event.summary.inaccessibleCount,
          currentPath: "",
        };
        break;
      case "error":
        error.value = event.message;
        if (current.value) current.value = { ...current.value, status: "Failed" };
        break;
    }
  }

  async function start(): Promise<void> {
    if (roots.value.length === 0 || isRunning.value) return;
    error.value = null;
    progress.value = null;
    inaccessible.value = [];
    try {
      const summary = await useApi().scan.start(roots.value, minFileSize.value, handleEvent);
      // A fast scan may already have finished through the channel before this resolves.
      if (current.value?.id !== summary.id) current.value = summary;
    } catch (e) {
      error.value = IpcError.from(e).message;
    }
  }

  async function cancel(): Promise<void> {
    if (!current.value || !isRunning.value) return;
    await useApi().scan.cancel(current.value.id);
  }

  function addRoot(path: string): void {
    if (!roots.value.includes(path)) roots.value.push(path);
  }

  function removeRoot(path: string): void {
    roots.value = roots.value.filter((r) => r !== path);
  }

  async function refreshPermissions(): Promise<void> {
    permissions.value = await useApi().system.permissions();
  }

  async function openFullDiskAccessSettings(): Promise<void> {
    await useApi().system.openFullDiskAccessSettings();
  }

  async function revealInFinder(path: string): Promise<void> {
    try {
      await useApi().system.revealInFinder(path);
    } catch (e) {
      error.value = IpcError.from(e).message;
    }
  }

  return {
    defaults,
    roots,
    minFileSize,
    current,
    progress,
    inaccessible,
    permissions,
    error,
    isRunning,
    hasCatalog,
    init,
    handleEvent,
    start,
    cancel,
    addRoot,
    removeRoot,
    refreshPermissions,
    openFullDiskAccessSettings,
    revealInFinder,
  };
});
