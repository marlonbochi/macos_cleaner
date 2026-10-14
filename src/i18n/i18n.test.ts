import { describe, expect, it } from "vitest";
import enUS from "./en-US";
import ptBR from "./pt-BR";
import {
  applyLocale,
  createAppI18n,
  currentLocale,
  detectSystemLocale,
  isAppLocale,
} from "./index";

/** Flattens a message catalogue into "a.b.c" keys. */
function keys(messages: object, prefix = ""): string[] {
  return Object.entries(messages).flatMap(([key, value]) =>
    typeof value === "object" && value !== null
      ? keys(value as object, `${prefix}${key}.`)
      : [`${prefix}${key}`],
  );
}

describe("system language detection", () => {
  it("uses Portuguese when it is the primary system language", () => {
    expect(detectSystemLocale(["pt-BR", "en-US"])).toBe("pt-BR");
    expect(detectSystemLocale(["pt-PT"])).toBe("pt-BR");
  });

  it("falls back to English for English and unsupported primary languages", () => {
    expect(detectSystemLocale(["en-GB", "pt-BR"])).toBe("en-US");
    expect(detectSystemLocale(["fr-FR", "pt-BR"])).toBe("en-US");
    expect(detectSystemLocale([])).toBe("en-US");
  });
});

describe("catalogues", () => {
  it("define exactly the same keys in every language", () => {
    expect(keys(ptBR).sort()).toEqual(keys(enUS).sort());
  });

  it("validates locale codes", () => {
    expect(isAppLocale("pt-BR")).toBe(true);
    expect(isAppLocale("fr-FR")).toBe(false);
    expect(isAppLocale(null)).toBe(false);
  });
});

describe("runtime switching", () => {
  it("changes texts, the html lang attribute and the active locale", () => {
    const i18n = createAppI18n("en-US");
    expect(i18n.global.t("nav.settings")).toBe("Settings");
    applyLocale("pt-BR");
    expect(i18n.global.t("nav.settings")).toBe("Configurações");
    expect(document.documentElement.lang).toBe("pt-BR");
    expect(currentLocale()).toBe("pt-BR");
  });
});
