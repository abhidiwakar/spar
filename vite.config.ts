import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

const host = process.env.TAURI_DEV_HOST;

function stripProblemSecrets(): Plugin {
  return {
    name: "strip-problem-secrets",
    enforce: "pre",
    transform(code, id) {
      if (!id.includes("/content/problems/") || !id.endsWith(".json")) return null;
      const data = JSON.parse(code) as { editorial?: unknown; tests?: { hidden?: unknown } };
      delete data.editorial;
      if (data.tests) delete data.tests.hidden;
      return { code: JSON.stringify(data), map: null };
    },
  };
}

export default defineConfig({
  plugins: [stripProblemSecrets(), react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
});
