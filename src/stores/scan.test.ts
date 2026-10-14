import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { provideApi } from "@/services/ipc";
import { FakeTransport } from "@/test/fakeTransport";
import { scanSummary } from "@/test/factories";
import { useScanStore } from "./scan";

function setup(transport = new FakeTransport()) {
  transport
    .on("get_scan_defaults", () => ({
      roots: ["/Users/alice", "/Applications"],
      home: "/Users/alice",
      minFileSize: 100_000_000,
    }))
    .on("get_latest_scan", () => null)
    .on("get_permissions_status", () => ({ fullDiskAccess: false }));
  provideApi(transport);
  setActivePinia(createPinia());
  return { transport, store: useScanStore() };
}

describe("scan store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("loads defaults, permissions and the latest scan on init", async () => {
    const { transport, store } = setup();
    transport.on("get_latest_scan", () => scanSummary({ id: 7, minFileSize: 50_000_000 }));

    await store.init();

    expect(store.roots).toEqual(["/Users/alice", "/Applications"]);
    expect(store.permissions).toEqual({ fullDiskAccess: false });
    expect(store.current?.id).toBe(7);
    expect(store.minFileSize).toBe(50_000_000);
    expect(store.hasCatalog).toBe(true);
  });

  it("starts a scan with the chosen roots and threshold and tracks streamed progress", async () => {
    const { transport, store } = setup();
    await store.init();
    store.removeRoot("/Applications");
    store.addRoot("/Volumes/USB");
    store.addRoot("/Volumes/USB");
    transport.on("start_scan", () => scanSummary({ id: 2, status: "Running" }));

    await store.start();

    expect(transport.lastCall("start_scan")).toMatchObject({
      roots: ["/Users/alice", "/Volumes/USB"],
      minFileSize: 100_000_000,
    });
    expect(store.isRunning).toBe(true);

    transport.emit({
      type: "progress",
      filesSeen: 10,
      bytesSeen: 2000,
      inaccessibleCount: 1,
      currentPath: "/Users/alice/x",
    });
    transport.emit({ type: "inaccessible", path: "/Users/alice/locked" });
    expect(store.progress).toMatchObject({ filesSeen: 10, bytesSeen: 2000 });
    expect(store.inaccessible).toEqual(["/Users/alice/locked"]);

    transport.emit({ type: "finished", summary: scanSummary({ id: 2, filesSeen: 11 }) });
    expect(store.isRunning).toBe(false);
    expect(store.current?.status).toBe("Completed");
    expect(store.progress?.filesSeen).toBe(11);
  });

  it("does not overwrite a summary that already finished through the channel", async () => {
    const { transport, store } = setup();
    await store.init();
    transport.on("start_scan", () => {
      transport.emit({ type: "finished", summary: scanSummary({ id: 3 }) });
      return scanSummary({ id: 3, status: "Running" });
    });

    await store.start();

    expect(store.current?.status).toBe("Completed");
  });

  it("cancels the running scan", async () => {
    const { transport, store } = setup();
    await store.init();
    transport.on("start_scan", () => scanSummary({ id: 4, status: "Running" }));
    transport.on("cancel_scan", () => undefined);
    await store.start();

    await store.cancel();

    expect(transport.lastCall("cancel_scan")).toEqual({ scanId: 4 });
  });

  it("surfaces backend errors", async () => {
    const { transport, store } = setup();
    await store.init();
    transport.on("start_scan", () => {
      throw { code: "SCAN_IN_PROGRESS", message: "a scan is already running" };
    });

    await store.start();

    expect(store.error).toBe("a scan is already running");
  });

  it("marks the scan failed when persisting fails", async () => {
    const { transport, store } = setup();
    await store.init();
    transport.on("start_scan", () => scanSummary({ id: 5, status: "Running" }));
    await store.start();

    store.handleEvent({ type: "error", message: "disk full" });

    expect(store.error).toBe("disk full");
    expect(store.current?.status).toBe("Failed");
  });

  it("does nothing without roots", async () => {
    const { transport, store } = setup();
    await store.init();
    store.roots = [];
    await store.start();
    expect(transport.lastCall("start_scan")).toBeUndefined();
  });

  it("refreshes permissions and opens settings", async () => {
    const { transport, store } = setup();
    transport.on("open_full_disk_access_settings", () => undefined);
    await store.openFullDiskAccessSettings();
    transport.on("get_permissions_status", () => ({ fullDiskAccess: true }));
    await store.refreshPermissions();
    expect(store.permissions?.fullDiskAccess).toBe(true);
    expect(transport.lastCall("open_full_disk_access_settings")).toEqual({});
  });

  it("reveals the app that needs Full Disk Access and reports failures", async () => {
    const { transport, store } = setup();
    transport.on("reveal_in_finder", () => undefined);
    await store.revealInFinder("/Applications/iTerm.app");
    expect(transport.lastCall("reveal_in_finder")).toEqual({ path: "/Applications/iTerm.app" });

    transport.on("reveal_in_finder", () => {
      throw { code: "NOT_FOUND", message: "gone" };
    });
    await store.revealInFinder("/x.app");
    expect(store.error).toBe("gone");
  });
});
