import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { IpcError, useApi } from "@/services/ipc";
import type { AiProviderIdDto } from "@/types/generated/AiProviderIdDto";
import type { AiSettingsDto } from "@/types/generated/AiSettingsDto";
import type { ConnectionTestDto } from "@/types/generated/ConnectionTestDto";

export interface AiSettingsForm {
  provider: AiProviderIdDto | null;
  model: string;
  baseUrl: string;
  /** Typed key; empty means "keep the stored key". */
  apiKey: string;
}

export const useAiSettingsStore = defineStore("aiSettings", () => {
  const saved = ref<AiSettingsDto | null>(null);
  const form = ref<AiSettingsForm>({ provider: null, model: "", baseUrl: "", apiKey: "" });
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

  function fill(settings: AiSettingsDto): void {
    saved.value = settings;
    form.value = {
      provider: settings.provider,
      model: settings.model ?? "",
      baseUrl: settings.baseUrl ?? "",
      apiKey: "",
    };
  }

  function reset(): void {
    testResult.value = null;
    message.value = null;
    error.value = null;
  }

  async function load(): Promise<void> {
    try {
      fill(await useApi().settings.ai());
    } catch (e) {
      error.value = IpcError.from(e).message;
    }
  }

  /** Switching provider resets model/base URL to that provider's defaults. */
  function selectProvider(provider: AiProviderIdDto | null): void {
    form.value = { provider, model: "", baseUrl: "", apiKey: "" };
    reset();
  }

  async function save(clearApiKey = false): Promise<boolean> {
    saving.value = true;
    reset();
    try {
      fill(
        await useApi().settings.saveAi({
          provider: form.value.provider,
          model: form.value.model || null,
          baseUrl: form.value.baseUrl || null,
          apiKey: form.value.apiKey || null,
          clearApiKey,
        }),
      );
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
    saving,
    testing,
    testResult,
    message,
    error,
    providerInfo,
    aiEnabled,
    load,
    selectProvider,
    save,
    test,
  };
});
