// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tracedFetch } from "@tron/chat-core";

beforeEach(() => vi.useFakeTimers());
afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); });
describe("browser HTTP tracing", () => {
  it("exports a measured client span whose ID is propagated to the server", async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response("{}", { status: 200 }));
    vi.stubGlobal("fetch", fetcher);
    await tracedFetch("/api/v1/chats/sessions", { method: "POST", headers: { Authorization: "Bearer test" } });
    await vi.runAllTimersAsync();
    const headers = new Headers(fetcher.mock.calls[0][1].headers);
    const parent = headers.get("traceparent")!;
    expect(parent).toMatch(/^00-[0-9a-f]{32}-[0-9a-f]{16}-01$/);
    await vi.runAllTimersAsync();
    const report = JSON.parse(fetcher.mock.calls[1][1].body).spans[0];
    expect(parent).toBe(`00-${report.traceId}-${report.spanId}-01`);
    expect(report.endedAt).toBeGreaterThanOrEqual(report.startedAt);
    expect(report.status).toBe(200);
    expect(new Headers(fetcher.mock.calls[1][1].headers).get("Authorization")).toBe("Bearer test");
  });
  it("creates a distinct child span while retaining explicit upstream trace context", async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response("{}"));
    vi.stubGlobal("fetch", fetcher);
    await tracedFetch("/api/v1/chats/sessions", { headers: { traceparent: "00-11111111111111111111111111111111-2222222222222222-01" } });
    await vi.runAllTimersAsync();
    const report = JSON.parse(fetcher.mock.calls[1][1].body).spans[0];
    expect(report.traceId).toBe("11111111111111111111111111111111");
    expect(report.parentSpanId).toBe("2222222222222222");
    expect(report.spanId).not.toBe(report.parentSpanId);
  });
  it("preserves network errors even when reporting fails", async () => {
    const failure = new TypeError("offline");
    const fetcher = vi.fn().mockRejectedValue(failure);
    vi.stubGlobal("fetch", fetcher);
    await expect(tracedFetch("/api/v1/chats/sessions")).rejects.toBe(failure);
    await vi.runAllTimersAsync();
    expect(JSON.parse(fetcher.mock.calls[1][1].body).spans[0].status).toBe(0);
  });
  it("does not recursively instrument exports or non API traffic", async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response(""));
    vi.stubGlobal("fetch", fetcher);
    await tracedFetch("/api/v1/observability/browser-spans");
    await tracedFetch("https://example.org/image.png");
    expect(fetcher).toHaveBeenCalledTimes(2);
    expect(fetcher.mock.calls[0][1]).toEqual({});
  });
});
