import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { provideApi } from "@/services/ipc";
import { FakeTransport } from "@/test/fakeTransport";
import { fileEntry, page } from "@/test/factories";
import { PAGE_SIZE, useCatalogStore } from "./catalog";

function setup() {
  const transport = new FakeTransport()
    .on("list_files", () =>
      page([fileEntry({ id: 1, sizeOnDisk: 10 }), fileEntry({ id: 2, sizeOnDisk: 30 })], 250),
    )
    .on("list_folder_children", () => page([fileEntry({ id: 9 })], 1));
  provideApi(transport);
  setActivePinia(createPinia());
  return { transport, store: useCatalogStore() };
}

describe("catalog store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("loads the first page of the largest items", async () => {
    const { transport, store } = setup();

    await store.open(5);

    expect(transport.lastCall("list_files")).toEqual({
      scanId: 5,
      offset: 0,
      limit: PAGE_SIZE,
      filter: { kinds: [], extensions: [], minSize: null },
      sort: "sizeDesc",
    });
    expect(store.items).toHaveLength(2);
    expect(store.total).toBe(250);
    expect(store.maxSize).toBe(30);
    expect(store.hasNext).toBe(true);
    expect(store.hasPrevious).toBe(false);
  });

  it("paginates forward and back", async () => {
    const { transport, store } = setup();
    await store.open(5);

    await store.nextPage();
    expect(transport.lastCall("list_files")?.offset).toBe(PAGE_SIZE);
    expect(store.hasPrevious).toBe(true);

    await store.previousPage();
    expect(transport.lastCall("list_files")?.offset).toBe(0);
  });

  it("resets to the first page when filters change", async () => {
    const { transport, store } = setup();
    await store.open(5);
    await store.nextPage();

    await store.setFilter({ kinds: ["Directory"], extensions: ["mov"] });

    expect(transport.lastCall("list_files")).toMatchObject({
      offset: 0,
      filter: { kinds: ["Directory"], extensions: ["mov"], minSize: null },
    });
  });

  it("drills into a folder and back to the global ranking", async () => {
    const { transport, store } = setup();
    await store.open(5);

    await store.openFolder("/Users/alice/Movies");
    expect(transport.lastCall("list_folder_children")).toEqual({
      scanId: 5,
      path: "/Users/alice/Movies",
      offset: 0,
      limit: PAGE_SIZE,
    });
    expect(store.items.map((i) => i.id)).toEqual([9]);

    await store.openFolder(null);
    expect(store.items.map((i) => i.id)).toEqual([1, 2]);
  });

  it("keeps the folder when reopening the same scan, resets it for a new scan", async () => {
    const { store } = setup();
    await store.open(5);
    await store.openFolder("/x");
    await store.open(5);
    expect(store.folder).toBe("/x");
    await store.open(6);
    expect(store.folder).toBeNull();
  });

  it("records errors and reveals in Finder", async () => {
    const { transport, store } = setup();
    transport.on("list_files", () => {
      throw { code: "NOT_FOUND", message: "scan 5" };
    });
    await store.open(5);
    expect(store.error).toBe("scan 5");

    transport.on("reveal_in_finder", () => undefined);
    await store.revealInFinder("/x");
    expect(transport.lastCall("reveal_in_finder")).toEqual({ path: "/x" });

    transport.on("reveal_in_finder", () => {
      throw new Error("no finder");
    });
    await store.revealInFinder("/x");
    expect(store.error).toBe("no finder");
  });

  it("does nothing before a scan is opened and can be reset", async () => {
    const { transport, store } = setup();
    await store.load();
    await store.nextPage();
    await store.previousPage();
    expect(transport.calls).toHaveLength(0);
    await store.open(5);
    store.reset();
    expect(store.scanId).toBeNull();
    expect(store.items).toEqual([]);
  });
});
