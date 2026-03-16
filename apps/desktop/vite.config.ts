// =============================================================================
// Vite Configuration for Fokus Desktop
// =============================================================================
// Vite is the build tool that compiles our Svelte + TypeScript into
// optimized HTML/CSS/JS that Tauri's WebView can load.
//
// WHY VITE?
// - Extremely fast dev server (HMR in <50ms)
// - Modern ES module support
// - First-class Svelte support
// - Tauri recommends it as the default bundler
// =============================================================================

import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "path";

export default defineConfig({
  plugins: [svelte()],

  // Resolve the $lib alias so imports like "$lib/api" work
  resolve: {
    alias: {
      $lib: path.resolve(__dirname, "./src/lib"),
    },
  },

  // Tauri expects a fixed port for development
  server: {
    port: 5173,
    strictPort: true,
  },

  // Clear the terminal on build (keeps output clean)
  clearScreen: false,

  // Environment variable prefix for Tauri
  envPrefix: ["VITE_", "TAURI_"],

  build: {
    // Tauri uses Chromium on Windows and WebKit on macOS, both of
    // which support modern ES2021 features. No need for legacy polyfills.
    target: "esnext",
    // Don't minify in debug builds for easier debugging
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
