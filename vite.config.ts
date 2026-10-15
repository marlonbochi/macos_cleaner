/// <reference types="vitest/config" />
import { fileURLToPath, URL } from "node:url";
import process from "node:process";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [vue()],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`:
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    // 3. tell Vite to ignore watching the Rust side
    watch: { ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**"] },
  },

  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts", "scripts/**/*.test.mjs"],
    restoreMocks: true,
    coverage: {
      provider: "v8" as const,
      include: ["src/**/*.{ts,vue}"],
      exclude: [
        "src/main.ts",
        "src/test/**",
        "src/types/generated/**",
        "src/**/*.test.ts",
        "src/**/*.d.ts",
      ],
      thresholds: { lines: 70, functions: 70, statements: 70, branches: 65 },
    },
  },
}));
