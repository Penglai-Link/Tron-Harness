import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  ChatSendResult,
  ChatSession,
} from "@tron/chat-core";

import { createOpenApiChatAdapter } from "./index";

const originalFetch = globalThis.fetch;

function successEnvelope(data: unknown) {
  return {
    code: "SUCCESS",
    data,
    message: "ok",
  };
}

function sessionFixture(overrides: Partial<ChatSession> = {}): ChatSession {
  return {
    createdAt: "2026-06-11T00:00:00Z",
    id: 1,
    sessionCode: "SESSION-1",
    status: "ACTIVE",
    title: "Open session",
    updatedAt: "2026-06-11T00:00:00Z",
    workspaceCode: "TRON_CORE",
    ...overrides,
  };
}

afterEach(() => {
  vi.restoreAllMocks();
  globalThis.fetch = originalFetch;
});

describe("createOpenApiChatAdapter", () => {
  it("signs JSON requests with HMAC headers and user-code semantics", async () => {
    const fetchMock = vi.fn(async () => new Response(JSON.stringify(successEnvelope(sessionFixture())), {
      headers: { "Content-Type": "application/json" },
      status: 200,
    }));
    globalThis.fetch = fetchMock as typeof fetch;

    const adapter = createOpenApiChatAdapter({
      openAppCode: "OPENAPP-TEST-0001",
      appKey: "secret-key-123",
      baseUrl: "https://openapi.example.com",
      nonceFactory: () => "abc123nonce789",
      timestampFactory: () => 1718100000,
      userCode: "USR-0001",
    });

    await adapter.createSession({
      title: "Open session",
      workspaceCode: "TRON_CORE",
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    const headers = new Headers(init?.headers);

    expect(url).toBe("https://openapi.example.com/open-api/v1/chats/sessions");
    expect(headers.get("Authorization")).toBeNull();
    expect(headers.get("X-Tron-User-Id")).toBeNull();
    expect(headers.get("X-Tron-App-Code")).toBe("OPENAPP-TEST-0001");
    expect(headers.get("X-Tron-User-Code")).toBe("USR-0001");
    expect(headers.get("X-Tron-Signature-Version")).toBe("TRON-HMAC-SHA256-V1");
    expect(headers.get("X-Tron-Timestamp")).toBe("1718100000");
    expect(headers.get("X-Tron-Nonce")).toBe("abc123nonce789");
    expect(headers.get("X-Tron-Signature")).toMatch(/^[a-f0-9]{64}$/);
  });

  it("signs stream requests before opening the SSE channel", async () => {
    const streamPayload = {
      type: "RUN_FINISHED",
      result: {
        messages: [],
        run: {
          finalOutput: "done",
          runCode: "RUN-1",
          status: "COMPLETED",
          stepCount: 1,
          workspaceCode: "TRON_CORE",
        },
        session: sessionFixture(),
      } satisfies ChatSendResult,
    };
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(`data: ${JSON.stringify(streamPayload)}\n\n`));
        controller.close();
      },
    });
    const fetchMock = vi.fn(async () => new Response(stream, {
      headers: { "Content-Type": "text/event-stream" },
      status: 200,
    }));
    globalThis.fetch = fetchMock as typeof fetch;

    const adapter = createOpenApiChatAdapter({
      openAppCode: "OPENAPP-TEST-0001",
      appKey: "secret-key-123",
      baseUrl: "https://openapi.example.com",
      nonceFactory: () => "streamnonce12",
      timestampFactory: () => 1718100001,
      userCode: "USR-0001",
    });

    await adapter.streamMessage(
      sessionFixture(),
      { content: "hello", workspaceCode: "TRON_CORE" },
    );

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    const headers = new Headers(init?.headers);

    expect(url).toBe("https://openapi.example.com/open-api/v1/chats/sessions/SESSION-1/messages/stream");
    expect(headers.get("Authorization")).toBeNull();
    expect(headers.get("X-Tron-User-Id")).toBeNull();
    expect(headers.get("X-Tron-User-Code")).toBe("USR-0001");
    expect(headers.get("X-Tron-Signature")).toMatch(/^[a-f0-9]{64}$/);
  });
});
