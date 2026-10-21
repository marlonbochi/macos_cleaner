import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { IpcError, useApi } from "@/services/ipc";
import type { AiModelDto } from "@/types/generated/AiModelDto";
import type { AiProviderIdDto } from "@/types/generated/AiProviderIdDto";
import type { AiSettingsDto } from "@/types/generated/AiSettingsDto";
import type { ConnectionTestDto } from "@/types/generated/ConnectionTestDto";

export interface AiSettingsForm {
  provider: AiProviderIdDto | null;
  /** Always one of the ids returned by the provider's model list. */
  model: string;
  /** Empty means "the provider's official URL". */
  baseUrl: string;
  /** Typed key; empty means "keep the stored key". */
  apiKey: string;
  /** Show the server URL field for cloud providers (always shown for Ollama). */
  customUrl: boolean;
}

const emptyForm = (provider: AiProviderIdDto | null = null): AiSettingsForm => ({
  provider,
  model: "",
  baseUrl: "",
  apiKey: "",
  customUrl: false,
});

export const useAiSettingsStore = defineStore("aiSettings", () => {
  const saved = ref<AiSettingsDto | null>(null);
  const form = ref<AiSettingsForm>(emptyForm());
  const models = ref<AiModelDto[]>([]);
  /** Provider + URL the current `models` list belongs to. */
  const modelsSource = ref<string | null>(null);
  const modelsLoading = ref(false);
  const modelsError = ref<string | null>(null);
  const saving = ref(false);
  const testing = ref(false);
  const testResult = ref<ConnectionTestDto | null>(null);
  const message = ref<string | null>(null);
  const error = ref<string | null>(null);

  const providerInfo = computed(
    () => saved.value?.providers.find((p) => p.id === form.value.provider) ?? null,
  );
  /** Whether analyses currently use AI (as saved, not as being edited). */
  const aiEnabled = computed(() => {
    const provider = saved.value?.providers.find((p) => p.id === saved.value?.provider);
    return !!provider && (!provider.needsApiKey || provider.hasApiKey);
  });
  const hasKey = computed(
    () =>
      !providerInfo.value?.needsApiKey ||
      providerInfo.value.hasApiKey ||
      form.value.apiKey.trim() !== "",
  );
  /** `false` when the system has no keyring, so API keys cannot be saved. */
  const canStoreKeys = computed(() => saved.value?.secretStoreAvailable ?? true);
  const sourceKey = computed(() => `${form.value.provider}|${form.value.baseUrl.trim()}`);
  /** The list on screen matches the current provider and URL. */
  const modelsCurrent = computed(() => modelsSource.value === sourceKey.value);
  const modelValid = computed(
    () => modelsCurrent.value && models.value.some((m) => m.id === form.value.model),
  );
  /** A saved model that the provider no longer offers. */
  const savedModelMissing = computed(
    () =>
      modelsCurrent.value &&
      !!saved.value?.model &&
      saved.value.provider === form.value.provider &&
      !models.value.some((m) => m.id === saved.value?.model),
  );
  const canSave = computed(() => form.value.provider === null || modelValid.value);

  function reset(): void {
    testResult.value = null;
    message.value = null;
    error.value = null;
  }

  function fill(settings: AiSettingsDto): void {
    saved.value = settings;
    form.value = {
      provider: settings.provider,
      model: settings.model ?? "",
      baseUrl: settings.baseUrl ?? "",
      apiKey: "",
      customUrl: !!settings.baseUrl,
    };
  }

  async function load(): Promise<void> {
    try {
      fill(await useApi().settings.ai());
      if (form.value.provider && hasKey.value) await fetchModels();
    } catch (e) {
      error.value = IpcError.from(e).message;
    }
  }

  /** Loads only the saved settings (no model list): enough to know whether AI is enabled. */
  async function loadSaved(): Promise<void> {
    try {
      fill(await useApi().settings.ai());
    } catch (e) {
      error.value = IpcError.from(e).message;
    }
  }

  /** Loads the models the (typed or stored) key can use and picks a sensible default. */
  async function fetchModels(): Promise<void> {
    const provider = form.value.provider;
    if (!provider) return;
    const source = sourceKey.value;
    modelsLoading.value = true;
    modelsError.value = null;
    try {
      const list = await useApi().settings.models(
        provider,
        form.value.baseUrl.trim() || null,
        form.value.apiKey.trim() || null,
      );
      if (source !== sourceKey.value) return; // the user changed provider/URL meanwhile
      models.value = list;
      modelsSource.value = source;
      if (!list.some((m) => m.id === form.value.model)) {
        const preferred = list.find((m) => m.id === providerInfo.value?.defaultModel);
        form.value.model = (preferred ?? list[0])?.id ?? "";
      }
    } catch (e) {
      if (source !== sourceKey.value) return;
      models.value = [];
      modelsSource.value = null;
      modelsError.value = IpcError.from(e).message;
    } finally {
      modelsLoading.value = false;
    }
  }

  /** Switching provider starts from that provider's defaults and reloads its models. */
  async function selectProvider(provider: AiProviderIdDto | null): Promise<void> {
    form.value = emptyForm(provider);
    models.value = [];
    modelsSource.value = null;
    modelsError.value = null;
    reset();
    if (provider && hasKey.value) await fetchModels();
  }

  /** Changing the server URL invalidates the model list. */
  function setBaseUrl(url: string): void {
    form.value.baseUrl = url;
    modelsError.value = null;
  }

  function setCustomUrl(enabled: boolean): void {
    form.value.customUrl = enabled;
    if (!enabled) setBaseUrl("");
  }

  async function save(clearApiKey = false): Promise<boolean> {
    if (!clearApiKey && !canSave.value) return false;
    saving.value = true;
    reset();
    try {
      const keepModel = form.value.model;
      fill(
        await useApi().settings.saveAi({
          provider: form.value.provider,
          model: form.value.provider ? form.value.model || null : null,
          baseUrl: form.value.baseUrl.trim() || null,
          apiKey: form.value.apiKey || null,
          clearApiKey,
        }),
      );
      if (clearApiKey) form.value.model = keepModel;
      message.value = clearApiKey ? "keyRemoved" : "saved";
      return true;
    } catch (e) {
      error.value = IpcError.from(e).message;
      return false;
    } finally {
      saving.value = false;
    }
  }

  /** Saves pending changes, then sends a tiny request to the provider. */
  async function test(): Promise<void> {
    if (!(await save())) return;
    testing.value = true;
    try {
      testResult.value = await useApi().settings.testAi();
      message.value = "connected";
    } catch (e) {
      error.value = IpcError.from(e).message;
    } finally {
      testing.value = false;
    }
  }

  return {
    saved,
    form,
    models,
    modelsLoading,
    modelsError,
    modelsCurrent,
    modelValid,
    savedModelMissing,
    hasKey,
    canStoreKeys,
    canSave,
    saving,
    testing,
    testResult,
    message,
    error,
    providerInfo,
    aiEnabled,
    load,
    loadSaved,
    fetchModels,
    selectProvider,
    setBaseUrl,
    setCustomUrl,
    save,
    test,
  };
});
