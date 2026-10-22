import { describe, expect, it } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { provideApi } from "@/services/ipc";
import { aiSettings } from "@/test/factories";
import { FakeTransport } from "@/test/fakeTransport";
import type { PlatformDto } from "@/types/generated/PlatformDto";
import { mountWithPlugins } from "@/test/mount";
import SettingsView from "./SettingsView.vue";

function backend(initial = aiSettings(), models: { id: string; label: string }[] = []) {
  const transport = new FakeTransport()
    .on("get_ai_settings", () => initial)
    .on("list_ai_models", (args) =>
      args.provider === "anthropic"
        ? [
            { id: "claude-opus-5-5", label: "Claude Opus 5.5" },
            { id: "claude-haiku-5-5", label: "Claude Haiku 5.5" },
          ]
        : models,
    )
    .on("save_ai_settings", (args) => {
      const input = args.input as { provider: never; apiKey: string | null; model: string };
      return aiSettings({
        provider: input.provider,
        model: input.model,
        providers: aiSettings().providers.map((p) =>
          p.id === input.provider ? { ...p, hasApiKey: !!input.apiKey } : p,
        ),
      });
    })
    .on("test_ai_connection", () => ({ provider: "anthropic", model: "claude-opus-5-5" }))
    .on("set_locale", () => undefined);
  provideApi(transport);
  return transport;
}

