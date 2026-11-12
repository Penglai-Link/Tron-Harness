import { readFileSync } from "node:fs";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";
import { defineConfig, loadEnv } from "vite";

const DEFAULT_BACKEND_PROXY_TARGET = "http://127.0.0.1:8080";

function readProductVersion() {
  const versionProperties = readFileSync(new URL("../version.properties", import.meta.url), "utf8");
  return versionProperties.match(/^revision\s*=\s*(\S+)\s*$/m)?.[1] ?? "unknown";
}

function readReleaseNotes(version: string, file: string) {
  const changelog = readFileSync(new URL(`../${file}`, import.meta.url), "utf8");
  const sections = changelog.split(/^##\s+/m).slice(1);
  const entries = sections.flatMap((section) => {
    const [heading, ...lines] = section.split(/\r?\n/);
    const entryVersion = heading.match(/^\[(\d+\.\d+\.\d+(?:-[\w.-]+)?(?:\+[\w.-]+)?)\]/)?.[1];
    if (!entryVersion) return [];
    const content = lines.join("\n").trim();
    if (!content) throw new Error(`${file} must contain release notes for [${entryVersion}]`);
    return [{ version: entryVersion, content }];
  });
  if (!entries.some((entry) => entry.version === version)) {
    throw new Error(`${file} must contain release notes for [${version}]`);
  }
  if (new Set(entries.map((entry) => entry.version)).size !== entries.length) {
    throw new Error(`${file} contains duplicate release versions`);
  }
  return entries.sort((left, right) => compareVersions(right.version, left.version));
}

function compareVersions(left: string, right: string) {
  const [leftCore, leftPre] = left.split("+", 1)[0].split(/-(.*)/);
  const [rightCore, rightPre] = right.split("+", 1)[0].split(/-(.*)/);
  const coreOrder = leftCore.localeCompare(rightCore, "en", { numeric: true });
  if (coreOrder) return coreOrder;
  if (leftPre === undefined || rightPre === undefined) {
    return Number(leftPre === undefined) - Number(rightPre === undefined);
  }
  const leftParts = leftPre.split(".");
  const rightParts = rightPre.split(".");
  for (let index = 0; index < Math.max(leftParts.length, rightParts.length); index += 1) {
    const a = leftParts[index];
    const b = rightParts[index];
    if (a === undefined || b === undefined) return leftParts.length - rightParts.length;
    if (a === b) continue;
    const aNumeric = /^\d+$/.test(a);
    const bNumeric = /^\d+$/.test(b);
    if (aNumeric !== bNumeric) return aNumeric ? -1 : 1;
    return aNumeric ? a.localeCompare(b, "en", { numeric: true }) : a < b ? -1 : 1;
  }
  return 0;
}

export default defineConfig(({ mode }) => {
  const runtimeEnv = {
    ...loadEnv(mode, fileURLToPath(new URL(".", import.meta.url)), ""),
    ...process.env,
  };
  const apiBaseUrl = runtimeEnv.VITE_TRON_API_BASE_URL ?? "";
  const backendProxyTarget = runtimeEnv.TRON_FRONTEND_API_PROXY_TARGET ?? DEFAULT_BACKEND_PROXY_TARGET;
  const frontendOutDir = runtimeEnv.TRON_FRONTEND_OUT_DIR;
  const productVersion = readProductVersion();
  const releaseNotes = {
    "zh-CN": readReleaseNotes(productVersion, "CHANGELOG.md"),
    "en-US": readReleaseNotes(productVersion, "CHANGELOG.en-US.md"),
  };
  if (JSON.stringify(releaseNotes["zh-CN"].map((entry) => entry.version))
    !== JSON.stringify(releaseNotes["en-US"].map((entry) => entry.version))) {
    throw new Error("Chinese and English changelogs must contain the same release versions");
  }

  return {
    define: {
      __TRON_APP_VERSION__: JSON.stringify(productVersion),
      __TRON_RELEASE_NOTES__: JSON.stringify(releaseNotes),
    },
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
            // Live desktop windows may still request chunks from the previous
            // build. Keep them for in-place development updates only.
            emptyOutDir: runtimeEnv.TRON_FRONTEND_KEEP_ASSETS !== "1",
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
      dedupe: ["react", "react-dom", "@personai/agent-ui-chat-core", "@personai/agent-ui-motion-react", "@personai/agent-ui-motion-core"],
      alias: {
        "@": fileURLToPath(new URL("./src", import.meta.url)),
      },
    },
  };
});
