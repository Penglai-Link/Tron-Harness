const DESKTOP_BACKEND_STATUS_COMMAND = "desktopBackendStatus";
const OPEN_TARGET_PAGE_COMMAND = "openTargetPage";
const CLOSE_TARGET_PAGE_COMMAND = "closeTargetPage";
const SET_INJECTION_ENABLED_COMMAND = "setInjectionEnabled";
const INJECTION_EVENT_COMMAND = "desktopInjectionEvent";

export type DesktopTargetMode = "iframe" | "window";

export interface DesktopBackendStatus {
  backendUrl: string;
  backendPort: number;
  ready: boolean;
  injectionEnabled: boolean;
  databasePath?: string;
  graphPath?: string;
}

export interface OpenTargetPageRequest {
  url: string;
  mode?: DesktopTargetMode;
  pageId?: string;
}

export interface OpenTargetPageResponse {
  pageId: string;
  opened: boolean;
  mode: DesktopTargetMode;
  reason?: string;
  backendUrl?: string;
}

export interface CloseTargetPageRequest {
  pageId: string;
}

export interface DesktopInjectionEvent {
  eventType: DesktopInjectionEventType;
  pageId: string;
  frameId: string;
  frameUrl?: string;
  timestamp: number;
  payload?: {
    [key: string]: unknown;
  };
}

export type DesktopInjectionEventType = "tron:frame-ready" | "tron:dom-event";

export interface DesktopInjectionEventRequest {
  eventType: DesktopInjectionEventType;
  pageId: string;
  frameId: string;
  frameUrl?: string;
  timestamp?: number;
  payload?: Record<string, unknown>;
}

type InvokeCommand<T> = (command: string, payload: Record<string, unknown>) => Promise<T>;

type WindowWithTauri = Window & {
  __TAURI__?: {
    core?: {
      invoke?: (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
    };
    invoke?: (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
  };
};

const ALLOWED_INJECTION_EVENTS = new Set<DesktopInjectionEventType>([
  "tron:frame-ready",
  "tron:dom-event",
]);

function resolveInvoke(): InvokeCommand<unknown> | null {
  if (typeof window === "undefined") {
    return null;
  }

  const host = (window as unknown as WindowWithTauri).__TAURI__;

  if (!host) {
    return null;
  }

  if (typeof host === "object" && host !== null) {
    const invoke = (host as { core?: { invoke?: InvokeCommand<unknown> } }).core?.invoke
      ?? (host as { invoke?: InvokeCommand<unknown> }).invoke;

    if (typeof invoke === "function") {
      return (command, payload) => invoke(command, payload);
    }
  }

  return null;
}

function sanitizeString(value: unknown): string {
  if (typeof value !== "string") {
    return "";
  }

  return value.trim().slice(0, 160);
}

function sanitizePayload(payload: unknown): Record<string, unknown> | undefined {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return undefined;
  }

  const output: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(payload as Record<string, unknown>)) {
    const safeKey = sanitizeString(key);
    if (safeKey.length === 0 || safeKey.length > 80) {
      continue;
    }

    if (typeof value === "string") {
      output[safeKey] = sanitizeString(value);
      continue;
    }

    if (typeof value === "number" || typeof value === "boolean" || value === null) {
      output[safeKey] = value;
      continue;
    }
  }

  return Object.keys(output).length > 0 ? output : undefined;
}

function normalizeTauriResponse<T>(value: unknown, fallback: T): T {
  return (value as T) ?? fallback;
}

export function isDesktopHost(): boolean {
  return resolveInvoke() !== null;
}

export function resolveDesktopPageId(pageSeed: string): string {
  const fallback = `desktop-${cryptoRandomId()}`;

  if (!pageSeed) {
    return fallback;
  }

  return sanitizeString(pageSeed).replace(/\s+/g, "-").replace(/[^A-Za-z0-9-_]/g, "").slice(0, 48) || fallback;
}

export function cryptoRandomId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }

  return `desktop-${Math.floor(Math.random() * 1_000_000_000)}`;
}

export class DesktopBridge {
  private readonly invoke: InvokeCommand<unknown> | null;

  constructor() {
    this.invoke = resolveInvoke();
  }

  public isDesktop(): boolean {
    return this.invoke !== null;
  }

