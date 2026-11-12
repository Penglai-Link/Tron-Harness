/// <reference types="vite/client" />

declare const __TRON_APP_VERSION__: string;
declare const __TRON_RELEASE_NOTES__: Readonly<Record<"zh-CN" | "en-US", readonly {
  version: string;
  content: string;
}[]>>;

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
