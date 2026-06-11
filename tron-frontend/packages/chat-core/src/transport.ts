export type EndpointParams = Record<string, string | number | boolean | null | undefined>;
type MaybePromise<T> = T | Promise<T>;
export type BodyInitLike = BodyInit | ArrayBuffer | ArrayBufferView | object | unknown[];

export interface FetchTransportRequestContext {
  body?: BodyInitLike;
  method: string;
  path: string;
  rawQuery: string;
  signal?: AbortSignal;
  url: string;
}

export interface FetchTransportOptions {
  baseUrl?: string;
  credentials?: RequestCredentials;
  getHeaders?: (context: FetchTransportRequestContext) => MaybePromise<HeadersInit | undefined>;
  unwrapEnvelope?: boolean;
}

export interface ApiEnvelope<T> {
  code: string;
  data: T;
  message: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function buildApiUrl(
  baseUrl: string | undefined,
  path: string,
  pathParams: EndpointParams = {},
  queryParams: EndpointParams = {},
) {
  const resolvedPath = path.replace(/\{([^}]+)\}/g, (_, key: string) => encodeURIComponent(String(pathParams[key] ?? "")));
  const fallbackOrigin = typeof window === "undefined" ? "http://localhost" : window.location.origin;
  const url = new URL(`${baseUrl ?? ""}${resolvedPath}`, fallbackOrigin);
  Object.entries(queryParams).forEach(([key, value]) => {
    if (value === undefined || value === null || value === "") return;
    url.searchParams.set(key, String(value));
  });
  return url.toString();
}

export function unwrapEnvelope<T>(payload: unknown): T {
  if (!isRecord(payload) || !("code" in payload) || !("data" in payload)) {
    return payload as T;
  }
  if (payload.code !== "SUCCESS") {
    throw new Error(typeof payload.message === "string" ? payload.message : "API request failed");
  }
  return payload.data as T;
}

function resolveRequestBody(body: BodyInitLike | undefined) {
  if (body === undefined) return undefined;
  if (
    body instanceof Blob
    || body instanceof FormData
    || body instanceof URLSearchParams
    || typeof body === "string"
    || body instanceof ArrayBuffer
  ) {
    return body;
  }
  if (ArrayBuffer.isView(body)) {
    const bytes = new Uint8Array(body.byteLength);
    bytes.set(new Uint8Array(body.buffer, body.byteOffset, body.byteLength));
    return bytes;
  }

  return JSON.stringify(body);
}

function requestContextUrl(url: string) {
  const fallbackOrigin = typeof window === "undefined" ? "http://localhost" : window.location.origin;
  return new URL(url, fallbackOrigin);
}

