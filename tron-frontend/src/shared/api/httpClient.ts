import { fallbackLocale, i18n } from "@/i18n";

export type EndpointParams = Record<string, string | number | boolean | null | undefined>;

interface ApiRequestOptions {
  body?: BodyInit | object | unknown[];
  credentials?: RequestCredentials;
  headers?: HeadersInit;
  pathParams?: EndpointParams;
  queryParams?: EndpointParams;
  retryOnUnauthorized?: boolean;
  signal?: AbortSignal;
}

export class ApiError extends Error {
  code?: string;
  data?: unknown;
  status: number;

  constructor(message: string, status: number, options?: { code?: string; data?: unknown }) {
    super(message);
    this.name = "ApiError";
    this.code = options?.code;
    this.data = options?.data;
    this.status = status;
  }
}

let apiAccessToken: string | null = null;
let unauthorizedHandler: (() => Promise<boolean>) | null = null;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readBearerToken(headers: Headers) {
  const authorization = headers.get("Authorization");
  if (!authorization) {
    return null;
  }

  const [scheme, token] = authorization.trim().split(/\s+/, 2);
  if (!scheme || !token || scheme.toLowerCase() !== "bearer") {
    return null;
  }

  return token;
}

function resolveRequestBody(body: ApiRequestOptions["body"]) {
  if (body === undefined) return undefined;
  if (
    body instanceof Blob
    || body instanceof FormData
    || body instanceof URLSearchParams
    || typeof body === "string"
  ) {
    return body;
  }

  return JSON.stringify(body);
}

function resolveHeaders(body: ApiRequestOptions["body"], headers?: HeadersInit) {
  const resolved = new Headers(headers);

  if (!(body === undefined || body instanceof Blob || body instanceof FormData || body instanceof URLSearchParams || typeof body === "string")) {
    resolved.set("Content-Type", "application/json");
  }

  if (apiAccessToken && !resolved.has("Authorization")) {
    resolved.set("Authorization", `Bearer ${apiAccessToken}`);
  }

  if (!resolved.has("Accept-Language")) {
    const locale = i18n.resolvedLanguage ?? i18n.language ?? fallbackLocale;
    resolved.set("Accept-Language", locale);
  }

  return resolved;
}

function createApiError(response: Response, payload: unknown, method: string) {
  const message = isRecord(payload) && typeof payload.message === "string"
    ? payload.message
    : `${method} ${response.url} failed with ${response.status}`;
  const code = isRecord(payload) && typeof payload.code === "string" ? payload.code : undefined;
  const data = isRecord(payload) && "data" in payload ? payload.data : undefined;
  return new ApiError(message, response.status, { code, data });
}

export function setApiUnauthorizedHandler(handler: (() => Promise<boolean>) | null) {
  unauthorizedHandler = handler;
}

export function getApiAccessToken() {
  return apiAccessToken;
}

export function setApiAccessToken(token: string | null) {
  apiAccessToken = token && token.trim() ? token.trim() : null;
}

export function isApiError(error: unknown): error is ApiError {
  return error instanceof ApiError;
}

export function isUnauthorizedApiError(error: unknown): error is ApiError {
  return isApiError(error) && error.status === 401;
}

export function getApiBaseUrl() {
  return (import.meta.env as Record<string, string | undefined>).VITE_TRON_API_BASE_URL ?? "";
}

export function parseEndpoint(endpoint: string) {
  const [first, second] = endpoint.split(/\s+/, 2);
  const method = second ? first : "GET";
  const path = second ?? first;

  if (!path) {
    throw new Error(`Invalid endpoint definition: ${endpoint}`);
  }

  return { method, path };
}

export function buildApiUrl(
  path: string,
  pathParams: EndpointParams = {},
  queryParams: EndpointParams = {},
) {
  const resolvedPath = path.replace(/\{([^}]+)\}/g, (_, key: string) => encodeURIComponent(String(pathParams[key] ?? "")));
  const url = new URL(`${getApiBaseUrl()}${resolvedPath}`, window.location.origin);

  Object.entries(queryParams).forEach(([key, value]) => {
    if (value === undefined || value === null || value === "") return;
    url.searchParams.set(key, String(value));
  });

  return url.toString();
}

export function unwrapApiEnvelope<T>(payload: unknown): T {
  if (!isRecord(payload) || !("code" in payload) || !("data" in payload)) {
    return payload as T;
  }

  if (payload.code !== "SUCCESS" && payload.code !== "0") {
    const message = typeof payload.message === "string" ? payload.message : "API request failed";
    throw new ApiError(message, 200, {
      code: typeof payload.code === "string" ? payload.code : undefined,
      data: payload.data,
    });
  }

  return payload.data as T;
}

export async function requestApi<T>(endpoint: string, options: ApiRequestOptions = {}): Promise<T> {
  const { method, path } = parseEndpoint(endpoint);
  const headers = resolveHeaders(options.body, options.headers);
  const response = await fetch(buildApiUrl(path, options.pathParams, options.queryParams), {
    body: resolveRequestBody(options.body),
    credentials: options.credentials ?? "include",
    headers,
    method,
    signal: options.signal,
  });
  const payload = await response.json().catch(() => null);

  if (
    response.status === 401
    && options.retryOnUnauthorized !== false
    && apiAccessToken
    && readBearerToken(headers) !== apiAccessToken
  ) {
    return requestApi<T>(endpoint, {
      ...options,
      retryOnUnauthorized: false,
    });
  }

  if (
    response.status === 401
    && options.retryOnUnauthorized !== false
    && unauthorizedHandler
    && await unauthorizedHandler()
  ) {
    return requestApi<T>(endpoint, {
      ...options,
      retryOnUnauthorized: false,
    });
  }

  if (!response.ok) {
    throw createApiError(response, payload, method);
  }

  return unwrapApiEnvelope<T>(payload);
}
