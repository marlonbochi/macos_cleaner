import { describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { provideApi } from "@/services/ipc";
import { FakeTransport } from "@/test/fakeTransport";
import { fileEntry, page, scanSummary } from "@/test/factories";
import { mountWithPlugins } from "@/test/mount";
import App from "@/App.vue";
import FilesView from "./FilesView.vue";
import ScanView from "./ScanView.vue";
import type { ScanSummaryDto } from "@/types/generated/ScanSummaryDto";

vi.mock("@/services/dialog", () => ({
  pickFolders: vi.fn(async () => ["/Volumes/USB"]),
}));

function backend(latest: ScanSummaryDto | null = scanSummary({ id: 3 })) {
  const transport = new FakeTransport()
    .on("get_scan_defaults", () => ({
      roots: ["/Users/alice"],
      home: "/Users/alice",
      minFileSize: 100_000_000,
    }))
    .on("get_latest_scan", () => latest)
    .on("get_permissions_status", () => ({ fullDiskAccess: false }))
    .on("list_files", () =>
      page([
        fileEntry({ id: 1, path: "/Users/alice/Movies", kind: "Directory", name: "Movies" }),
        fileEntry({ id: 2 }),
      ]),
    )
    .on("list_folder_children", () => page([fileEntry({ id: 5, name: "inner.mov" })]));
  provideApi(transport);
  return transport;
}

describe("ScanView", () => {
  it("shows the permission banner, roots and last scan", async () => {
    backend();
    const { wrapper } = await mountWithPlugins(ScanView);
    await flushPromises();

    expect(wrapper.text()).toContain("Acesso Total ao Disco não concedido");
    expect(wrapper.text()).toContain("~");
    expect(wrapper.find('[data-test="view-results"]').exists()).toBe(true);
  });

  it("adds roots and starts a scan", async () => {
    const transport = backend(null);
    transport.on("start_scan", () => scanSummary({ id: 9, status: "Running" }));
    const { wrapper } = await mountWithPlugins(ScanView);
    await flushPromises();

    await wrapper.find('[data-test="whole-disk"]').trigger("click");
    const addFolder = wrapper.findAll("button").find((b) => b.text() === "Adicionar pasta…");
    await addFolder?.trigger("click");
    await flushPromises();
    await wrapper.find('[data-test="start"]').trigger("click");
    await flushPromises();

    expect(transport.lastCall("start_scan")?.roots).toEqual(["/Users/alice", "/", "/Volumes/USB"]);
    expect(wrapper.find('[data-test="cancel"]').exists()).toBe(true);
  });

  it("navigates to the results", async () => {
    backend();
    const { wrapper, router } = await mountWithPlugins(ScanView);
    await flushPromises();
    await wrapper.find('[data-test="view-results"]').trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.name).toBe("files");
  });
});

describe("FilesView", () => {
  it("asks for a scan when there is no catalog", async () => {
    backend(null);
    const { wrapper } = await mountWithPlugins(FilesView, {}, { path: "/files" });
    await flushPromises();
    expect(wrapper.text()).toContain("Nenhuma varredura encontrada.");
  });

  it("lists the largest items and drills into folders", async () => {
    const transport = backend();
    const { wrapper } = await mountWithPlugins(FilesView, {}, { path: "/files" });
    await flushPromises();

    expect(transport.lastCall("list_files")?.scanId).toBe(3);
    expect(wrapper.findAll("tbody tr")).toHaveLength(2);

    await wrapper.find('[data-test="row-1"] [data-test="open-folder"]').trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("inner.mov");
    expect(wrapper.text()).toContain("~/Movies");

    await wrapper.find('[data-test="up"]').trigger("click");
    await flushPromises();
    expect(transport.lastCall("list_folder_children")?.path).toBe("/Users/alice");

    // Going up from a scan root returns to the global ranking.
    const listFilesCalls = transport.calls.filter((c) => c.command === "list_files").length;
    await wrapper.find('[data-test="up"]').trigger("click");
    await flushPromises();
    expect(transport.calls.filter((c) => c.command === "list_files")).toHaveLength(
      listFilesCalls + 1,
    );
    expect(wrapper.find('[data-test="up"]').exists()).toBe(false);
  });

  it("applies kind and extension filters", async () => {
    vi.useFakeTimers();
    const transport = backend();
    const { wrapper } = await mountWithPlugins(FilesView, {}, { path: "/files" });
    await flushPromises();

    await wrapper.find("#kind").setValue("folders");
    await flushPromises();
    expect(transport.lastCall("list_files")?.filter).toMatchObject({
      kinds: ["Directory", "Package"],
    });

    await wrapper.find("#ext").setValue(".MOV, zip");
    vi.advanceTimersByTime(300);
    await flushPromises();
    expect(transport.lastCall("list_files")?.filter).toMatchObject({ extensions: ["mov", "zip"] });
    vi.useRealTimers();
  });
});

describe("App shell", () => {
  it("renders navigation", async () => {
    backend();
    const { wrapper } = await mountWithPlugins(App);
    await flushPromises();
    expect(wrapper.findAll(".nav-link").map((l) => l.text())).toEqual([
      "🔍 Varredura",
      "📊 Maiores itens",
    ]);
  });
});
