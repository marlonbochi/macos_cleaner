import { describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { provideApi } from "@/services/ipc";
import { FakeTransport } from "@/test/fakeTransport";
import {
  aiSettings,
  aiSettingsWithClaude,
  analysis,
  fileEntry,
  page,
  scanSummary,
} from "@/test/factories";
import { mountWithPlugins } from "@/test/mount";
import App from "@/App.vue";
import FilesView from "./FilesView.vue";
import ScanView from "./ScanView.vue";
import type { ScanSummaryDto } from "@/types/generated/ScanSummaryDto";

vi.mock("@/services/dialog", () => ({
  pickFolders: vi.fn(async () => ["/Volumes/USB"]),
}));

function backend(latest: ScanSummaryDto | null = scanSummary({ id: 3 }), withAi = false) {
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
    .on("list_folder_children", () => page([fileEntry({ id: 5, name: "inner.mov" })]))
    .on("get_analyses", () => [analysis({ fileId: 2, level: "Safe" })])
    .on("analyze_file", (args) => analysis({ fileId: args.fileId as number }))
    .on("analyze_files", (args) =>
      (args.fileIds as number[]).map((fileId) => analysis({ fileId, level: "Safe" })),
    )
    .on("get_ai_settings", () => (withAi ? aiSettingsWithClaude() : aiSettings()))
    .on("list_ai_models", () => []);
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

describe("ScanView on Windows", () => {
  it("offers each drive and the program folders, and talks about Explorer", async () => {
    const transport = backend(scanSummary({ id: 3 }));
    transport.on("get_scan_defaults", () => ({
      roots: ["C:\\Users\\bob"],
      home: "C:\\Users\\bob",
      minFileSize: 100_000_000,
      platform: "windows",
      applications: ["C:\\Program Files", "C:\\Program Files (x86)"],
      disks: ["C:\\", "D:\\"],
    }));
    transport.on("get_permissions_status", () => ({ fullDiskAccess: true, target: null }));
    const { wrapper } = await mountWithPlugins(ScanView);
    await flushPromises();

    expect(wrapper.text()).not.toContain("Acesso Total ao Disco");
    const drives = wrapper.findAll('[data-test="whole-disk"]').map((b) => b.text());
    expect(drives).toEqual(["Disco C:", "Disco D:"]);

    await wrapper.find('[data-test="applications"]').trigger("click");
    await wrapper.findAll('[data-test="whole-disk"]')[1]?.trigger("click");
    const { useScanStore } = await import("@/stores/scan");
    expect(useScanStore().roots).toEqual([
      "C:\\Users\\bob",
      "C:\\Program Files",
      "C:\\Program Files (x86)",
      "D:\\",
    ]);
  });

  it("labels the reveal button after the system's file manager", async () => {
    const transport = backend();
    transport.on("get_scan_defaults", () => ({
      roots: ["C:\\Users\\bob"],
      home: "C:\\Users\\bob",
      minFileSize: 100_000_000,
      platform: "windows",
      applications: [],
      disks: ["C:\\"],
    }));
    const { wrapper } = await mountWithPlugins(FilesView, {}, { path: "/files" });
    await flushPromises();
    const reveal = wrapper.find('[data-test="row-2"] [data-test="reveal"]');
    expect(reveal.text()).toBe("Explorer");
    expect(reveal.attributes("title")).toBe("Mostrar no Explorador de Arquivos");
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

describe("FilesView analysis", () => {
  it("shows stored risk badges and opens the analysis panel", async () => {
    const transport = backend();
    const { wrapper } = await mountWithPlugins(FilesView, {}, { path: "/files" });
    await flushPromises();

    expect(transport.lastCall("get_analyses")).toEqual({ fileIds: [1, 2] });
    expect(wrapper.find('[data-test="row-2"] [data-level="Safe"]').exists()).toBe(true);

    await wrapper.find('[data-test="row-1"] [data-test="analyze"]').trigger("click");
    await flushPromises();
    expect(transport.lastCall("analyze_file")).toEqual({ fileId: 1, force: false });
    expect(wrapper.find('[role="dialog"]').text()).toContain("Simuladores do Xcode");
    expect(wrapper.find('[data-test="row-1"] [data-level="Caution"]').exists()).toBe(true);

    await wrapper.find('[data-test="reanalyze"]').trigger("click");
    await flushPromises();
    expect(transport.lastCall("analyze_file")).toEqual({ fileId: 1, force: true });

    await wrapper.find('[data-test="close"]').trigger("click");
    expect(wrapper.find('[role="dialog"]').exists()).toBe(false);
  });
});

describe("FilesView batch analysis", () => {
  it("hides the selection without an AI provider", async () => {
    backend();
    const { wrapper } = await mountWithPlugins(FilesView, {}, { path: "/files" });
    await flushPromises();
    expect(wrapper.find('[data-test="select-page"]').exists()).toBe(false);
    expect(wrapper.find('[data-test="select"]').exists()).toBe(false);
    expect(wrapper.find('[data-test="analyze"]').exists()).toBe(true);
  });

  it("analyses the selected items in one request and clears the selection", async () => {
    const transport = backend(scanSummary({ id: 3 }), true);
    const { wrapper } = await mountWithPlugins(FilesView, {}, { path: "/files" });
    await flushPromises();
    expect(wrapper.find('[data-test="selection-bar"]').exists()).toBe(false);

    await wrapper.find('[data-test="select-page"]').trigger("change");
    expect(wrapper.find('[data-test="selection-bar"]').text()).toContain("2 selecionado(s)");

    await wrapper.find('[data-test="analyze-selected"]').trigger("click");
    await flushPromises();

    expect(transport.calls.filter((c) => c.command === "analyze_files")).toHaveLength(1);
    expect(transport.lastCall("analyze_files")).toEqual({ fileIds: [1, 2], force: false });
    expect(wrapper.find('[data-test="batch-result"]').text()).toContain("2 item(ns) analisado(s).");
    expect(wrapper.find('[data-test="row-1"] [data-level="Safe"]').exists()).toBe(true);
    expect(wrapper.find('[data-test="selection-bar"]').exists()).toBe(false);
  });
});

describe("App shell", () => {
  it("renders navigation without the AI diagnosis when no AI is configured", async () => {
    backend();
    const { wrapper } = await mountWithPlugins(App);
    await flushPromises();
    expect(wrapper.findAll(".nav-link").map((l) => l.text())).toEqual([
      "🔍 Varredura",
      "📊 Maiores itens",
      "⚙️ Configurações",
    ]);
  });

  it("shows the AI diagnosis tab when an AI provider is configured", async () => {
    backend(scanSummary({ id: 3 }), true);
    const { wrapper } = await mountWithPlugins(App);
    await flushPromises();
    expect(wrapper.findAll(".nav-link").map((l) => l.text())).toContain("✨ Diagnóstico IA");
  });
});