describe("SettingsView", () => {
  it("lists the account's models after the key is entered and saves the chosen one", async () => {
    const transport = backend();
    const { wrapper } = await mountWithPlugins(SettingsView, {}, { path: "/settings" });
    await flushPromises();

    await wrapper.find("#provider").setValue("anthropic");
    await flushPromises();
    expect(wrapper.find('[data-test="models-need-key"]').exists()).toBe(true);
    expect(wrapper.find('[data-test="save"]').attributes("disabled")).toBeDefined();
    expect(wrapper.find("#base-url").exists()).toBe(false);
    expect(wrapper.text()).toContain("Usando a URL oficial: https://anthropic.example");

    await wrapper.find("#api-key").setValue("sk-ant-123");
    await wrapper.find('[data-test="load-models"]').trigger("click");
    await flushPromises();

    const options = wrapper.findAll("#model option").map((o) => o.text());
    expect(options).toEqual([
      "Escolha um modelo",
      "Claude Opus 5.5 — claude-opus-5-5 (recomendado)",
      "Claude Haiku 5.5 — claude-haiku-5-5",
    ]);
    expect((wrapper.find("#model").element as HTMLSelectElement).value).toBe("claude-opus-5-5");

    await wrapper.find("#model").setValue("claude-haiku-5-5");
    await wrapper.find('[data-test="save"]').trigger("click");
    await flushPromises();

    expect(transport.lastCall("save_ai_settings")).toMatchObject({
      input: { provider: "anthropic", apiKey: "sk-ant-123", model: "claude-haiku-5-5" },
    });
    expect(wrapper.find('[data-test="message"]').text()).toBe("Configurações salvas.");
    expect(wrapper.find('[data-test="remove-key"]').exists()).toBe(true);
  });

  it("tests the connection of saved settings", async () => {
    const providers = aiSettings().providers.map((p) =>
      p.id === "anthropic" ? { ...p, hasApiKey: true } : p,
    );
    backend(aiSettings({ provider: "anthropic", model: "claude-opus-5-5", providers }));
    const { wrapper } = await mountWithPlugins(SettingsView, {}, { path: "/settings" });
    await flushPromises();
    await wrapper.find('[data-test="test"]').trigger("click");
    await flushPromises();
    expect(wrapper.find('[data-test="message"]').text()).toBe(
      "Conexão ok: anthropic · claude-opus-5-5",
    );
  });

  it("shows the URL for Ollama, guides when no model is installed, and supports custom URLs", async () => {
    backend();
    const { wrapper } = await mountWithPlugins(SettingsView, {}, { path: "/settings" });
    await flushPromises();
    await wrapper.find("#provider").setValue("ollama");
    await flushPromises();
    expect(wrapper.find("#api-key").exists()).toBe(false);
    expect(wrapper.find("#base-url").exists()).toBe(true);
    expect(wrapper.find('[data-test="models-empty"]').text()).toContain("ollama pull");

    await wrapper.find("#provider").setValue("gemini");
    await wrapper.find('[data-test="custom-url"]').setValue(true);
    expect(wrapper.find("#base-url").exists()).toBe(true);
    await wrapper.find("#base-url").setValue("https://proxy.example.com");
    expect(wrapper.text()).toContain("Informe a API key");

    expect(wrapper.text()).toContain("O conteúdo dos arquivos nunca é enviado");
    await wrapper.find("#provider").setValue("");
    expect(wrapper.find("#model").exists()).toBe(false);
  });

  it("shows listing errors", async () => {
    const transport = backend();
    transport.on("list_ai_models", () => {
      throw { code: "AI_UNAVAILABLE", message: "the provider rejected the API key" };
    });
    const { wrapper } = await mountWithPlugins(SettingsView, {}, { path: "/settings" });
    await flushPromises();
    await wrapper.find("#provider").setValue("openai");
    await wrapper.find("#api-key").setValue("bad");
    await wrapper.find('[data-test="load-models"]').trigger("click");
    await flushPromises();
    expect(wrapper.find('[data-test="models-error"]').text()).toContain("rejected the API key");
  });

  it("switches the interface language and saves it", async () => {
    const transport = backend();
    const { wrapper } = await mountWithPlugins(SettingsView, {}, { path: "/settings" });
    await flushPromises();
    expect(wrapper.find("h1").text()).toBe("Configurações");
    expect(wrapper.findAll("#locale option").map((o) => o.text())).toEqual([
      "English (US)",
      "Português (Brasil)",
    ]);

    await wrapper.find("#locale").setValue("en-US");
    await flushPromises();

    expect(wrapper.find("h1").text()).toBe("Settings");
    expect(wrapper.text()).toContain("Artificial intelligence");
    expect(transport.lastCall("set_locale")).toEqual({ locale: "en-US" });
  });

  describe("without a keyring", () => {
    function noKeyring(platform: PlatformDto) {
      return backend(aiSettings({ secretStoreAvailable: false }))
        .on("get_scan_defaults", () => ({
          roots: ["/home/alice"],
          home: "/home/alice",
          minFileSize: 100_000_000,
          platform,
          applications: [],
          disks: ["/"],
        }))
        .on("get_latest_scan", () => null)
        .on("get_permissions_status", () => ({ fullDiskAccess: true }));
    }

    it("explains on Linux how to install one and that Ollama still works", async () => {
      noKeyring("linux");
      const { wrapper } = await mountWithPlugins(SettingsView, {}, { path: "/settings" });
      await flushPromises();

      const notice = wrapper.find('[data-test="no-keyring"]');
      expect(notice.exists()).toBe(true);
      expect(notice.text()).toContain("sudo apt install gnome-keyring");
      expect(notice.text()).toContain("KWallet");
      expect(notice.text()).toContain("Ollama");

      await wrapper.find("#provider").setValue("anthropic");
      await flushPromises();
      const key = wrapper.find("#api-key");
      expect(key.attributes("disabled")).toBeDefined();
      expect(key.attributes("placeholder")).toBe("Sem chaveiro do sistema para guardar a chave");
    });

    it("uses the system's terms elsewhere", async () => {
      noKeyring("windows");
      const { wrapper } = await mountWithPlugins(SettingsView, {}, { path: "/settings" });
      await flushPromises();
      const notice = wrapper.find('[data-test="no-keyring"]').text();
      expect(notice).toContain("Gerenciador de Credenciais");
      expect(notice).not.toContain("apt install");
    });

    it("is not shown when keys can be saved", async () => {
      backend();
      const { wrapper } = await mountWithPlugins(SettingsView, {}, { path: "/settings" });
      await flushPromises();
      expect(wrapper.find('[data-test="no-keyring"]').exists()).toBe(false);
      await wrapper.find("#provider").setValue("anthropic");
      expect(wrapper.find("#api-key").attributes("disabled")).toBeUndefined();
    });
  });
});
