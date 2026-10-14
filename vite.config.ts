import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri expects a fixed port in dev and a relative base so assets load from file://.
export default defineConfig({
  plugins: [react()],
  // Vite builds to ../dist, which tauri.conf.json points at as frontendDist.
  base: "./",
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    target: "es2021",
    sourcemap: false,
  },
  test: {
    environment: "jsdom",
    globals: true,
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
