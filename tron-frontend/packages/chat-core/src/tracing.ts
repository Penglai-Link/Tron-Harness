/** First-party browser spans. Query strings and request/response bodies are never exported. */
export const BROWSER_SPANS_PATH = "/api/v1/observability/browser-spans";
function identifier(bytes: number) {
  const data = crypto.getRandomValues(new Uint8Array(bytes));
  if (data.every((byte) => byte === 0)) data[0] = 1;
  return Array.from(data, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
function ignored(path: string) {
  return path.startsWith("/api/v1/observability/") ||
    (path.startsWith("/api/v1/agents/agui/streams/") && (path.endsWith("/protocol-events") || path.endsWith("/events"))) ||
    (path.startsWith("/api/v1/chats/sessions/") && (path.endsWith("/subagents") || path.includes("/subagents/") || path.endsWith("/history/query"))) ||
    ["/api/v1/health", "/api/v1/ready", "/api/v1/readiness", "/health", "/ready", "/api/v1/profiles/events", "/api/v1/system/skins/events"].includes(path);
}
type BrowserRequestFailure = "Cancelled" | "Timeout" | "Network" | "Http";
type BrowserSpan = {
  kind: "Page" | "Resource" | "Request"; phase: string | null;
  traceId: string; spanId: string; parentSpanId: string | null;
  startedAt: number; endedAt: number; method: string; path: string; status: number;
  failure?: BrowserRequestFailure;
};
type PageScope = { traceId: string; spanId: string; path: string; startedAt: number; endedAt: number; status: number; failure?: BrowserRequestFailure };
let page: PageScope | undefined;
const pages: PageScope[] = [];
let destination: { endpoint: URL; headers: Headers; credentials: RequestCredentials } | undefined;
let apiOrigin: string | undefined;
let installed = false;
let pending: BrowserSpan[] = [];
let flushTimer: ReturnType<typeof setTimeout> | undefined;
function send(span: BrowserSpan) {
  const index = pending.findIndex((item) => item.traceId === span.traceId && item.spanId === span.spanId);
  if (index >= 0) {
    const previous = pending[index];
    pending[index] = { ...span, endedAt: Math.max(previous.endedAt, span.endedAt) };
  } else if (pending.length < 1000) pending.push(span);
  if (destination && !flushTimer) flushTimer = setTimeout(() => { flushTimer = undefined; flush(); }, 200);
}
function root(scope: PageScope, endedAt = Date.now()) {
  scope.endedAt = Math.max(scope.endedAt, endedAt);
  send({ ...scope, kind: "Page", phase: "navigation", parentSpanId: null,
    endedAt: Math.max(scope.startedAt, scope.endedAt), method: "GET" });
}
function flush() {
  if (!destination || !pending.length) return;
  const spans = pending.splice(0, 40);
  void fetch(destination.endpoint, {
    method: "POST", credentials: destination.credentials, keepalive: true,
    headers: destination.headers, body: JSON.stringify({ spans }),
  }).catch(() => { /* Telemetry must not affect application behavior. */ }).finally(() => {
    if (pending.length && !flushTimer) flushTimer = setTimeout(() => { flushTimer = undefined; flush(); }, 200);
  });
}
function observe(entries: PerformanceEntry[]) {
  if (!page) return;
  for (const entry of entries) {
    const timing = entry as PerformanceResourceTiming;
    if (entry.entryType === "resource" && ["fetch", "xmlhttprequest", "beacon"].includes(timing.initiatorType)) continue;
    const url = new URL(entry.name, location.origin);
    if (url.origin !== location.origin || ignored(url.pathname) || url.pathname.startsWith("/api/")) continue;
    const start = Math.round(performance.timeOrigin + entry.startTime);
    const scope = [...pages].reverse().find((item) => item.startedAt <= start);
    if (!scope) continue;
    const parent = identifier(8);
    send({ kind: "Resource", phase: timing.initiatorType || "document", traceId: scope.traceId,
      spanId: parent, parentSpanId: scope.spanId, startedAt: start,
      endedAt: start + Math.round(entry.duration), method: "GET", path: url.pathname, status: timing.responseStatus || 0 });
    for (const [phase, from, to] of [
      ["dns", timing.domainLookupStart, timing.domainLookupEnd],
      ["connect", timing.connectStart, timing.connectEnd],
      ["request", timing.requestStart, timing.responseStart],
      ["download", timing.responseStart, timing.responseEnd],
    ] as const) {
      if (to <= from || from <= 0) continue;
      send({ kind: "Resource", phase, traceId: scope.traceId, spanId: identifier(8), parentSpanId: parent,
        startedAt: Math.round(performance.timeOrigin + from), endedAt: Math.round(performance.timeOrigin + to),
        method: "GET", path: url.pathname, status: 200 });
    }
    root(scope, start + Math.round(entry.duration));
  }
}
/** Initialize once before React renders; the caller supplies the configured first-party API origin. */
export function installBrowserTracing(apiUrl: string) {
  if (installed || typeof window === "undefined") return;
  installed = true;
  apiOrigin = new URL(apiUrl, location.origin).origin;
  const begin = (initial = false) => {
    page = { traceId: identifier(16), spanId: identifier(8), path: location.pathname,
      startedAt: initial ? Math.round(performance.timeOrigin) : Date.now(), endedAt: Date.now(), status: 200 };
    pages.push(page);
    if (pages.length > 50) pages.shift();
    root(page);
  };
  begin(true);
  for (const method of ["pushState", "replaceState"] as const) {
    const original = history[method].bind(history);
    history[method] = (...args: Parameters<History[typeof method]>) => {
      const before = location.pathname + location.search;
      original(args[0], args[1], args[2]);
      if (before !== location.pathname + location.search) begin();
    };
  }
  window.addEventListener("popstate", () => begin());
  window.addEventListener("pagehide", () => flush());
  if (typeof PerformanceObserver !== "undefined") {
    for (const type of ["navigation", "resource"]) {
      try { new PerformanceObserver((list) => observe(list.getEntries())).observe({ type, buffered: true }); }
      catch { /* Unsupported performance entry types do not block startup. */ }
    }
  }
}
export async function tracedFetch(input: string | URL, init: RequestInit = {}): Promise<Response> {
  const origin = typeof window === "undefined" ? "http://localhost" : window.location.origin;
  const url = new URL(input, origin);
  if (url.origin !== (apiOrigin ?? origin) || !url.pathname.startsWith("/api/v1/") || ignored(url.pathname)) return fetch(input, init);
  const headers = new Headers(init.headers);
  const existing = headers.get("traceparent");
  const parent = existing?.match(/^00-([0-9a-f]{32})-([0-9a-f]{16})-([0-9a-f]{2})$/);
  const validParent = parent && !/^0+$/.test(parent[1]) && !/^0+$/.test(parent[2]);
  const scope = page;
  const traceId = validParent ? parent[1] : scope?.traceId ?? identifier(16);
  const parentSpanId = validParent ? parent[2] : scope?.spanId ?? null;
  const spanId = identifier(8);
  const flags = validParent ? parent[3] : "01";
  headers.set("traceparent", `00-${traceId}-${spanId}-${flags}`);
  const reportHeaders = new Headers({ "Content-Type": "application/json" });
  for (const name of ["Authorization", "Accept-Language", "X-Workspace-Code"]) {
    const value = headers.get(name);
    if (value) reportHeaders.set(name, value);
  }
  destination = { endpoint: new URL(BROWSER_SPANS_PATH, url.origin), headers: reportHeaders, credentials: init.credentials ?? "include" };
  const startedAt = Date.now();
  const started = performance.now();
  let status = 0;
  let failure: BrowserRequestFailure | undefined;
  try {
    const response = await fetch(input, { ...init, headers });
    status = response.status;
    if (status >= 400) failure = "Http";
    return response;
  } catch (error) {
    const name = error instanceof Error || error instanceof DOMException ? error.name : "";
    const reason = init.signal?.reason;
    const reasonName = reason instanceof Error || reason instanceof DOMException ? reason.name : "";
    failure = name === "TimeoutError" || (init.signal?.aborted && reasonName === "TimeoutError")
      ? "Timeout"
      : init.signal?.aborted || name === "AbortError" ? "Cancelled" : "Network";
    // Classify the failure without exporting arbitrary abort reasons, URLs or payloads.
    throw error;
  } finally {
    const endedAt = startedAt + Math.round(performance.now() - started);
    send({ kind: "Request", phase: "response-headers", traceId, spanId, parentSpanId, startedAt, endedAt,
      method: init.method ?? "GET", path: url.pathname, status, failure });
    if (scope && scope.traceId === traceId) {
      if (failure !== "Cancelled" && (status === 0 || status >= 400)) { scope.status = status; scope.failure = failure; }
      root(scope, endedAt);
    }

  }
}
