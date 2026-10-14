import { createI18n } from "vue-i18n";
import { setFormatLocale } from "@/composables/format";
import enUS from "./en-US";
import ptBR from "./pt-BR";

/** English is the reference catalogue; every other language must provide the same keys. */
export type MessageSchema = typeof enUS;
export type AppLocale = "en-US" | "pt-BR";

export const SUPPORTED_LOCALES: readonly AppLocale[] = ["en-US", "pt-BR"];
export const FALLBACK_LOCALE: AppLocale = "en-US";

/** Native names, shown as-is in every language. */
export const LOCALE_NAMES: Record<AppLocale, string> = {
  "en-US": "English (US)",
  "pt-BR": "Português (Brasil)",
};

export function isAppLocale(value: unknown): value is AppLocale {
  return typeof value === "string" && (SUPPORTED_LOCALES as readonly string[]).includes(value);
}

/**
 * The system's primary language if the app supports it (any Portuguese → pt-BR, any English →
 * en-US); otherwise English.
 */
export function detectSystemLocale(languages: readonly string[] = navigator.languages): AppLocale {
  const primary = (languages[0] ?? "").toLowerCase();
  if (primary.startsWith("pt")) return "pt-BR";
  return FALLBACK_LOCALE;
}

function buildI18n(locale: AppLocale) {
  return createI18n<[MessageSchema], AppLocale, false>({
    legacy: false,
    locale,
    fallbackLocale: FALLBACK_LOCALE,
    messages: { "en-US": enUS, "pt-BR": ptBR },
  });
}

export type AppI18n = ReturnType<typeof buildI18n>;

/** The instance installed in the app, so stores can switch the language. */
let active: AppI18n | null = null;

export function createAppI18n(locale: AppLocale = FALLBACK_LOCALE): AppI18n {
  active = buildI18n(locale);
  setFormatLocale(locale);
  document.documentElement.lang = locale;
  return active;
}

export function currentLocale(): AppLocale {
  return active?.global.locale.value ?? FALLBACK_LOCALE;
}

/** Switches the interface language at runtime (texts, number/date formats and `<html lang>`). */
export function applyLocale(locale: AppLocale): void {
  if (active) active.global.locale.value = locale;
  setFormatLocale(locale);
  document.documentElement.lang = locale;
}
