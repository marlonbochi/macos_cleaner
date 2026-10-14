import { watch } from "vue";
import { useAiSettingsStore } from "@/stores/aiSettings";
import { useDiagnosisStore } from "@/stores/diagnosis";
import { useScanStore } from "@/stores/scan";

/**
 * Runs the AI diagnosis as soon as a scan completes, when an AI provider is configured.
 * Only a running → completed transition triggers it, so opening the app never spends tokens.
 */
export function useAutoDiagnosis(): void {
  const scan = useScanStore();
  const ai = useAiSettingsStore();
  const diagnosis = useDiagnosisStore();

  watch(
    () => [scan.current?.id, scan.current?.status] as const,
    ([id, status], [previousId, previousStatus]) => {
      const justCompleted =
        id !== undefined &&
        id === previousId &&
        previousStatus === "Running" &&
        status === "Completed";
      if (justCompleted && ai.aiEnabled) void diagnosis.run(id);
    },
    // Synchronous: a fast scan can go running → completed within a single tick.
    { flush: "sync" },
  );
}
