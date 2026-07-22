import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";
import { defineConfig, loadEnv } from "vite";
import dts from "vite-plugin-dts";

const DEFAULT_BACKEND_PROXY_TARGET = "http://127.0.0.1:8080";
const packageRoot = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig(({ mode }) => {
  const runtimeEnv = {
    ...loadEnv(mode, fileURLToPath(new URL(".", import.meta.url)), ""),
    ...process.env,
  };
  const apiBaseUrl = runtimeEnv.VITE_TRON_API_BASE_URL ?? "";
  const backendProxyTarget =
    runtimeEnv.TRON_FRONTEND_API_PROXY_TARGET ?? DEFAULT_BACKEND_PROXY_TARGET;

  return {
    root: packageRoot,
    plugins: [
      react(),
      tailwindcss(),
      dts({
        entryRoot: fileURLToPath(new URL("./src", import.meta.url)),
        exclude: ["src/**/*.test.ts", "src/**/*.test.tsx", "src/testLocalStorage.ts"],
        include: ["src/**/*.ts", "src/**/*.tsx"],
        tsconfigPath: fileURLToPath(new URL("./tsconfig.build.json", import.meta.url)),
      }),
    ],
    build: {
      lib: {
        cssFileName: "styles",
        entry: fileURLToPath(new URL("./src/index.ts", import.meta.url)),
        fileName: "index",
        formats: ["es"],
      },
      rollupOptions: {
        external: [
          "@tron/chat-core",
          "lucide-react",
          "react",
          "react-dom",
          "react/jsx-runtime",
          "react-markdown",
          "remark-gfm",
        ],
      },
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
          },
    },
    resolve: {
      alias: {
        "@": fileURLToPath(new URL("./src", import.meta.url)),
      },
    },
  };
});
