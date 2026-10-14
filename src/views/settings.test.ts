import { describe, expect, it } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { provideApi } from "@/services/ipc";
import { aiSettings } from "@/test/factories";
import { FakeTransport } from "@/test/fakeTransport";
import { mountWithPlugins } from "@/test/mount";
import SettingsView from "./SettingsView.vue";

function backend(initial = aiSettings()) {
  const transport = new FakeTransport()
    .on("get_ai_settings", () => initial)
    .on("save_ai_settings", (args) => {
      const input = args.input as { provider: never; apiKey: string | null };
      return aiSettings({
        provider: input.provider,
        providers: aiSettings().providers.map((p) =>
          p.id === input.provider ? { ...p, hasApiKey: !!input.apiKey } : p,
        ),
      });
    })
    .on("test_ai_connection", () => ({ provider: "anthropic", model: "claude-opus-5-5" }));
  provideApi(transport);
  return transport;
}

describe("SettingsView", () => {
  it("lets the user pick a provider, paste a key and save", async () => {
    const transport = backend();
    const { wrapper } = await mountWithPlugins(SettingsView, {}, { path: "/settings" });
    await flushPromises();

    expect(wrapper.find("#api-key").exists()).toBe(false);
    await wrapper.find("#provider").setValue("anthropic");
    expect(wrapper.find("#model").attributes("placeholder")).toBe("claude-opus-5-5");
    await wrapper.find("#api-key").setValue("sk-ant-123");
    await wrapper.find('[data-test="save"]').trigger("click");
    await flushPromises();

    expect(transport.lastCall("save_ai_settings")).toMatchObject({
      input: { provider: "anthropic", apiKey: "sk-ant-123" },
    });
    expect(wrapper.find('[data-test="message"]').text()).toBe("Configurações salvas.");
    expect(wrapper.find("#api-key").attributes("placeholder")).toContain("Keychain");
    expect(wrapper.find('[data-test="remove-key"]').exists()).toBe(true);
  });

  it("tests the connection", async () => {
    backend(aiSettings({ provider: "anthropic" }));
    const { wrapper } = await mountWithPlugins(SettingsView, {}, { path: "/settings" });
    await flushPromises();
    await wrapper.find('[data-test="test"]').trigger("click");
    await flushPromises();
    expect(wrapper.find('[data-test="message"]').text()).toBe(
      "Conexão ok: anthropic · claude-opus-5-5",
    );
  });

  it("hides the key field for Ollama and explains privacy", async () => {
    backend();
    const { wrapper } = await mountWithPlugins(SettingsView, {}, { path: "/settings" });
    await flushPromises();
    await wrapper.find("#provider").setValue("ollama");
    expect(wrapper.find("#api-key").exists()).toBe(false);
    expect(wrapper.find("#base-url").exists()).toBe(true);
    expect(wrapper.text()).toContain("O conteúdo dos arquivos nunca é enviado");

    await wrapper.find("#provider").setValue("");
    expect(wrapper.find("#model").exists()).toBe(false);
  });
});
