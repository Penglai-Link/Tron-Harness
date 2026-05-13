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

  return {
    plugins: [react(), tailwindcss()],
    server: {
      host: "0.0.0.0",
      proxy: apiBaseUrl
        ? undefined
        : {
            "/api": {
              changeOrigin: true,
              target: backendProxyTarget,
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
