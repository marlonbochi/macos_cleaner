import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { createAppI18n, detectSystemLocale, isAppLocale, type AppLocale } from "./i18n";
import { createAppRouter } from "./router";
import { useApi } from "./services/ipc";
import "./styles/main.css";

/** The saved language wins; otherwise the system language (English when unsupported). */
async function initialLocale(): Promise<AppLocale> {
  try {
    const { locale } = await useApi().preferences.get();
    if (isAppLocale(locale)) return locale;
  } catch {
    // No saved preference available: fall through to the system language.
  }
  return detectSystemLocale();
}

async function bootstrap(): Promise<void> {
  const i18n = createAppI18n(await initialLocale());
  createApp(App).use(createPinia()).use(createAppRouter()).use(i18n).mount("#app");
}

void bootstrap();
