const DESKTOP_BACKEND_STATUS_COMMAND = "desktopBackendStatus";
const CHECK_TARGET_PAGE_EMBEDDING_COMMAND = "checkTargetPageEmbedding";
const OPEN_TARGET_PAGE_COMMAND = "openTargetPage";
const INJECTION_EVENT_COMMAND = "desktopInjectionEvent";

export type DesktopTargetMode = "browser" | "iframe" | "view" | "window";

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

export interface CheckTargetPageEmbeddingRequest {
  url: string;
}

export interface CheckTargetPageEmbeddingResponse {
  url: string;
  embeddable: boolean;
  blocked: boolean;
  reason?: string;
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

export type DesktopInjectionEventType = "tron:frame-ready" | "tron:dom-event" | "tron:node-inspected" | "tron:inspection-exited" | "tron:navigation-requested" | "tron:navigation-changed";

export interface DesktopInjectionEventRequest {
  eventType: DesktopInjectionEventType;
  pageId: string;
  frameId: string;
  frameUrl?: string;
  timestamp?: number;
  payload?: Record<string, unknown>;
}

type InvokeCommand<T> = (command: string, payload: Record<string, unknown>) => Promise<T>;

type WindowWithElectronDesktop = Window & {
  tronDesktop?: {
    targetPreloadUrl?: string;
    invoke?: (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
  };
  __TAURI_INTERNALS__?: {
    invoke?: (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
  };
};

const ALLOWED_INJECTION_EVENTS = new Set<DesktopInjectionEventType>([
  "tron:frame-ready",
  "tron:dom-event",
  "tron:node-inspected",
  "tron:inspection-exited",
  "tron:navigation-requested",
  "tron:navigation-changed",
]);
const MAX_URL_LENGTH = 8192;
const MAX_IMAGE_DATA_URL_LENGTH = 80_000_000;
const URL_PAYLOAD_KEYS = new Set(["frameUrl", "href", "location", "url"]);

function resolveInvoke(): InvokeCommand<unknown> | null {
  if (typeof window === "undefined") {
    return null;
  }

  const desktopWindow = window as unknown as WindowWithElectronDesktop;
  const electronInvoke = desktopWindow.tronDesktop?.invoke;
  if (typeof electronInvoke === "function") {
    return (command, payload) => electronInvoke(command, payload);
  }

  const tauriInvoke = desktopWindow.__TAURI_INTERNALS__?.invoke;
  if (typeof tauriInvoke === "function") {
    return (command, payload) => tauriInvoke(command, payload);
  }

  return null;
}

function sanitizeString(value: unknown): string {
  if (typeof value !== "string") {
    return "";
  }

  return value.trim().slice(0, 160);
}

function sanitizeUrlString(value: unknown): string {
  if (typeof value !== "string") {
    return "";
  }

  return value.trim().slice(0, MAX_URL_LENGTH);
}

function sanitizePayloadString(key: string, value: string): string {
  const trimmed = value.trim();
  if (
    key === "dataUrl"
    && (trimmed.startsWith("data:image/svg+xml") || trimmed.startsWith("data:image/png"))
  ) {
    return trimmed.slice(0, MAX_IMAGE_DATA_URL_LENGTH);
  }

  if (URL_PAYLOAD_KEYS.has(key)) {
    return trimmed.slice(0, MAX_URL_LENGTH);
  }

  return trimmed.slice(0, 160);
}

function sanitizePayloadValue(value: unknown, key = "", depth = 0): unknown {
  if (typeof value === "string") {
    return sanitizePayloadString(key, value);
  }

  if (typeof value === "number" || typeof value === "boolean" || value === null) {
    return value;
  }

  if (Array.isArray(value)) {
    if (depth >= 3) {
      return undefined;
    }

    const output = value
      .slice(0, 20)
      .map((item) => sanitizePayloadValue(item, key, depth + 1))
      .filter((item) => item !== undefined);

    return output.length > 0 ? output : undefined;
  }

  if (value && typeof value === "object") {
    if (depth >= 3) {
      return undefined;
    }

    return sanitizePayload(value, depth + 1);
  }

  return undefined;
}

function sanitizePayload(payload: unknown, depth = 0): Record<string, unknown> | undefined {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return undefined;
  }

  const output: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(payload as Record<string, unknown>).slice(0, 40)) {
    const safeKey = sanitizeString(key);
    if (safeKey.length === 0 || safeKey.length > 80) {
      continue;
    }

    const safeValue = sanitizePayloadValue(value, safeKey, depth);
    if (safeValue !== undefined) {
      output[safeKey] = safeValue;
      continue;
    }
  }

  return Object.keys(output).length > 0 ? output : undefined;
}

function describeInvokeError(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  if (typeof error === "string" && error.trim()) {
    return error.trim();
  }

  return "desktop-command-failed";
}

export function isDesktopHost(): boolean {
  return resolveInvoke() !== null;
}

export function resolveTargetWebviewPreloadUrl(): string {
  if (typeof window === "undefined") {
    return "";
  }

  const value = (window as WindowWithElectronDesktop).tronDesktop?.targetPreloadUrl;
  return typeof value === "string" && value.startsWith("file:") ? value : "";
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

    const payload: {
      mode: DesktopTargetMode;
      pageId: string;
      url: string;
    } = {
      mode: request.mode ?? "iframe",
      pageId,
      url: sanitizeUrlInput(request.url),
    };
    let response: unknown;
    try {
      response = await this.invoke(OPEN_TARGET_PAGE_COMMAND, { request: payload });
    } catch (error) {
      return {
        pageId,
        opened: false,
        mode: payload.mode,
        reason: describeInvokeError(error),
      };
    }

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

  public async checkTargetPageEmbedding(
    request: CheckTargetPageEmbeddingRequest,
  ): Promise<CheckTargetPageEmbeddingResponse> {
    const url = sanitizeUrlInput(request.url);

    if (!this.invoke) {
      return {
        url,
        embeddable: true,
        blocked: false,
        reason: "running-in-browser",
      };
    }

    try {
      const response = await this.invoke(CHECK_TARGET_PAGE_EMBEDDING_COMMAND, {
        request: {
          url,
        },
      });
      const payload = response as {
        url?: unknown;
        embeddable?: unknown;
        blocked?: unknown;
        reason?: unknown;
      };
      const blocked = Boolean(payload.blocked);

      return {
        url: sanitizePayloadString("url", String(payload.url ?? url)) || url,
        embeddable: Boolean(payload.embeddable ?? !blocked),
        blocked,
        reason: typeof payload.reason === "string" ? payload.reason : undefined,
      };
    } catch (error) {
      return {
        url,
        embeddable: true,
        blocked: false,
        reason: describeInvokeError(error),
      };
    }
  }

  public async emitInjectionEvent(event: DesktopInjectionEventRequest): Promise<void> {
    if (!this.invoke) {
      return;
    }

    const sanitized = sanitizeInjectionEventPayload(event);
    if (!sanitized) {
      return;
    }

    await this.invoke(INJECTION_EVENT_COMMAND, {
      request: {
        event: sanitized,
      },
    } as Record<string, unknown>);
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
    frameUrl: sanitizeUrlString(data.frameUrl),
    payload: sanitizePayload(data.payload),
    timestamp: Number(data.timestamp ?? Date.now()),
  };
}

export function sanitizeUrlInput(value: string): string {
  const normalized = sanitizeUrlString(value);
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
