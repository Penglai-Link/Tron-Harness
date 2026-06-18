/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_TRON_API_BASE_URL?: string;
  readonly VITE_TRON_AUTH_DATA_SOURCE?: "mock" | "api";
  readonly VITE_TRON_DATA_SOURCE?: "mock" | "api";
  readonly [key: `VITE_TRON_${string}_DATA_SOURCE`]: "mock" | "api" | undefined;
}

interface Window {
  __TAURI__?: {
    core?: {
      invoke?: (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
    };
    invoke?: (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
  };
}
