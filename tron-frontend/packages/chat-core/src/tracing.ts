/** W3C context for each browser HTTP request; only first-party API traffic is exported. */
export const BROWSER_SPANS_PATH = "/api/v1/observability/browser-spans";
function identifier(bytes: number) {
  const data = crypto.getRandomValues(new Uint8Array(bytes));
  if (data.every((byte) => byte === 0)) data[0] = 1;
  return Array.from(data, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export async function tracedFetch(input: string | URL, init: RequestInit = {}): Promise<Response> {
  const url = new URL(input, typeof window === "undefined" ? "http://localhost" : window.location.origin);
  if (!url.pathname.startsWith("/api/v1/") || url.pathname === BROWSER_SPANS_PATH) {
    return fetch(input, init);
  }
  const headers = new Headers(init.headers);
  const existing = headers.get("traceparent");
  const parent = existing?.match(/^00-([0-9a-f]{32})-([0-9a-f]{16})-([0-9a-f]{2})$/);
  const validParent = parent && !/^0+$/.test(parent[1]) && !/^0+$/.test(parent[2]);
  const traceId = validParent ? parent[1] : identifier(16);
  const parentSpanId = validParent ? parent[2] : undefined;
  const spanId = identifier(8);
  const flags = validParent ? parent[3] : "01";
  headers.set("traceparent", `00-${traceId}-${spanId}-${flags}`);
  const startedAt = Date.now();
  const started = performance.now();
  let status = 0;
  try {
    const response = await fetch(input, { ...init, headers });
    status = response.status;
    return response;
  } finally {
    // Fetch resolves when response headers arrive; record this browser-measured request latency.
    const reportHeaders = new Headers({ "Content-Type": "application/json" });
    for (const name of ["Authorization", "Accept-Language", "X-Workspace-Code"]) {
      const value = headers.get(name);
      if (value) reportHeaders.set(name, value);
    }
    const endpoint = new URL(BROWSER_SPANS_PATH, url.origin);
    void fetch(endpoint, {
      method: "POST", credentials: init.credentials ?? "include", keepalive: true,
      headers: reportHeaders,
      body: JSON.stringify({ traceId, spanId, parentSpanId, startedAt,
        endedAt: startedAt + Math.round(performance.now() - started),
        method: init.method ?? "GET", path: url.pathname, status }),
    }).catch(() => { /* Telemetry failure must not change the application request result. */ });
  }
}
