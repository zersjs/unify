import { fileURLToPath, URL } from "node:url";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

function normalizeBasePath(raw: string | undefined): string {
  const trimmed = raw?.trim();
  if (!trimmed || trimmed === "/") {
    return "/";
  }
  const withLeading = trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
  return withLeading.endsWith("/") ? withLeading : `${withLeading}/`;
}

export default defineConfig({
  base: normalizeBasePath(process.env.VITE_BASE_PATH),
  plugins: [react()],
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  clearScreen: false,
  server: { port: 4173, strictPort: false, host: "127.0.0.1" },
  envPrefix: ["VITE_"],
  build: { target: "es2022", sourcemap: false, outDir: "dist", emptyOutDir: true },
  test: { environment: "jsdom", include: ["src/**/*.test.ts", "src/**/*.test.tsx"] },
});
