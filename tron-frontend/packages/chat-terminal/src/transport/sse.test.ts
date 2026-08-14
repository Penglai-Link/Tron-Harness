// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const sseHarness = vi.hoisted(() => ({
  fetchEventSource: vi.fn(),
}));

vi.mock("@microsoft/fetch-event-source", () => ({
  fetchEventSource: sseHarness.fetchEventSource,
}));

import { SseAITransport } from "./sse";

describe("SseAITransport", () => {
  beforeEach(() => {
    sseHarness.fetchEventSource.mockReset();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("converts gateway SSE messages into AI chunks", async () => {
    sseHarness.fetchEventSource.mockImplementation(
      async (_url: string, options: Record<string, unknown>) => {
        const onmessage = options.onmessage as (event: { data: string }) => void;
        onmessage({ data: JSON.stringify(message("TEXT_MESSAGE_START", "")) });
        onmessage({
          data: JSON.stringify(message("TEXT_MESSAGE_CONTENT", "hello")),
        });
        onmessage({ data: JSON.stringify(message("TEXT_MESSAGE_END", "")) });
      },
    );
    const onChunk = vi.fn();
    const transport = new SseAITransport({
      headers: { Authorization: "Bearer token" },
      url: "http://terminal.test/chat",
    });

    await expect(transport.stream("question", { onChunk })).resolves.toEqual({
      requestId: "request-1",
    });
    expect(onChunk).toHaveBeenCalledWith({
      delta: "hello",
      requestId: "request-1",
    });

    const [, options] = sseHarness.fetchEventSource.mock.calls[0];
    expect(JSON.parse(options.body as string)).toEqual({
      messages: [{ content: "question", role: "user" }],
      stream: true,
    });
    expect((options.headers as Headers).get("Accept")).toBe(
      "text/event-stream",
    );
  });

  it("parses non-streaming gateway responses", async () => {
    const fetchMock = vi.fn(async () => ({
      json: async () => message("TEXT_MESSAGE_SUGGESTION", "npm test"),
      ok: true,
      status: 200,
    }));
    const transport = new SseAITransport({
      fetch: fetchMock as unknown as typeof fetch,
      url: "http://terminal.test/chat",
    });

    await expect(transport.request("generate command")).resolves.toBe(
      "npm test",
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "http://terminal.test/chat",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("binds the default fetch implementation for non-streaming requests", async () => {
    const fetchMock = installHostSensitiveFetch();
    const transport = new SseAITransport({
      url: "http://terminal.test/chat",
    });

    await expect(transport.request("generate command")).resolves.toBe(
      "npm test",
    );
    expect(fetchMock).toHaveBeenCalledOnce();
  });

  it("passes a bound default fetch implementation to SSE requests", async () => {
    const fetchMock = installHostSensitiveFetch();
    sseHarness.fetchEventSource.mockImplementation(
      async (_url: string, options: Record<string, unknown>) => {
        const fetchImplementation = options.fetch as typeof fetch;
        await fetchImplementation("http://terminal.test/chat");

        const onmessage = options.onmessage as (event: { data: string }) => void;
        onmessage({ data: JSON.stringify(message("TEXT_MESSAGE_END", "")) });
      },
    );
    const transport = new SseAITransport({
      url: "http://terminal.test/chat",
    });

    await expect(
      transport.stream("question", { onChunk: vi.fn() }),
    ).resolves.toEqual({ requestId: "request-1" });
    expect(fetchMock).toHaveBeenCalledOnce();
  });

  it("rejects gateway error messages", async () => {
    sseHarness.fetchEventSource.mockImplementation(
      async (_url: string, options: Record<string, unknown>) => {
        const onmessage = options.onmessage as (event: { data: string }) => void;
        onmessage({
          data: JSON.stringify(message("TEXT_MESSAGE_ERROR", "model failed")),
        });
      },
    );
    const transport = new SseAITransport({
      url: "http://terminal.test/chat",
    });

    await expect(
      transport.stream("question", { onChunk: vi.fn() }),
    ).rejects.toThrow("model failed");
  });

  it("aborts active streams when disposed", async () => {
    let requestSignal: AbortSignal | undefined;
    sseHarness.fetchEventSource.mockImplementation(
      async (_url: string, options: Record<string, unknown>) => {
        requestSignal = options.signal as AbortSignal;
        await new Promise((_, reject) => {
          requestSignal?.addEventListener("abort", () => {
            reject(requestSignal?.reason);
          });
        });
      },
    );
    const transport = new SseAITransport({
      url: "http://terminal.test/chat",
    });
    const stream = transport.stream("question", { onChunk: vi.fn() });
    await Promise.resolve();

    transport.dispose();

    expect(requestSignal?.aborted).toBe(true);
    await expect(stream).rejects.toMatchObject({ name: "AbortError" });
  });
});

function message(type: string, delta: string) {
  return {
    delta,
    messageId: "request-1",
    timestamp: 1,
    type,
  };
}

function installHostSensitiveFetch() {
  const fetchMock = vi.fn(function (this: unknown) {
    if (this !== globalThis) {
      throw new TypeError("Illegal invocation");
    }

    return Promise.resolve({
      json: async () => message("TEXT_MESSAGE_SUGGESTION", "npm test"),
      ok: true,
      status: 200,
    } as Response);
  });
  vi.stubGlobal("fetch", fetchMock);

  return fetchMock;
}
