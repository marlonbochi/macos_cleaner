import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";

export const routes: RouteRecordRaw[] = [
  { path: "/", name: "scan", component: () => import("@/views/ScanView.vue") },
  { path: "/files", name: "files", component: () => import("@/views/FilesView.vue") },
  { path: "/settings", name: "settings", component: () => import("@/views/SettingsView.vue") },
];

export function createAppRouter() {
  // Hash history: the app is served from a custom protocol, not a web server.
  return createRouter({ history: createWebHashHistory(), routes });
}
