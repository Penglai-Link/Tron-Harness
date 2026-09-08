/// <reference types="vite/client" />
interface ImportMetaEnv {
  readonly VITE_TRON_API_BASE_URL?: string;
  readonly VITE_TRON_TERMINAL_APP_CODE?: string;
  readonly VITE_TRON_TERMINAL_MODEL_CODE?: string;
  readonly VITE_TRON_TERMINAL_WORKSPACE_CODE?: string;
  readonly VITE_TRON_AUTH_DATA_SOURCE?: "mock" | "api";
  readonly VITE_TRON_DATA_SOURCE?: "mock" | "api";
  readonly [key: `VITE_TRON_${string}_DATA_SOURCE`]: "mock" | "api" | undefined;
}

interface Window {
  tronDesktop?: {
    targetPreloadUrl?: string;
    invoke?: (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
  };
  __TAURI__?: {
    core?: {
      invoke?: (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
    };
    invoke?: (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
  };
}
