import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { flushPromises } from "@vue/test-utils";
import { effectScope } from "vue";
import { provideApi } from "@/services/ipc";
import { useAiSettingsStore } from "@/stores/aiSettings";
import { useScanStore } from "@/stores/scan";
import { aiSettings, aiSettingsWithClaude, scanSummary } from "@/test/factories";
import { FakeTransport } from "@/test/fakeTransport";
import { useAutoDiagnosis } from "./autoDiagnosis";

function setup(withAi: boolean) {
  const transport = new FakeTransport()
    .on("get_ai_settings", () => (withAi ? aiSettingsWithClaude() : aiSettings()))
    .on("run_ai_diagnosis", () => []);
  provideApi(transport);
  effectScope().run(useAutoDiagnosis);
  return { transport, scan: useScanStore(), ai: useAiSettingsStore() };
}

const runs = (t: FakeTransport) => t.calls.filter((c) => c.command === "run_ai_diagnosis");

describe("auto diagnosis", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("runs once when a scan completes and AI is configured", async () => {
    const { transport, scan, ai } = setup(true);
    await ai.loadSaved();

    scan.handleEvent({ type: "started", summary: scanSummary({ id: 4, status: "Running" }) });
    scan.handleEvent({ type: "finished", summary: scanSummary({ id: 4 }) });
    await flushPromises();

    expect(runs(transport)).toEqual([
      { command: "run_ai_diagnosis", args: { scanId: 4, force: false } },
    ]);
  });

  it("does nothing without AI", async () => {
    const { transport, scan, ai } = setup(false);
    await ai.loadSaved();
    scan.handleEvent({ type: "started", summary: scanSummary({ id: 4, status: "Running" }) });
    scan.handleEvent({ type: "finished", summary: scanSummary({ id: 4 }) });
    await flushPromises();
    expect(runs(transport)).toHaveLength(0);
  });

  it("does nothing for cancelled scans or when the app opens", async () => {
    const { transport, scan, ai } = setup(true);
    await ai.loadSaved();
    scan.current = scanSummary({ id: 5 });
    scan.handleEvent({ type: "started", summary: scanSummary({ id: 6, status: "Running" }) });
    scan.handleEvent({ type: "finished", summary: scanSummary({ id: 6, status: "Cancelled" }) });
    await flushPromises();
    expect(runs(transport)).toHaveLength(0);
  });
});