export function createFetchTransport(options: FetchTransportOptions = {}) {
  async function requestJson<T>(args: {
    body?: BodyInitLike;
    method?: string;
    path: string;
    pathParams?: EndpointParams;
    queryParams?: EndpointParams;
    signal?: AbortSignal;
  }) {
    const body = resolveRequestBody(args.body);
    const method = args.method ?? "GET";
    const url = buildApiUrl(options.baseUrl, args.path, args.pathParams, args.queryParams);
    const parsedUrl = requestContextUrl(url);
    const headers = new Headers(await options.getHeaders?.({
      body: args.body,
      method,
      path: parsedUrl.pathname,
      rawQuery: parsedUrl.search.startsWith("?") ? parsedUrl.search.slice(1) : parsedUrl.search,
      signal: args.signal,
      url,
    }) ?? {});
    if (
      body
      && !(body instanceof Blob)
      && !(body instanceof FormData)
      && !(body instanceof URLSearchParams)
      && !(body instanceof ArrayBuffer)
      && !ArrayBuffer.isView(body)
      && typeof body !== "string"
    ) {
      headers.set("Content-Type", "application/json");
    }
    if (body && typeof body === "string" && !headers.has("Content-Type")) {
      headers.set("Content-Type", "application/json");
    }
    const response = await fetch(url, {
      body,
      credentials: options.credentials ?? "include",
      headers,
      method,
      signal: args.signal,
    });
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      if (payload && isRecord(payload) && typeof payload.message === "string") {
        throw new Error(payload.message);
      }
      throw new Error(`${args.method ?? "GET"} ${response.url} failed with ${response.status}`);
    }
    return (options.unwrapEnvelope === false ? payload : unwrapEnvelope<T>(payload)) as T;
  }

  async function streamSse<T>(args: {
    body?: BodyInitLike;
    handlers?: {
      onFrame?: (eventName: string, payload: Record<string, unknown>) => void;
    };
    method?: string;
    onResult: (payload: Record<string, unknown>) => T | null;
    path: string;
    pathParams?: EndpointParams;
    signal?: AbortSignal;
  }) {
    const method = args.method ?? "POST";
    const body = resolveRequestBody(args.body ?? {});
    const url = buildApiUrl(options.baseUrl, args.path, args.pathParams);
    const parsedUrl = requestContextUrl(url);
    const headers = new Headers(await options.getHeaders?.({
      body: args.body ?? {},
      method,
      path: parsedUrl.pathname,
      rawQuery: parsedUrl.search.startsWith("?") ? parsedUrl.search.slice(1) : parsedUrl.search,
      signal: args.signal,
      url,
    }) ?? {});
    headers.set("Content-Type", "application/json");
    const response = await fetch(url, {
      body: body === undefined ? undefined : body,
      credentials: options.credentials ?? "include",
      headers,
      method,
      signal: args.signal,
    });

    if (!response.ok) {
      const payload = await response.json().catch(() => null);
      if (payload) {
        return options.unwrapEnvelope === false
          ? payload as T
          : unwrapEnvelope<T>(payload);
      }
      throw new Error(`${args.method ?? "POST"} ${response.url} failed with ${response.status}`);
    }
    if (!response.body) {
      throw new Error("Streaming response did not include a body");
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    let result: T | null = null;

    function nextFrameBoundary(value: string): { index: number; separatorLength: number } | null {
      const lfBoundary = value.indexOf("\n\n");
      const crlfBoundary = value.indexOf("\r\n\r\n");
      if (lfBoundary < 0 && crlfBoundary < 0) return null;
      if (lfBoundary < 0) return { index: crlfBoundary, separatorLength: 4 };
      if (crlfBoundary < 0) return { index: lfBoundary, separatorLength: 2 };
      return lfBoundary < crlfBoundary
        ? { index: lfBoundary, separatorLength: 2 }
        : { index: crlfBoundary, separatorLength: 4 };
    }

    function consumeFrame(frame: string) {
      let eventName = "";
      const dataLines: string[] = [];
      frame.split(/\r?\n/).forEach((line) => {
        if (line.startsWith("event:")) eventName = line.slice(6).trim();
        if (line.startsWith("data:")) dataLines.push(line.slice(5).trimStart());
      });
      if (!dataLines.length) return;
      const payload = JSON.parse(dataLines.join("\n")) as Record<string, unknown>;
      args.handlers?.onFrame?.(eventName, payload);
      const nextResult = args.onResult(payload);
      if (nextResult) result = nextResult;
    }

    while (!result) {
      const { done, value } = await reader.read();
      if (value) {
        buffer += decoder.decode(value, { stream: !done });
        let boundary = nextFrameBoundary(buffer);
        while (boundary) {
          const frame = buffer.slice(0, boundary.index);
          buffer = buffer.slice(boundary.index + boundary.separatorLength);
          consumeFrame(frame);
          boundary = nextFrameBoundary(buffer);
        }
      }
      if (done) break;
    }

    if (!result && buffer.trim()) consumeFrame(buffer);
    if (!result) throw new Error("Chat stream ended before completion");
    return result;
  }

  return {
    requestJson,
    streamSse,
  };
}
