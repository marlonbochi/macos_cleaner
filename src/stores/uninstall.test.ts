import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { provideApi } from "@/services/ipc";
import { FakeTransport } from "@/test/fakeTransport";
import { useUninstallStore } from "./uninstall";

function setup() {
  const transport = new FakeTransport()
    .on("get_app_footprint", () => ({
      items: [
        { kind: "appData", path: "/a", size: 100 },
        { kind: "cache", path: "/b", size: 20 },
      ],
      storedKeys: [],
      bundleFound: true,
      confirmationPhrase: "macOS Cleaner",
    }))
    .on("uninstall_app", () => ({
      trashed: ["/a", "/b"],
      keysRemoved: 0,
      failures: [],
      quitting: true,
    }));
  provideApi(transport);
  return { transport, store: useUninstallStore() };
}

describe("uninstall store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("totals the footprint and requires the exact phrase", async () => {
    const { transport, store } = setup();
    expect(store.confirms("macOS Cleaner")).toBe(false); // nothing loaded yet

    await store.load();
    expect(store.totalSize).toBe(120);
    expect(store.confirms(" macOS Cleaner ")).toBe(true);
    expect(store.confirms("MACOS CLEANER")).toBe(false);

    await store.uninstall("nope");
    expect(transport.lastCall("uninstall_app")).toBeUndefined();

    await store.uninstall(" macOS Cleaner ");
    expect(transport.lastCall("uninstall_app")).toEqual({ confirmation: "macOS Cleaner" });
    expect(store.report?.quitting).toBe(true);

    store.reset();
    expect(store.footprint).toBeNull();
  });
});
