import { describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { provideApi } from "@/services/ipc";
import { FakeTransport } from "@/test/fakeTransport";
import {
  aiSettings,
  aiSettingsWithClaude,
  diagnosisItem,
  page,
  scanSummary,
} from "@/test/factories";
import { mountWithPlugins } from "@/test/mount";
import DiagnosisView from "./DiagnosisView.vue";
import ScanView from "./ScanView.vue";

const GB = 1_000_000_000;

function backend(withAi = true) {
  const transport = new FakeTransport()
    .on("get_scan_defaults", () => ({
      roots: ["/Users/alice"],
      home: "/Users/alice",
      minFileSize: 100_000_000,
    }))
    .on("get_latest_scan", () => scanSummary({ id: 3 }))
    .on("get_permissions_status", () => ({ fullDiskAccess: true }))
    .on("get_ai_settings", () => (withAi ? aiSettingsWithClaude() : aiSettings()))
    .on("get_ai_diagnosis", () => [
      diagnosisItem(
        { id: 1, path: "/Users/alice/Library/Containers/docker/Docker.raw", sizeOnDisk: 50 * GB },
        { level: "Caution", summary: "Docker disk image" },
      ),
      diagnosisItem({ id: 2, path: "/Users/alice/Library/Caches", kind: "Directory" }, null),
    ])
    .on("run_ai_diagnosis", (args) => [
      diagnosisItem(
        { id: 1, path: "/Users/alice/Library/Containers/docker/Docker.raw", sizeOnDisk: 50 * GB },
        { level: "Caution", summary: "Docker disk image", createdAt: args.force ? 2 : 1 },
      ),
      diagnosisItem(
        { id: 2, path: "/Users/alice/Library/Caches", kind: "Directory", sizeOnDisk: 5 * GB },
        { level: "Safe", summary: "App caches" },
      ),
    ])
    .on("list_folder_children", () => page([]))
    .on("list_files", () => page([]))
    .on("get_analyses", () => []);
  provideApi(transport);
  return transport;
}

describe("DiagnosisView", () => {
  it("asks for an AI provider when none is configured", async () => {
    const transport = backend(false);
    const { wrapper } = await mountWithPlugins(DiagnosisView, {}, { path: "/diagnosis" });
    await flushPromises();
    expect(wrapper.find('[data-test="needs-ai"]').exists()).toBe(true);
    expect(transport.lastCall("run_ai_diagnosis")).toBeUndefined();
  });

  it("lists the largest items with what the AI thinks of them", async () => {
    const transport = backend();
    const { wrapper } = await mountWithPlugins(DiagnosisView, {}, { path: "/diagnosis" });
    await flushPromises();

    expect(transport.lastCall("get_ai_diagnosis")).toEqual({ scanId: 3 });
    expect(transport.lastCall("run_ai_diagnosis")).toBeUndefined();
    expect(wrapper.findAll("tbody tr")).toHaveLength(2);
    expect(wrapper.find('[data-test="row-1"] [data-test="summary"]').text()).toBe(
      "Docker disk image",
    );
    expect(wrapper.find('[data-test="select"]').exists()).toBe(false);
    expect(wrapper.find('[data-test="pending"]').text()).toContain("1 item(ns)");
    expect(wrapper.find('[data-test="totals"]').text()).toContain("50,0 GB");
  });

  it("analyses pending items, then offers to analyse everything again", async () => {
    const transport = backend();
    const { wrapper } = await mountWithPlugins(DiagnosisView, {}, { path: "/diagnosis" });
    await flushPromises();

    await wrapper.find('[data-test="run"]').trigger("click");
    await flushPromises();
    expect(transport.lastCall("run_ai_diagnosis")).toEqual({ scanId: 3, force: false });
    expect(wrapper.find('[data-test="row-2"] [data-level="Safe"]').exists()).toBe(true);
    expect(wrapper.find('[data-test="pending"]').exists()).toBe(false);

    await wrapper.find('[data-test="run"]').trigger("click");
    await flushPromises();
    expect(transport.lastCall("run_ai_diagnosis")).toEqual({ scanId: 3, force: true });
  });

  it("opens folders in the largest-items list", async () => {
    const transport = backend();
    const { wrapper, router } = await mountWithPlugins(DiagnosisView, {}, { path: "/diagnosis" });
    await flushPromises();

    await wrapper.find('[data-test="row-2"] [data-test="open-folder"]').trigger("click");
    await flushPromises();
    expect(transport.lastCall("list_folder_children")?.path).toBe("/Users/alice/Library/Caches");
    // The list view is loaded lazily by the router.
    await vi.waitFor(() => expect(router.currentRoute.value.name).toBe("files"));
  });
});

describe("ScanView with AI", () => {
  it("links to the AI diagnosis", async () => {
    backend();
    const { wrapper, router } = await mountWithPlugins(ScanView);
    await flushPromises();
    await wrapper.find('[data-test="view-diagnosis"]').trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.name).toBe("diagnosis");
  });
});
