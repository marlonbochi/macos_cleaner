import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { IpcError, useApi } from "@/services/ipc";
import type { FileEntryDto } from "@/types/generated/FileEntryDto";
import type { FileKindDto } from "@/types/generated/FileKindDto";
import type { FileSortDto } from "@/types/generated/FileSortDto";

export const PAGE_SIZE = 100;

export interface CatalogFilter {
  kinds: FileKindDto[];
  extensions: string[];
  minSize: number | null;
}

const emptyFilter = (): CatalogFilter => ({ kinds: [], extensions: [], minSize: null });

export const useCatalogStore = defineStore("catalog", () => {
  const scanId = ref<number | null>(null);
  const items = ref<FileEntryDto[]>([]);
  const total = ref(0);
  const offset = ref(0);
  const filter = ref<CatalogFilter>(emptyFilter());
  const sort = ref<FileSortDto>("sizeDesc");
  /** When set, shows the direct children of this folder (drill-down). */
  const folder = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  /** Ids chosen with the checkboxes; kept across pages and filters of the same scan. */
  const selected = ref<number[]>([]);

  const hasPrevious = computed(() => offset.value > 0);
  const hasNext = computed(() => offset.value + items.value.length < total.value);
  /** Largest size on the current page, used to scale size bars. */
  const maxSize = computed(() => items.value.reduce((m, e) => Math.max(m, e.sizeOnDisk), 0));

  const isSelected = (id: number) => selected.value.includes(id);

  function toggleSelected(id: number): void {
    selected.value = isSelected(id)
      ? selected.value.filter((s) => s !== id)
      : [...selected.value, id];
  }

  /** Selects or clears the given ids (the header checkbox acts on the current page). */
  function setSelected(ids: number[], value: boolean): void {
    const others = selected.value.filter((id) => !ids.includes(id));
    selected.value = value ? [...others, ...ids] : others;
  }

  function clearSelection(): void {
    selected.value = [];
  }

  let requestSeq = 0;

  async function load(): Promise<void> {
    if (scanId.value === null) return;
    const seq = ++requestSeq;
    loading.value = true;
    error.value = null;
    try {
      const api = useApi();
      const page =
        folder.value === null
          ? await api.catalog.listFiles({
              scanId: scanId.value,
              offset: offset.value,
              limit: PAGE_SIZE,
              filter: {
                kinds: filter.value.kinds,
                extensions: filter.value.extensions,
                minSize: filter.value.minSize,
              },
              sort: sort.value,
            })
          : await api.catalog.listFolderChildren(
              scanId.value,
              folder.value,
              offset.value,
              PAGE_SIZE,
            );
      // Ignore responses that arrive after a newer request (fast typing in filters).
      if (seq !== requestSeq) return;
      items.value = page.items;
      total.value = page.total;
    } catch (e) {
      if (seq === requestSeq) error.value = IpcError.from(e).message;
    } finally {
      if (seq === requestSeq) loading.value = false;
    }
  }

  async function open(id: number): Promise<void> {
    if (scanId.value !== id) {
      scanId.value = id;
      folder.value = null;
      offset.value = 0;
      selected.value = [];
    }
    await load();
  }

  async function setFilter(patch: Partial<CatalogFilter>): Promise<void> {
    filter.value = { ...filter.value, ...patch };
    offset.value = 0;
    await load();
  }

  async function openFolder(path: string | null): Promise<void> {
    folder.value = path;
    offset.value = 0;
    await load();
  }

  async function nextPage(): Promise<void> {
    if (!hasNext.value) return;
    offset.value += PAGE_SIZE;
    await load();
  }

  async function previousPage(): Promise<void> {
    if (!hasPrevious.value) return;
    offset.value = Math.max(0, offset.value - PAGE_SIZE);
    await load();
  }

  async function revealInFinder(path: string): Promise<void> {
    try {
      await useApi().system.revealInFinder(path);
    } catch (e) {
      error.value = IpcError.from(e).message;
    }
  }

  function reset(): void {
    scanId.value = null;
    items.value = [];
    total.value = 0;
    offset.value = 0;
    folder.value = null;
    filter.value = emptyFilter();
    selected.value = [];
  }

  return {
    scanId,
    items,
    total,
    offset,
    filter,
    sort,
    folder,
    loading,
    error,
    selected,
    isSelected,
    toggleSelected,
    setSelected,
    clearSelection,
    hasPrevious,
    hasNext,
    maxSize,
    load,
    open,
    setFilter,
    openFolder,
    nextPage,
    previousPage,
    revealInFinder,
    reset,
  };
});