  public async getBackendStatus(): Promise<DesktopBackendStatus> {
    if (!this.invoke) {
      return {
        backendUrl: "",
        backendPort: 0,
        ready: false,
        injectionEnabled: false,
      };
    }

    const response = await this.invoke(DESKTOP_BACKEND_STATUS_COMMAND, {});
    const payload = response as {
      backendUrl?: unknown;
      backendPort?: unknown;
      ready?: unknown;
      injectionEnabled?: unknown;
      databasePath?: unknown;
      graphPath?: unknown;
      backend_url?: unknown;
      backend_port?: unknown;
      injection_enabled?: unknown;
      database_path?: unknown;
      graph_path?: unknown;
    };

    return {
      backendUrl: sanitizeString(payload.backendUrl ?? payload.backend_url) || "",
      backendPort: Number(payload.backendPort ?? payload.backend_port ?? 0),
      ready: Boolean(payload.ready),
      injectionEnabled: Boolean(payload.injectionEnabled ?? payload.injection_enabled),
      databasePath: sanitizeString(payload.databasePath ?? payload.database_path),
      graphPath: sanitizeString(payload.graphPath ?? payload.graph_path),
    };
  }

  public async openTargetPage(request: OpenTargetPageRequest): Promise<OpenTargetPageResponse> {
    const pageId = request.pageId ?? resolveDesktopPageId(`page-${Date.now()}`);

    if (!this.invoke) {
      return {
        pageId,
        opened: false,
        mode: request.mode ?? "iframe",
        reason: "running-in-browser",
      };
    }

    const payload = {
      mode: request.mode ?? "iframe",
      pageId,
      url: sanitizeUrlInput(request.url),
    };
    const response = await this.invoke(OPEN_TARGET_PAGE_COMMAND, payload);
    const responseData = response as {
      pageId?: string;
      opened?: boolean;
      mode?: DesktopTargetMode;
      reason?: string;
      backendUrl?: string;
      page_id?: string;
      backend_url?: string;
      backendPort?: number;
      backend_port?: number;
    };

    return {
      pageId: responseData.pageId ?? pageId,
      opened: Boolean(responseData.opened),
      mode: responseData.mode ?? payload.mode,
      reason: typeof responseData.reason === "string" ? responseData.reason : undefined,
      backendUrl: sanitizeString(responseData.backendUrl ?? responseData.backend_url),
    };
  }

  public async closeTargetPage(request: CloseTargetPageRequest): Promise<boolean> {
    if (!this.invoke) {
      return false;
    }

    const response = await this.invoke(CLOSE_TARGET_PAGE_COMMAND, {
      pageId: sanitizeString(request.pageId),
    });
    if (typeof response === "boolean") {
      return response;
    }

    const payload = response as { closed?: boolean };

    return Boolean(normalizeTauriResponse(payload.closed, false));
  }

  public async setInjectionEnabled(enabled: boolean): Promise<boolean> {
    if (!this.invoke) {
      return enabled;
    }

    const response = await this.invoke(SET_INJECTION_ENABLED_COMMAND, {
      enabled: Boolean(enabled),
    });
    if (typeof response === "boolean") {
      return response;
    }

    const payload = response as { enabled?: boolean };

    return Boolean(normalizeTauriResponse(payload.enabled, enabled));
  }

  public async emitInjectionEvent(event: DesktopInjectionEventRequest): Promise<void> {
    if (!this.invoke) {
      return;
    }

    const sanitized = sanitizeInjectionEventPayload(event);
    if (!sanitized) {
      return;
    }

    await this.invoke(INJECTION_EVENT_COMMAND, { event: sanitized } as Record<string, unknown>);
  }
}

export function sanitizeInjectionEventPayload(raw: unknown): DesktopInjectionEvent | null {
  if (!raw || typeof raw !== "object") {
    return null;
  }

  const data = raw as DesktopInjectionEventRequest;

  if (!ALLOWED_INJECTION_EVENTS.has(data.eventType)) {
    return null;
  }

  const pageId = sanitizeString(data.pageId);
  const frameId = sanitizeString(data.frameId);
  if (!pageId || !frameId) {
    return null;
  }

  return {
    eventType: data.eventType,
    frameId,
    pageId,
    frameUrl: sanitizeString(data.frameUrl),
    payload: sanitizePayload(data.payload),
    timestamp: Number(data.timestamp ?? Date.now()),
  };
}

export function sanitizeUrlInput(value: string): string {
  const normalized = sanitizeString(value);
  if (!normalized) {
    return "";
  }

  try {
    const candidate = new URL(normalized);
    if (candidate.protocol === "http:" || candidate.protocol === "https:") {
      return candidate.href;
    }
  } catch {}

  try {
    const candidate = new URL(`https://${normalized}`);
    return candidate.href;
  } catch {
    return normalized;
  }
}
