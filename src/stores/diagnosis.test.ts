import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { provideApi } from "@/services/ipc";
import { diagnosisItem } from "@/test/factories";
import { FakeTransport } from "@/test/fakeTransport";
import { useAnalysisStore } from "./analysis";
import { useDiagnosisStore } from "./diagnosis";

const GB = 1_000_000_000;

function setup() {
  const transport = new FakeTransport()
    .on("get_ai_diagnosis", () => [
      diagnosisItem({ id: 1, sizeOnDisk: 50 * GB }, null),
      diagnosisItem(
        { id: 2, sizeOnDisk: 20 * GB },
        { level: "Safe", source: { kind: "heuristic" } },
      ),
    ])
    .on("run_ai_diagnosis", () => [
      diagnosisItem({ id: 1, sizeOnDisk: 50 * GB }, { level: "Caution" }),
      diagnosisItem({ id: 2, sizeOnDisk: 20 * GB }, { level: "Safe" }),
      diagnosisItem(
        { id: 3, sizeOnDisk: 10 * GB },
        { level: "Safe", source: { kind: "heuristic" }, aiError: "rate limited" },
      ),
    ]);
  provideApi(transport);
  return { transport, store: useDiagnosisStore() };
}

describe("diagnosis store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("loads stored analyses and counts items the AI has not assessed", async () => {
    const { transport, store } = setup();
    await store.load(7);

    expect(transport.lastCall("get_ai_diagnosis")).toEqual({ scanId: 7 });
    expect(store.items).toHaveLength(2);
    expect(store.pending).toBe(2);
    expect(store.maxSize).toBe(50 * GB);
    expect(useAnalysisStore().byFileId[2]?.level).toBe("Safe");
  });

  it("runs the AI, totals the space per risk level and reports fallbacks", async () => {
    const { transport, store } = setup();
    await store.run(7);

    expect(transport.lastCall("run_ai_diagnosis")).toEqual({ scanId: 7, force: false });
    expect(store.totals.Safe).toEqual({ count: 2, bytes: 30 * GB });
    expect(store.totals.Caution).toEqual({ count: 1, bytes: 50 * GB });
    expect(store.totals.Dangerous.count).toBe(0);
    expect(store.fallbacks).toHaveLength(1);
    expect(store.running).toBe(false);
    expect(useAnalysisStore().byFileId[1]?.level).toBe("Caution");
  });

  it("does not start a second run for the same scan", async () => {
    const { transport, store } = setup();
    const first = store.run(7);
    expect(store.running).toBe(true);
    await store.run(7);
    await first;
    expect(transport.calls.filter((c) => c.command === "run_ai_diagnosis")).toHaveLength(1);
  });

  it("shows errors from the backend", async () => {
    const { transport, store } = setup();
    transport.on("run_ai_diagnosis", () => {
      throw { code: "ai", message: "AI provider not configured" };
    });
    await store.run(7, true);
    expect(store.error).toContain("not configured");
    expect(store.running).toBe(false);
  });
});
