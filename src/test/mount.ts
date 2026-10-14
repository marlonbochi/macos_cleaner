import { mount, type ComponentMountingOptions } from "@vue/test-utils";
import { createPinia, setActivePinia, type Pinia } from "pinia";
import { createMemoryHistory, createRouter, type Router } from "vue-router";
import type { Component } from "vue";
import { createAppI18n } from "@/i18n";
import { routes } from "@/router";

export interface Mounted {
  router: Router;
  pinia: Pinia;
}

/** Mounts a component with the same plugins as the app (i18n, Pinia, in-memory router). */
export async function mountWithPlugins<C extends Component>(
  component: C,
  options: ComponentMountingOptions<C> = {},
  { pinia = createPinia(), path = "/" }: { pinia?: Pinia; path?: string } = {},
) {
  setActivePinia(pinia);
  const router = createRouter({ history: createMemoryHistory(), routes });
  await router.push(path);
  await router.isReady();
  const wrapper = mount(component, {
    ...options,
    global: { plugins: [createAppI18n(), pinia, router], ...options.global },
  });
  return { wrapper, router, pinia };
}
