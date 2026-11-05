import { tracedFetch } from "@tron/chat-core";
import { applyCurrentLocaleHeader } from "@/i18n";
import { chatWorkspaceForSession } from "./chatWorkspaceScope";

export type EndpointParams = Record<string, string | number | boolean | null | undefined>;

export interface ApiRequestOptions {
  body?: BodyInit | object | unknown[];
  cache?: RequestCache;
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

let apiWorkspaceCode: string | null = null;
export function setApiWorkspaceCode(code: string | null) { apiWorkspaceCode = code; }

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

  return applyCurrentLocaleHeader(resolved);
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

  if (isWorkspaceChatPath(url.pathname) && !url.searchParams.has("workspaceCode")) {
    const sessionCode = (url.searchParams.get("sessionCode")
      ?? String(pathParams.sessionCode ?? pathParams.streamKey ?? ""))
      || decodeURIComponent(url.pathname.match(/\/api\/v1\/(?:chats\/sessions|agents\/agui\/streams)\/([^/]+)/)?.[1] ?? "");
    const workspaceCode = chatWorkspaceForSession(sessionCode) ?? apiWorkspaceCode;
    if (workspaceCode) url.searchParams.set("workspaceCode", workspaceCode);
  }
  return url.toString();
}

function isWorkspaceChatPath(path: string) {
  return path.startsWith("/api/v1/chats/") || path.startsWith("/api/v1/agents/agui/streams/") || path.startsWith("/api/v1/agents/runs");
}

function requestUrl(path: string, options: ApiRequestOptions) {
  const body = isRecord(options.body) ? options.body : undefined;
  const workspaceCode = typeof body?.workspaceCode === "string" ? body.workspaceCode
    : typeof body?.sessionCode === "string" ? chatWorkspaceForSession(body.sessionCode) : undefined;
  return buildApiUrl(path, options.pathParams, {
    ...(isWorkspaceChatPath(path) && workspaceCode ? { workspaceCode } : {}),
    ...options.queryParams,
  });
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
  return executeApiRequest<T>(endpoint, options, requestUrl(parseEndpoint(endpoint).path, options));
}

async function executeApiRequest<T>(endpoint: string, options: ApiRequestOptions, url: string): Promise<T> {
  const { method } = parseEndpoint(endpoint);
  const headers = resolveHeaders(options.body, options.headers);
  const response = await tracedFetch(url, {
    body: resolveRequestBody(options.body),
    cache: options.cache,
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
    return executeApiRequest<T>(endpoint, {
      ...options,
      retryOnUnauthorized: false,
    }, url);
  }

  if (
    response.status === 401
    && options.retryOnUnauthorized !== false
    && unauthorizedHandler
    && await unauthorizedHandler()
  ) {
    return executeApiRequest<T>(endpoint, {
      ...options,
      retryOnUnauthorized: false,
    }, url);
  }

  if (!response.ok) {
    throw createApiError(response, payload, method);
  }

  return unwrapApiEnvelope<T>(payload);
}

export interface ApiBlobResponse {
  blob: Blob;
  contentDisposition: string | null;
  contentType: string | null;
}

/**
 * The Resource binary download endpoint returns a raw stream on success and
 * the standard JSON envelope on failure. Keep that distinction here so callers
 * do not accidentally attempt `response.json()` on a binary body.
 */
export async function requestApiBlob(
  endpoint: string,
  options: ApiRequestOptions = {},
): Promise<ApiBlobResponse> {
  return executeBlobRequest(endpoint, options, requestUrl(parseEndpoint(endpoint).path, options));
}

async function executeBlobRequest(endpoint: string, options: ApiRequestOptions, url: string): Promise<ApiBlobResponse> {
  const { method } = parseEndpoint(endpoint);
  const headers = resolveHeaders(options.body, options.headers);
  const response = await tracedFetch(url, {
    body: resolveRequestBody(options.body),
    credentials: options.credentials ?? "include",
    headers,
    method,
    signal: options.signal,
  });

  if (
    response.status === 401
    && options.retryOnUnauthorized !== false
    && apiAccessToken
    && readBearerToken(headers) !== apiAccessToken
  ) {
    return executeBlobRequest(endpoint, {
      ...options,
      retryOnUnauthorized: false,
    }, url);
  }

  if (
    response.status === 401
    && options.retryOnUnauthorized !== false
    && unauthorizedHandler
    && await unauthorizedHandler()
  ) {
    return executeBlobRequest(endpoint, {
      ...options,
      retryOnUnauthorized: false,
    }, url);
  }

  if (!response.ok) {
    const payload = await response.json().catch(() => null);
    throw createApiError(response, payload, method);
  }

  return {
    blob: await response.blob(),
    contentDisposition: response.headers.get("Content-Disposition"),
    contentType: response.headers.get("Content-Type"),
  };
}
