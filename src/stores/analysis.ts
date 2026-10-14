import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { IpcError, useApi } from "@/services/ipc";
import type { AnalysisDto } from "@/types/generated/AnalysisDto";
import type { FileEntryDto } from "@/types/generated/FileEntryDto";

export const useAnalysisStore = defineStore("analysis", () => {
  /** Latest known analysis per catalogue item id. */
  const byFileId = ref<Record<number, AnalysisDto>>({});
  const analyzing = ref<number[]>([]);
  /** Item shown in the analysis panel. */
  const selected = ref<FileEntryDto | null>(null);
  const error = ref<string | null>(null);
  /** Summary of the last multi-item analysis, shown above the table. */
  const lastBatch = ref<{ count: number; fallbacks: number; aiError: string | null } | null>(null);

  const current = computed(() =>
    selected.value ? (byFileId.value[selected.value.id] ?? null) : null,
  );
  const isAnalyzing = (id: number) => analyzing.value.includes(id);

  /** Loads stored analyses for the visible rows (risk badges). */
  async function loadFor(ids: number[]): Promise<void> {
    if (ids.length === 0) return;
    try {
      const found = await useApi().analysis.list(ids);
      for (const analysis of found) byFileId.value[analysis.fileId] = analysis;
    } catch (e) {
      error.value = IpcError.from(e).message;
    }
  }

  async function analyze(item: FileEntryDto, force = false): Promise<void> {
    if (isAnalyzing(item.id)) return;
    error.value = null;
    analyzing.value.push(item.id);
    try {
      byFileId.value[item.id] = await useApi().analysis.analyze(item.id, force);
    } catch (e) {
      error.value = IpcError.from(e).message;
    } finally {
      analyzing.value = analyzing.value.filter((id) => id !== item.id);
    }
  }

  /** Analyses many items at once (one AI request per batch of items on the backend). */
  async function analyzeMany(ids: number[], force = false): Promise<void> {
    const todo = ids.filter((id) => !isAnalyzing(id));
    if (todo.length === 0) return;
    error.value = null;
    lastBatch.value = null;
    analyzing.value = [...analyzing.value, ...todo];
    try {
      const results = await useApi().analysis.analyzeMany(todo, force);
      for (const analysis of results) byFileId.value[analysis.fileId] = analysis;
      const failed = results.filter((r) => r.aiError);
      lastBatch.value = {
        count: results.length,
        fallbacks: failed.length,
        aiError: failed[0]?.aiError ?? null,
      };
    } catch (e) {
      error.value = IpcError.from(e).message;
    } finally {
      analyzing.value = analyzing.value.filter((id) => !todo.includes(id));
    }
  }

  /** Opens the panel for an item and analyses it if there is no analysis yet. */
  async function open(item: FileEntryDto): Promise<void> {
    selected.value = item;
    if (!byFileId.value[item.id]) await analyze(item);
  }

  function close(): void {
    selected.value = null;
    error.value = null;
  }

  return {
    byFileId,
    analyzing,
    selected,
    error,
    current,
    isAnalyzing,
    lastBatch,
    loadFor,
    analyze,
    analyzeMany,
    open,
    close,
  };
});
