import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [react()],
  // Tauri owns the terminal output, so keep Vite from wiping it.
  clearScreen: false,
  server: {
    // Fixed so tauri.conf.json can point at it, and derived from the project name
    // so two of these apps can run side by side in dev.
    port: 1832,
    strictPort: true,
    watch: {
      // Rust rebuilds are driven by cargo, not by Vite.
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    // WebView2 on Windows 10+ tracks Chromium, so no legacy transpilation.
    target: "esnext",
  },
  test: {
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
