import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { createAppI18n } from "@/i18n";
import { provideApi } from "@/services/ipc";
import { FakeTransport } from "@/test/fakeTransport";
import { usePreferencesStore } from "./preferences";

describe("preferences store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("starts from the active locale, switches immediately and saves the choice", async () => {
    const i18n = createAppI18n("en-US");
    const transport = new FakeTransport().on("set_locale", () => undefined);
    provideApi(transport);
    const store = usePreferencesStore();
    expect(store.locale).toBe("en-US");

    await store.setLocale("pt-BR");

    expect(i18n.global.locale.value).toBe("pt-BR");
    expect(transport.lastCall("set_locale")).toEqual({ locale: "pt-BR" });
    expect(store.error).toBeNull();
  });

  it("keeps the new language on screen even if saving fails", async () => {
    const i18n = createAppI18n("pt-BR");
    provideApi(
      new FakeTransport().on("set_locale", () => {
        throw { code: "STORAGE", message: "disk full" };
      }),
    );
    const store = usePreferencesStore();
    await store.setLocale("en-US");
    expect(i18n.global.locale.value).toBe("en-US");
    expect(store.error).toBe("disk full");
  });
});
