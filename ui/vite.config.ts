import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  server: {
    // Avoid 5173 (Vite default) — some local security software intercepts it
    // and force-redirects HTTP→HTTPS, breaking Tauri's webview connection.
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    allowedHosts: true,
    watch: {
      ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**"],
    },
  },
});
