import { defineStore } from "pinia";
import { ref } from "vue";
import { applyLocale, currentLocale, type AppLocale } from "@/i18n";
import { IpcError, useApi } from "@/services/ipc";

export const usePreferencesStore = defineStore("preferences", () => {
  const locale = ref<AppLocale>(currentLocale());
  const error = ref<string | null>(null);

  /** Switches the language immediately and saves it so the app reopens in it. */
  async function setLocale(next: AppLocale): Promise<void> {
    error.value = null;
    locale.value = next;
    applyLocale(next);
    try {
      await useApi().preferences.setLocale(next);
    } catch (e) {
      error.value = IpcError.from(e).message;
    }
  }

  return { locale, error, setLocale };
});
