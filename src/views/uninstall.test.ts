import { describe, expect, it } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { provideApi } from "@/services/ipc";
import { aiSettings } from "@/test/factories";
import { FakeTransport } from "@/test/fakeTransport";
import { mountWithPlugins } from "@/test/mount";
import type { AppFootprintDto } from "@/types/generated/AppFootprintDto";
import SettingsView from "./SettingsView.vue";

const footprint = (bundleFound = true): AppFootprintDto => ({
  items: [
    {
      kind: "appData",
      path: "/Users/alice/Library/Application Support/com.marlonbochi.macoscleaner",
      size: 7_600_000,
    },
    {
      kind: "appBundle",
      path: "/Applications/macOS Cleaner.app",
      size: 10_000_000,
    },
  ],
  storedKeys: ["anthropic"],
  bundleFound,
  confirmationPhrase: "macOS Cleaner",
});

function backend(found = footprint()) {
  const transport = new FakeTransport()
    .on("get_ai_settings", () => aiSettings())
    .on("list_ai_models", () => [])
    .on("get_scan_defaults", () => ({
      roots: ["/Users/alice"],
      home: "/Users/alice",
      minFileSize: 100_000_000,
    }))
    .on("get_latest_scan", () => null)
    .on("get_permissions_status", () => ({ fullDiskAccess: true }))
    .on("get_app_footprint", () => found)
    .on("uninstall_app", () => ({
      trashed: found.items.map((i) => i.path),
      keysRemoved: 1,
      failures: [],
      quitting: true,
    }));
  provideApi(transport);
  return transport;
}

async function openDialog() {
  const result = await mountWithPlugins(SettingsView, {}, { path: "/settings" });
  await flushPromises();
  await result.wrapper.find('[data-test="open-uninstall"]').trigger("click");
  await flushPromises();
  return result;
}

describe("Uninstall", () => {
  it("lists what will be removed before anything happens", async () => {
    const transport = backend();
    const { wrapper } = await openDialog();

    const items = wrapper.find('[data-test="uninstall-items"]').text();
    expect(items).toContain("Dados (catálogo)");
    expect(items).toContain("~/Library/Application Support/com.marlonbochi.macoscleaner");
    expect(items).toContain("O app");
    expect(wrapper.find('[data-test="uninstall-keys"]').text()).toContain("anthropic");
    expect(wrapper.text()).toContain("Total: 17,6 MB");
    expect(transport.lastCall("uninstall_app")).toBeUndefined();
  });

  it("only unlocks the button when the exact phrase is typed", async () => {
    const transport = backend();
    const { wrapper } = await openDialog();
    const confirm = wrapper.find('[data-test="uninstall-confirm"]');
    const input = wrapper.find("#uninstall-confirm");

    expect(confirm.attributes("disabled")).toBeDefined();
    for (const wrong of ["macos cleaner", "macOS", "sim"]) {
      await input.setValue(wrong);
      expect(confirm.attributes("disabled")).toBeDefined();
    }
    await wrapper.find("form").trigger("submit");
    await flushPromises();
    expect(transport.lastCall("uninstall_app")).toBeUndefined();

    await input.setValue("macOS Cleaner");
    expect(confirm.attributes("disabled")).toBeUndefined();
    await wrapper.find("form").trigger("submit");
    await flushPromises();

    expect(transport.lastCall("uninstall_app")).toEqual({ confirmation: "macOS Cleaner" });
    expect(wrapper.find('[data-test="uninstall-result"]').text()).toContain(
      "2 item(ns) movido(s) para o Lixo e 1 chave(s) removida(s).",
    );
    expect(wrapper.text()).toContain("vai fechar agora");
  });

  it("can be cancelled and warns when the app itself cannot be found", async () => {
    backend(footprint(false));
    const { wrapper } = await openDialog();
    expect(wrapper.find('[data-test="uninstall-dev"]').exists()).toBe(true);

    await wrapper.find('[data-test="uninstall-cancel"]').trigger("click");
    expect(wrapper.find('[data-test="uninstall-dialog"]').exists()).toBe(false);
  });

  it("shows backend errors and lets the user try again", async () => {
    const transport = backend();
    transport.on("uninstall_app", () => {
      throw { code: "INVALID_INPUT", message: "type the phrase" };
    });
    const { wrapper } = await openDialog();
    await wrapper.find("#uninstall-confirm").setValue("macOS Cleaner");
    await wrapper.find("form").trigger("submit");
    await flushPromises();
    expect(wrapper.find('[role="alert"]').text()).toContain("type the phrase");
    expect(wrapper.find('[data-test="uninstall-confirm"]').attributes("disabled")).toBeUndefined();
  });
});
