import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { IpcError, useApi } from "@/services/ipc";
import type { DiagnosisItemDto } from "@/types/generated/DiagnosisItemDto";
import type { RiskLevelDto } from "@/types/generated/RiskLevelDto";
import { useAnalysisStore } from "./analysis";

export interface LevelTotal {
  count: number;
  bytes: number;
}

const LEVELS: RiskLevelDto[] = ["Safe", "Caution", "Dangerous", "Unknown"];

/** AI diagnosis of the largest items of the last scan (feature 008). */
export const useDiagnosisStore = defineStore("diagnosis", () => {
  const scanId = ref<number | null>(null);
  const items = ref<DiagnosisItemDto[]>([]);
  const loading = ref(false);
  /** Scan whose diagnosis is being run by the AI, if any. */
  const runningFor = ref<number | null>(null);
  const error = ref<string | null>(null);

  const running = computed(() => runningFor.value !== null && runningFor.value === scanId.value);
  /** Items the AI has not assessed yet (no analysis, or only the local rules'). */
  const pending = computed(
    () => items.value.filter((i) => !i.analysis || i.analysis.source.kind === "heuristic").length,
  );
  /** Items where the AI was asked but local rules answered instead. */
  const fallbacks = computed(() => items.value.filter((i) => i.analysis?.aiError));
  const totals = computed(() => {
    const out = Object.fromEntries(
      LEVELS.map((level) => [level, { count: 0, bytes: 0 }]),
    ) as Record<RiskLevelDto, LevelTotal>;
    for (const { file, analysis } of items.value) {
      if (!analysis) continue;
      out[analysis.level].count += 1;
      out[analysis.level].bytes += file.sizeOnDisk;
    }
    return out;
  });
  const maxSize = computed(() => items.value.reduce((m, i) => Math.max(m, i.file.sizeOnDisk), 0));

  /** Keeps the shared analysis cache in sync so badges and the analysis panel agree. */
  function show(scan: number, list: DiagnosisItemDto[]): void {
    if (scan !== scanId.value) return;
    items.value = list;
    const analyses = useAnalysisStore();
    for (const { analysis } of list) {
      if (analysis) analyses.byFileId[analysis.fileId] = analysis;
    }
  }

  async function load(scan: number): Promise<void> {
    if (scan !== scanId.value) items.value = [];
    scanId.value = scan;
    loading.value = true;
    error.value = null;
    try {
      show(scan, await useApi().diagnosis.get(scan));
    } catch (e) {
      error.value = IpcError.from(e).message;
    } finally {
      loading.value = false;
    }
  }

  /** Asks the AI about the largest items; ignored while a run for the same scan is going on. */
  async function run(scan: number, force = false): Promise<void> {
    if (runningFor.value === scan) return;
    if (scan !== scanId.value) {
      scanId.value = scan;
      items.value = [];
    }
    runningFor.value = scan;
    error.value = null;
    try {
      show(scan, await useApi().diagnosis.run(scan, force));
    } catch (e) {
      if (scan === scanId.value) error.value = IpcError.from(e).message;
    } finally {
      if (runningFor.value === scan) runningFor.value = null;
    }
  }

  return {
    scanId,
    items,
    loading,
    running,
    runningFor,
    error,
    pending,
    fallbacks,
    totals,
    maxSize,
    load,
    run,
  };
});
