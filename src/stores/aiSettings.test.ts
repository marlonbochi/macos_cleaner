import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { provideApi } from "@/services/ipc";
import { aiSettings } from "@/test/factories";
import { FakeTransport } from "@/test/fakeTransport";
import { useAiSettingsStore } from "./aiSettings";

const MODELS: Record<string, { id: string; label: string }[]> = {
  anthropic: [
    { id: "claude-opus-5-5", label: "Claude Opus 5.5" },
    { id: "claude-haiku-5-5", label: "Claude Haiku 5.5" },
  ],
  openai: [{ id: "gpt-6-luna", label: "gpt-6-luna" }],
  ollama: [],
};

type SaveInput = {
  provider: string | null;
  model: string | null;
  apiKey: string | null;
  clearApiKey: boolean;
  baseUrl: string | null;
};

function setup(initial = aiSettings()) {
  const transport = new FakeTransport()
    .on("get_ai_settings", () => initial)
    .on("list_ai_models", (args) => MODELS[args.provider as string] ?? [])
    .on("save_ai_settings", (args) => {
      const input = args.input as SaveInput;
      return aiSettings({
        provider: input.provider as never,
        model: input.model,
        baseUrl: input.baseUrl,
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

const withKey = (id: string) =>
  aiSettings().providers.map((p) => (p.id === id ? { ...p, hasApiKey: true } : p));

describe("AI settings store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("loads settings and the model list when a key is stored", async () => {
    const { transport, store } = setup(
      aiSettings({
        provider: "anthropic",
        model: "claude-haiku-5-5",
        providers: withKey("anthropic"),
      }),
    );
    await store.load();
    expect(transport.lastCall("list_ai_models")).toEqual({
      provider: "anthropic",
      baseUrl: null,
      apiKey: null,
    });
    expect(store.form.model).toBe("claude-haiku-5-5");
    expect(store.modelValid).toBe(true);
    expect(store.aiEnabled).toBe(true);
    expect(store.savedModelMissing).toBe(false);
  });

  it("flags a saved model the provider no longer offers and picks the default", async () => {
    const { store } = setup(
      aiSettings({ provider: "anthropic", model: "claude-old", providers: withKey("anthropic") }),
    );
    await store.load();
    expect(store.savedModelMissing).toBe(true);
    expect(store.form.model).toBe("claude-opus-5-5");
  });

  it("needs a key before listing cloud models, then lists with the typed key", async () => {
    const { transport, store } = setup();
    await store.load();
    await store.selectProvider("anthropic");
    expect(store.hasKey).toBe(false);
    expect(transport.lastCall("list_ai_models")).toBeUndefined();
    expect(store.canSave).toBe(false);

    store.form.apiKey = "sk-typed";
    await store.fetchModels();
    expect(transport.lastCall("list_ai_models")).toMatchObject({ apiKey: "sk-typed" });
    expect(store.form.model).toBe("claude-opus-5-5");
    expect(store.canSave).toBe(true);
  });

  it("saves the chosen model with the new key", async () => {
    const { transport, store } = setup();
    await store.load();
    await store.selectProvider("anthropic");
    store.form.apiKey = "sk-ant";
    await store.fetchModels();
    store.form.model = "claude-haiku-5-5";

    expect(await store.save()).toBe(true);

    expect(transport.lastCall("save_ai_settings")).toEqual({
      input: {
        provider: "anthropic",
        model: "claude-haiku-5-5",
        baseUrl: null,
        apiKey: "sk-ant",
        clearApiKey: false,
      },
    });
    expect(store.form.apiKey).toBe("");
    expect(store.aiEnabled).toBe(true);
    expect(store.message).toBe("saved");
  });

  it("changing the URL invalidates the list until it is reloaded", async () => {
    const { transport, store } = setup();
    await store.load();
    await store.selectProvider("ollama");
    expect(transport.lastCall("list_ai_models")).toMatchObject({ provider: "ollama" });
    expect(store.models).toEqual([]);

    store.setBaseUrl("http://gpu-box:11434");
    expect(store.modelsCurrent).toBe(false);
    expect(store.canSave).toBe(false);
    await store.fetchModels();
    expect(transport.lastCall("list_ai_models")).toMatchObject({ baseUrl: "http://gpu-box:11434" });

    store.setCustomUrl(true);
    store.setCustomUrl(false);
    expect(store.form.baseUrl).toBe("");
  });

  it("reports listing errors and allows choosing no provider", async () => {
    const { transport, store } = setup();
    await store.load();
    transport.on("list_ai_models", () => {
      throw { code: "AI_UNAVAILABLE", message: "the provider rejected the API key" };
    });
    store.form.provider = "openai";
    store.form.apiKey = "bad";
    await store.fetchModels();
    expect(store.modelsError).toBe("the provider rejected the API key");
    expect(store.models).toEqual([]);

    await store.selectProvider(null);
    expect(store.canSave).toBe(true);
    await store.save();
    expect(transport.lastCall("save_ai_settings")).toMatchObject({
      input: { provider: null, model: null },
    });
  });

  it("removes the stored key without requiring a model", async () => {
    const { transport, store } = setup(
      aiSettings({ provider: "gemini", providers: withKey("gemini") }),
    );
    await store.load();
    await store.save(true);
    expect(transport.lastCall("save_ai_settings")).toMatchObject({ input: { clearApiKey: true } });
    expect(store.message).toBe("keyRemoved");
  });

  it("tests the connection after saving and surfaces errors", async () => {
    const { transport, store } = setup(
      aiSettings({
        provider: "anthropic",
        model: "claude-opus-5-5",
        providers: withKey("anthropic"),
      }),
    );
    await store.load();
    await store.test();
    expect(store.testResult).toEqual({ provider: "anthropic", model: "claude-opus-5-5" });
    expect(store.message).toBe("connected");

    transport.on("test_ai_connection", () => {
      throw new Error("timeout");
    });
    await store.test();
    expect(store.error).toBe("timeout");

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
