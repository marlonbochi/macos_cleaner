import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { provideApi } from "@/services/ipc";
import { analysis, fileEntry } from "@/test/factories";
import { FakeTransport } from "@/test/fakeTransport";
import { useAnalysisStore } from "./analysis";

function setup() {
  const transport = new FakeTransport()
    .on("analyze_file", (args) => analysis({ fileId: args.fileId as number }))
    .on("get_analyses", () => [analysis({ fileId: 2, level: "Safe" })])
    .on("analyze_files", (args) =>
      (args.fileIds as number[]).map((fileId) =>
        analysis({ fileId, aiError: fileId === 3 ? "rate limited" : null }),
      ),
    );
  provideApi(transport);
  setActivePinia(createPinia());
  return { transport, store: useAnalysisStore() };
}

describe("analysis store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("loads stored analyses for visible rows", async () => {
    const { transport, store } = setup();
    await store.loadFor([1, 2]);
    expect(transport.lastCall("get_analyses")).toEqual({ fileIds: [1, 2] });
    expect(store.byFileId[2]?.level).toBe("Safe");

    await store.loadFor([]);
    expect(transport.calls.filter((c) => c.command === "get_analyses")).toHaveLength(1);
  });

  it("opens the panel and analyses only when there is no analysis yet", async () => {
    const { transport, store } = setup();
    await store.open(fileEntry({ id: 1 }));
    expect(transport.lastCall("analyze_file")).toEqual({ fileId: 1, force: false });
    expect(store.current?.summary).toBe("Simuladores do Xcode");

    await store.open(fileEntry({ id: 1 }));
    expect(transport.calls.filter((c) => c.command === "analyze_file")).toHaveLength(1);

    await store.analyze(fileEntry({ id: 1 }), true);
    expect(transport.lastCall("analyze_file")).toEqual({ fileId: 1, force: true });
    store.close();
    expect(store.selected).toBeNull();
    expect(store.current).toBeNull();
  });

  it("tracks in-flight analyses and ignores duplicate clicks", async () => {
    const { transport, store } = setup();
    let release: () => void = () => undefined;
    transport.on(
      "analyze_file",
      () => new Promise((resolve) => (release = () => resolve(analysis()))),
    );
    const item = fileEntry({ id: 1 });

    const first = store.analyze(item);
    expect(store.isAnalyzing(1)).toBe(true);
    await store.analyze(item);
    release();
    await first;

    expect(store.isAnalyzing(1)).toBe(false);
    expect(transport.calls.filter((c) => c.command === "analyze_file")).toHaveLength(1);
  });

  it("reports errors", async () => {
    const { transport, store } = setup();
    transport.on("analyze_file", () => {
      throw { code: "NOT_FOUND", message: "file 9" };
    });
    await store.analyze(fileEntry({ id: 9 }));
    expect(store.error).toBe("file 9");

    transport.on("get_analyses", () => {
      throw new Error("db");
    });
    await store.loadFor([1]);
    expect(store.error).toBe("db");
  });

  it("analyses many items in one call and summarises fallbacks", async () => {
    const { transport, store } = setup();

    await store.analyzeMany([1, 2, 3]);

    expect(transport.lastCall("analyze_files")).toEqual({ fileIds: [1, 2, 3], force: false });
    expect(Object.keys(store.byFileId)).toEqual(["1", "2", "3"]);
    expect(store.lastBatch).toEqual({ count: 3, fallbacks: 1, aiError: "rate limited" });
    expect(store.analyzing).toEqual([]);
  });

  it("skips items already being analysed and reports batch errors", async () => {
    const { transport, store } = setup();
    store.analyzing.push(1);
    await store.analyzeMany([1]);
    expect(transport.lastCall("analyze_files")).toBeUndefined();

    transport.on("analyze_files", () => {
      throw { code: "INVALID_INPUT", message: "select at most 200 items" };
    });
    await store.analyzeMany([2]);
    expect(store.error).toBe("select at most 200 items");
    expect(store.analyzing).toEqual([1]);
  });
});
