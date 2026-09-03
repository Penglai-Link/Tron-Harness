import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";
import { defineConfig, loadEnv } from "vite";

const DEFAULT_BACKEND_PROXY_TARGET = "http://127.0.0.1:8080";

export default defineConfig(({ mode }) => {
  const runtimeEnv = {
    ...loadEnv(mode, fileURLToPath(new URL(".", import.meta.url)), ""),
    ...process.env,
  };
  const apiBaseUrl = runtimeEnv.VITE_TRON_API_BASE_URL ?? "";
  const backendProxyTarget = runtimeEnv.TRON_FRONTEND_API_PROXY_TARGET ?? DEFAULT_BACKEND_PROXY_TARGET;
  const frontendOutDir = runtimeEnv.TRON_FRONTEND_OUT_DIR;

  return {
    plugins: [react(), tailwindcss()],
    build: {
      // Desktop packages are distributed to end users.  Keep source maps out
      // of every production build, and state the minifier explicitly so a
      // future Vite default change cannot expose readable source by accident.
      sourcemap: false,
      minify: "esbuild",
      rollupOptions: {
        output: {
          // Force lucide-react to be fully bundled for Electron compatibility
          inlineDynamicImports: false,
          manualChunks: (id) => {
            // Bundle lucide-react into a single chunk to avoid module resolution issues
            if (id.includes('node_modules/lucide-react')) {
              return 'lucide-react';
            }
          },
        },
      },
      ...(frontendOutDir
        ? {
            emptyOutDir: true,
            outDir: frontendOutDir,
          }
        : {}),
    },
    server: {
      host: "0.0.0.0",
      proxy: apiBaseUrl
        ? undefined
        : {
            "/api": {
              changeOrigin: true,
              target: backendProxyTarget,
            },
            "/socket.io": {
              changeOrigin: true,
              target: backendProxyTarget,
              ws: true,
            },
          },
    },
    resolve: {
      alias: {
        "@": fileURLToPath(new URL("./src", import.meta.url)),
      },
    },
  };
});
