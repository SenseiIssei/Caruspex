import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The Tauri window loads the dev server on a fixed port, see
// src-tauri/tauri.conf.json.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 5287, strictPort: true },
  build: {
    target: "es2022",
    chunkSizeWarningLimit: 4096,
  },
});
