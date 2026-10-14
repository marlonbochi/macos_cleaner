import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { provideApi } from "@/services/ipc";
import { aiSettings } from "@/test/factories";
import { FakeTransport } from "@/test/fakeTransport";
import { useAiSettingsStore } from "./aiSettings";

function setup(initial = aiSettings()) {
  const transport = new FakeTransport()
    .on("get_ai_settings", () => initial)
    .on("save_ai_settings", (args) => {
      const input = args.input as {
        provider: string | null;
        model: string | null;
        apiKey: string | null;
        clearApiKey: boolean;
      };
      return aiSettings({
        provider: input.provider as never,
        model: input.model,
        providers: aiSettings().providers.map((p) =>
          p.id === input.provider
            ? { ...p, hasApiKey: !input.clearApiKey && (p.hasApiKey || !!input.apiKey) }
            : p,
        ),
      });
    })
    .on("test_ai_connection", () => ({ provider: "anthropic", model: "claude-opus-5-5" }));
  provideApi(transport);
  setActivePinia(createPinia());
  return { transport, store: useAiSettingsStore() };
}

describe("AI settings store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("loads settings into the form without exposing a key", async () => {
    const { store } = setup(aiSettings({ provider: "openai", model: "gpt-5" }));
    await store.load();
    expect(store.form).toEqual({ provider: "openai", model: "gpt-5", baseUrl: "", apiKey: "" });
    expect(store.providerInfo?.label).toBe("OpenAI");
    expect(store.aiEnabled).toBe(false);
  });

  it("saves the selected provider with a new key and reports success", async () => {
    const { transport, store } = setup();
    await store.load();
    store.selectProvider("anthropic");
    store.form.apiKey = "sk-ant";

    expect(await store.save()).toBe(true);

    expect(transport.lastCall("save_ai_settings")).toEqual({
      input: {
        provider: "anthropic",
        model: null,
        baseUrl: null,
        apiKey: "sk-ant",
        clearApiKey: false,
      },
    });
    expect(store.form.apiKey).toBe("");
    expect(store.aiEnabled).toBe(true);
    expect(store.message).toBe("saved");
  });

  it("removes the stored key", async () => {
    const { transport, store } = setup();
    await store.load();
    store.selectProvider("gemini");
    await store.save(true);
    expect(transport.lastCall("save_ai_settings")).toMatchObject({ input: { clearApiKey: true } });
    expect(store.message).toBe("keyRemoved");
  });

  it("ollama is enabled without a key", async () => {
    const { store } = setup();
    await store.load();
    store.selectProvider("ollama");
    await store.save();
    expect(store.aiEnabled).toBe(true);
  });

  it("tests the connection after saving", async () => {
    const { transport, store } = setup();
    await store.load();
    store.selectProvider("anthropic");
    store.form.apiKey = "k";
    await store.test();
    expect(transport.calls.map((c) => c.command)).toEqual([
      "get_ai_settings",
      "save_ai_settings",
      "test_ai_connection",
    ]);
    expect(store.testResult).toEqual({ provider: "anthropic", model: "claude-opus-5-5" });
    expect(store.message).toBe("connected");
  });

  it("surfaces provider errors and skips the test when saving fails", async () => {
    const { transport, store } = setup();
    await store.load();
    transport.on("test_ai_connection", () => {
      throw { code: "AI_UNAVAILABLE", message: "the provider rejected the API key" };
    });
    store.selectProvider("anthropic");
    await store.test();
    expect(store.error).toBe("the provider rejected the API key");

    transport.on("save_ai_settings", () => {
      throw new Error("keychain locked");
    });
    await store.test();
    expect(store.error).toBe("keychain locked");

    transport.on("get_ai_settings", () => {
      throw new Error("db");
    });
    await store.load();
    expect(store.error).toBe("db");
  });
});
